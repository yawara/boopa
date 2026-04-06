use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    http::StatusCode,
    test::{self, TestRequest},
};
use boopa::{app_state::AppState, config::Config, http, tftp::resolve_request};
use boot_recipe::DistroId;
use tempfile::TempDir;

fn test_config(temp_dir: &TempDir) -> Config {
    Config {
        api_bind: ([127, 0, 0, 1], 0).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        data_dir: temp_dir.path().join("data"),
        frontend_dir: temp_dir.path().join("frontend"),
    }
}

async fn seed_ubuntu_uefi_assets(temp_dir: &TempDir) -> Vec<(&'static str, &'static [u8])> {
    let assets = vec![
        ("ubuntu/uefi/grubx64.efi", b"grubx64-efi-bytes".as_slice()),
        ("ubuntu/uefi/kernel", b"kernel-bytes".as_slice()),
        ("ubuntu/uefi/initrd", b"initrd-bytes".as_slice()),
    ];

    for (relative_path, bytes) in &assets {
        let path = temp_dir.path().join("data/cache").join(relative_path);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.expect("cache dir");
        }
        tokio::fs::write(path, bytes).await.expect("seed asset");
    }

    assets
}

async fn build_state(temp_dir: &TempDir) -> Arc<AppState> {
    tokio::fs::create_dir_all(temp_dir.path().join("frontend"))
        .await
        .expect("frontend dir");

    Arc::new(AppState::new(test_config(temp_dir)).await.expect("state"))
}

#[actix_web::test]
async fn http_serves_seeded_ubuntu_uefi_static_assets() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let assets = seed_ubuntu_uefi_assets(&temp_dir).await;
    let state = build_state(&temp_dir).await;
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    for (relative_path, expected_bytes) in assets {
        let response = test::call_service(
            &app,
            TestRequest::get()
                .uri(&format!("/boot/{relative_path}"))
                .to_request(),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK, "path: {relative_path}");
        let body = to_bytes(response.into_body()).await.expect("body");
        assert_eq!(body.as_ref(), expected_bytes, "path: {relative_path}");
    }
}

#[actix_web::test]
async fn http_rejects_ubuntu_uefi_assets_when_selected_distro_changes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    seed_ubuntu_uefi_assets(&temp_dir).await;
    let state = build_state(&temp_dir).await;
    state
        .set_selected_distro(DistroId::Fedora)
        .await
        .expect("set selected distro");
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/grubx64.efi")
            .to_request(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn tftp_resolves_seeded_ubuntu_uefi_static_assets() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let assets = seed_ubuntu_uefi_assets(&temp_dir).await;
    let state = build_state(&temp_dir).await;

    for (relative_path, _) in assets {
        let resolution = resolve_request(state.clone(), relative_path).await;
        let resolution = resolution.expect("resolution");
        assert_eq!(resolution.requested_path, relative_path);
        assert_eq!(resolution.served_path, relative_path);
        assert!(!resolution.generated);
    }
}

#[tokio::test]
async fn tftp_rejects_ubuntu_uefi_assets_when_selected_distro_changes() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    seed_ubuntu_uefi_assets(&temp_dir).await;
    let state = build_state(&temp_dir).await;
    state
        .set_selected_distro(DistroId::Fedora)
        .await
        .expect("set selected distro");

    let resolution = resolve_request(state, "ubuntu/uefi/kernel").await;
    assert!(resolution.is_none());
}
