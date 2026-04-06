use std::{
    future::Future,
    io::ErrorKind,
    path::Path,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use boot_recipe::{BootMode, DistroId, get_recipe};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, info};

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
    manifest_lock: Arc<Mutex<()>>,
}

pub type DownloadFuture = Pin<Box<dyn Future<Output = Result<String, CacheError>> + Send>>;

pub trait AssetDownloader: Send + Sync {
    fn download(&self, url: &str, destination: &Path) -> DownloadFuture;
}

#[derive(Debug, Clone, Default)]
struct ReqwestDownloader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sha256Command {
    Sha256sum,
    Shasum,
    Openssl,
}

impl AssetDownloader for ReqwestDownloader {
    fn download(&self, url: &str, destination: &Path) -> DownloadFuture {
        let url = url.to_owned();
        let destination = destination.to_path_buf();
        Box::pin(async move {
            let mut response = reqwest::get(url).await?.error_for_status()?;
            let tmp_path = destination.with_extension("download");
            let mut file = tokio::fs::File::create(&tmp_path).await?;
            let mut hasher = Sha256::new();

            while let Some(chunk) = response.chunk().await? {
                hasher.update(&chunk);
                file.write_all(&chunk).await?;
            }

            file.flush().await?;
            drop(file);
            tokio::fs::rename(&tmp_path, &destination).await?;

            Ok(sha256_hex(hasher.finalize().as_slice()))
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
        Ok(Self {
            root,
            downloader,
            manifest_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn refresh_distro(&self, distro: DistroId) -> Result<Vec<CacheEntry>, CacheError> {
        info!(%distro, cache_root = %self.root.display(), "refreshing distro cache");
        let _manifest_guard = self.manifest_lock.lock().await;
        let mut manifest = self.load_manifest().await?;
        let mut entries = Vec::new();

        for mode in [BootMode::Bios, BootMode::Uefi] {
            let recipe = get_recipe(distro, mode)?;
            for asset in recipe.assets {
                let path = self.resolve(&asset.relative_path);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                let modified = file_modified_unix(&path).await?;

                let (status, synced_at, sha256) = if modified.is_none() {
                    info!(
                        %distro,
                        ?mode,
                        logical_name = %asset.logical_name,
                        relative_path = %asset.relative_path,
                        local_path = %path.display(),
                        "cache miss; downloading asset"
                    );
                    self.download_and_store_asset(&path, &asset.source_url)
                        .await?
                } else {
                    let local_sha256 = sha256_file(&path).await?;
                    if let Some(entry) = manifest.entry_for(&asset.relative_path) {
                        if entry.sha256 == local_sha256 && entry.source_url == asset.source_url {
                            debug!(
                                %distro,
                                ?mode,
                                logical_name = %asset.logical_name,
                                relative_path = %asset.relative_path,
                                local_path = %path.display(),
                                "cache hit; manifest sha/source_url match"
                            );
                            (CacheStatus::Cached, entry.synced_at, local_sha256)
                        } else {
                            info!(
                                %distro,
                                ?mode,
                                logical_name = %asset.logical_name,
                                relative_path = %asset.relative_path,
                                local_path = %path.display(),
                                "cache stale; re-downloading asset"
                            );
                            self.download_and_store_asset(&path, &asset.source_url)
                                .await?
                        }
                    } else {
                        info!(
                            %distro,
                            ?mode,
                            logical_name = %asset.logical_name,
                            relative_path = %asset.relative_path,
                            local_path = %path.display(),
                            "cache file present without manifest entry; adopting local file"
                        );
                        (
                            CacheStatus::Cached,
                            modified.unwrap_or_else(now_unix),
                            local_sha256,
                        )
                    }
                };

                manifest.upsert(ManifestEntry {
                    relative_path: asset.relative_path.clone(),
                    source_url: asset.source_url.clone(),
                    sha256,
                    synced_at,
                });

                entries.push(CacheEntry {
                    distro_id: distro,
                    boot_mode: mode,
                    logical_name: asset.logical_name,
                    relative_path: asset.relative_path,
                    source_url: asset.source_url,
                    local_path: path.display().to_string(),
                    status,
                    last_synced_at: Some(synced_at),
                });
            }
        }

        self.save_manifest(&manifest).await?;
        info!(%distro, asset_count = entries.len(), "cache refresh completed");
        Ok(entries)
    }

    pub async fn status_for_distro(&self, distro: DistroId) -> Result<Vec<CacheEntry>, CacheError> {
        debug!(%distro, cache_root = %self.root.display(), "reading cache status");
        let manifest = self.load_manifest().await?;
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
                let last_synced_at = manifest
                    .entry_for(&asset.relative_path)
                    .map(|entry| entry.synced_at)
                    .or(modified);

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
                    last_synced_at,
                });
            }
        }

        debug!(%distro, asset_count = entries.len(), "cache status assembled");
        Ok(entries)
    }

    pub fn resolve(&self, relative_path: &str) -> PathBuf {
        self.root.join(relative_path.trim_start_matches('/'))
    }

    fn manifest_path(&self) -> PathBuf {
        self.root.join("manifest.json")
    }

    async fn load_manifest(&self) -> Result<CacheManifest, CacheError> {
        match tokio::fs::read(self.manifest_path()).await {
            Ok(payload) => Ok(serde_json::from_slice(&payload)?),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(CacheManifest::default()),
            Err(error) => Err(error.into()),
        }
    }

    async fn save_manifest(&self, manifest: &CacheManifest) -> Result<(), CacheError> {
        let path = self.manifest_path();
        let payload = serde_json::to_vec_pretty(manifest)?;
        let tmp_path = path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, payload).await?;
        tokio::fs::rename(tmp_path, path).await?;
        Ok(())
    }

