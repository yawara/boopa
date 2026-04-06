use std::{collections::HashMap, path::Path, sync::Arc};

use boot_recipe::{BootMode, DistroId, get_recipe};
use image_cache::{AssetDownloader, CacheStatus, DownloadFuture, ImageCache};
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct FakeDownloader {
    payloads: Arc<HashMap<String, Vec<u8>>>,
}

impl AssetDownloader for FakeDownloader {
    fn download(&self, url: &str, destination: &Path) -> DownloadFuture {
        let url = url.to_string();
        let payload = self.payloads.get(&url).cloned();
        let destination = destination.to_path_buf();
        Box::pin(async move {
            let payload = payload.ok_or_else(|| {
                image_cache::CacheError::from(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("missing fake payload for {url}"),
                ))
            })?;
            if let Some(parent) = destination.parent() {
                tokio::fs::create_dir_all(parent).await.expect("parent dir");
            }
            tokio::fs::write(&destination, &payload)
                .await
                .expect("write payload");
            Ok(sha256_hex(&payload))
        })
    }
}

#[tokio::test]
async fn refresh_distro_writes_expected_ubuntu_uefi_assets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let mut payloads = HashMap::new();
    for mode in [BootMode::Bios, BootMode::Uefi] {
        for asset in get_recipe(DistroId::Ubuntu, mode).expect("recipe").assets {
            payloads.insert(
                asset.source_url.clone(),
                format!("payload:{}", asset.relative_path).into_bytes(),
            );
        }
    }
    let cache = ImageCache::with_downloader(
        tempdir.path(),
        Arc::new(FakeDownloader {
            payloads: Arc::new(payloads),
        }),
    )
    .await
    .expect("cache");

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("refresh distro");

    let expected_assets = get_recipe(DistroId::Ubuntu, BootMode::Uefi)
        .expect("ubuntu uefi recipe")
        .assets;

    for asset in expected_assets {
        let entry = entries
            .iter()
            .find(|entry| entry.relative_path == asset.relative_path)
            .expect("entry for ubuntu uefi asset");
        assert_eq!(entry.boot_mode, BootMode::Uefi);
        assert_eq!(entry.status, CacheStatus::Refreshed);
        assert_eq!(entry.source_url, asset.source_url);

        let bytes = tokio::fs::read(&entry.local_path)
            .await
            .expect("payload bytes");
        assert!(
            !bytes.is_empty(),
            "refreshed asset should write bytes for {}",
            asset.relative_path
        );
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
