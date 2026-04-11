from __future__ import annotations

import json
import os
import shlex
import shutil
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path

from . import boot_modes
from .models import CommandSpec, SmokeError, SmokePlan


def _write_command_log(command: CommandSpec) -> None:
    if not command.log_path:
        return
    log_path = Path(command.log_path)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    log_path.write_text(shlex.join(command.argv) + "\n")


def _ensure_cache_link(plan: SmokePlan) -> None:
    cache_link = plan.request.service_data_dir / "cache"
    cache_source = plan.request.source_data_dir / "cache"
    cache_source.mkdir(parents=True, exist_ok=True)
    if cache_link.exists() or cache_link.is_symlink():
        return
    cache_link.symlink_to(cache_source)


def _prepare_workspace(plan: SmokePlan) -> None:
    plan.request.run_dir.mkdir(parents=True, exist_ok=True)
    plan.request.service_data_dir.mkdir(parents=True, exist_ok=True)
    plan.request.log_dir.mkdir(parents=True, exist_ok=True)
    if plan.request.boot_mode == "uefi" and plan.request.lane == "backend":
        (plan.request.boot_root / "EFI" / "BOOT").mkdir(parents=True, exist_ok=True)
    _ensure_cache_link(plan)


def _prepare_firmware(plan: SmokePlan) -> None:
    if not boot_modes.requires_firmware(plan.request.boot_mode):
        return
    assert plan.request.firmware_code is not None
    assert plan.request.firmware_vars is not None
    vars_copy = plan.request.run_dir / "edk2-vars.fd"
    if plan.request.dry_run:
        plan.request.firmware_code.parent.mkdir(parents=True, exist_ok=True)
        plan.request.firmware_vars.parent.mkdir(parents=True, exist_ok=True)
        plan.request.firmware_code.write_text("dry-run firmware code\n")
        plan.request.firmware_vars.write_text("dry-run firmware vars\n")
        vars_copy.write_text("dry-run writable vars copy\n")
        return
    shutil.copyfile(plan.request.firmware_vars, vars_copy)


def _prepare_system_disk(plan: SmokePlan) -> None:
    if plan.request.system_disk_path.exists():
        plan.request.system_disk_path.unlink()
    if plan.request.dry_run:
        plan.request.system_disk_path.touch()
        return
    subprocess.run(
        [
            "qemu-img",
            "create",
            "-f",
            "qcow2",
            str(plan.request.system_disk_path),
            f"{plan.request.system_disk_gb}G",
        ],
        check=True,
        stdout=subprocess.DEVNULL,
    )


def _seed_uefi_boot_root(plan: SmokePlan) -> None:
    if plan.request.boot_mode != "uefi" or plan.request.lane != "backend":
        return
    assert plan.startup_script is not None
    startup_path = plan.request.boot_root / "startup.nsh"
    startup_path.write_text(plan.startup_script)
    destination = plan.request.boot_root / "EFI" / "BOOT" / "BOOTX64.EFI"
    if plan.bootloader_seed_path:
        source = Path(plan.bootloader_seed_path)
        if source.is_file():
            shutil.copyfile(source, destination)
            return
    destination.write_text("dry-run placeholder bootloader\n")


def _write_plan_json(plan: SmokePlan) -> None:
    output = plan.request.log_dir / "plan.json"
    output.write_text(json.dumps(plan.to_json_dict(), indent=2, sort_keys=True) + "\n")


def _materialize_dry_run(plan: SmokePlan) -> None:
    _prepare_workspace(plan)
    _prepare_firmware(plan)
    _prepare_system_disk(plan)
    _seed_uefi_boot_root(plan)
    _write_plan_json(plan)
    for command in plan.commands:
        if plan.request.lane == "custom-image" and command.name == "custom-image-build":
            if plan.request.custom_image_output_iso and plan.request.custom_image_output_iso.is_file():
                continue
        _write_command_log(command)
    for helper in plan.helpers:
        _write_command_log(helper.command)


def _http_json(method: str, url: str, payload: dict[str, object] | None = None) -> None:
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(url, method=method, data=data)
    if data is not None:
        request.add_header("content-type", "application/json")
    with urllib.request.urlopen(request):
        return


def _fetch_file(url: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url) as response:
        destination.write_bytes(response.read())


