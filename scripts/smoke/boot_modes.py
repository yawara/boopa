from __future__ import annotations

from pathlib import Path


def startup_script(boot_mode: str) -> str | None:
    if boot_mode != "uefi":
        return None
    return "fs0:\\EFI\\BOOT\\BOOTX64.EFI\nfs1:\\EFI\\BOOT\\BOOTX64.EFI\n"


def boot_media_args(boot_mode: str, lane: str, boot_root: Path, custom_iso: Path | None) -> list[str]:
    if lane == "custom-image":
        return [
            "-boot",
            "order=d,menu=off",
            "-drive",
            f"file={custom_iso},media=cdrom,if=ide,index=0",
        ]
    if boot_mode == "uefi":
        return [
            "-boot",
            "order=c,menu=off",
            "-drive",
            f"file=fat:rw:{boot_root},format=raw,if=ide,index=0",
        ]
    return [
        "-boot",
        "order=n,menu=off",
    ]


def requires_firmware(boot_mode: str) -> bool:
    return boot_mode == "uefi"
