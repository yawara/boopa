use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, bail};
use boot_recipe::{BootMode, get_recipe};
use tokio::net::UdpSocket;

use crate::app_state::AppState;

const BOOTREQUEST: u8 = 1;
const BOOTREPLY: u8 = 2;
const HTYPE_ETHERNET: u8 = 1;
const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];
const FLAG_BROADCAST: u16 = 0x8000;

const OPTION_SUBNET_MASK: u8 = 1;
const OPTION_ROUTER: u8 = 3;
const OPTION_DNS_SERVER: u8 = 6;
const OPTION_HOST_NAME: u8 = 12;
const OPTION_REQUESTED_IP: u8 = 50;
const OPTION_LEASE_TIME: u8 = 51;
const OPTION_MESSAGE_TYPE: u8 = 53;
const OPTION_SERVER_IDENTIFIER: u8 = 54;
const OPTION_PARAMETER_REQUEST_LIST: u8 = 55;
const OPTION_VENDOR_CLASS: u8 = 60;
const OPTION_CLIENT_IDENTIFIER: u8 = 61;
const OPTION_TFTP_SERVER_NAME: u8 = 66;
const OPTION_BOOTFILE_NAME: u8 = 67;
const OPTION_CLIENT_ARCH: u8 = 93;
const OPTION_END: u8 = 255;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let bind = state.config().dhcp.bind;
    let socket = UdpSocket::bind(bind)
        .await
        .with_context(|| format!("failed to bind DHCP socket at {}", bind))?;
    socket
        .set_broadcast(true)
        .context("failed to enable broadcast on DHCP socket")?;

    let mut buffer = [0_u8; 1500];
    loop {
        let (len, peer) = socket.recv_from(&mut buffer).await?;
        let Some((response, destination)) =
            response_for_datagram(state.as_ref(), &buffer[..len], peer).await?
        else {
            continue;
        };

        socket
            .send_to(&response, destination)
            .await
            .with_context(|| format!("failed to send DHCP response to {}", destination))?;
    }
}

pub async fn response_for_datagram(
    state: &AppState,
    payload: &[u8],
    peer: SocketAddr,
) -> anyhow::Result<Option<(Vec<u8>, SocketAddr)>> {
    let request = match DhcpPacket::parse(payload) {
        Ok(packet) => packet,
        Err(error) => {
            tracing::debug!(?error, %peer, "ignoring invalid DHCP packet");
            return Ok(None);
        }
    };

    if request.op != BOOTREQUEST
        || request.htype != HTYPE_ETHERNET
        || request.giaddr != Ipv4Addr::UNSPECIFIED
    {
        return Ok(None);
    }

    let response_type = match request.message_type {
        Some(DhcpMessageType::Discover) => DhcpMessageType::Offer,
        Some(DhcpMessageType::Request) => {
            if let Some(server_identifier) = request.server_identifier
                && server_identifier
                    != state
                        .config()
                        .dhcp
                        .authoritative_subnet()
                        .expect("dhcp subnet")
                        .server_ip
            {
                return Ok(None);
            }
            DhcpMessageType::Ack
        }
        _ => return Ok(None),
    };

    let boot_mode = request.boot_mode();
    let selected = state.selected_distro().await;
    let recipe = get_recipe(selected, boot_mode)?;
    let lease = state
        .allocate_dhcp_lease(
            request.client_key(),
            request.client_mac_string(),
            request
                .requested_ip
                .or_else(|| (request.ciaddr != Ipv4Addr::UNSPECIFIED).then_some(request.ciaddr)),
        )
        .await?;
    let subnet = state
        .config()
        .dhcp
        .authoritative_subnet()
        .expect("authoritative subnet");

    let response = request.build_response(
        response_type,
        lease.ip_address,
        subnet,
        recipe.dhcp.boot_filename.as_bytes(),
    );
    let destination = if request.flags & FLAG_BROADCAST != 0 || peer.ip().is_unspecified() {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), 68)
    } else {
        SocketAddr::new(peer.ip(), peer.port())
    };

    Ok(Some((response, destination)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DhcpMessageType {
    Discover = 1,
    Offer = 2,
    Request = 3,
    Ack = 5,
}

impl TryFrom<u8> for DhcpMessageType {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Discover),
            2 => Ok(Self::Offer),
            3 => Ok(Self::Request),
            5 => Ok(Self::Ack),
            other => bail!("unsupported DHCP message type {}", other),
        }
    }
}

