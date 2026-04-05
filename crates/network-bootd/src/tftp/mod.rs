use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

use tokio::{fs, net::UdpSocket, time::timeout};

use crate::app_state::AppState;

const DEFAULT_BLOCK_SIZE: usize = 512;
const MAX_BLOCK_SIZE: usize = 65464;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TftpResolution {
    pub requested_path: String,
    pub cache_relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadRequest {
    filename: String,
    options: RequestOptions,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct RequestOptions {
    block_size: Option<usize>,
    transfer_size: bool,
}

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let socket = UdpSocket::bind(state.config().tftp_bind).await?;
    let mut buffer = [0_u8; 1500];

    loop {
        let (bytes_read, peer) = socket.recv_from(&mut buffer).await?;
        if let Some(request) = parse_rrq(&buffer[..bytes_read]) {
            tracing::info!(%peer, requested_path = %request.filename, ?request.options, "tftp rrq");
            if let Some(local_path) = state.resolve_boot_path(&request.filename).await {
                // Keep DATA packets on the RRQ port so GRUB can complete transfers
                // when booting through an explicitly configured non-default TFTP port.
                if let Err(error) = send_file(&socket, peer, local_path, &request).await {
                    tracing::warn!(?error, %peer, "tftp transfer failed");
                }
            } else {
                tracing::warn!(%peer, requested_path = %request.filename, "tftp file not found");
                send_error(&socket, peer, 1, "file not found").await?;
            }
        }
    }
}

pub async fn run_tftp_server(state: Arc<AppState>) -> anyhow::Result<()> {
    serve(state).await
}

pub async fn resolve_request(state: Arc<AppState>, requested_path: &str) -> Option<TftpResolution> {
    state
        .resolve_boot_path(requested_path)
        .await
        .map(|path| TftpResolution {
            requested_path: requested_path.to_string(),
            cache_relative_path: path
                .strip_prefix(state.config().cache_dir())
                .ok()
                .map(|relative| relative.display().to_string())
                .unwrap_or_else(|| path.display().to_string()),
        })
}

pub fn resolve_path(root: &std::path::Path, relative_path: &str) -> PathBuf {
    root.join(relative_path.trim_start_matches('/'))
}

async fn send_file(
    socket: &UdpSocket,
    peer: SocketAddr,
    path: PathBuf,
    request: &ReadRequest,
) -> anyhow::Result<()> {
    let bytes = fs::read(path).await?;
    let block_size = request
        .options
        .block_size
        .map(|size| size.clamp(DEFAULT_BLOCK_SIZE, MAX_BLOCK_SIZE))
        .unwrap_or(DEFAULT_BLOCK_SIZE);

    if let Some(packet) = option_ack_packet(request, bytes.len(), block_size) {
        send_with_retries(socket, peer, 0, packet, "option ack").await?;
    }

    let mut offset = 0usize;
    let mut block = 1u16;

    loop {
        let end = bytes.len().min(offset + block_size);
        let chunk = &bytes[offset..end];
        let mut packet = Vec::with_capacity(chunk.len() + 4);
        packet.extend_from_slice(&3_u16.to_be_bytes());
        packet.extend_from_slice(&block.to_be_bytes());
        packet.extend_from_slice(chunk);

        send_with_retries(socket, peer, block, packet, "data block").await?;

        if chunk.len() < block_size {
            break;
        }
        offset += block_size;
        block = block.wrapping_add(1);
    }

    Ok(())
}

async fn send_with_retries(
    socket: &UdpSocket,
    peer: SocketAddr,
    expected_ack: u16,
    packet: Vec<u8>,
    label: &str,
) -> anyhow::Result<()> {
    let mut retries = 0u8;

    loop {
        socket.send_to(&packet, peer).await?;

        if wait_for_ack(socket, peer, expected_ack).await? {
            return Ok(());
        }

        retries += 1;
        if retries >= 3 {
            anyhow::bail!("timed out waiting for ack for {} {}", label, expected_ack);
        }
    }
}

async fn wait_for_ack(socket: &UdpSocket, peer: SocketAddr, block: u16) -> anyhow::Result<bool> {
    let mut ack = [0_u8; 4];
    loop {
        let result = timeout(Duration::from_secs(3), socket.recv_from(&mut ack)).await;
        let Ok(Ok((len, sender))) = result else {
            return Ok(false);
        };

        if sender != peer || len != 4 {
            continue;
        }

        if parse_ack(&ack) == Some(block) {
            return Ok(true);
        }
    }
}

async fn send_error(
    socket: &UdpSocket,
    peer: SocketAddr,
    code: u16,
    message: &str,
) -> anyhow::Result<()> {
    let mut packet = Vec::with_capacity(message.len() + 5);
    packet.extend_from_slice(&5_u16.to_be_bytes());
    packet.extend_from_slice(&code.to_be_bytes());
    packet.extend_from_slice(message.as_bytes());
    packet.push(0);
    socket.send_to(&packet, peer).await?;
    Ok(())
}

fn option_ack_packet(request: &ReadRequest, file_len: usize, block_size: usize) -> Option<Vec<u8>> {
    let mut payload = Vec::new();

    if request.options.block_size.is_some() {
        payload.extend_from_slice(b"blksize");
        payload.push(0);
        payload.extend_from_slice(block_size.to_string().as_bytes());
        payload.push(0);
    }

    if request.options.transfer_size {
        payload.extend_from_slice(b"tsize");
        payload.push(0);
        payload.extend_from_slice(file_len.to_string().as_bytes());
        payload.push(0);
    }

    if payload.is_empty() {
        return None;
    }

    let mut packet = Vec::with_capacity(payload.len() + 2);
    packet.extend_from_slice(&6_u16.to_be_bytes());
    packet.extend_from_slice(&payload);
    Some(packet)
}

fn parse_rrq(packet: &[u8]) -> Option<ReadRequest> {
    if packet.len() < 4 || u16::from_be_bytes([packet[0], packet[1]]) != 1 {
        return None;
    }

    let fields = packet[2..]
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    if fields.len() < 2 {
        return None;
    }

    let mut options = RequestOptions::default();
    for chunk in fields[2..].chunks_exact(2) {
        let key = chunk[0].to_ascii_lowercase();
        let value = chunk[1];
        match key.as_str() {
            "blksize" => {
                let parsed = value.parse::<usize>().ok()?;
                if (8..=MAX_BLOCK_SIZE).contains(&parsed) {
                    options.block_size = Some(parsed);
                }
            }
            "tsize" => options.transfer_size = true,
            _ => {}
        }
    }

    Some(ReadRequest {
        filename: fields[0].to_string(),
        options,
    })
}

fn parse_ack(packet: &[u8]) -> Option<u16> {
    if packet.len() != 4 || u16::from_be_bytes([packet[0], packet[1]]) != 4 {
        return None;
    }

    Some(u16::from_be_bytes([packet[2], packet[3]]))
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::Arc,
    };

    use tokio::{fs, net::UdpSocket};

    #[test]
    fn parses_rrq_packets() {
        let packet = b"\x00\x01ubuntu/bios/kernel\x00octet\x00";
        assert_eq!(
            super::parse_rrq(packet).map(|request| request.filename),
            Some("ubuntu/bios/kernel".to_string())
        );
    }

    #[test]
    fn parses_ack_packets() {
        assert_eq!(super::parse_ack(b"\x00\x04\x00\x02"), Some(2));
        assert_eq!(super::parse_ack(b"\x00\x03\x00\x02"), None);
    }

    #[test]
    fn parses_rrq_options() {
        let packet = b"\x00\x01/ubuntu/uefi/kernel\x00octet\x00blksize\x001468\x00tsize\x000\x00";
        let request = super::parse_rrq(packet).expect("request");

        assert_eq!(request.filename, "/ubuntu/uefi/kernel");
        assert_eq!(request.options.block_size, Some(1468));
        assert!(request.options.transfer_size);
    }

    #[tokio::test]
    async fn sends_file_and_waits_for_acknowledgements() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("kernel");
        fs::write(&path, vec![b'a'; 700]).await.expect("seed file");

        let server = Arc::new(
            UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .await
                .expect("bind server"),
        );
        let client = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .expect("bind client");
        let client_addr = client.local_addr().expect("client addr");
        let server_addr = server.local_addr().expect("server addr");

        let transfer = tokio::spawn({
            let server = Arc::clone(&server);
            async move {
                super::send_file(
                    server.as_ref(),
                    client_addr,
                    path,
                    &super::ReadRequest {
                        filename: "ubuntu/bios/kernel".to_string(),
                        options: super::RequestOptions::default(),
                    },
                )
                .await
            }
        });

        let mut buffer = [0_u8; 516];
        let (len1, packet1_addr) = client.recv_from(&mut buffer).await.expect("data packet 1");
        assert_eq!(&buffer[..4], b"\x00\x03\x00\x01");
        assert_eq!(len1, 516);
        assert_eq!(packet1_addr, server_addr);
        client
            .send_to(b"\x00\x04\x00\x01", server_addr)
            .await
            .expect("ack block 1");

        let (len2, packet2_addr) = client.recv_from(&mut buffer).await.expect("data packet 2");
        assert_eq!(packet2_addr, server_addr);
        assert_eq!(&buffer[..4], b"\x00\x03\x00\x02");
        assert_eq!(len2, 192);
        client
            .send_to(b"\x00\x04\x00\x02", packet2_addr)
            .await
            .expect("ack block 2");

        transfer.await.expect("join").expect("transfer ok");
    }

    #[tokio::test]
    async fn negotiates_rrq_options_before_streaming_data() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("kernel");
        fs::write(&path, vec![b'a'; 2_000])
            .await
            .expect("seed file");

        let server = Arc::new(
            UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
                .await
                .expect("bind server"),
        );
        let client = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))
            .await
            .expect("bind client");
        let client_addr = client.local_addr().expect("client addr");
        let server_addr = server.local_addr().expect("server addr");

        let transfer = tokio::spawn({
            let server = Arc::clone(&server);
            async move {
                super::send_file(
                    server.as_ref(),
                    client_addr,
                    path,
                    &super::ReadRequest {
                        filename: "/ubuntu/uefi/kernel".to_string(),
                        options: super::RequestOptions {
                            block_size: Some(1468),
                            transfer_size: true,
                        },
                    },
                )
                .await
            }
        });

        let mut buffer = [0_u8; 1600];
        let (oack_len, oack_addr) = client.recv_from(&mut buffer).await.expect("oack packet");
        assert_eq!(oack_addr, server_addr);
        assert_eq!(&buffer[..2], b"\x00\x06");
        let payload = &buffer[2..oack_len];
        assert!(payload.starts_with(b"blksize\x001468\x00"));
        assert!(payload.ends_with(b"tsize\x002000\x00"));

        client
            .send_to(b"\x00\x04\x00\x00", server_addr)
            .await
            .expect("ack oack");

        let (len1, packet1_addr) = client.recv_from(&mut buffer).await.expect("data packet 1");
        assert_eq!(packet1_addr, server_addr);
        assert_eq!(&buffer[..4], b"\x00\x03\x00\x01");
        assert_eq!(len1, 1472);
        client
            .send_to(b"\x00\x04\x00\x01", server_addr)
            .await
            .expect("ack block 1");

        let (len2, packet2_addr) = client.recv_from(&mut buffer).await.expect("data packet 2");
        assert_eq!(packet2_addr, server_addr);
        assert_eq!(&buffer[..4], b"\x00\x03\x00\x02");
        assert_eq!(len2, 536);
        client
            .send_to(b"\x00\x04\x00\x02", packet2_addr)
            .await
            .expect("ack block 2");

        transfer.await.expect("join").expect("transfer ok");
    }
}
