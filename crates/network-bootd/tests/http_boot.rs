use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use boopa::{app_state::AppState, config::Config, http::router};
use boot_recipe::DistroId;
use http_body_util::BodyExt;
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
        tftp_advertise_addr: ([127, 0, 0, 1], 6969).into(),
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

#[tokio::test]
async fn serves_generated_grub_config_for_ubuntu_uefi() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/boot/ubuntu/uefi/grub.cfg")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("root=(tftp,10.0.2.2:16969)"));
    assert!(payload.contains("/ubuntu/uefi/kernel"));
}

#[tokio::test]
async fn rejects_generated_grub_config_when_selected_distro_is_not_ubuntu() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    state
        .set_selected_distro(DistroId::Fedora)
        .await
        .expect("set distro");
    let app = router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/boot/ubuntu/uefi/grub.cfg")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
