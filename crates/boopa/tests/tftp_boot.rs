use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    test::{self, TestRequest},
};
use boopa::{app_state::AppState, config::Config, http, tftp::resolve_request};
use boot_recipe::DistroId;

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
            tftp_advertise_addr: ([127, 0, 0, 1], 6969).into(),
            data_dir: tempdir.path().join("data"),
            frontend_dir: tempdir.path().join("frontend"),
        })
        .await
        .expect("state"),
    );

    let resolution = resolve_request(state, "ubuntu/bios/kernel").await;
    assert!(resolution.is_some());
}

#[tokio::test]
async fn resolves_tftp_request_for_generated_grub_alias() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let state = Arc::new(
        AppState::new(Config {
            api_bind: ([127, 0, 0, 1], 0).into(),
            tftp_bind: ([127, 0, 0, 1], 0).into(),
            tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
            data_dir: tempdir.path().join("data"),
            frontend_dir: tempdir.path().join("frontend"),
        })
        .await
        .expect("state"),
    );

    let resolution = resolve_request(state, "grub/grub.cfg")
        .await
        .expect("resolution");
    assert_eq!(resolution.served_path, "ubuntu/uefi/grub.cfg");
    assert!(resolution.generated);
}

#[actix_web::test]
async fn generated_grub_config_contains_ubuntu_iso_url() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let state = Arc::new(
        AppState::new(Config {
            api_bind: ([127, 0, 0, 1], 18080).into(),
            tftp_bind: ([127, 0, 0, 1], 0).into(),
            tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
            data_dir: tempdir.path().join("data"),
            frontend_dir: tempdir.path().join("frontend"),
        })
        .await
        .expect("state"),
    );
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/grub.cfg")
            .to_request(),
    )
    .await;

    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("boot=casper"));
    assert!(payload.contains("iso-url=http://10.0.2.2:18080/boot/ubuntu/uefi/live-server.iso"));
    assert!(payload.contains("autoinstall"));
    assert!(
        payload.contains("ds=nocloud-net;s=http://10.0.2.2:18080/boot/ubuntu/uefi/autoinstall/")
    );
}

#[tokio::test]
async fn generated_tftp_grub_aliases_follow_selected_fedora_distro() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let state = Arc::new(
        AppState::new(Config {
            api_bind: ([127, 0, 0, 1], 18080).into(),
            tftp_bind: ([127, 0, 0, 1], 0).into(),
            tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
            data_dir: tempdir.path().join("data"),
            frontend_dir: tempdir.path().join("frontend"),
        })
        .await
        .expect("state"),
    );
    state
        .set_selected_distro(DistroId::Fedora)
        .await
        .expect("set distro");

    let resolution = resolve_request(state.clone(), "grub/grub.cfg")
        .await
        .expect("resolution");
    assert_eq!(resolution.served_path, "fedora/uefi/grub.cfg");
    assert!(resolution.generated);

    let kickstart = resolve_request(state, "fedora/uefi/kickstart/ks.cfg").await;
    assert!(kickstart.is_none());
}
