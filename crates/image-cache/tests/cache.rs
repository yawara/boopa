use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use boot_recipe::{BootMode, DistroId, get_recipe};
use image_cache::{AssetDownloader, CacheStatus, DownloadFuture, ImageCache};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Clone)]
struct FakeDownloader {
    payloads: Arc<HashMap<String, Vec<u8>>>,
    calls: Arc<Mutex<Vec<String>>>,
}

impl FakeDownloader {
    fn new(payloads: HashMap<String, Vec<u8>>) -> Self {
        Self {
            payloads: Arc::new(payloads),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.lock().expect("calls lock").len()
    }
}

impl AssetDownloader for FakeDownloader {
    fn download(&self, url: &str, destination: &Path) -> DownloadFuture {
        let url = url.to_string();
        let payload = self.payloads.get(&url).cloned();
        let calls = self.calls.clone();
        let destination = destination.to_path_buf();
        Box::pin(async move {
            calls.lock().expect("calls lock").push(url.clone());
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
                .expect("write fake payload");
            Ok(sha256_hex(&payload))
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
    let downloader = FakeDownloader::new(payloads);

    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
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

    let manifest: Value = serde_json::from_slice(
        &tokio::fs::read(tempdir.path().join("manifest.json"))
            .await
            .expect("manifest"),
    )
    .expect("manifest json");
    let manifest_entries = manifest["entries"].as_array().expect("manifest entries");
    assert!(!manifest_entries.is_empty());
    assert_eq!(
        downloader.call_count(),
        entries.len(),
        "initial refresh should fetch every asset once"
    );
}

#[tokio::test]
async fn refresh_skips_assets_when_manifest_and_file_hash_match() {
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
    let downloader = FakeDownloader::new(payloads);

    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
        .await
        .expect("cache");

    cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("initial refresh");
    let initial_call_count = downloader.call_count();

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("second refresh");

    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.status, CacheStatus::Cached))
    );
    assert_eq!(
        downloader.call_count(),
        initial_call_count,
        "matching manifest+file should not trigger extra downloads"
    );
}

#[tokio::test]
async fn refresh_rebuilds_manifest_from_existing_files_without_redownloading() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let mut payloads = HashMap::new();
    for mode in [BootMode::Bios, BootMode::Uefi] {
        for asset in get_recipe(DistroId::Ubuntu, mode).expect("recipe").assets {
            let path = tempdir.path().join(&asset.relative_path);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.expect("parent");
            }
            let bytes = format!("payload:{}", asset.relative_path).into_bytes();
            tokio::fs::write(&path, &bytes).await.expect("seed file");
            payloads.insert(asset.source_url.clone(), bytes);
        }
    }

    let downloader = FakeDownloader::new(payloads);
    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
        .await
        .expect("cache");

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("refresh");

    assert!(
        entries
            .iter()
            .all(|entry| matches!(entry.status, CacheStatus::Cached))
    );
    assert_eq!(downloader.call_count(), 0);
    assert!(
        tokio::fs::try_exists(tempdir.path().join("manifest.json"))
            .await
            .expect("manifest exists")
    );
}

#[tokio::test]
async fn refresh_redownloads_missing_asset_even_when_manifest_exists() {
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
    let downloader = FakeDownloader::new(payloads);
    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
        .await
        .expect("cache");

    cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("initial refresh");
    let missing_asset = tempdir.path().join("ubuntu/uefi/kernel");
    tokio::fs::remove_file(&missing_asset)
        .await
        .expect("remove cached asset");
    let initial_call_count = downloader.call_count();

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("second refresh");

    let kernel_entry = entries
        .iter()
        .find(|entry| entry.relative_path == "ubuntu/uefi/kernel")
        .expect("kernel entry");
    assert_eq!(kernel_entry.status, CacheStatus::Refreshed);
    assert_eq!(downloader.call_count(), initial_call_count + 1);
}

#[tokio::test]
async fn refresh_redownloads_asset_when_manifest_hash_mismatches() {
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
    let downloader = FakeDownloader::new(payloads);
    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
        .await
        .expect("cache");

    cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("initial refresh");
    tokio::fs::write(tempdir.path().join("ubuntu/uefi/kernel"), b"tampered")
        .await
        .expect("tamper asset");
    let initial_call_count = downloader.call_count();

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("second refresh");

    let kernel_entry = entries
        .iter()
        .find(|entry| entry.relative_path == "ubuntu/uefi/kernel")
        .expect("kernel entry");
    assert_eq!(kernel_entry.status, CacheStatus::Refreshed);
    assert_eq!(downloader.call_count(), initial_call_count + 1);
    assert_eq!(
        tokio::fs::read(tempdir.path().join("ubuntu/uefi/kernel"))
            .await
            .expect("kernel bytes"),
        b"payload:ubuntu/uefi/kernel"
    );
}

#[tokio::test]
async fn refresh_redownloads_asset_when_source_url_changes() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let mut payloads = HashMap::new();
    let old_url = "https://example.test/ubuntu/uefi/kernel-old";
    let new_url = get_recipe(DistroId::Ubuntu, BootMode::Uefi)
        .expect("recipe")
        .assets
        .into_iter()
        .find(|asset| asset.relative_path == "ubuntu/uefi/kernel")
        .expect("kernel asset")
        .source_url;

    for mode in [BootMode::Bios, BootMode::Uefi] {
        for asset in get_recipe(DistroId::Ubuntu, mode).expect("recipe").assets {
            payloads.insert(
                asset.source_url.clone(),
                format!("payload:{}", asset.relative_path).into_bytes(),
            );
        }
    }

    let downloader = FakeDownloader::new(payloads);
    let cache = ImageCache::with_downloader(tempdir.path(), Arc::new(downloader.clone()))
        .await
        .expect("cache");

    tokio::fs::create_dir_all(tempdir.path().join("ubuntu/uefi"))
        .await
        .expect("uefi dir");
    tokio::fs::write(
        tempdir.path().join("ubuntu/uefi/kernel"),
        b"payload:ubuntu/uefi/kernel",
    )
    .await
    .expect("seed kernel");
    let seeded_kernel = b"payload:ubuntu/uefi/kernel";
    tokio::fs::write(
        tempdir.path().join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "entries": [{
                "relativePath": "ubuntu/uefi/kernel",
                "sourceUrl": old_url,
                "sha256": sha256_hex(seeded_kernel),
                "syncedAt": 1,
            }]
        }))
        .expect("manifest json"),
    )
    .await
    .expect("seed manifest");

    let entries = cache
        .refresh_distro(DistroId::Ubuntu)
        .await
        .expect("refresh");

    let kernel_entry = entries
        .iter()
        .find(|entry| entry.relative_path == "ubuntu/uefi/kernel")
        .expect("kernel entry");
    assert_eq!(kernel_entry.status, CacheStatus::Refreshed);
    assert!(
        downloader
            .calls
            .lock()
            .expect("calls lock")
            .iter()
            .any(|url| url == &new_url)
    );
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .as_slice()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
