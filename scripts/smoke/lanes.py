from __future__ import annotations

from .models import SmokeRequest


def lane_steps(request: SmokeRequest, boot_sync_paths: list[str], probe_paths: list[str]) -> list[str]:
    if request.lane == "custom-image":
        return [
            "validate custom image inputs",
            "build or reuse the requested custom image ISO",
            "boot QEMU with the generated ISO attached as a CD-ROM",
        ]

    steps = [
        "prepare workspace and cache links",
        "prepare QEMU firmware/disk state",
    ]
    if request.boot_mode == "uefi":
        steps.append("stage a minimal firmware carrier for the first-stage UEFI handoff")
    steps.extend(
        [
            "start any required network backend plumbing",
            "start boopa and wait for the health endpoint",
            "refresh the selected distro/mode boot assets through boopa",
        ]
    )
    if boot_sync_paths:
        steps.append("sync firmware-carrier boot assets from boopa into the local boot root")
    if probe_paths:
        steps.append("probe the boot asset endpoints that this target expects before guest boot")
    if request.network_mode == "vmnet-host":
        steps.append("start the podman DHCP relay helper")
    if request.network_mode == "vde":
        steps.append("start the VDE host helper")
    steps.extend(
        [
            "launch QEMU using the planned boot and network configuration",
            "verify markers and any guest-path evidence promised by the selected lane",
        ]
    )
    return steps


def lane_side_effects(request: SmokeRequest) -> list[str]:
    if request.lane == "custom-image":
        return [
            "writes or reuses a custom Ubuntu UEFI ISO",
            "creates a QEMU installer disk under the run directory",
        ]
    effects = [
        "creates a per-run workspace with logs and service data",
        "runs boopa against a temporary data directory",
        "writes QEMU command logs and serial output under the run directory",
    ]
    if request.network_mode == "vmnet-host":
        effects.append("starts a podman relay container for guest DHCP traffic")
    if request.network_mode == "vde":
        effects.append("starts a vde_switch process plus a Python host helper")
    return effects
