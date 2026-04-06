use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    http::StatusCode,
    test::{self, TestRequest},
};
use boopa::{app_state::AppState, config::Config, http};
use boot_recipe::DistroId;

#[actix_web::test]
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
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/bios/kernel")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]
async fn serves_generated_grub_config_for_ubuntu_uefi() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/grub.cfg")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("root=(tftp,10.0.2.2:16969)"));
    assert!(payload.contains("/ubuntu/uefi/kernel"));
    assert!(payload.contains("boot=casper"));
    assert!(payload.contains("iso-url=http://10.0.2.2:18080/boot/ubuntu/uefi/live-server.iso"));
}

#[actix_web::test]
async fn rejects_generated_grub_config_when_selected_distro_is_not_ubuntu() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
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
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/grub.cfg")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn serves_cached_live_server_iso_over_http() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache_dir = tempdir.path().join("data/cache/ubuntu/uefi");
    tokio::fs::create_dir_all(&cache_dir)
        .await
        .expect("cache dir");
    tokio::fs::write(cache_dir.join("live-server.iso"), b"iso-bytes")
        .await
        .expect("seed iso");

    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/live-server.iso")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    assert_eq!(body.as_ref(), b"iso-bytes");
}

#[actix_web::test]
async fn serves_built_frontend_index_for_spa_routes() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let frontend_dir = tempdir.path().join("frontend");
    tokio::fs::create_dir_all(&frontend_dir)
        .await
        .expect("frontend dir");
    tokio::fs::write(frontend_dir.join("index.html"), "<div>frontend-ready</div>")
        .await
        .expect("seed index");

    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: tempdir.path().join("data"),
        frontend_dir,
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response =
        test::call_service(&app, TestRequest::get().uri("/dashboard").to_request()).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("frontend-ready"));
}
