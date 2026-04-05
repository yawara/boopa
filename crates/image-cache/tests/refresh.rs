use boot_recipe::{BootMode, DistroId, get_recipe};
use image_cache::{CacheStatus, ImageCache};

#[tokio::test]
async fn refresh_distro_writes_expected_ubuntu_uefi_assets() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache = ImageCache::new(tempdir.path()).await.expect("cache");

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
