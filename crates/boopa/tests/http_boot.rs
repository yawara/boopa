use std::sync::Arc;

use actix_web::{
    App,
    body::to_bytes,
    http::StatusCode,
    test::{self, TestRequest},
};
use boopa::{
    app_state::AppState,
    config::{Config, DhcpConfig},
    http,
};
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
        dhcp: DhcpConfig::default(),
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
        dhcp: DhcpConfig::default(),
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
    assert!(payload.contains("autoinstall"));
    assert!(
        payload.contains("ds=nocloud-net;s=http://10.0.2.2:18080/boot/ubuntu/uefi/autoinstall/")
    );
}

#[actix_web::test]
async fn rejects_generated_grub_config_when_selected_distro_is_not_ubuntu() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        dhcp: DhcpConfig::default(),
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
        dhcp: DhcpConfig::default(),
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
async fn serves_generated_ubuntu_autoinstall_seed_files_over_http() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        dhcp: DhcpConfig::default(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let user_data = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/autoinstall/user-data")
            .to_request(),
    )
    .await;
    assert_eq!(user_data.status(), StatusCode::OK);
    let user_data_body = to_bytes(user_data.into_body()).await.expect("body");
    let user_data_payload = String::from_utf8(user_data_body.to_vec()).expect("utf8");
    assert!(user_data_payload.contains("#cloud-config"));
    assert!(user_data_payload.contains("hostname: boopa-ubuntu"));

    let meta_data = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/autoinstall/meta-data")
            .to_request(),
    )
    .await;
    assert_eq!(meta_data.status(), StatusCode::OK);
    let meta_data_body = to_bytes(meta_data.into_body()).await.expect("body");
    let meta_data_payload = String::from_utf8(meta_data_body.to_vec()).expect("utf8");
    assert!(meta_data_payload.contains("instance-id:"));
    assert!(meta_data_payload.contains("local-hostname: boopa-ubuntu"));
}

#[actix_web::test]
async fn generated_ubuntu_autoinstall_seed_reflects_saved_config() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        dhcp: DhcpConfig::default(),
        data_dir: tempdir.path().join("data"),
        frontend_dir: tempdir.path().join("frontend"),
    };
    let state = Arc::new(AppState::new(config).await.expect("state"));
    state
        .update_ubuntu_autoinstall(
            serde_json::from_value(serde_json::json!({
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
            }))
            .expect("update payload"),
        )
        .await
        .expect("update");
    let app =
        test::init_service(App::new().configure(|cfg| http::configure(cfg, state.clone()))).await;

    let response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/ubuntu/uefi/autoinstall/user-data")
            .to_request(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body()).await.expect("body");
    let payload = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(payload.contains("hostname: lab-node"));
    assert!(payload.contains("layout: jp"));
    assert!(payload.contains("timezone: Asia/Tokyo"));
    assert!(payload.contains("- curl"));
    assert!(payload.contains("- git"));
}

#[actix_web::test]
async fn serves_generated_fedora_kickstart_and_grub_over_http() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let config = Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        dhcp: DhcpConfig::default(),
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

    let grub_response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/fedora/uefi/grub.cfg")
            .to_request(),
    )
    .await;
    assert_eq!(grub_response.status(), StatusCode::OK);
    let grub_body = to_bytes(grub_response.into_body()).await.expect("body");
    let grub_payload = String::from_utf8(grub_body.to_vec()).expect("utf8");
    assert!(grub_payload.contains("linuxefi /fedora/uefi/kernel"));
    assert!(
        grub_payload.contains("inst.ks=http://10.0.2.2:18080/boot/fedora/uefi/kickstart/ks.cfg")
    );

    let kickstart_response = test::call_service(
        &app,
        TestRequest::get()
            .uri("/boot/fedora/uefi/kickstart/ks.cfg")
            .to_request(),
    )
    .await;
    assert_eq!(kickstart_response.status(), StatusCode::OK);
    let kickstart_body = to_bytes(kickstart_response.into_body())
        .await
        .expect("body");
    let kickstart_payload = String::from_utf8(kickstart_body.to_vec()).expect("utf8");
    assert!(kickstart_payload.contains("lang en_US.UTF-8"));
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
        dhcp: DhcpConfig::default(),
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