def _sync_boot_root(plan: SmokePlan) -> None:
    if not plan.boot_sync_paths:
        return
    base = f"http://{plan.request.api_host}:{plan.request.api_port}/boot"
    if plan.request.distro == "ubuntu":
        grub_cfg = plan.request.run_dir / "grub.cfg"
        _fetch_file(f"{base}/ubuntu/uefi/grubx64.efi", plan.request.boot_root / "EFI" / "BOOT" / "BOOTX64.EFI")
        _fetch_file(f"{base}/ubuntu/uefi/grub.cfg", grub_cfg)
        for alias in [
            plan.request.boot_root / "grub" / "grub.cfg",
            plan.request.boot_root / "boot" / "grub" / "grub.cfg",
            plan.request.boot_root / "ubuntu" / "uefi" / "grub.cfg",
            plan.request.boot_root / "ubuntu" / "uefi" / "grub" / "grub.cfg",
        ]:
            alias.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(grub_cfg, alias)
        grub_cfg.unlink()
        return

    grub_cfg = plan.request.run_dir / "grub.cfg"
    _fetch_file(f"{base}/fedora/uefi/shimx64.efi", plan.request.boot_root / "EFI" / "BOOT" / "BOOTX64.EFI")
    _fetch_file(f"{base}/fedora/uefi/grubx64.efi", plan.request.boot_root / "EFI" / "BOOT" / "grubx64.efi")
    _fetch_file(f"{base}/fedora/uefi/grub.cfg", grub_cfg)
    for alias in [
        plan.request.boot_root / "EFI" / "BOOT" / "grub.cfg",
        plan.request.boot_root / "EFI" / "fedora" / "grub.cfg",
        plan.request.boot_root / "grub" / "grub.cfg",
        plan.request.boot_root / "boot" / "grub" / "grub.cfg",
        plan.request.boot_root / "grub2" / "grub.cfg",
        plan.request.boot_root / "boot" / "grub2" / "grub.cfg",
        plan.request.boot_root / "fedora" / "uefi" / "grub.cfg",
        plan.request.boot_root / "fedora" / "uefi" / "grub" / "grub.cfg",
    ]:
        alias.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(grub_cfg, alias)
    grub_cfg.unlink()


def _probe_assets(plan: SmokePlan) -> None:
    base = f"http://{plan.request.api_host}:{plan.request.api_port}/boot"
    for relative_path in plan.probe_paths:
        with urllib.request.urlopen(f"{base}/{relative_path}"):
            continue


def _run_logged(command: list[str], *, stdout_path: Path | None = None, env: dict[str, str] | None = None, background: bool = False) -> subprocess.Popen[bytes] | subprocess.CompletedProcess[bytes]:
    if background:
        stdout = open(stdout_path, "ab") if stdout_path else subprocess.DEVNULL
        return subprocess.Popen(command, stdout=stdout, stderr=subprocess.STDOUT, env=env)
    if stdout_path:
        with open(stdout_path, "ab") as handle:
            return subprocess.run(command, check=True, stdout=handle, stderr=subprocess.STDOUT, env=env)
    return subprocess.run(command, check=True, env=env)


def _build_backend_env(plan: SmokePlan) -> dict[str, str]:
    env = os.environ.copy()
    env.update(
        {
            "BOOPA_API_BIND": f"{plan.request.api_bind_host}:{plan.request.api_port}",
            "BOOPA_TFTP_BIND": f"{plan.request.tftp_bind_host}:{plan.request.tftp_port}",
            "BOOPA_TFTP_ADVERTISE_ADDR": f"{plan.request.guest_host_ip}:{plan.request.tftp_port}",
            "BOOPA_DATA_DIR": str(plan.request.service_data_dir),
            "BOOPA_FRONTEND_DIR": str(plan.request.frontend_dir),
        }
    )
    if plan.request.network_mode in {"vmnet-host", "vde"} and plan.request.dhcp_upstream_port is not None:
        env.update(
            {
                "BOOPA_DHCP_MODE": "authoritative",
                "BOOPA_DHCP_BIND": f"127.0.0.1:{plan.request.dhcp_upstream_port}",
                "BOOPA_DHCP_SUBNET": str(plan.request.dhcp_subnet),
                "BOOPA_DHCP_POOL_START": str(plan.request.dhcp_pool_start),
                "BOOPA_DHCP_POOL_END": str(plan.request.dhcp_pool_end),
                "BOOPA_DHCP_ROUTER": str(plan.request.dhcp_router),
            }
        )
    return env


def _start_vde_switch(plan: SmokePlan) -> subprocess.Popen[bytes] | None:
    if plan.request.network_mode != "vde" or plan.request.vde_switch_dir is None or plan.request.vde_switch_pidfile is None:
        return None
    plan.request.vde_switch_dir.mkdir(parents=True, exist_ok=True)
    _run_logged(
        ["vde_switch", "-s", str(plan.request.vde_switch_dir), "-d", "-p", str(plan.request.vde_switch_pidfile)]
    )
    control_socket = plan.request.vde_switch_dir / "ctl"
    deadline = time.time() + 5
    while time.time() < deadline:
        if control_socket.exists():
            return None
        time.sleep(0.1)
    raise SmokeError(f"vde_switch did not create {control_socket}")


