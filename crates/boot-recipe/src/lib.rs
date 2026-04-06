use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistroId {
    Ubuntu,
    Fedora,
    Arch,
}

impl DistroId {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ubuntu => "Ubuntu",
            Self::Fedora => "Fedora",
            Self::Arch => "Arch Linux",
        }
    }
}

impl std::fmt::Display for DistroId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Ubuntu => "Ubuntu",
            Self::Fedora => "Fedora",
            Self::Arch => "Arch Linux",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootMode {
    Bios,
    Uefi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpOptionHint {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpGuidance {
    pub boot_filename: String,
    pub next_server: String,
    pub architecture: String,
    pub notes: Vec<String>,
    pub options: Vec<DhcpOptionHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BootAsset {
    pub logical_name: String,
    pub relative_path: String,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeOutput {
    pub distro: DistroId,
    pub boot_mode: BootMode,
    pub label: String,
    pub assets: Vec<BootAsset>,
    pub dhcp: DhcpGuidance,
}

pub fn all_distros() -> Vec<DistroId> {
    vec![DistroId::Ubuntu, DistroId::Fedora, DistroId::Arch]
}

pub fn get_recipe(distro: DistroId, mode: BootMode) -> Result<RecipeOutput, RecipeError> {
    Ok(match (distro, mode) {
        (DistroId::Ubuntu, BootMode::Bios) => recipe(
            distro,
            mode,
            "Ubuntu BIOS",
            "ubuntu/bios/lpxelinux.0",
            vec![
                asset(
                    "pxelinux",
                    "ubuntu/bios/lpxelinux.0",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/pxelinux.0",
                ),
                asset(
                    "kernel",
                    "ubuntu/bios/kernel",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/linux",
                ),
                asset(
                    "initrd",
                    "ubuntu/bios/initrd",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/initrd",
                ),
            ],
        ),
        (DistroId::Ubuntu, BootMode::Uefi) => recipe(
            distro,
            mode,
            "Ubuntu UEFI",
            "ubuntu/uefi/grubx64.efi",
            vec![
                asset(
                    "grub",
                    "ubuntu/uefi/grubx64.efi",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/grubx64.efi",
                ),
                asset(
                    "kernel",
                    "ubuntu/uefi/kernel",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/linux",
                ),
                asset(
                    "initrd",
                    "ubuntu/uefi/initrd",
                    "https://releases.ubuntu.com/24.04/netboot/amd64/initrd",
                ),
                asset(
                    "iso",
                    "ubuntu/uefi/live-server.iso",
                    "https://releases.ubuntu.com/24.04/ubuntu-24.04.4-live-server-amd64.iso",
                ),
            ],
        ),
        (DistroId::Fedora, BootMode::Bios) => recipe(
            distro,
            mode,
            "Fedora BIOS",
            "fedora/bios/pxelinux.0",
            vec![
                asset(
                    "pxelinux",
                    "fedora/bios/pxelinux.0",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/pxelinux.0",
                ),
                asset(
                    "kernel",
                    "fedora/bios/kernel",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/vmlinuz",
                ),
                asset(
                    "initrd",
                    "fedora/bios/initrd",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/initrd.img",
                ),
            ],
        ),
        (DistroId::Fedora, BootMode::Uefi) => recipe(
            distro,
            mode,
            "Fedora UEFI",
            "fedora/uefi/shimx64.efi",
            vec![
                asset(
                    "shim",
                    "fedora/uefi/shimx64.efi",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/shimx64.efi",
                ),
                asset(
                    "kernel",
                    "fedora/uefi/kernel",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/vmlinuz",
                ),
                asset(
                    "initrd",
                    "fedora/uefi/initrd",
                    "https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/images/pxeboot/initrd.img",
                ),
            ],
        ),
        (DistroId::Arch, BootMode::Bios) => recipe(
            distro,
            mode,
            "Arch Linux BIOS",
            "arch/bios/lpxelinux.0",
            vec![
                asset(
                    "pxelinux",
                    "arch/bios/lpxelinux.0",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/boot/syslinux/lpxelinux.0",
                ),
                asset(
                    "kernel",
                    "arch/bios/kernel",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/boot/x86_64/vmlinuz-linux",
                ),
                asset(
                    "initrd",
                    "arch/bios/initrd",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/boot/x86_64/initramfs-linux.img",
                ),
            ],
        ),
        (DistroId::Arch, BootMode::Uefi) => recipe(
            distro,
            mode,
            "Arch Linux UEFI",
            "arch/uefi/bootx64.efi",
            vec![
                asset(
                    "bootx64",
                    "arch/uefi/bootx64.efi",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/EFI/BOOT/BOOTX64.EFI",
                ),
                asset(
                    "kernel",
                    "arch/uefi/kernel",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/boot/x86_64/vmlinuz-linux",
                ),
                asset(
                    "initrd",
                    "arch/uefi/initrd",
                    "https://geo.mirror.pkgbuild.com/iso/latest/arch/boot/x86_64/initramfs-linux.img",
                ),
            ],
        ),
    })
}

fn recipe(
    distro: DistroId,
    boot_mode: BootMode,
    label: &str,
    boot_filename: &str,
    assets: Vec<BootAsset>,
) -> RecipeOutput {
    let architecture = match boot_mode {
        BootMode::Bios => "x86 BIOS",
        BootMode::Uefi => "x86_64 UEFI",
    };

    RecipeOutput {
        distro,
        boot_mode,
        label: label.to_string(),
        dhcp: DhcpGuidance {
            boot_filename: boot_filename.to_string(),
            next_server: "set to the boopa host IP".to_string(),
            architecture: architecture.to_string(),
            notes: vec![
                "Configure next-server to the host running boopa.".to_string(),
                "Expose the listed boot filename over TFTP; kernel and initrd are also available over HTTP under /boot/.".to_string(),
            ],
            options: vec![
                DhcpOptionHint {
                    key: "next-server".to_string(),
                    value: "server-ip".to_string(),
                    description: "IP address of the boopa host".to_string(),
                },
                DhcpOptionHint {
                    key: "filename".to_string(),
                    value: boot_filename.to_string(),
                    description: "Boot file for the selected distro and architecture".to_string(),
                },
            ],
        },
        assets,
    }
}

fn asset(logical_name: &str, relative_path: &str, source_url: &str) -> BootAsset {
    BootAsset {
        logical_name: logical_name.to_string(),
        relative_path: relative_path.to_string(),
        source_url: source_url.to_string(),
    }
}

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("unsupported recipe")]
    Unsupported,
}
