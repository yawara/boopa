from __future__ import annotations

from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any


class SmokeError(RuntimeError):
    """Raised when smoke planning or execution cannot continue."""


@dataclass(frozen=True)
class CommandSpec:
    name: str
    argv: list[str]
    log_path: str | None = None
    background: bool = False
    side_effect: str | None = None


@dataclass(frozen=True)
class HelperSpec:
    name: str
    command: CommandSpec
    required_for_execution: bool = True


@dataclass(frozen=True)
class ArtifactSpec:
    path: str
    purpose: str


@dataclass(frozen=True)
class SupportStatus:
    plan_supported: bool
    execution_ready: bool
    level: str
    notes: list[str]


@dataclass(frozen=True)
class SmokeRequest:
    repo_root: Path
    command: str
    distro: str
    boot_mode: str
    lane: str
    network_mode: str
    dry_run: bool
    interactive: bool
    qemu_display: str
    qemu_bin: str
    qemu_accel: str
    ram_mb: int
    system_disk_gb: int
    timeout_secs: int
    timestamp: str
    work_root: Path
    run_dir: Path
    service_data_dir: Path
    boot_root: Path
    log_dir: Path
    serial_log: Path
    backend_log: Path
    debug_log: Path
    qemu_log: Path
    qemu_cmd_log: Path
    dhcp_helper_cmd_log: Path
    host_helper_log: Path
    host_helper_cmd_log: Path
    custom_image_build_log: Path
    custom_image_build_cmd_log: Path
    system_disk_path: Path
    frontend_dir: Path
    source_data_dir: Path
    cache_source_dir: Path
    api_host: str
    api_bind_host: str
    api_port: int
    tftp_bind_host: str
    tftp_port: int
    guest_host_ip: str
    vmnet_net_uuid: str | None = None
    vmnet_start_address: str | None = None
    vmnet_end_address: str | None = None
    vmnet_subnet_mask: str | None = None
    dhcp_helper_mode: str = "none"
    dhcp_helper_image: str | None = None
    dhcp_helper_name: str | None = None
    dhcp_host_port: int | None = None
    dhcp_upstream_port: int | None = None
    dhcp_subnet: str | None = None
    dhcp_pool_start: str | None = None
    dhcp_pool_end: str | None = None
    dhcp_router: str | None = None
    vde_switch_dir: Path | None = None
    vde_switch_pidfile: Path | None = None
    vde_helper_mode: str = "none"
    custom_image_base_iso: Path | None = None
    custom_image_manifest: Path | None = None
    custom_image_output_iso: Path | None = None
    firmware_code: Path | None = None
    firmware_vars: Path | None = None
    ideal_markers: str = ""
    fallback_markers: str = ""
    support: SupportStatus | None = None


@dataclass
class SmokePlan:
    request: SmokeRequest
    title: str
    summary_lines: list[str]
    support: SupportStatus
    inputs: list[str]
    side_effects: list[str]
    artifacts: list[ArtifactSpec]
    steps: list[str]
    commands: list[CommandSpec]
    helpers: list[HelperSpec] = field(default_factory=list)
    probe_paths: list[str] = field(default_factory=list)
    boot_sync_paths: list[str] = field(default_factory=list)
    bootloader_seed_path: str | None = None
    startup_script: str | None = None
    http_evidence_path: str | None = None
    structured_notes: list[str] = field(default_factory=list)

    def to_json_dict(self) -> dict[str, Any]:
        return {
            "title": self.title,
            "request": {
                key: (str(value) if isinstance(value, Path) else value)
                for key, value in asdict(self.request).items()
                if key != "support"
            },
            "support": asdict(self.support),
            "summaryLines": self.summary_lines,
            "inputs": self.inputs,
            "sideEffects": self.side_effects,
            "artifacts": [asdict(item) for item in self.artifacts],
            "steps": self.steps,
            "commands": [asdict(item) for item in self.commands],
            "helpers": [
                {
                    "name": helper.name,
                    "requiredForExecution": helper.required_for_execution,
                    "command": asdict(helper.command),
                }
                for helper in self.helpers
            ],
            "probePaths": self.probe_paths,
            "bootSyncPaths": self.boot_sync_paths,
            "bootloaderSeedPath": self.bootloader_seed_path,
            "startupScript": self.startup_script,
            "httpEvidencePath": self.http_evidence_path,
            "notes": self.structured_notes,
        }
