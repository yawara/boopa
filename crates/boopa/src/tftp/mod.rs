use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_tftp::{
    packet,
    server::{Handler, TftpServer, TftpServerBuilder},
};
use futures_lite::io::{Cursor, Sink};

use crate::{app_state::AppState, boot_assets::BootAssetTransport};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TftpResolution {
    pub requested_path: String,
    pub served_path: String,
    pub generated: bool,
}

#[derive(Clone)]
struct BoopaTftpHandler {
    state: Arc<AppState>,
}

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    build_server(state).await?.serve().await?;
    Ok(())
}

pub async fn run_tftp_server(state: Arc<AppState>) -> anyhow::Result<()> {
    serve(state).await
}

pub async fn resolve_request(state: Arc<AppState>, requested_path: &str) -> Option<TftpResolution> {
    state
        .resolve_boot_asset(requested_path, BootAssetTransport::Tftp)
        .await
        .map(|asset| TftpResolution {
            requested_path: requested_path.to_string(),
            served_path: asset.logical_path().to_string(),
            generated: asset.is_generated(),
        })
}

pub fn resolve_path(root: &std::path::Path, relative_path: &str) -> PathBuf {
    root.join(relative_path.trim_start_matches('/'))
}

async fn build_server(state: Arc<AppState>) -> async_tftp::Result<TftpServer<BoopaTftpHandler>> {
    TftpServerBuilder::with_handler(BoopaTftpHandler {
        state: state.clone(),
    })
    .bind(state.config().tftp_bind)
    .build()
    .await
}

fn normalize_request_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

impl Handler for BoopaTftpHandler {
    type Reader = Cursor<Vec<u8>>;
    type Writer = Sink;

    async fn read_req_open(
        &mut self,
        client: &SocketAddr,
        path: &Path,
    ) -> Result<(Self::Reader, Option<u64>), packet::Error> {
        let requested_path = normalize_request_path(path);
        tracing::info!(%client, requested_path = %requested_path, "tftp rrq");

        let Some(asset) = self
            .state
            .resolve_boot_asset(&requested_path, BootAssetTransport::Tftp)
            .await
        else {
            tracing::warn!(%client, requested_path = %requested_path, "tftp file not found");
            return Err(packet::Error::FileNotFound);
        };

        let served_path = asset.logical_path().to_string();
        let bytes = asset.read_bytes().await.map_err(|error| {
            tracing::warn!(?error, %client, requested_path = %requested_path, served_path = %served_path, "tftp asset read failed");
            packet::Error::FileNotFound
        })?;
        let size = bytes.len() as u64;

        tracing::info!(%client, requested_path = %requested_path, served_path = %served_path, generated = asset.is_generated(), "tftp serving asset");

        Ok((Cursor::new(bytes), Some(size)))
    }

    async fn write_req_open(
        &mut self,
        client: &SocketAddr,
        path: &Path,
        _size: Option<u64>,
    ) -> Result<Self::Writer, packet::Error> {
        let requested_path = normalize_request_path(path);
        tracing::warn!(%client, requested_path = %requested_path, "tftp wrq rejected");
        Err(packet::Error::IllegalOperation)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::{Arc, OnceLock},
        time::Duration,
    };

    use tokio::{fs, net::UdpSocket, sync::Mutex, task::JoinHandle, time::timeout};

    use crate::{app_state::AppState, config::Config};