def _start_backend(plan: SmokePlan) -> subprocess.Popen[bytes]:
    env = _build_backend_env(plan)
    return _run_logged(
        ["cargo", "run", "-p", "boopa", "--quiet"],
        stdout_path=plan.request.backend_log,
        env=env,
        background=True,
    )  # type: ignore[return-value]


def _wait_for_backend(plan: SmokePlan, backend_proc: subprocess.Popen[bytes]) -> None:
    health_url = f"http://{plan.request.api_host}:{plan.request.api_port}/api/health"
    for _ in range(60):
        if backend_proc.poll() is not None:
            raise SmokeError(f"boopa exited before becoming healthy; see {plan.request.backend_log}")
        try:
            with urllib.request.urlopen(health_url):
                return
        except urllib.error.URLError:
            time.sleep(1)
    raise SmokeError(f"boopa did not become healthy; see {plan.request.backend_log}")


def _refresh_backend_assets(plan: SmokePlan) -> None:
    _http_json(
        "PUT",
        f"http://{plan.request.api_host}:{plan.request.api_port}/api/selection",
        {"distro": plan.request.distro},
    )
    _http_json(
        "POST",
        f"http://{plan.request.api_host}:{plan.request.api_port}/api/cache/refresh",
        {"distro": plan.request.distro, "mode": plan.request.boot_mode},
    )


def _start_vmnet_helper(plan: SmokePlan) -> subprocess.CompletedProcess[bytes] | None:
    helper = next((item for item in plan.helpers if item.name == "dhcp-relay"), None)
    if helper is None:
        return None
    return _run_logged(helper.command.argv)


def _start_vde_helper(plan: SmokePlan) -> subprocess.Popen[bytes] | None:
    helper = next((item for item in plan.helpers if item.name == "vde-host-helper"), None)
    if helper is None:
        return None
    return _run_logged(helper.command.argv, stdout_path=plan.request.host_helper_log, background=True)  # type: ignore[return-value]


def _mark_backend_evidence_offset(plan: SmokePlan) -> int:
    if not plan.request.backend_log.exists():
        return 0
    return plan.request.backend_log.stat().st_size


def _http_evidence_seen(plan: SmokePlan) -> bool:
    if plan.http_evidence_path is None:
        return True
    if plan.request.network_mode == "vmnet-host" and plan.request.backend_log.exists():
        return plan.http_evidence_path in plan.request.backend_log.read_text(errors="ignore")
    if plan.request.network_mode == "vde" and plan.request.host_helper_log.exists():
        return f"/boot/{plan.http_evidence_path}" in plan.request.host_helper_log.read_text(errors="ignore")
    return True


def _wait_for_markers(plan: SmokePlan, qemu_proc: subprocess.Popen[bytes]) -> None:
    deadline = time.time() + plan.request.timeout_secs
    while time.time() < deadline:
        serial_text = plan.request.serial_log.read_text(errors="ignore") if plan.request.serial_log.exists() else ""
        if serial_text and any(marker in serial_text for marker in plan.request.ideal_markers.split("|")):
            return
        if serial_text and any(marker in serial_text for marker in plan.request.fallback_markers.split("|")):
            if _http_evidence_seen(plan):
                return
        if qemu_proc.poll() is not None:
            qemu_output = plan.request.qemu_log.read_text(errors="ignore") if plan.request.qemu_log.exists() else ""
            if "cannot create vmnet interface" in qemu_output:
                raise SmokeError(
                    "qemu vmnet-host backend failed; this host/qemu combination does not permit creating the vmnet interface without extra privileges or entitlements"
                )
            raise SmokeError(f"qemu exited before success markers were observed; see {plan.request.qemu_log}")
        time.sleep(2)
    raise SmokeError(f"no success markers matched before timeout; inspect {plan.request.serial_log} and {plan.request.qemu_log}")


