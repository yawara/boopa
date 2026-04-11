from __future__ import annotations

import os
import random
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path

from . import boot_modes, lanes, networks, support_matrix, targets
from .models import ArtifactSpec, CommandSpec, SmokeError, SmokePlan, SmokeRequest


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _env_int(name: str, default: int) -> int:
    value = os.environ.get(name)
    if value is None or value == "":
        return default
    return int(value)


def _env_boolish(name: str, default: str = "0") -> str:
    return os.environ.get(name, default)


def _env_path(name: str, default: Path) -> Path:
    value = os.environ.get(name)
    return Path(value) if value else default


def _timestamp() -> str:
    return datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")


def _uuid() -> str:
    value = shutil.which("uuidgen")
    if not value:
        return "00000000-0000-0000-0000-000000000000"
    output = subprocess.run([value], capture_output=True, text=True, check=False)
    if output.returncode != 0:
        return "00000000-0000-0000-0000-000000000000"
    return output.stdout.strip().lower()


def _detect_interactive(dry_run: bool) -> bool:
    setting = os.environ.get("SMOKE_INTERACTIVE", "auto")
    if setting == "auto":
        return (not dry_run) and os.isatty(0) and os.isatty(1)
    if setting.lower() in {"1", "true", "yes"}:
        return True
    if setting.lower() in {"0", "false", "no"}:
        return False
    raise SmokeError("SMOKE_INTERACTIVE must be auto, 0, or 1")


def _resolve_source_data_dir(repo_root: Path) -> Path:
    return _env_path("SMOKE_SOURCE_DATA_DIR", repo_root / "var/boopa")


def _resolve_work_root(repo_root: Path) -> Path:
    return _env_path("SMOKE_WORK_ROOT", repo_root / "var/smoke-work")