    static TFTP_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    async fn test_lock() -> tokio::sync::MutexGuard<'static, ()> {
        TFTP_TEST_LOCK.get_or_init(|| Mutex::new(())).lock().await
    }

    fn rrq_packet(path: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&1_u16.to_be_bytes());
        packet.extend_from_slice(path.as_bytes());
        packet.push(0);
        packet.extend_from_slice(b"octet");
        packet.push(0);
        packet
    }

    async fn recv_packet(socket: &UdpSocket, buffer: &mut [u8]) -> (usize, SocketAddr) {
        timeout(Duration::from_secs(3), socket.recv_from(buffer))
            .await
            .expect("packet timeout")
            .expect("recv packet")
    }

    async fn fetch_tftp(addr: SocketAddr, path: &str) -> Result<Vec<u8>, u16> {
        let client = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .expect("bind client");
        client
            .send_to(&rrq_packet(path), addr)
            .await
            .expect("send rrq");

        let mut buffer = [0_u8; 2048];
        let mut payload = Vec::new();
        let mut transfer_addr = None;

        loop {
            let (len, sender) = recv_packet(&client, &mut buffer).await;
            transfer_addr.get_or_insert(sender);

            match u16::from_be_bytes([buffer[0], buffer[1]]) {
                3 => {
                    let block = u16::from_be_bytes([buffer[2], buffer[3]]);
                    payload.extend_from_slice(&buffer[4..len]);

                    let mut ack = [0_u8; 4];
                    ack[..2].copy_from_slice(&4_u16.to_be_bytes());
                    ack[2..].copy_from_slice(&block.to_be_bytes());
                    client
                        .send_to(&ack, transfer_addr.expect("transfer addr"))
                        .await
                        .expect("send ack");

                    if len < 516 {
                        return Ok(payload);
                    }
                }
                5 => {
                    let code = u16::from_be_bytes([buffer[2], buffer[3]]);
                    return Err(code);
                }
                opcode => panic!("unexpected opcode {opcode}"),
            }
        }
    }

    async fn seed_asset(tempdir: &tempfile::TempDir, relative_path: &str, bytes: &[u8]) {
        let path = tempdir.path().join("data/cache").join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.expect("cache dir");
        }
        fs::write(path, bytes).await.expect("seed asset");
    }

    async fn spawn_test_server(
        tempdir: &tempfile::TempDir,
        selected_distro: Option<boot_recipe::DistroId>,
    ) -> (Arc<AppState>, SocketAddr, JoinHandle<()>) {
        let listener =
            std::net::UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .expect("reserve udp port");
        let tftp_bind = listener.local_addr().expect("reserved addr");
        drop(listener);

        fs::create_dir_all(tempdir.path().join("frontend"))
            .await
            .expect("frontend dir");

        let state = Arc::new(
            AppState::new(Config {
                api_bind: ([127, 0, 0, 1], 0).into(),
                tftp_bind,
                tftp_advertise_addr: ([10, 0, 2, 2], 16969).into(),
                data_dir: tempdir.path().join("data"),
                frontend_dir: tempdir.path().join("frontend"),
            })
            .await
            .expect("state"),
        );

        if let Some(distro) = selected_distro {
            state.set_selected_distro(distro).await.expect("set distro");
        }

        let server = super::build_server(state.clone())
            .await
            .expect("build server");
        let listen_addr = server.listen_addr().expect("listen addr");
        let handle = tokio::spawn(async move {
            server.serve().await.expect("serve");
        });

        (state, listen_addr, handle)
    }

    #[tokio::test]
    async fn serves_cached_asset_over_tftp() {
        let _lock = test_lock().await;
        let tempdir = tempfile::tempdir().expect("tempdir");
        seed_asset(&tempdir, "ubuntu/bios/kernel", b"kernel-bytes").await;
        let (_state, addr, handle) = spawn_test_server(&tempdir, None).await;

        let payload = fetch_tftp(addr, "ubuntu/bios/kernel")
            .await
            .expect("tftp payload");
        handle.abort();

        assert_eq!(payload, b"kernel-bytes");
    }

    #[tokio::test]
    async fn serves_generated_grub_config_over_tftp() {
        let _lock = test_lock().await;
        let tempdir = tempfile::tempdir().expect("tempdir");
        let (_state, addr, handle) = spawn_test_server(&tempdir, None).await;

        let payload = fetch_tftp(addr, "grub/grub.cfg")
            .await
            .expect("tftp payload");
        handle.abort();

        let payload = String::from_utf8(payload).expect("utf8");
        assert!(payload.contains("root=(tftp,10.0.2.2:16969)"));
        assert!(payload.contains("linux /ubuntu/uefi/kernel"));
    }

    #[tokio::test]
    async fn returns_file_not_found_for_missing_asset() {
        let _lock = test_lock().await;
        let tempdir = tempfile::tempdir().expect("tempdir");
        let (_state, addr, handle) = spawn_test_server(&tempdir, None).await;

        let error = fetch_tftp(addr, "ubuntu/bios/missing")
            .await
            .expect_err("expected tftp error");
        handle.abort();

        assert_eq!(error, 1);
    }

    #[tokio::test]
    async fn serves_fedora_generated_grub_for_fedora_selection() {
        let _lock = test_lock().await;
        let tempdir = tempfile::tempdir().expect("tempdir");
        let (_state, addr, handle) =
            spawn_test_server(&tempdir, Some(boot_recipe::DistroId::Fedora)).await;

        let payload = fetch_tftp(addr, "grub/grub.cfg")
            .await
            .expect("expected grub payload");
        handle.abort();

        let payload = String::from_utf8(payload).expect("utf8");
        assert!(payload.contains("linuxefi /fedora/uefi/kernel"));
        assert!(payload.contains("inst.ks=http://10.0.2.2:0/boot/fedora/uefi/kickstart/ks.cfg"));
    }
}
