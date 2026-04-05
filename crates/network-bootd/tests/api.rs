use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use boopa::app_state::AppState;
use boopa::config::Config;
use boopa::http::router;
use boot_recipe::DistroId;
use http_body_util::BodyExt;
use tempfile::tempdir;
use tower::ServiceExt;

fn test_config() -> (tempfile::TempDir, Config) {
    let temp_dir = tempdir().expect("temp dir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        data_dir: temp_dir.path().join("data"),
        frontend_dir: temp_dir.path().join("frontend"),
    };
    (temp_dir, config)
}

#[tokio::test]
async fn health_endpoint_returns_ok() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn selection_persists_across_restart() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config.clone()).await.expect("state"));
    let app = router(state.clone());

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/selection")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"distro":"fedora"}"#))
                .unwrap(),
        )
        .await
        .expect("selection response");

    assert_eq!(response.status(), StatusCode::OK);

    let restarted = AppState::new(config).await.expect("restarted state");
    assert_eq!(restarted.selected_distro().await, DistroId::Fedora);
}

#[tokio::test]
async fn dhcp_endpoint_returns_both_boot_modes_by_default() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/dhcp")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("dhcp response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("bios"));
    assert!(payload.contains("uefi"));
}
