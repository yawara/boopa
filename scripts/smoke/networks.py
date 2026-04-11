from __future__ import annotations

from pathlib import Path

from .models import CommandSpec, HelperSpec, SmokeRequest


def qemu_network_args(request: SmokeRequest) -> list[str]:
    if request.network_mode == "user":
        return ["-netdev", "user,id=net0,ipv6=off", "-device", "e1000,netdev=net0"]
    if request.network_mode == "vmnet-host":
        return [
            "-netdev",
            "vmnet-host,id=net0,isolated=on,"
            f"net-uuid={request.vmnet_net_uuid},"
            f"start-address={request.vmnet_start_address},"
            f"end-address={request.vmnet_end_address},"
            f"subnet-mask={request.vmnet_subnet_mask}",
            "-device",
            "e1000,netdev=net0",
        ]
    return [
        "-netdev",
        f"vde,id=net0,sock={request.vde_switch_dir}",
        "-device",
        "e1000,netdev=net0",
    ]


def helper_specs(request: SmokeRequest) -> list[HelperSpec]:
    helpers: list[HelperSpec] = []
    if request.network_mode == "vmnet-host":
        helpers.append(
            HelperSpec(
                name="dhcp-relay",
                command=CommandSpec(
                    name="dhcp-relay",
                    argv=[
                        "podman",
                        "run",
                        "--rm",
                        "-d",
                        "--name",
                        str(request.dhcp_helper_name),
                        "-p",
                        f"{request.dhcp_host_port}:67/udp",
                        "-v",
                        f"{request.repo_root / 'scripts/smoke/dhcp_relay.py'}:/relay.py:ro",
                        str(request.dhcp_helper_image),
                        "python3",
                        "/relay.py",
                        "--listen-port",
                        "67",
                        "--upstream-host",
                        "host.containers.internal",
                        "--upstream-port",
                        str(request.dhcp_upstream_port),
                    ],
                    log_path=str(request.dhcp_helper_cmd_log),
                    side_effect="bridges privileged guest DHCP traffic back to the host boopa process",
                ),
            )
        )
    if request.network_mode == "vde":
        helpers.append(
            HelperSpec(
                name="vde-host-helper",
                command=CommandSpec(
                    name="vde-host-helper",
                    argv=[
                        "python3",
                        str(request.repo_root / "scripts/smoke/vde_host_helper.py"),
                        "--switch-dir",
                        str(request.vde_switch_dir),
                        "--host-ip",
                        request.guest_host_ip,
                        "--dhcp-upstream-host",
                        "127.0.0.1",
                        "--dhcp-upstream-port",
                        str(request.dhcp_upstream_port),
                        "--tftp-upstream-host",
                        "127.0.0.1",
                        "--tftp-upstream-port",
                        str(request.tftp_port),
                        "--http-upstream-host",
                        "127.0.0.1",
                        "--http-upstream-port",
                        str(request.api_port),
                    ],
                    log_path=str(request.host_helper_cmd_log),
                    side_effect="bridges guest DHCP, TFTP, and HTTP traffic over a user-space VDE switch",
                ),
            )
        )
    return helpers


def start_vde_command(request: SmokeRequest) -> list[str] | None:
    if request.network_mode != "vde":
        return None
    return [
        "vde_switch",
        "-s",
        str(request.vde_switch_dir),
        "-d",
        "-p",
        str(request.vde_switch_pidfile),
    ]
