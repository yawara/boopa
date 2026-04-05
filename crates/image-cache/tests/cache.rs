use std::{collections::HashMap, sync::Arc};

use boot_recipe::{BootMode, DistroId, get_recipe};
use image_cache::{AssetDownloader, CacheStatus, DownloadFuture, ImageCache};

#[derive(Clone)]
struct FakeDownloader {
    payloads: Arc<HashMap<String, Vec<u8>>>,
}

impl AssetDownloader for FakeDownloader {
    fn download(&self, url: &str) -> DownloadFuture {
        let url = url.to_string();
        let payload = self.payloads.get(&url).cloned();
        Box::pin(async move {
            payload.ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("missing fake payload for {url}"),
                )
                .into()
            })
        })
    }
}

#[tokio::test]
async fn reports_missing_status_for_unfetched_assets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache = ImageCache::new(tempdir.path()).await.expect("cache");

    let entries = cache
        .status_for_distro(DistroId::Ubuntu)
        .await
        .expect("status");

    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.status, CacheStatus::Missing))
    );
}

#[tokio::test]
async fn resolves_relative_paths_under_root() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache = ImageCache::new(tempdir.path()).await.expect("cache");

    assert_eq!(
        cache.resolve("ubuntu/bios/kernel"),
        tempdir.path().join("ubuntu/bios/kernel")
    );
}

#[tokio::test]
async fn refresh_writes_downloaded_payload_bytes() {
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
        .expect("refresh");

    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.status, CacheStatus::Refreshed))
    );
    assert_eq!(
        tokio::fs::read(tempdir.path().join("ubuntu/uefi/kernel"))
            .await
            .expect("kernel bytes"),
        b"payload:ubuntu/uefi/kernel"
    );
}
