from __future__ import annotations


DEFAULT_IDEAL_MARKERS = {
    ("ubuntu", "uefi"): "Reached target System Initialization|Ubuntu installer|Subiquity|Starting system log daemon",
    ("ubuntu", "bios"): "Booting Ubuntu|Loading kernel|Reached target System Initialization|Subiquity",
    ("fedora", "uefi"): "Reached target System Initialization|Starting Anaconda|anaconda:|Starting Kickstart|Installation complete",
    ("fedora", "bios"): "Starting Anaconda|anaconda:|Loading vmlinuz|PXELINUX|Installation complete",
}

DEFAULT_FALLBACK_MARKERS = (
    "Linux version|EFI stub:|Run /init as init process|Loading initial ramdisk|Freeing initrd memory"
)


def default_timeout(distro: str, boot_mode: str) -> int:
    if distro == "fedora":
        return 600 if boot_mode == "uefi" else 300
    return 180


def probe_paths(distro: str, boot_mode: str) -> list[str]:
    if distro == "ubuntu" and boot_mode == "uefi":
        return [
            "ubuntu/uefi/grubx64.efi",
            "ubuntu/uefi/grub.cfg",
            "ubuntu/uefi/kernel",
            "ubuntu/uefi/initrd",
            "ubuntu/uefi/live-server.iso",
        ]
    if distro == "ubuntu" and boot_mode == "bios":
        return [
            "ubuntu/bios/lpxelinux.0",
            "ubuntu/bios/kernel",
            "ubuntu/bios/initrd",
        ]
    if distro == "fedora" and boot_mode == "uefi":
        return [
            "fedora/uefi/shimx64.efi",
            "fedora/uefi/grubx64.efi",
            "fedora/uefi/grub.cfg",
            "fedora/uefi/kernel",
            "fedora/uefi/initrd",
            "fedora/uefi/kickstart/ks.cfg",
        ]
    return [
        "fedora/bios/pxelinux.0",
        "fedora/bios/kernel",
        "fedora/bios/initrd",
    ]


def boot_sync_paths(distro: str, boot_mode: str) -> list[str]:
    if boot_mode != "uefi":
        return []
    if distro == "ubuntu":
        return [
            "ubuntu/uefi/grubx64.efi",
            "ubuntu/uefi/grub.cfg",
        ]
    return [
        "fedora/uefi/shimx64.efi",
        "fedora/uefi/grubx64.efi",
        "fedora/uefi/grub.cfg",
    ]


def bootloader_seed_name(distro: str, boot_mode: str) -> str | None:
    if boot_mode == "uefi":
        return "grubx64.efi" if distro == "ubuntu" else "shimx64.efi"
    return None


def http_evidence_path(distro: str, boot_mode: str) -> str | None:
    if boot_mode != "uefi":
        return None
    if distro == "ubuntu":
        return "ubuntu/uefi/live-server.iso"
    return "fedora/uefi/kickstart/ks.cfg"


def title(distro: str, boot_mode: str, lane: str) -> str:
    if lane == "custom-image":
        return "Ubuntu UEFI custom-image smoke"
    return f"{distro} {boot_mode} smoke"