#[derive(Debug, Clone)]
struct DhcpPacket {
    op: u8,
    htype: u8,
    hlen: u8,
    hops: u8,
    xid: u32,
    secs: u16,
    flags: u16,
    ciaddr: Ipv4Addr,
    giaddr: Ipv4Addr,
    chaddr: [u8; 16],
    message_type: Option<DhcpMessageType>,
    requested_ip: Option<Ipv4Addr>,
    server_identifier: Option<Ipv4Addr>,
    client_identifier: Option<Vec<u8>>,
    vendor_class: Option<String>,
    architecture: Option<u16>,
}

impl DhcpPacket {
    fn parse(payload: &[u8]) -> anyhow::Result<Self> {
        if payload.len() < 240 {
            bail!("DHCP packet too short");
        }
        if payload[236..240] != MAGIC_COOKIE {
            bail!("DHCP magic cookie missing");
        }

        let mut packet = Self {
            op: payload[0],
            htype: payload[1],
            hlen: payload[2],
            hops: payload[3],
            xid: u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]),
            secs: u16::from_be_bytes([payload[8], payload[9]]),
            flags: u16::from_be_bytes([payload[10], payload[11]]),
            ciaddr: Ipv4Addr::new(payload[12], payload[13], payload[14], payload[15]),
            giaddr: Ipv4Addr::new(payload[24], payload[25], payload[26], payload[27]),
            chaddr: {
                let mut chaddr = [0_u8; 16];
                chaddr.copy_from_slice(&payload[28..44]);
                chaddr
            },
            message_type: None,
            requested_ip: None,
            server_identifier: None,
            client_identifier: None,
            vendor_class: None,
            architecture: None,
        };

        let mut cursor = 240;
        while cursor < payload.len() {
            let code = payload[cursor];
            cursor += 1;
            if code == OPTION_END {
                break;
            }
            if code == 0 {
                continue;
            }
            if cursor >= payload.len() {
                bail!("DHCP option length missing");
            }
            let length = payload[cursor] as usize;
            cursor += 1;
            if cursor + length > payload.len() {
                bail!("DHCP option payload truncated");
            }
            let value = &payload[cursor..cursor + length];
            cursor += length;

            match code {
                OPTION_MESSAGE_TYPE if length == 1 => {
                    packet.message_type = Some(DhcpMessageType::try_from(value[0])?);
                }
                OPTION_REQUESTED_IP if length == 4 => {
                    packet.requested_ip =
                        Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                }
                OPTION_SERVER_IDENTIFIER if length == 4 => {
                    packet.server_identifier =
                        Some(Ipv4Addr::new(value[0], value[1], value[2], value[3]));
                }
                OPTION_CLIENT_IDENTIFIER => {
                    packet.client_identifier = Some(value.to_vec());
                }
                OPTION_VENDOR_CLASS => {
                    packet.vendor_class = Some(String::from_utf8_lossy(value).to_string());
                }
                OPTION_CLIENT_ARCH if length >= 2 => {
                    packet.architecture = Some(u16::from_be_bytes([value[0], value[1]]));
                }
                OPTION_HOST_NAME | OPTION_PARAMETER_REQUEST_LIST => {}
                _ => {}
            }
        }

        Ok(packet)
    }

    fn boot_mode(&self) -> BootMode {
        match self.architecture {
            Some(0) => BootMode::Bios,
            Some(_) => BootMode::Uefi,
            None => match self.vendor_class.as_deref() {
                Some(value) if value.contains("Arch:00000") => BootMode::Bios,
                Some(value)
                    if value.contains("Arch:00006")
                        || value.contains("Arch:00007")
                        || value.contains("Arch:00009") =>
                {
                    BootMode::Uefi
                }
                _ => BootMode::Bios,
            },
        }
    }

    fn client_key(&self) -> String {
        if let Some(client_identifier) = &self.client_identifier {
            return format!("client-id:{}", hex_bytes(client_identifier));
        }

        format!("mac:{}", self.client_mac_string())
    }

    fn client_mac_string(&self) -> String {
        self.chaddr[..self.hlen as usize]
            .iter()
            .map(|octet| format!("{octet:02x}"))
            .collect::<Vec<_>>()
            .join(":")
    }

    fn build_response(
        &self,
        message_type: DhcpMessageType,
        yiaddr: Ipv4Addr,
        subnet: &crate::config::DhcpSubnetConfig,
        bootfile_name: &[u8],
    ) -> Vec<u8> {
        let mut response = vec![0_u8; 240];
        response[0] = BOOTREPLY;
        response[1] = self.htype;
        response[2] = self.hlen;
        response[3] = self.hops;
        response[4..8].copy_from_slice(&self.xid.to_be_bytes());
        response[8..10].copy_from_slice(&self.secs.to_be_bytes());
        response[10..12].copy_from_slice(&self.flags.to_be_bytes());
        response[16..20].copy_from_slice(&yiaddr.octets());
        response[20..24].copy_from_slice(&subnet.server_ip.octets());
        response[24..28].copy_from_slice(&self.giaddr.octets());
        response[28..44].copy_from_slice(&self.chaddr);

        let bootfile_len = bootfile_name.len().min(127);
        response[108..108 + bootfile_len].copy_from_slice(&bootfile_name[..bootfile_len]);
        response[236..240].copy_from_slice(&MAGIC_COOKIE);

        append_option(&mut response, OPTION_MESSAGE_TYPE, &[message_type as u8]);
        append_option(
            &mut response,
            OPTION_SERVER_IDENTIFIER,
            &subnet.server_ip.octets(),
        );
        append_option(
            &mut response,
            OPTION_LEASE_TIME,
            &subnet.lease_duration_secs.to_be_bytes(),
        );
        append_option(
            &mut response,
            OPTION_SUBNET_MASK,
            &subnet.subnet.netmask().octets(),
        );
        if let Some(router) = subnet.router {
            append_option(&mut response, OPTION_ROUTER, &router.octets());
        }
        if !subnet.dns_servers.is_empty() {
            let mut payload = Vec::with_capacity(subnet.dns_servers.len() * 4);
            for dns in &subnet.dns_servers {
                payload.extend_from_slice(&dns.octets());
            }
            append_option(&mut response, OPTION_DNS_SERVER, &payload);
        }
        append_option(
            &mut response,
            OPTION_TFTP_SERVER_NAME,
            subnet.server_ip.to_string().as_bytes(),
        );
        append_option(
            &mut response,
            OPTION_BOOTFILE_NAME,
            &bootfile_name[..bootfile_len],
        );
        response.push(OPTION_END);
        response
    }
}

