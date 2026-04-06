use std::{
    fmt,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use boot_recipe::{BootMode, DistroId, get_recipe};

use crate::autoinstall::{PersistedUbuntuAutoinstallConfig, render_meta_data, render_user_data};

const BINARY_CONTENT_TYPE: &str = "application/octet-stream";
const TEXT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const UBUNTU_UEFI_GRUB_CFG_ALIASES: [&str; 4] = [
    "ubuntu/uefi/grub.cfg",
    "ubuntu/uefi/grub/grub.cfg",
    "grub/grub.cfg",
    "boot/grub/grub.cfg",
];
const FEDORA_UEFI_GRUB_CFG_ALIASES: [&str; 6] = [
    "fedora/uefi/grub.cfg",
    "fedora/uefi/grub/grub.cfg",
    "grub/grub.cfg",
    "boot/grub/grub.cfg",
    "grub2/grub.cfg",
    "boot/grub2/grub.cfg",
];
const UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH: &str = "ubuntu/uefi/autoinstall/user-data";
const UBUNTU_UEFI_AUTOINSTALL_META_DATA_PATH: &str = "ubuntu/uefi/autoinstall/meta-data";
const FEDORA_UEFI_KICKSTART_PATH: &str = "fedora/uefi/kickstart/ks.cfg";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedBootAsset {
    CachedFile {
        logical_path: String,
        local_path: PathBuf,
        content_type: &'static str,
    },
    Generated {
        logical_path: String,
        bytes: Vec<u8>,
        content_type: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootAssetTransport {
    Http,
    Tftp,
}

impl fmt::Display for BootAssetTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Http => "http",
            Self::Tftp => "tftp",
        })
    }
}

impl ResolvedBootAsset {
    pub fn logical_path(&self) -> &str {
        match self {
            Self::CachedFile { logical_path, .. } | Self::Generated { logical_path, .. } => {
                logical_path
            }
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            Self::CachedFile { content_type, .. } | Self::Generated { content_type, .. } => {
                content_type
            }
        }
    }

    pub fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }

    pub async fn read_bytes(&self) -> std::io::Result<Vec<u8>> {
        match self {
            Self::CachedFile { local_path, .. } => tokio::fs::read(local_path).await,
            Self::Generated { bytes, .. } => Ok(bytes.clone()),
        }
    }
}

pub fn resolve_asset(
    cache_root: &Path,
    distro: DistroId,
    requested_path: &str,
    tftp_endpoint: SocketAddr,
    guest_http_base_url: &str,
    transport: BootAssetTransport,
) -> Option<ResolvedBootAsset> {
    resolve_asset_with_ubuntu_autoinstall(
        cache_root,
        distro,
        &PersistedUbuntuAutoinstallConfig::default(),
        requested_path,
        tftp_endpoint,
        guest_http_base_url,
        transport,
    )
}

pub fn resolve_asset_with_ubuntu_autoinstall(
    cache_root: &Path,
    distro: DistroId,
    ubuntu_autoinstall: &PersistedUbuntuAutoinstallConfig,
    requested_path: &str,
    tftp_endpoint: SocketAddr,
    guest_http_base_url: &str,
    transport: BootAssetTransport,
) -> Option<ResolvedBootAsset> {
    let normalized = requested_path.trim_start_matches('/');

    if let Some(asset) = resolve_generated_asset(
        distro,
        ubuntu_autoinstall,
        normalized,
        tftp_endpoint,
        guest_http_base_url,
        transport,
    ) {
        return Some(asset);
    }

    [BootMode::Bios, BootMode::Uefi]
        .into_iter()
        .filter_map(|mode| get_recipe(distro, mode).ok())
        .flat_map(|recipe| recipe.assets.into_iter())
        .find_map(|asset| {
            if asset.relative_path == normalized && asset_is_available_over(transport, normalized) {
                Some(ResolvedBootAsset::CachedFile {
                    logical_path: asset.relative_path,
                    local_path: cache_root.join(normalized),
                    content_type: BINARY_CONTENT_TYPE,
                })
            } else {
                None
            }
        })
}

