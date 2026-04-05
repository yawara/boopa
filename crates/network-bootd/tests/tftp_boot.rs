use std::sync::Arc;

use boopa::{app_state::AppState, config::Config, tftp::resolve_request};

#[tokio::test]
async fn resolves_tftp_request_for_cached_asset() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache_dir = tempdir.path().join("data/cache/ubuntu/bios");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .expect("cache dir");
    tokio::fs::write(cache_dir.join("kernel"), b"kernel-bytes")
        .await
        .expect("seed asset");

    let state = Arc::new(
        AppState::new(Config {
            api_bind: ([127, 0, 0, 1], 0).into(),
            tftp_bind: ([127, 0, 0, 1], 0).into(),
            data_dir: tempdir.path().join("data"),
            frontend_dir: tempdir.path().join("frontend"),
        })
        .await
        .expect("state"),
    );

    let resolution = resolve_request(state, "ubuntu/bios/kernel").await;
    assert!(resolution.is_some());
}