fn append_option(target: &mut Vec<u8>, code: u8, payload: &[u8]) {
    target.push(code);
    target.push(payload.len() as u8);
    target.extend_from_slice(payload);
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|octet| format!("{octet:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_discover(architecture: u16) -> Vec<u8> {
        let mut payload = vec![0_u8; 240];
        payload[0] = BOOTREQUEST;
        payload[1] = HTYPE_ETHERNET;
        payload[2] = 6;
        payload[4..8].copy_from_slice(&0x1234_5678_u32.to_be_bytes());
        payload[28..34].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        payload[236..240].copy_from_slice(&MAGIC_COOKIE);
        append_option(
            &mut payload,
            OPTION_MESSAGE_TYPE,
            &[DhcpMessageType::Discover as u8],
        );
        append_option(
            &mut payload,
            OPTION_CLIENT_ARCH,
            &architecture.to_be_bytes(),
        );
        append_option(
            &mut payload,
            OPTION_VENDOR_CLASS,
            b"PXEClient:Arch:00007:UNDI:003016",
        );
        payload.push(OPTION_END);
        payload
    }

    #[test]
    fn parses_bios_architecture() {
        let payload = sample_discover(0);
        let packet = DhcpPacket::parse(&payload).expect("packet");
        assert_eq!(packet.boot_mode(), BootMode::Bios);
    }

    #[test]
    fn parses_uefi_architecture() {
        let payload = sample_discover(7);
        let packet = DhcpPacket::parse(&payload).expect("packet");
        assert_eq!(packet.boot_mode(), BootMode::Uefi);
    }

    #[test]
    fn response_includes_bootfile_and_server_identifier() {
        let payload = sample_discover(7);
        let packet = DhcpPacket::parse(&payload).expect("packet");
        let subnet = crate::config::DhcpSubnetConfig {
            subnet: "10.0.2.0/24".parse().expect("subnet"),
            pool_start: Ipv4Addr::new(10, 0, 2, 50),
            pool_end: Ipv4Addr::new(10, 0, 2, 99),
            router: Some(Ipv4Addr::new(10, 0, 2, 1)),
            dns_servers: vec![Ipv4Addr::new(10, 0, 2, 2)],
            lease_duration_secs: 3600,
            server_ip: Ipv4Addr::new(10, 0, 2, 2),
        };

        let response = packet.build_response(
            DhcpMessageType::Offer,
            Ipv4Addr::new(10, 0, 2, 50),
            &subnet,
            b"ubuntu/uefi/grubx64.efi",
        );

        assert_eq!(response[16..20], [10, 0, 2, 50]);
        assert_eq!(response[20..24], [10, 0, 2, 2]);
        assert!(String::from_utf8_lossy(&response[108..236]).contains("ubuntu/uefi/grubx64.efi"));
    }
}