fn resolve_generated_asset(
    distro: DistroId,
    ubuntu_autoinstall: &PersistedUbuntuAutoinstallConfig,
    normalized: &str,
    tftp_endpoint: SocketAddr,
    guest_http_base_url: &str,
    transport: BootAssetTransport,
) -> Option<ResolvedBootAsset> {
    let generated = match distro {
        DistroId::Ubuntu if UBUNTU_UEFI_GRUB_CFG_ALIASES.contains(&normalized) => Some((
            "ubuntu/uefi/grub.cfg",
            render_ubuntu_uefi_grub_cfg(tftp_endpoint, guest_http_base_url),
            true,
        )),
        DistroId::Ubuntu if normalized == UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH => Some((
            UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH,
            render_user_data(ubuntu_autoinstall).ok()?,
            false,
        )),
        DistroId::Ubuntu if normalized == UBUNTU_UEFI_AUTOINSTALL_META_DATA_PATH => Some((
            UBUNTU_UEFI_AUTOINSTALL_META_DATA_PATH,
            render_meta_data(ubuntu_autoinstall),
            false,
        )),
        DistroId::Fedora if FEDORA_UEFI_GRUB_CFG_ALIASES.contains(&normalized) => Some((
            "fedora/uefi/grub.cfg",
            render_fedora_uefi_grub_cfg(tftp_endpoint, guest_http_base_url),
            true,
        )),
        DistroId::Fedora if normalized == FEDORA_UEFI_KICKSTART_PATH => Some((
            FEDORA_UEFI_KICKSTART_PATH,
            render_fedora_uefi_kickstart(),
            false,
        )),
        _ => None,
    }?;

    if !generated.2 && transport == BootAssetTransport::Tftp {
        return None;
    }

    Some(ResolvedBootAsset::Generated {
        logical_path: generated.0.to_string(),
        bytes: generated.1.into_bytes(),
        content_type: TEXT_CONTENT_TYPE,
    })
}

fn asset_is_available_over(transport: BootAssetTransport, path: &str) -> bool {
    !matches!(
        (transport, path),
        (BootAssetTransport::Tftp, "ubuntu/uefi/live-server.iso")
    )
}

fn render_ubuntu_uefi_grub_cfg(tftp_endpoint: SocketAddr, guest_http_base_url: &str) -> String {
    let autoinstall_seed = format!("{guest_http_base_url}/boot/ubuntu/uefi/autoinstall/");
    format!(
        "set default=0\nset timeout=2\nset pager=1\n\ninsmod efinet\ninsmod net\ninsmod tftp\nnet_bootp\nset root=(tftp,{tftp_endpoint})\n\nmenuentry \"boopa ubuntu uefi autoinstall\" {{\n    echo \"Booting Ubuntu UEFI autoinstall through boopa TFTP\"\n    linux /ubuntu/uefi/kernel ip=dhcp boot=casper iso-url={guest_http_base_url}/boot/ubuntu/uefi/live-server.iso autoinstall 'ds=nocloud-net;s={autoinstall_seed}' console=ttyS0,115200n8 ---\n    initrd /ubuntu/uefi/initrd\n    boot\n}}\n"
    )
}

fn render_fedora_uefi_grub_cfg(tftp_endpoint: SocketAddr, guest_http_base_url: &str) -> String {
    let kickstart_url = format!("{guest_http_base_url}/boot/{FEDORA_UEFI_KICKSTART_PATH}");
    format!(
        "set default=0\nset timeout=2\nset pager=1\n\ninsmod efinet\ninsmod net\ninsmod tftp\nnet_bootp\nset root=(tftp,{tftp_endpoint})\n\nmenuentry \"boopa fedora uefi kickstart\" {{\n    echo \"Booting Fedora UEFI Kickstart through boopa TFTP\"\n    linuxefi /fedora/uefi/kernel ip=dhcp inst.ks={kickstart_url} console=ttyS0,115200n8\n    initrdefi /fedora/uefi/initrd\n    boot\n}}\n"
    )
}

