from __future__ import annotations

from .models import SmokeError, SupportStatus


SUPPORTED_DISTROS = ("ubuntu", "fedora")
SUPPORTED_BOOT_MODES = ("uefi", "bios")
SUPPORTED_NETWORK_MODES = ("user", "vmnet-host", "vde")
SUPPORTED_LANES = ("backend", "custom-image")


def support_status(
    distro: str,
    boot_mode: str,
    lane: str,
    network_mode: str,
) -> SupportStatus:
    if distro not in SUPPORTED_DISTROS:
        raise SmokeError(f"unsupported distro: {distro}")
    if boot_mode not in SUPPORTED_BOOT_MODES:
        raise SmokeError(f"unsupported boot mode: {boot_mode}")
    if lane not in SUPPORTED_LANES:
        raise SmokeError(f"unsupported smoke lane: {lane}")
    if network_mode not in SUPPORTED_NETWORK_MODES:
        raise SmokeError(f"unsupported SMOKE_NETWORK_MODE: {network_mode}")

    if lane == "custom-image":
        if distro != "ubuntu" or boot_mode != "uefi":
            raise SmokeError("custom-image lane only supports ubuntu uefi")
        return SupportStatus(
            plan_supported=True,
            execution_ready=True,
            level="verified",
            notes=[
                "custom-image remains an Ubuntu UEFI-only lane",
                "boopa is not started for the custom-image lane",
            ],
        )

    notes = [
        f"formal support matrix includes {distro} {boot_mode}",
        f"network lane {network_mode} is modeled explicitly",
    ]
    execution_ready = True
    level = "verified"

    if boot_mode == "bios":
        level = "planned"
        execution_ready = False
        notes.append("BIOS planning is explicit, but live execution is not yet verified on this host")

    if network_mode == "user":
        notes.append("user networking remains a debug/support path and is not boopa-origin DHCP acceptance")
    elif network_mode == "vmnet-host":
        level = "experimental" if level == "verified" else level
        notes.append("vmnet-host may fail on hosts without vmnet entitlements")
    elif network_mode == "vde":
        notes.append("vde is the preferred mac-host guest-path acceptance lane")

    return SupportStatus(
        plan_supported=True,
        execution_ready=execution_ready,
        level=level,
        notes=notes,
    )
