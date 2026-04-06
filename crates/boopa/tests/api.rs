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
use boot_recipe::{BootMode, DistroId, get_recipe};
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn test_config() -> (tempfile::TempDir, Config) {
    let temp_dir = tempdir().expect("temp dir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
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
    assert!(payload.contains("iso-url=http://127.0.0.1:18080/boot/ubuntu/uefi/live-server.iso"));
}

#[actix_web::test]
async fn refresh_cache_skips_redownload_when_manifest_matches_seeded_assets() {
    let (_temp_dir, config) = test_config();
    seed_cache_manifest(&config, DistroId::Ubuntu).await;

    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::post()
            .uri("/api/cache/refresh")
            .insert_header(("content-type", "application/json"))
            .set_payload(r#"{"distro":"ubuntu"}"#)
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains(r#""status":"cached""#));
    assert!(!payload.contains(r#""status":"refreshed""#));
}

async fn seed_cache_manifest(config: &Config, distro: DistroId) {
    let mut entries = Vec::new();

    for mode in [BootMode::Bios, BootMode::Uefi] {
        for asset in get_recipe(distro, mode).expect("recipe").assets {
            let path = config.cache_dir().join(&asset.relative_path);
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.expect("parent dir");
            }
            let payload = format!("payload:{}", asset.relative_path).into_bytes();
            tokio::fs::write(&path, &payload).await.expect("seed asset");

            entries.push(serde_json::json!({
                "relativePath": asset.relative_path,
                "sourceUrl": asset.source_url,
                "sha256": sha256_hex(&payload),
                "syncedAt": 1,
            }));
        }
    }

    tokio::fs::create_dir_all(config.cache_dir())
        .await
        .expect("cache dir");
    tokio::fs::write(
        config.cache_dir().join("manifest.json"),
        serde_json::to_vec_pretty(&serde_json::json!({ "entries": entries })).expect("manifest"),
    )
    .await
    .expect("manifest write");
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}
