use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use boopa::{app_state::AppState, config::Config, http::router};
use tower::ServiceExt;

#[tokio::test]
async fn serves_boot_asset_when_cached() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache_dir = tempdir.path().join("data/cache/ubuntu/bios");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .expect("cache dir");
    tokio::fs::write(cache_dir.join("kernel"), b"kernel-bytes")
        .await
        .expect("seed asset");

    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/boot/ubuntu/bios/kernel")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}