    async fn download_and_store_asset(
        &self,
        path: &Path,
        source_url: &str,
    ) -> Result<(CacheStatus, u64, String), CacheError> {
        info!(
            source_url = %source_url,
            destination = %path.display(),
            "downloading cache asset"
        );
        let sha256 = self.downloader.download(source_url, path).await?;
        info!(
            destination = %path.display(),
            sha256 = %sha256,
            "cache asset download completed"
        );
        Ok((CacheStatus::Refreshed, now_unix(), sha256))
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time after unix epoch")
        .as_secs()
}

async fn sha256_file(path: &Path) -> Result<String, CacheError> {
    let mut last_error = None;

    for command in [
        Sha256Command::Sha256sum,
        Sha256Command::Shasum,
        Sha256Command::Openssl,
    ] {
        debug!(command = %command.name(), path = %path.display(), "trying external sha256 command");
        match run_sha256_command(path, command).await {
            Ok(digest) => {
                debug!(command = %command.name(), path = %path.display(), "external sha256 command succeeded");
                return Ok(digest);
            }
            Err(CacheError::Io(error)) if error.kind() == ErrorKind::NotFound => {
                debug!(command = %command.name(), path = %path.display(), "sha256 command unavailable");
                last_error = Some(CacheError::Io(error));
            }
            Err(error) => {
                debug!(command = %command.name(), path = %path.display(), ?error, "sha256 command failed");
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| {
        CacheError::Sha256Command("no usable sha256 command found in PATH".to_string())
    }))
}

async fn run_sha256_command(path: &Path, command: Sha256Command) -> Result<String, CacheError> {
    let output = match command {
        Sha256Command::Sha256sum => Command::new("sha256sum").arg(path).output().await?,
        Sha256Command::Shasum => {
            Command::new("shasum")
                .args(["-a", "256"])
                .arg(path)
                .output()
                .await?
        }
        Sha256Command::Openssl => {
            Command::new("openssl")
                .args(["dgst", "-sha256", "-r"])
                .arg(path)
                .output()
                .await?
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(CacheError::Sha256Command(format!(
            "{} exited with {}: {}",
            command.name(),
            output.status,
            stderr
        )));
    }

    parse_sha256_stdout(&output.stdout, command)
}

fn parse_sha256_stdout(stdout: &[u8], command: Sha256Command) -> Result<String, CacheError> {
    let output = String::from_utf8_lossy(stdout);
    let token = output.split_whitespace().next().ok_or_else(|| {
        CacheError::Sha256Command(format!("{} returned empty stdout", command.name()))
    })?;

    if token.len() != 64 || !token.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err(CacheError::Sha256Command(format!(
            "{} returned unexpected digest format: {}",
            command.name(),
            output.trim()
        )));
    }

    Ok(token.to_ascii_lowercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

async fn file_modified_unix(path: &Path) -> Result<Option<u64>, CacheError> {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };

    Ok(metadata
        .modified()
        .ok()
        .and_then(|timestamp| timestamp.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs()))
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CacheManifest {
    entries: Vec<ManifestEntry>,
}

impl CacheManifest {
    fn entry_for(&self, relative_path: &str) -> Option<&ManifestEntry> {
        self.entries
            .iter()
            .find(|entry| entry.relative_path == relative_path)
    }

    fn upsert(&mut self, manifest_entry: ManifestEntry) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.relative_path == manifest_entry.relative_path)
        {
            *entry = manifest_entry;
        } else {
            self.entries.push(manifest_entry);
            self.entries
                .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestEntry {
    relative_path: String,
    source_url: String,
    sha256: String,
    synced_at: u64,
}

#[derive(Debug, Error)]
pub enum CacheError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Recipe(#[from] boot_recipe::RecipeError),
    #[error(transparent)]
    Download(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("sha256 command failed: {0}")]
    Sha256Command(String),
}

impl Sha256Command {
    fn name(self) -> &'static str {
        match self {
            Self::Sha256sum => "sha256sum",
            Self::Shasum => "shasum",
            Self::Openssl => "openssl",
        }
    }
}
