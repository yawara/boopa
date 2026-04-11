use std::net::Ipv4Addr;
use std::sync::Arc;

use boopa::app_state::AppState;
use boopa::config::{Config, DhcpConfig, DhcpMode, DhcpSubnetConfig};
use boopa::dhcp::response_for_datagram;
use boot_recipe::DistroId;

fn authoritative_config(temp_dir: &tempfile::TempDir) -> Config {
    Config {
        api_bind: ([127, 0, 0, 1], 18080).into(),
        tftp_bind: ([127, 0, 0, 1], 0).into(),
        tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
        dhcp: DhcpConfig {
            mode: DhcpMode::Authoritative,
            bind: ([127, 0, 0, 1], 1067).into(),
            subnet: Some(DhcpSubnetConfig {
                subnet: "10.0.2.0/24".parse().expect("subnet"),
                pool_start: Ipv4Addr::new(10, 0, 2, 50),
                pool_end: Ipv4Addr::new(10, 0, 2, 99),
                router: Some(Ipv4Addr::new(10, 0, 2, 1)),
                dns_servers: vec![Ipv4Addr::new(10, 0, 2, 2)],
                lease_duration_secs: 3600,
                server_ip: Ipv4Addr::new(10, 0, 2, 2),
            }),
        },
        data_dir: temp_dir.path().join("data"),
        frontend_dir: temp_dir.path().join("frontend"),
    }
}

fn dhcp_packet(
    message_type: u8,
    mac: [u8; 6],
    architecture: u16,
    requested_ip: Option<Ipv4Addr>,
    server_identifier: Option<Ipv4Addr>,
) -> Vec<u8> {
    let mut payload = vec![0_u8; 240];
    payload[0] = 1;
    payload[1] = 1;
    payload[2] = 6;
    payload[4..8].copy_from_slice(&0x1234_5678_u32.to_be_bytes());
    payload[28..34].copy_from_slice(&mac);
    payload[236..240].copy_from_slice(&[99, 130, 83, 99]);
    push_option(&mut payload, 53, &[message_type]);
    push_option(&mut payload, 93, &architecture.to_be_bytes());
    push_option(
        &mut payload,
        60,
        if architecture == 0 {
            b"PXEClient:Arch:00000:UNDI:002001"
        } else {
            b"PXEClient:Arch:00007:UNDI:003016"
        },
    );
    push_option(
        &mut payload,
        61,
        &[1, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]],
    );
    if let Some(requested_ip) = requested_ip {
        push_option(&mut payload, 50, &requested_ip.octets());
    }
    if let Some(server_identifier) = server_identifier {
        push_option(&mut payload, 54, &server_identifier.octets());
    }
    payload.push(255);
    payload
}

fn push_option(target: &mut Vec<u8>, code: u8, payload: &[u8]) {
    target.push(code);
    target.push(payload.len() as u8);
    target.extend_from_slice(payload);
}

fn reply_yiaddr(payload: &[u8]) -> Ipv4Addr {
    Ipv4Addr::new(payload[16], payload[17], payload[18], payload[19])
}

fn reply_file(payload: &[u8]) -> String {
    let file = &payload[108..236];
    let end = file
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(file.len());
    String::from_utf8_lossy(&file[..end]).to_string()
}

fn option_u8(payload: &[u8], target: u8) -> Option<u8> {
    let mut cursor = 240;
    while cursor < payload.len() {
        let code = payload[cursor];
        cursor += 1;
        if code == 255 {
            break;
        }
        if code == 0 {
            continue;
        }
        let len = payload[cursor] as usize;
        cursor += 1;
        if code == target {
            return payload.get(cursor).copied();
        }
        cursor += len;
    }
    None
}

#[tokio::test]
async fn discover_and_request_allocate_dynamic_lease() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state = Arc::new(
        AppState::new(authoritative_config(&temp_dir))
            .await
            .expect("state"),
    );

    let discover = dhcp_packet(1, [0x52, 0x54, 0x00, 0x12, 0x34, 0x56], 0, None, None);
    let (offer, _) =
        response_for_datagram(state.as_ref(), &discover, ([127, 0, 0, 1], 1068).into())
            .await
            .expect("discover response")
            .expect("offer");
    assert_eq!(option_u8(&offer, 53), Some(2));
    assert_eq!(reply_yiaddr(&offer), Ipv4Addr::new(10, 0, 2, 50));
    assert_eq!(reply_file(&offer), "ubuntu/bios/lpxelinux.0");

    let request = dhcp_packet(
        3,
        [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
        0,
        Some(Ipv4Addr::new(10, 0, 2, 50)),
        Some(Ipv4Addr::new(10, 0, 2, 2)),
    );
    let (ack, _) = response_for_datagram(state.as_ref(), &request, ([127, 0, 0, 1], 1068).into())
        .await
        .expect("request response")
        .expect("ack");
    assert_eq!(option_u8(&ack, 53), Some(5));
    assert_eq!(reply_yiaddr(&ack), Ipv4Addr::new(10, 0, 2, 50));

    let status = state.dhcp_runtime_status().await;
    assert_eq!(status.mode, "authoritative");
    assert_eq!(status.active_lease_count, 1);
}

#[tokio::test]
async fn selected_distro_changes_future_bootfile() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let state = Arc::new(
        AppState::new(authoritative_config(&temp_dir))
            .await
            .expect("state"),
    );

    state
        .set_selected_distro(DistroId::Fedora)
        .await
        .expect("set distro");
    let discover = dhcp_packet(1, [0x52, 0x54, 0x00, 0xab, 0xcd, 0xef], 7, None, None);
    let (offer, _) =
        response_for_datagram(state.as_ref(), &discover, ([127, 0, 0, 1], 1068).into())
            .await
            .expect("discover response")
            .expect("offer");

    assert_eq!(reply_file(&offer), "fedora/uefi/shimx64.efi");
}

#[tokio::test]
async fn persisted_lease_is_reused_after_restart() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let config = authoritative_config(&temp_dir);
    let state = Arc::new(AppState::new(config.clone()).await.expect("state"));
    let discover = dhcp_packet(1, [0x52, 0x54, 0x00, 0x00, 0x00, 0x01], 0, None, None);

    let (offer, _) =
        response_for_datagram(state.as_ref(), &discover, ([127, 0, 0, 1], 1068).into())
            .await
            .expect("discover response")
            .expect("offer");
    assert_eq!(reply_yiaddr(&offer), Ipv4Addr::new(10, 0, 2, 50));
    drop(state);

    let restarted = Arc::new(AppState::new(config).await.expect("restarted state"));
    let (offer, _) =
        response_for_datagram(restarted.as_ref(), &discover, ([127, 0, 0, 1], 1068).into())
            .await
            .expect("discover response")
            .expect("offer");
    assert_eq!(reply_yiaddr(&offer), Ipv4Addr::new(10, 0, 2, 50));
    assert_eq!(restarted.dhcp_runtime_status().await.active_lease_count, 1);
}