fn render_fedora_uefi_kickstart() -> String {
    "lang en_US.UTF-8\nkeyboard us\ntimezone UTC --utc\nnetwork --bootproto=dhcp --device=link --activate\nrootpw --lock\ntext\nreboot\nzerombr\nclearpart --all --initlabel\nautopart\n%packages\n@^minimal-environment\n%end\n".to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::Path,
    };

    use boot_recipe::DistroId;

    use super::{
        BootAssetTransport, FEDORA_UEFI_KICKSTART_PATH, ResolvedBootAsset,
        UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH, resolve_asset,
        resolve_asset_with_ubuntu_autoinstall,
    };
    use crate::autoinstall::{PersistedUbuntuAutoinstallConfig, UbuntuStorageLayout};

    #[test]
    fn resolves_known_asset_path() {
        let resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            "ubuntu/bios/kernel",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
            "http://127.0.0.1:8080",
            BootAssetTransport::Http,
        );
        assert_eq!(
            resolved,
            Some(ResolvedBootAsset::CachedFile {
                logical_path: "ubuntu/bios/kernel".to_string(),
                local_path: Path::new("/tmp/cache/ubuntu/bios/kernel").to_path_buf(),
                content_type: super::BINARY_CONTENT_TYPE,
            })
        );
    }

    #[test]
    fn resolves_generated_ubuntu_uefi_grub_aliases_to_identical_bytes() {
        let endpoint = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 2)), 16969);

        let assets = [
            "ubuntu/uefi/grub.cfg",
            "ubuntu/uefi/grub/grub.cfg",
            "grub/grub.cfg",
            "boot/grub/grub.cfg",
        ]
        .into_iter()
        .map(|path| {
            resolve_asset(
                Path::new("/tmp/cache"),
                DistroId::Ubuntu,
                path,
                endpoint,
                "http://10.0.2.2:18080",
                BootAssetTransport::Tftp,
            )
        })
        .collect::<Option<Vec<_>>>()
        .expect("all aliases resolve");

        assert!(assets.iter().all(ResolvedBootAsset::is_generated));
        let bytes = assets[0].read_bytes();
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let first = runtime.block_on(bytes).expect("bytes");
        for asset in assets.iter().skip(1) {
            assert_eq!(asset.logical_path(), "ubuntu/uefi/grub.cfg");
            assert_eq!(runtime.block_on(asset.read_bytes()).expect("bytes"), first);
        }
        let payload = String::from_utf8(first).expect("utf8");
        assert!(payload.contains("root=(tftp,10.0.2.2:16969)"));
        assert!(payload.contains("boot=casper"));
        assert!(payload.contains("iso-url=http://10.0.2.2:18080/boot/ubuntu/uefi/live-server.iso"));
        assert!(payload.contains("autoinstall"));
        assert!(
            payload
                .contains("ds=nocloud-net;s=http://10.0.2.2:18080/boot/ubuntu/uefi/autoinstall/")
        );
    }

    #[test]
    fn ubuntu_uefi_live_server_iso_is_http_only() {
        let http_resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            "ubuntu/uefi/live-server.iso",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 2)), 16969),
            "http://10.0.2.2:18080",
            BootAssetTransport::Http,
        );
        assert!(matches!(
            http_resolved,
            Some(ResolvedBootAsset::CachedFile { .. })
        ));

        let tftp_resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            "ubuntu/uefi/live-server.iso",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 2)), 16969),
            "http://10.0.2.2:18080",
            BootAssetTransport::Tftp,
        );
        assert!(tftp_resolved.is_none());
    }

    #[test]
    fn generated_grub_config_is_unavailable_for_non_matching_distro() {
        let resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Fedora,
            "ubuntu/uefi/grub.cfg",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
            "http://127.0.0.1:8080",
            BootAssetTransport::Http,
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn ubuntu_autoinstall_seed_assets_are_http_only() {
        let user_data = resolve_asset_with_ubuntu_autoinstall(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            &PersistedUbuntuAutoinstallConfig {
                hostname: "custom-host".to_string(),
                username: "ubuntu".to_string(),
                password_hash: PersistedUbuntuAutoinstallConfig::default().password_hash,
                locale: "ja_JP.UTF-8".to_string(),
                keyboard_layout: "jp".to_string(),
                timezone: "Asia/Tokyo".to_string(),
                storage_layout: UbuntuStorageLayout::Lvm,
                install_open_ssh: true,
                allow_password_auth: false,
                authorized_keys: vec!["ssh-ed25519 AAAA test@example".to_string()],
                packages: vec!["curl".to_string(), "git".to_string()],
            },
            UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
            "http://127.0.0.1:8080",
            BootAssetTransport::Http,
        )
        .expect("user-data");
        assert!(user_data.is_generated());
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let payload = String::from_utf8(runtime.block_on(user_data.read_bytes()).expect("bytes"))
            .expect("utf8");
        assert!(payload.contains("#cloud-config"));
        assert!(payload.contains("autoinstall:"));
        assert!(payload.contains("hostname: custom-host"));
        assert!(payload.contains("layout: jp"));
        assert!(payload.contains("name: lvm"));
        assert!(payload.contains("- curl"));

        let tftp_user_data = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            UBUNTU_UEFI_AUTOINSTALL_USER_DATA_PATH,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
            "http://127.0.0.1:8080",
            BootAssetTransport::Tftp,
        );
        assert!(tftp_user_data.is_none());
    }

    #[test]
    fn resolves_generated_fedora_uefi_grub_and_kickstart() {
        let endpoint = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 2, 2)), 16969);
        let grub = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Fedora,
            "grub/grub.cfg",
            endpoint,
            "http://10.0.2.2:18080",
            BootAssetTransport::Tftp,
        )
        .expect("grub");
        assert!(grub.is_generated());
        assert_eq!(grub.logical_path(), "fedora/uefi/grub.cfg");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let grub_payload =
            String::from_utf8(runtime.block_on(grub.read_bytes()).expect("bytes")).expect("utf8");
        assert!(grub_payload.contains("linuxefi /fedora/uefi/kernel"));
        assert!(
            grub_payload
                .contains("inst.ks=http://10.0.2.2:18080/boot/fedora/uefi/kickstart/ks.cfg")
        );

        let kickstart = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Fedora,
            FEDORA_UEFI_KICKSTART_PATH,
            endpoint,
            "http://10.0.2.2:18080",
            BootAssetTransport::Http,
        )
        .expect("ks");
        let kickstart_payload =
            String::from_utf8(runtime.block_on(kickstart.read_bytes()).expect("bytes"))
                .expect("utf8");
        assert!(kickstart_payload.contains("lang en_US.UTF-8"));
        assert!(kickstart_payload.contains("%packages"));
    }
}
