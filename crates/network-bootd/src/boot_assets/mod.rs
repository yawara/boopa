use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};

use boot_recipe::{BootMode, DistroId, get_recipe};

const BINARY_CONTENT_TYPE: &str = "application/octet-stream";
const GRUB_CFG_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const UBUNTU_UEFI_GRUB_CFG_ALIASES: [&str; 4] = [
    "ubuntu/uefi/grub.cfg",
    "ubuntu/uefi/grub/grub.cfg",
    "grub/grub.cfg",
    "boot/grub/grub.cfg",
];

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
) -> Option<ResolvedBootAsset> {
    let normalized = requested_path.trim_start_matches('/');

    if let Some(asset) = resolve_generated_asset(distro, normalized, tftp_endpoint) {
        return Some(asset);
    }

    [BootMode::Bios, BootMode::Uefi]
        .into_iter()
        .filter_map(|mode| get_recipe(distro, mode).ok())
        .flat_map(|recipe| recipe.assets.into_iter())
        .find_map(|asset| {
            if asset.relative_path == normalized {
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
    normalized: &str,
    tftp_endpoint: SocketAddr,
) -> Option<ResolvedBootAsset> {
    if distro == DistroId::Ubuntu && UBUNTU_UEFI_GRUB_CFG_ALIASES.contains(&normalized) {
        return Some(ResolvedBootAsset::Generated {
            logical_path: "ubuntu/uefi/grub.cfg".to_string(),
            bytes: render_ubuntu_uefi_grub_cfg(tftp_endpoint).into_bytes(),
            content_type: GRUB_CFG_CONTENT_TYPE,
        });
    }

    None
}

fn render_ubuntu_uefi_grub_cfg(tftp_endpoint: SocketAddr) -> String {
    format!(
        "set default=0\nset timeout=2\nset pager=1\n\ninsmod efinet\ninsmod net\ninsmod tftp\nnet_bootp\nset root=(tftp,{tftp_endpoint})\n\nmenuentry \"boopa ubuntu uefi smoke\" {{\n    echo \"Booting Ubuntu UEFI installer through boopa TFTP\"\n    linux /ubuntu/uefi/kernel ip=dhcp console=ttyS0,115200n8 ---\n    initrd /ubuntu/uefi/initrd\n    boot\n}}\n"
    )
}

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::Path,
    };

    use boot_recipe::DistroId;

    use super::{ResolvedBootAsset, resolve_asset};

    #[test]
    fn resolves_known_asset_path() {
        let resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            "ubuntu/bios/kernel",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
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
        .map(|path| resolve_asset(Path::new("/tmp/cache"), DistroId::Ubuntu, path, endpoint))
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
    }

    #[test]
    fn generated_grub_config_is_unavailable_for_non_ubuntu_distros() {
        let resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Fedora,
            "ubuntu/uefi/grub.cfg",
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6969),
        );
        assert!(resolved.is_none());
    }
}
