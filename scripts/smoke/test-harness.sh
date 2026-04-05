#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/boopa-smoke-test.XXXXXX")"
trap 'rm -rf "${TMP_DIR}"' EXIT

assert_contains() {
  local needle="$1"
  local file_path="$2"
  if ! grep -F -q "${needle}" "${file_path}"; then
    echo "expected to find '${needle}' in ${file_path}" >&2
    exit 1
  fi
}

unsupported_log="${TMP_DIR}/unsupported.log"
if "${REPO_ROOT}/scripts/smoke/common.sh" fedora uefi >"${unsupported_log}" 2>&1; then
  echo "expected unsupported target to fail" >&2
  exit 1
fi
assert_contains "only ubuntu uefi is implemented right now" "${unsupported_log}"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170000Z" \
SMOKE_API_PORT=18080 \
SMOKE_TFTP_PORT=16969 \
"${REPO_ROOT}/scripts/smoke/common.sh" ubuntu uefi >"${TMP_DIR}/dry-run.log"

GRUB_CFG="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/grub/grub.cfg"
QEMU_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/logs/qemu-command.txt"
BOOTX64="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/EFI/BOOT/BOOTX64.EFI"
STARTUP_NSH="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/startup.nsh"

[[ -f "${GRUB_CFG}" ]] || { echo "expected grub config at ${GRUB_CFG}" >&2; exit 1; }
[[ -f "${QEMU_CMD_LOG}" ]] || { echo "expected qemu command log at ${QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${BOOTX64}" ]] || { echo "expected bootloader at ${BOOTX64}" >&2; exit 1; }
[[ -f "${STARTUP_NSH}" ]] || { echo "expected startup.nsh at ${STARTUP_NSH}" >&2; exit 1; }

assert_contains "Booting Ubuntu UEFI installer through boopa TFTP" "${GRUB_CFG}"
assert_contains "root=(tftp,10.0.2.2:16969)" "${GRUB_CFG}"
assert_contains "/ubuntu/uefi/kernel" "${GRUB_CFG}"
assert_contains "file=fat:rw:${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root" "${QEMU_CMD_LOG}"
assert_contains "BOOTX64.EFI" "${STARTUP_NSH}"

echo "smoke harness dry-run regression checks passed"