def _supported_accelerators(qemu_bin: str) -> set[str]:
    resolved = shutil.which(qemu_bin) or qemu_bin
    result = subprocess.run(
        [resolved, "-accel", "help"],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return set()
    return {
        line.strip()
        for line in result.stdout.splitlines()
        if line.strip() and not line.startswith("Accelerators supported")
    }


def _resolve_qemu_accel(qemu_bin: str) -> str:
    configured = os.environ.get("SMOKE_QEMU_ACCEL")
    if configured:
        return configured

    supported = _supported_accelerators(qemu_bin)
    if "hvf" in supported:
        return "hvf"
    return "tcg"


def _resolve_firmware_paths(request: SmokeRequest) -> tuple[Path | None, Path | None]:
    code = os.environ.get("QEMU_FIRMWARE_CODE")
    vars_path = os.environ.get("QEMU_FIRMWARE_VARS")
    if not boot_modes.requires_firmware(request.boot_mode):
        return None, None
    if request.dry_run:
        return (
            Path(code) if code else request.run_dir / "dry-run-firmware-code.fd",
            Path(vars_path) if vars_path else request.run_dir / "dry-run-firmware-vars.fd",
        )

    if code and vars_path:
        return Path(code), Path(vars_path)

    qemu_bin = shutil.which(request.qemu_bin) or request.qemu_bin
    qemu_path = Path(qemu_bin)
    candidates = []
    if shutil.which("brew"):
        brew_prefix = subprocess.run(
            ["brew", "--prefix", "qemu"],
            check=False,
            capture_output=True,
            text=True,
        )
        if brew_prefix.returncode == 0:
            prefix = Path(brew_prefix.stdout.strip()) / "share" / "qemu"
            candidates.append(prefix)
    candidates.append(qemu_path.parent.parent / "share" / "qemu")
    for directory in candidates:
        code_candidate = directory / "edk2-x86_64-code.fd"
        vars_candidate = directory / "edk2-i386-vars.fd"
        if code_candidate.is_file() and vars_candidate.is_file():
            return code_candidate, vars_candidate

    raise SmokeError(
        "unable to locate edk2-x86_64-code.fd and edk2-i386-vars.fd; "
        "set QEMU_FIRMWARE_CODE and QEMU_FIRMWARE_VARS explicitly"
    )


def build_request(
    *,
    command: str,
    distro: str,
    boot_mode: str,
    lane: str,
    network_mode: str,
    dry_run: bool,
) -> SmokeRequest:
    repo_root = _repo_root()
    timestamp = os.environ.get("SMOKE_TIMESTAMP", _timestamp())
    target_name = os.environ.get("SMOKE_TARGET_NAME", f"{distro}-{boot_mode}")
    if lane == "custom-image":
        target_name = os.environ.get("SMOKE_TARGET_NAME", "ubuntu-custom-image")

    work_root = _resolve_work_root(repo_root)
    run_dir = work_root / f"{target_name}-{timestamp}"
    service_data_dir = run_dir / "service-data"
    boot_root = run_dir / "boot-root"
    log_dir = run_dir / "logs"
    source_data_dir = _resolve_source_data_dir(repo_root)
    cache_source_dir = source_data_dir / "cache" / distro / boot_mode

    support = support_matrix.support_status(distro, boot_mode, lane, network_mode)

    request = SmokeRequest(
        repo_root=repo_root,
        command=command,
        distro=distro,
        boot_mode=boot_mode,
        lane=lane,
        network_mode=network_mode,
        dry_run=dry_run,
        interactive=_detect_interactive(dry_run),
        qemu_display=os.environ.get("SMOKE_QEMU_DISPLAY", "default"),
        qemu_bin=os.environ.get("QEMU_BIN", "qemu-system-x86_64"),
        qemu_accel=_resolve_qemu_accel(os.environ.get("QEMU_BIN", "qemu-system-x86_64")),
        ram_mb=_env_int("RAM_MB", 8192),
        system_disk_gb=_env_int("SYSTEM_DISK_GB", 32),
        timeout_secs=_env_int("SMOKE_TIMEOUT_SECS", targets.default_timeout(distro, boot_mode)),
        timestamp=timestamp,
        work_root=work_root,
        run_dir=run_dir,
        service_data_dir=service_data_dir,
        boot_root=boot_root,
        log_dir=log_dir,
        serial_log=log_dir / "serial.log",
        backend_log=log_dir / "backend.log",
        debug_log=log_dir / "debugcon.log",
        qemu_log=log_dir / "qemu.log",
        qemu_cmd_log=log_dir / "qemu-command.txt",
        dhcp_helper_cmd_log=log_dir / "dhcp-helper-command.txt",
        host_helper_log=log_dir / "host-helper.log",
        host_helper_cmd_log=log_dir / "host-helper-command.txt",
        custom_image_build_log=log_dir / "custom-image-build.log",
        custom_image_build_cmd_log=log_dir / "custom-image-build-command.txt",
        system_disk_path=run_dir / "system-disk.qcow2",
        frontend_dir=_env_path("SMOKE_FRONTEND_DIR", repo_root / "frontend/dist"),
        source_data_dir=source_data_dir,
        cache_source_dir=cache_source_dir,
        api_host=os.environ.get("SMOKE_API_HOST", "127.0.0.1"),
        api_bind_host=os.environ.get("SMOKE_API_BIND_HOST", "0.0.0.0"),
        api_port=_env_int("SMOKE_API_PORT", 18080 + random.randint(0, 1999)),
        tftp_bind_host=os.environ.get("SMOKE_TFTP_BIND_HOST", "0.0.0.0"),
        tftp_port=_env_int("SMOKE_TFTP_PORT", 24000 + random.randint(0, 1999)),
        guest_host_ip=os.environ.get(
            "SMOKE_GUEST_HOST_IP",
            "10.0.2.2" if network_mode == "user" else "192.168.127.1",
        ),
        vmnet_net_uuid=os.environ.get("SMOKE_VMNET_NET_UUID", _uuid()) if network_mode == "vmnet-host" else None,
        vmnet_start_address=os.environ.get("SMOKE_VMNET_START_ADDRESS", "192.168.127.10") if network_mode == "vmnet-host" else None,
        vmnet_end_address=os.environ.get("SMOKE_VMNET_END_ADDRESS", "192.168.127.99") if network_mode == "vmnet-host" else None,
        vmnet_subnet_mask=os.environ.get("SMOKE_VMNET_SUBNET_MASK", "255.255.255.0") if network_mode == "vmnet-host" else None,
        dhcp_helper_mode=os.environ.get("SMOKE_DHCP_HELPER_MODE", "podman-relay" if network_mode == "vmnet-host" else "none"),
        dhcp_helper_image=os.environ.get("SMOKE_DHCP_HELPER_IMAGE", "docker.io/library/python:3.12-alpine") if network_mode == "vmnet-host" else None,
        dhcp_helper_name=os.environ.get("SMOKE_DHCP_HELPER_NAME", f"boopa-dhcp-relay-{timestamp}") if network_mode == "vmnet-host" else None,
        dhcp_host_port=_env_int("SMOKE_DHCP_HOST_PORT", 67) if network_mode == "vmnet-host" else None,
        dhcp_upstream_port=_env_int("SMOKE_DHCP_UPSTREAM_PORT", 30000 + random.randint(0, 9999)) if network_mode in {"vmnet-host", "vde"} else None,
        dhcp_subnet=os.environ.get("SMOKE_DHCP_SUBNET", "192.168.127.0/24") if network_mode in {"vmnet-host", "vde"} else None,
        dhcp_pool_start=os.environ.get("SMOKE_DHCP_POOL_START", "192.168.127.50") if network_mode in {"vmnet-host", "vde"} else None,
        dhcp_pool_end=os.environ.get("SMOKE_DHCP_POOL_END", "192.168.127.99") if network_mode in {"vmnet-host", "vde"} else None,
        dhcp_router=os.environ.get("SMOKE_DHCP_ROUTER", "192.168.127.1") if network_mode in {"vmnet-host", "vde"} else None,
        vde_switch_dir=run_dir / "vde.ctl" if network_mode == "vde" else None,
        vde_switch_pidfile=run_dir / "vde-switch.pid" if network_mode == "vde" else None,
        vde_helper_mode=os.environ.get("SMOKE_VDE_HELPER_MODE", "python-host-helper" if network_mode == "vde" else "none"),
        custom_image_base_iso=Path(os.environ["CUSTOM_IMAGE_BASE_ISO"]) if os.environ.get("CUSTOM_IMAGE_BASE_ISO") else None,
        custom_image_manifest=Path(os.environ["CUSTOM_IMAGE_MANIFEST"]) if os.environ.get("CUSTOM_IMAGE_MANIFEST") else None,
        custom_image_output_iso=Path(os.environ["CUSTOM_IMAGE_OUTPUT_ISO"]) if os.environ.get("CUSTOM_IMAGE_OUTPUT_ISO") else None,
        firmware_code=None,
        firmware_vars=None,
        ideal_markers=targets.DEFAULT_IDEAL_MARKERS[(distro, boot_mode)],
        fallback_markers=targets.DEFAULT_FALLBACK_MARKERS,
        support=support,
    )

    firmware_code, firmware_vars = _resolve_firmware_paths(request)
    return SmokeRequest(**{**request.__dict__, "firmware_code": firmware_code, "firmware_vars": firmware_vars})


def build_plan(request: SmokeRequest) -> SmokePlan:
    boot_media = boot_modes.boot_media_args(
        request.boot_mode,
        request.lane,
        request.boot_root,
        request.custom_image_output_iso,
    )
    network_args = networks.qemu_network_args(request)
    qemu_args = [
        request.qemu_bin,
        "-machine",
        "q35",
        "-accel",
        request.qemu_accel,
        "-m",
        str(request.ram_mb),
        "-display",
        request.qemu_display if request.interactive else "none",
        "-monitor",
        "none",
        "-serial",
        "stdio" if request.interactive else f"file:{request.serial_log}",
        "-debugcon",
        f"file:{request.debug_log}",
        "-global",
        "isa-debugcon.iobase=0x402",
    ]
    if boot_modes.requires_firmware(request.boot_mode):
        qemu_args.extend(
            [
                "-drive",
                f"if=pflash,format=raw,readonly=on,file={request.firmware_code}",
                "-drive",
                f"if=pflash,format=raw,file={request.run_dir / 'edk2-vars.fd'}",
            ]
        )
    qemu_args.extend(boot_media)
    qemu_args.extend(
        [
            "-drive",
            f"file={request.system_disk_path},format=qcow2,if=virtio",
        ]
    )
    qemu_args.extend(network_args)
    qemu_args.append("-no-reboot")

    commands: list[CommandSpec] = [
        CommandSpec(
            name="qemu",
            argv=qemu_args,
            log_path=str(request.qemu_cmd_log),
            background=not request.interactive,
            side_effect="boots the selected target under QEMU",
        )
    ]
    helpers = networks.helper_specs(request)
    if request.lane == "backend":
        backend_env = [
            f"BOOPA_API_BIND={request.api_bind_host}:{request.api_port}",
            f"BOOPA_TFTP_BIND={request.tftp_bind_host}:{request.tftp_port}",
            f"BOOPA_TFTP_ADVERTISE_ADDR={request.guest_host_ip}:{request.tftp_port}",
            f"BOOPA_DATA_DIR={request.service_data_dir}",
            f"BOOPA_FRONTEND_DIR={request.frontend_dir}",
        ]
        if request.network_mode in {"vmnet-host", "vde"} and request.dhcp_upstream_port is not None:
            backend_env.extend(
                [
                    "BOOPA_DHCP_MODE=authoritative",
                    f"BOOPA_DHCP_BIND=127.0.0.1:{request.dhcp_upstream_port}",
                    f"BOOPA_DHCP_SUBNET={request.dhcp_subnet}",
                    f"BOOPA_DHCP_POOL_START={request.dhcp_pool_start}",
                    f"BOOPA_DHCP_POOL_END={request.dhcp_pool_end}",
                    f"BOOPA_DHCP_ROUTER={request.dhcp_router}",
                ]
            )
        commands.insert(
            0,
            CommandSpec(
                name="boopa-backend",
                argv=backend_env + ["cargo", "run", "-p", "boopa", "--quiet"],
                log_path=str(request.backend_log),
                background=True,
                side_effect="starts boopa against the temporary smoke data directory",
            ),
        )
    if request.lane == "custom-image" and request.custom_image_output_iso is not None:
        commands.insert(
            0,
            CommandSpec(
                name="custom-image-build",
                argv=[
                    "cargo",
                    "run",
                    "-p",
                    "ubuntu-custom-image",
                    "--",
                    "build",
                    "--base-iso",
                    str(request.custom_image_base_iso),
                    "--manifest",
                    str(request.custom_image_manifest),
                    "--output",
                    str(request.custom_image_output_iso),
                ],
                log_path=str(request.custom_image_build_cmd_log),
                side_effect="builds the requested custom Ubuntu ISO when it does not already exist",
            ),
        )

    probe_paths = targets.probe_paths(request.distro, request.boot_mode) if request.lane == "backend" else []
    boot_sync_paths = targets.boot_sync_paths(request.distro, request.boot_mode) if request.lane == "backend" else []
    artifacts = [
        ArtifactSpec(str(request.run_dir), "per-run workspace"),
        ArtifactSpec(str(request.system_disk_path), "installer disk image"),
        ArtifactSpec(str(request.qemu_cmd_log), "quoted QEMU command log"),
    ]
    if request.lane == "backend":
        artifacts.extend(
            [
                ArtifactSpec(str(request.backend_log), "boopa backend log"),
                ArtifactSpec(str(request.serial_log), "guest serial log"),
            ]
        )
    if request.lane == "custom-image":
        artifacts.append(
            ArtifactSpec(str(request.custom_image_build_cmd_log), "custom-image build command log")
        )
    for helper in helpers:
        if helper.command.log_path:
            artifacts.append(ArtifactSpec(helper.command.log_path, f"{helper.name} command log"))

    summary_lines = [
        f"Smoke target: {targets.title(request.distro, request.boot_mode, request.lane)}",
        f"Run dir: {request.run_dir}",
        f"Support level: {request.support.level}",
        f"Network mode: {request.network_mode}",
        f"Mode: {'dry-run' if request.dry_run else 'interactive' if request.interactive else 'headless'}",
    ]
    if request.lane == "backend":
        summary_lines.extend(
            [
                f"API base: http://{request.api_host}:{request.api_port}",
                f"TFTP endpoint: {request.guest_host_ip}:{request.tftp_port}",
            ]
        )
    else:
        summary_lines.extend(
            [
                f"Base ISO: {request.custom_image_base_iso}",
                f"Manifest: {request.custom_image_manifest}",
                f"Output ISO: {request.custom_image_output_iso}",
            ]
        )

    return SmokePlan(
        request=request,
        title=targets.title(request.distro, request.boot_mode, request.lane),
        summary_lines=summary_lines,
        support=request.support,
        inputs=[
            f"distro={request.distro}",
            f"boot_mode={request.boot_mode}",
            f"lane={request.lane}",
            f"network_mode={request.network_mode}",
        ],
        side_effects=lanes.lane_side_effects(request),
        artifacts=artifacts,
        steps=lanes.lane_steps(request, boot_sync_paths, probe_paths),
        commands=commands,
        helpers=helpers,
        probe_paths=probe_paths,
        boot_sync_paths=boot_sync_paths,
        bootloader_seed_path=(
            str(request.cache_source_dir / targets.bootloader_seed_name(request.distro, request.boot_mode))
            if targets.bootloader_seed_name(request.distro, request.boot_mode)
            else None
        ),
        startup_script=boot_modes.startup_script(request.boot_mode),
        http_evidence_path=targets.http_evidence_path(request.distro, request.boot_mode),
        structured_notes=request.support.notes,
    )
