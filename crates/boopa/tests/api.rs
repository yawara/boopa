use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    http::StatusCode,
    test::{self, TestRequest},
};
use boopa::app_state::AppState;
use boopa::autoinstall::{default_password_hash, fingerprint_password_hash};
use boopa::config::{Config, DhcpConfig};
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
        dhcp: DhcpConfig::default(),
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
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["runtime"]["mode"], "disabled");
    assert_eq!(payload["runtime"]["activeLeaseCount"], 0);
    assert_eq!(payload["bios"]["architecture"], "x86 BIOS");
    assert_eq!(payload["uefi"]["architecture"], "x86_64 UEFI");
    assert!(
        payload["uefi"]["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .any(|note| note
                .as_str()
                .expect("note")
                .contains("iso-url=http://127.0.0.1:18080/boot/ubuntu/uefi/live-server.iso"))
    );
}

#[actix_web::test]
async fn ubuntu_autoinstall_endpoint_returns_default_config_and_yaml() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/api/autoinstall/ubuntu")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("\"hostname\":\"boopa-ubuntu\""));
    assert!(payload.contains("\"hasPassword\":true"));
    assert!(payload.contains("#cloud-config"));
}

#[actix_web::test]
async fn ubuntu_autoinstall_endpoint_persists_config_and_reuses_password_hash_when_blank() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config.clone()).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let update_payload = serde_json::json!({
        "hostname": "lab-node",
        "username": "ubuntu",
        "password": "correcthorsebattery",
        "locale": "ja_JP.UTF-8",
        "keyboardLayout": "jp",
        "timezone": "Asia/Tokyo",
        "storageLayout": "lvm",
        "installOpenSsh": true,
        "allowPasswordAuth": false,
        "authorizedKeys": ["ssh-ed25519 AAAA test@example"],
        "packages": ["curl", "git"]
    });

    let response = test::call_service(
        &app,
        TestRequest::put()
            .uri("/api/autoinstall/ubuntu")
            .insert_header(("content-type", "application/json"))
            .set_payload(update_payload.to_string())
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body()).await.expect("body");
    let payload: serde_json::Value = serde_json::from_slice(&body).expect("json");
    assert_eq!(payload["config"]["hostname"], "lab-node");
    assert_eq!(payload["config"]["storageLayout"], "lvm");
    assert_eq!(payload["hasPassword"], true);
    assert!(
        payload["renderedYaml"]
            .as_str()
            .expect("yaml")
            .contains("hostname: lab-node")
    );

    let persisted = tokio::fs::read(config.ubuntu_autoinstall_path())
        .await
        .expect("persisted file");
    let persisted_json: serde_json::Value = serde_json::from_slice(&persisted).expect("json");
    let first_hash = persisted_json["passwordHash"]
        .as_str()
        .expect("password hash")
        .to_string();
    assert_ne!(first_hash, default_password_hash());
    assert_ne!(
        fingerprint_password_hash(&first_hash),
        fingerprint_password_hash("correcthorsebattery")
    );

    let response = test::call_service(
        &app,
        TestRequest::put()
            .uri("/api/autoinstall/ubuntu")
            .insert_header(("content-type", "application/json"))
            .set_payload(
                serde_json::json!({
                    "hostname": "lab-node-2",
                    "username": "ubuntu",
                    "locale": "ja_JP.UTF-8",
                    "keyboardLayout": "jp",
                    "timezone": "Asia/Tokyo",
                    "storageLayout": "lvm",
                    "installOpenSsh": true,
                    "allowPasswordAuth": false,
                    "authorizedKeys": ["ssh-ed25519 AAAA test@example"],
                    "packages": ["curl", "git"]
                })
                .to_string(),
            )
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let persisted = tokio::fs::read(config.ubuntu_autoinstall_path())
        .await
        .expect("persisted file");
    let persisted_json: serde_json::Value = serde_json::from_slice(&persisted).expect("json");
    assert_eq!(persisted_json["hostname"], "lab-node-2");
    assert_eq!(persisted_json["passwordHash"], first_hash);
}

#[actix_web::test]
async fn ubuntu_autoinstall_endpoint_rejects_invalid_payload() {
    let (_temp_dir, config) = test_config();
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::put()
            .uri("/api/autoinstall/ubuntu")
            .insert_header(("content-type", "application/json"))
            .set_payload(
                serde_json::json!({
                    "hostname": "-bad",
                    "username": "BadUser",
                    "password": "short",
                    "locale": "",
                    "keyboardLayout": "",
                    "timezone": "",
                    "storageLayout": "direct",
                    "installOpenSsh": true,
                    "allowPasswordAuth": true,
                    "authorizedKeys": ["bad"],
                    "packages": []
                })
                .to_string(),
            )
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("\"fieldErrors\""));
    assert!(payload.contains("hostname"));
    assert!(payload.contains("username"));
    assert!(payload.contains("password"));
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
    hasher
        .finalize()
        .as_slice()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
