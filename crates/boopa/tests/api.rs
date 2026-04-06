use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    http::StatusCode,
    test::{self, TestRequest},
};
use boopa::app_state::AppState;
use boopa::config::Config;
use boopa::http;
use boot_recipe::DistroId;
use tempfile::tempdir;

fn test_config() -> (tempfile::TempDir, Config) {
    let temp_dir = tempdir().expect("temp dir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([127, 0, 0, 1], 6969).into(),
        data_dir: temp_dir.path().join("data"),
        frontend_dir: temp_dir.path().join("frontend"),
    };
    (temp_dir, config)
}

#[actix_web::test]
async fn health_endpoint_returns_ok() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response =
        test::call_service(&app, TestRequest::get().uri("/api/health").to_request()).await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[actix_web::test]
async fn frontend_fallback_returns_placeholder_html_when_assets_are_missing() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response =
        test::call_service(&app, TestRequest::get().uri("/dashboard").to_request()).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("frontend assets not built yet"));
}

#[actix_web::test]
async fn selection_persists_across_restart() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config.clone()).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::put()
            .uri("/api/selection")
            .insert_header(("content-type", "application/json"))
            .set_payload(r#"{"distro":"fedora"}"#)
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    let restarted = AppState::new(config).await.expect("restarted state");
    assert_eq!(restarted.selected_distro().await, DistroId::Fedora);
}

#[actix_web::test]
async fn dhcp_endpoint_returns_both_boot_modes_by_default() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(&app, TestRequest::get().uri("/api/dhcp").to_request()).await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("bios"));
    assert!(payload.contains("uefi"));
}
