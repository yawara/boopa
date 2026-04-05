use boot_recipe::DistroId;
use image_cache::{CacheStatus, ImageCache};

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
