from __future__ import annotations

import argparse
import sys

from .executor import execute
from .models import SmokeError
from .plan_render import render_json, render_text
from .planner import build_plan, build_request


def _common_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--distro", choices=["ubuntu", "fedora"], default="ubuntu")
    parser.add_argument("--boot-mode", choices=["uefi", "bios"], default="uefi")
    parser.add_argument("--network-mode", choices=["user", "vmnet-host", "vde"], default=None)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--format", choices=["text", "json"], default="text")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Canonical Python surface for the boopa smoke harness")
    subparsers = parser.add_subparsers(dest="command", required=True)

    run_parser = subparsers.add_parser("run", help="plan and execute a backend smoke run")
    _common_arguments(run_parser)

    plan_parser = subparsers.add_parser("plan", help="render a structured execution plan without executing it")
    _common_arguments(plan_parser)

    custom_parser = subparsers.add_parser("custom-image", help="run the Ubuntu custom-image smoke lane")
    custom_parser.add_argument("--dry-run", action="store_true")
    custom_parser.add_argument("--format", choices=["text", "json"], default="text")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        if args.command == "custom-image":
            request = build_request(
                command="custom-image",
                distro="ubuntu",
                boot_mode="uefi",
                lane="custom-image",
                network_mode="user",
                dry_run=args.dry_run,
            )
        else:
            network_mode = args.network_mode or "user"
            request = build_request(
                command=args.command,
                distro=args.distro,
                boot_mode=args.boot_mode,
                lane="backend",
                network_mode=network_mode,
                dry_run=args.dry_run or args.command == "plan",
            )
        plan = build_plan(request)
        rendered = render_json(plan) if args.format == "json" else render_text(plan)
        sys.stdout.write(rendered)
        if args.command != "plan":
            execute(plan)
        return 0
    except SmokeError as error:
        print(f"smoke: {error}", file=sys.stderr)
        return 1
