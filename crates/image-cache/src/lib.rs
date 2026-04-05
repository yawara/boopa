use std::{
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use boot_recipe::{BootMode, DistroId, get_recipe};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntry {
    pub distro_id: DistroId,
    pub boot_mode: BootMode,
    pub logical_name: String,
    pub relative_path: String,
    pub source_url: String,
    pub local_path: String,
    pub status: CacheStatus,
    pub last_synced_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CacheStatus {
    Missing,
    Cached,
    Refreshed,
}

#[derive(Clone)]
pub struct ImageCache {
    root: PathBuf,
    downloader: Arc<dyn AssetDownloader>,
}

pub type DownloadFuture = Pin<Box<dyn Future<Output = Result<Vec<u8>, CacheError>> + Send>>;

pub trait AssetDownloader: Send + Sync {
    fn download(&self, url: &str) -> DownloadFuture;
}

#[derive(Debug, Clone, Default)]
struct ReqwestDownloader;

impl AssetDownloader for ReqwestDownloader {
    fn download(&self, url: &str) -> DownloadFuture {
        let url = url.to_owned();
        Box::pin(async move {
            let response = reqwest::get(url).await?.error_for_status()?;
            Ok(response.bytes().await?.to_vec())
        })
    }
}

impl ImageCache {
    pub async fn new(root: impl Into<PathBuf>) -> Result<Self, CacheError> {
        Self::with_downloader(root, Arc::new(ReqwestDownloader)).await
    }

    pub async fn with_downloader(
        root: impl Into<PathBuf>,
        downloader: Arc<dyn AssetDownloader>,
    ) -> Result<Self, CacheError> {
        let root = root.into();
        tokio::fs::create_dir_all(&root).await?;
        Ok(Self { root, downloader })
    }

    pub async fn refresh_distro(&self, distro: DistroId) -> Result<Vec<CacheEntry>, CacheError> {
        let mut entries = Vec::new();

        for mode in [BootMode::Bios, BootMode::Uefi] {
            let recipe = get_recipe(distro, mode)?;
            for asset in recipe.assets {
                let path = self.resolve(&asset.relative_path);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                let payload = self.downloader.download(&asset.source_url).await?;
                tokio::fs::write(&path, payload).await?;

                entries.push(CacheEntry {
                    distro_id: distro,
                    boot_mode: mode,
                    logical_name: asset.logical_name,
                    relative_path: asset.relative_path,
                    source_url: asset.source_url,
                    local_path: path.display().to_string(),
                    status: CacheStatus::Refreshed,
                    last_synced_at: Some(now_unix()),
                });
            }
        }

        Ok(entries)
    }

    pub async fn status_for_distro(&self, distro: DistroId) -> Result<Vec<CacheEntry>, CacheError> {
        let mut entries = Vec::new();

        for mode in [BootMode::Bios, BootMode::Uefi] {
            let recipe = get_recipe(distro, mode)?;
            for asset in recipe.assets {
                let path = self.resolve(&asset.relative_path);
                let metadata = tokio::fs::metadata(&path).await.ok();
                let modified = metadata
                    .as_ref()
                    .and_then(|entry| entry.modified().ok())
                    .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs());

                entries.push(CacheEntry {
                    distro_id: distro,
                    boot_mode: mode,
                    logical_name: asset.logical_name,
                    relative_path: asset.relative_path,
                    source_url: asset.source_url,
                    local_path: path.display().to_string(),
                    status: if metadata.is_some() {
                        CacheStatus::Cached
                    } else {
                        CacheStatus::Missing
                    },
                    last_synced_at: modified,
                });
            }
        }

        Ok(entries)
    }

    pub fn resolve(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path.trim_start_matches('/'))
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_secs()
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Recipe(#[from] boot_recipe::RecipeError),
    #[error(transparent)]
    Download(#[from] reqwest::Error),
}
