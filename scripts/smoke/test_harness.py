from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from scripts.smoke.planner import build_plan, build_request


REPO_ROOT = Path(__file__).resolve().parents[2]


def run_command(command: list[str], *, env: dict[str, str], check: bool = True) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, env=env, text=True, capture_output=True)
    if check and result.returncode != 0:
        raise AssertionError(
            f"command failed: {' '.join(command)}\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        )
    return result


def smoke_command(*args: str) -> list[str]:
    return ["python3", "-m", "scripts.smoke", *args]


class SmokeHarnessTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory(prefix="boopa-smoke-test.")
        self.tmp = Path(self.tempdir.name)
        self.source_data = self.tmp / "source-data"
        (self.source_data / "cache" / "ubuntu" / "uefi").mkdir(parents=True)
        (self.source_data / "cache" / "fedora" / "uefi").mkdir(parents=True)
        (self.source_data / "cache" / "ubuntu" / "uefi" / "grubx64.efi").write_text("grub\n")
        (self.source_data / "cache" / "fedora" / "uefi" / "shimx64.efi").write_text("shim\n")
        (self.source_data / "cache" / "fedora" / "uefi" / "grubx64.efi").write_text("grub\n")
        self.base_env = {
            **os.environ,
            "SMOKE_WORK_ROOT": str(self.tmp / "work"),
            "SMOKE_SOURCE_DATA_DIR": str(self.source_data),
            "SMOKE_API_PORT": "18080",
            "SMOKE_TFTP_PORT": "16969",
        }

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def test_arch_is_rejected(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170000Z"}
        result = run_command(
            smoke_command("run", "--distro", "arch", "--boot-mode", "bios"),
            env=env,
            check=False,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid choice: 'arch'", result.stderr)

    def test_ubuntu_uefi_dry_run_materializes_workspace(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170000Z", "SMOKE_DRY_RUN": "1"}
        run_command(
            smoke_command("run", "--distro", "ubuntu", "--boot-mode", "uefi", "--dry-run"),
            env=env,
        )
        run_dir = self.tmp / "work" / "ubuntu-uefi-20260405T170000Z"
        self.assertTrue((run_dir / "logs/qemu-command.txt").is_file())
        self.assertTrue((run_dir / "boot-root/EFI/BOOT/BOOTX64.EFI").is_file())
        self.assertTrue((run_dir / "boot-root/startup.nsh").is_file())
        self.assertTrue((run_dir / "system-disk.qcow2").exists())
        self.assertTrue((run_dir / "service-data/cache").is_symlink())
        plan = json.loads((run_dir / "logs/plan.json").read_text())
        self.assertEqual(plan["support"]["plan_supported"], True)
        self.assertEqual(plan["request"]["distro"], "ubuntu")
        qemu_cmd = (run_dir / "logs/qemu-command.txt").read_text()
        self.assertIn("file=fat:rw:", qemu_cmd)
        self.assertNotIn("-kernel", qemu_cmd)

    def test_fedora_uefi_dry_run_materializes_workspace(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170010Z", "SMOKE_DRY_RUN": "1"}
        run_command(
            smoke_command("run", "--distro", "fedora", "--boot-mode", "uefi", "--dry-run"),
            env=env,
        )
        run_dir = self.tmp / "work" / "fedora-uefi-20260405T170010Z"
        self.assertTrue((run_dir / "logs/qemu-command.txt").is_file())
        self.assertTrue((run_dir / "boot-root/EFI/BOOT/BOOTX64.EFI").is_file())
        self.assertTrue((run_dir / "system-disk.qcow2").exists())

    def test_vmnet_and_vde_helper_logs_are_planned(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170050Z", "SMOKE_DRY_RUN": "1", "SMOKE_NETWORK_MODE": "vmnet-host"}
        run_command(
            smoke_command("run", "--distro", "ubuntu", "--boot-mode", "uefi", "--network-mode", "vmnet-host", "--dry-run"),
            env=env,
        )
        vmnet_dir = self.tmp / "work" / "ubuntu-uefi-20260405T170050Z"
        self.assertIn("vmnet-host,id=net0", (vmnet_dir / "logs/qemu-command.txt").read_text())
        self.assertIn("podman run", (vmnet_dir / "logs/dhcp-helper-command.txt").read_text())

        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170060Z", "SMOKE_DRY_RUN": "1", "SMOKE_NETWORK_MODE": "vde"}
        run_command(
            smoke_command("run", "--distro", "ubuntu", "--boot-mode", "uefi", "--network-mode", "vde", "--dry-run"),
            env=env,
        )
        vde_dir = self.tmp / "work" / "ubuntu-uefi-20260405T170060Z"
        self.assertIn("vde,id=net0", (vde_dir / "logs/qemu-command.txt").read_text())
        self.assertIn("vde_host_helper.py", (vde_dir / "logs/host-helper-command.txt").read_text())

    def test_custom_image_lane_uses_python_surface(self) -> None:
        custom_dir = self.tmp / "custom"
        custom_dir.mkdir()
        base_iso = custom_dir / "base.iso"
        manifest = custom_dir / "manifest.yaml"
        output_iso = custom_dir / "custom.iso"
        base_iso.write_text("base\n")
        manifest.write_text("packages:\n  - openssh-server\n")
        env = {
            **self.base_env,
            "SMOKE_TIMESTAMP": "20260405T170100Z",
            "SMOKE_DRY_RUN": "1",
            "CUSTOM_IMAGE_BASE_ISO": str(base_iso),
            "CUSTOM_IMAGE_MANIFEST": str(manifest),
            "CUSTOM_IMAGE_OUTPUT_ISO": str(output_iso),
        }
        run_command(smoke_command("custom-image", "--dry-run"), env=env)
        run_dir = self.tmp / "work" / "ubuntu-custom-image-20260405T170100Z"
        self.assertTrue((run_dir / "logs/qemu-command.txt").is_file())
        self.assertTrue((run_dir / "logs/custom-image-build-command.txt").is_file())

    def test_structured_plan_covers_bios_support_matrix(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170200Z"}
        result = run_command(
            [
                "python3",
                "-m",
                "scripts.smoke",
                "plan",
                "--distro",
                "fedora",
                "--boot-mode",
                "bios",
                "--network-mode",
                "user",
                "--format",
                "json",
            ],
            env=env,
        )
        plan = json.loads(result.stdout)
        self.assertEqual(plan["request"]["boot_mode"], "bios")
        self.assertEqual(plan["support"]["plan_supported"], True)
        self.assertEqual(plan["support"]["execution_ready"], False)
        self.assertIn("BIOS planning is explicit", " ".join(plan["support"]["notes"]))

    def test_prefers_hvf_when_qemu_advertises_it(self) -> None:
        env = {**self.base_env, "SMOKE_TIMESTAMP": "20260405T170300Z"}
        with mock.patch.dict(os.environ, env, clear=False):
            with mock.patch("scripts.smoke.planner._supported_accelerators", return_value={"tcg", "hvf"}):
                request = build_request(
                    command="plan",
                    distro="ubuntu",
                    boot_mode="uefi",
                    lane="backend",
                    network_mode="user",
                    dry_run=True,
                )
                plan = build_plan(request).to_json_dict()
        qemu_argv = next(command["argv"] for command in plan["commands"] if command["name"] == "qemu")
        accel_index = qemu_argv.index("-accel")
        self.assertEqual(qemu_argv[accel_index + 1], "hvf")

    def test_explicit_accel_override_wins(self) -> None:
        env = {
            **self.base_env,
            "SMOKE_TIMESTAMP": "20260405T170310Z",
            "SMOKE_QEMU_ACCEL": "tcg",
        }
        with mock.patch.dict(os.environ, env, clear=False):
            with mock.patch("scripts.smoke.planner._supported_accelerators", return_value={"tcg", "hvf"}):
                request = build_request(
                    command="plan",
                    distro="ubuntu",
                    boot_mode="uefi",
                    lane="backend",
                    network_mode="user",
                    dry_run=True,
                )
                plan = build_plan(request).to_json_dict()
        qemu_argv = next(command["argv"] for command in plan["commands"] if command["name"] == "qemu")
        accel_index = qemu_argv.index("-accel")
        self.assertEqual(qemu_argv[accel_index + 1], "tcg")


def main() -> int:
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(SmokeHarnessTests)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    return 0 if result.wasSuccessful() else 1


if __name__ == "__main__":
    raise SystemExit(main())