def _verify_guest_evidence(plan: SmokePlan, backend_offset: int) -> None:
    if plan.request.lane != "backend":
        return
    prefix = f"{plan.request.distro}/{plan.request.boot_mode}"
    if plan.request.network_mode == "vmnet-host":
        if not plan.request.backend_log.exists():
            raise SmokeError("backend log missing for guest evidence verification")
        content = plan.request.backend_log.read_text(errors="ignore")
        guest_content = content[backend_offset:]
        (plan.request.log_dir / "backend-guest-evidence.log").write_text(guest_content)
        required = [
            "dhcp lease response",
            f"served_path = {prefix}/kernel",
            f"served_path = {prefix}/initrd",
        ]
        for item in required:
            if item not in guest_content:
                raise SmokeError(f"guest-path run lacked evidence: {item}")
        if plan.http_evidence_path and f"requested_path = {plan.http_evidence_path}" not in guest_content:
            raise SmokeError(f"guest-path run lacked HTTP evidence for {plan.http_evidence_path}")
        return
    if plan.request.network_mode == "vde":
        if not plan.request.host_helper_log.exists():
            raise SmokeError("host helper log missing for VDE guest evidence verification")
        content = plan.request.host_helper_log.read_text(errors="ignore")
        if "dhcp relay" not in content:
            raise SmokeError("vde guest-path run lacked DHCP relay evidence")
        if f"{prefix}/kernel" not in content:
            raise SmokeError("vde guest-path run lacked kernel TFTP evidence")
        if plan.http_evidence_path and f"/boot/{plan.http_evidence_path}" not in content:
            raise SmokeError(f"vde guest-path run lacked HTTP evidence for {plan.http_evidence_path}")


def _run_qemu(plan: SmokePlan) -> subprocess.Popen[bytes] | None:
    qemu = next(command for command in plan.commands if command.name == "qemu")
    if plan.request.interactive:
        _run_logged(qemu.argv)
        return None
    return _run_logged(qemu.argv, stdout_path=plan.request.qemu_log, background=True)  # type: ignore[return-value]


def _cleanup(
    plan: SmokePlan,
    backend_proc: subprocess.Popen[bytes] | None,
    vde_helper_proc: subprocess.Popen[bytes] | None,
    qemu_proc: subprocess.Popen[bytes] | None,
) -> None:
    def stop_process(proc: subprocess.Popen[bytes] | None, label: str) -> None:
        if proc is None or proc.poll() is not None:
            return
        proc.terminate()
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            proc.kill()
            try:
                proc.wait(timeout=5)
            except subprocess.TimeoutExpired as error:
                raise SmokeError(f"{label} did not exit after terminate/kill") from error

    stop_process(qemu_proc, "qemu")
    stop_process(backend_proc, "backend")
    stop_process(vde_helper_proc, "vde host helper")
    if plan.request.vde_switch_pidfile and plan.request.vde_switch_pidfile.exists():
        pid = plan.request.vde_switch_pidfile.read_text().strip()
        if pid:
            subprocess.run(["kill", pid], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if plan.request.dhcp_helper_name:
        subprocess.run(
            ["podman", "stop", plan.request.dhcp_helper_name],
            check=False,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )


def execute(plan: SmokePlan) -> None:
    if plan.request.dry_run:
        _materialize_dry_run(plan)
        return

    if not plan.support.execution_ready and plan.request.command == "run":
        raise SmokeError(
            f"{plan.request.distro} {plan.request.boot_mode} is modeled but not yet verified for live execution on this host; use plan or dry-run"
        )

    _prepare_workspace(plan)
    _prepare_firmware(plan)
    _prepare_system_disk(plan)
    _seed_uefi_boot_root(plan)
    _write_plan_json(plan)
    for command in plan.commands:
      _write_command_log(command)
    for helper in plan.helpers:
      _write_command_log(helper.command)

    if plan.request.lane == "custom-image":
        if plan.request.custom_image_output_iso is None:
            raise SmokeError("CUSTOM_IMAGE_OUTPUT_ISO must be set")
        if not plan.request.custom_image_output_iso.is_file():
            build = next(command for command in plan.commands if command.name == "custom-image-build")
            _run_logged(build.argv, stdout_path=plan.request.custom_image_build_log)
        qemu_proc = _run_qemu(plan)
        if qemu_proc is not None:
            _wait_for_markers(plan, qemu_proc)
            _cleanup(plan, None, None, qemu_proc)
        return

    backend_proc: subprocess.Popen[bytes] | None = None
    vde_helper_proc: subprocess.Popen[bytes] | None = None
    qemu_proc: subprocess.Popen[bytes] | None = None
    backend_offset = 0
    try:
        _start_vde_switch(plan)
        backend_proc = _start_backend(plan)
        _wait_for_backend(plan, backend_proc)
        _refresh_backend_assets(plan)
        _sync_boot_root(plan)
        _probe_assets(plan)
        _start_vmnet_helper(plan)
        vde_helper_proc = _start_vde_helper(plan)
        backend_offset = _mark_backend_evidence_offset(plan)
        qemu_proc = _run_qemu(plan)
        if qemu_proc is not None:
            _wait_for_markers(plan, qemu_proc)
        _verify_guest_evidence(plan, backend_offset)
    finally:
        _cleanup(plan, backend_proc, vde_helper_proc, qemu_proc)
