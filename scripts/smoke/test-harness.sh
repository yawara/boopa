#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/boopa-smoke-test.XXXXXX")"
trap 'rm -rf "${TMP_DIR}"' EXIT

assert_contains() {
  local needle="$1"
  local file_path="$2"
  if ! grep -F -q -- "${needle}" "${file_path}"; then
    echo "expected to find '${needle}' in ${file_path}" >&2
    exit 1
  fi
}

assert_not_contains() {
  local needle="$1"
  local file_path="$2"
  if grep -F -q -- "${needle}" "${file_path}"; then
    echo "did not expect to find '${needle}' in ${file_path}" >&2
    exit 1
  fi
}

assert_missing() {
  local file_path="$1"
  if [[ -e "${file_path}" ]]; then
    echo "expected ${file_path} to be absent" >&2
    exit 1
  fi
}

unsupported_log="${TMP_DIR}/unsupported.log"
if "${REPO_ROOT}/scripts/smoke/common.sh" fedora uefi >"${unsupported_log}" 2>&1; then
  echo "expected unsupported target to fail" >&2
  exit 1
fi
assert_contains "only ubuntu uefi is implemented right now" "${unsupported_log}"

SOURCE_DATA_DIR="${TMP_DIR}/source-data"
SOURCE_CACHE_DIR="${SOURCE_DATA_DIR}/cache/ubuntu/uefi"
mkdir -p "${SOURCE_CACHE_DIR}"
printf 'shim-bytes\n' >"${SOURCE_CACHE_DIR}/grubx64.efi"
printf 'kernel-bytes\n' >"${SOURCE_CACHE_DIR}/kernel"
printf 'initrd-bytes\n' >"${SOURCE_CACHE_DIR}/initrd"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170000Z" \
SMOKE_API_PORT=18080 \
SMOKE_TFTP_PORT=16969 \
SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}" \
"${REPO_ROOT}/scripts/smoke/common.sh" ubuntu uefi >"${TMP_DIR}/dry-run.log"

QEMU_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/logs/qemu-command.txt"
BOOTX64="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/EFI/BOOT/BOOTX64.EFI"
STARTUP_NSH="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/startup.nsh"
GRUB_CFG="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/grub/grub.cfg"
GRUB_CFG_ALIAS="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/boot/grub/grub.cfg"
GRUB_CFG_ALIAS_NESTED="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/ubuntu/uefi/grub/grub.cfg"
KERNEL_PATH="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/ubuntu/uefi/kernel"
INITRD_PATH="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root/ubuntu/uefi/initrd"

[[ -f "${QEMU_CMD_LOG}" ]] || { echo "expected qemu command log at ${QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${BOOTX64}" ]] || { echo "expected bootloader at ${BOOTX64}" >&2; exit 1; }
[[ -f "${STARTUP_NSH}" ]] || { echo "expected startup.nsh at ${STARTUP_NSH}" >&2; exit 1; }
[[ ! -e "${GRUB_CFG}" ]] || { echo "did not expect locally staged grub config at ${GRUB_CFG}" >&2; exit 1; }
[[ ! -e "${KERNEL_PATH}" ]] || { echo "did not expect locally staged kernel at ${KERNEL_PATH}" >&2; exit 1; }
[[ ! -e "${INITRD_PATH}" ]] || { echo "did not expect locally staged initrd at ${INITRD_PATH}" >&2; exit 1; }

assert_contains "file=fat:rw:${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root" "${QEMU_CMD_LOG}"
assert_contains "BOOTX64.EFI" "${STARTUP_NSH}"
assert_contains "file=fat:rw:${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root" "${QEMU_CMD_LOG}"
assert_not_contains "-kernel" "${QEMU_CMD_LOG}"
assert_not_contains "-initrd" "${QEMU_CMD_LOG}"
assert_not_contains "/boot-root/ubuntu/uefi/kernel" "${QEMU_CMD_LOG}"
assert_not_contains "/boot-root/ubuntu/uefi/initrd" "${QEMU_CMD_LOG}"

assert_missing "${GRUB_CFG}"
assert_missing "${GRUB_CFG_ALIAS}"
assert_missing "${GRUB_CFG_ALIAS_NESTED}"
assert_missing "${KERNEL_PATH}"
assert_missing "${INITRD_PATH}"

SYNC_RUN_DIR="${TMP_DIR}/sync/ubuntu-uefi-20260405T170500Z"
SYNC_BOOT_ROOT="${SYNC_RUN_DIR}/boot-root"
SYNC_GRUB_CFG="${SYNC_BOOT_ROOT}/grub/grub.cfg"
SYNC_GRUB_CFG_ALIAS="${SYNC_BOOT_ROOT}/boot/grub/grub.cfg"
SYNC_UBUNTU_GRUB_CFG="${SYNC_BOOT_ROOT}/ubuntu/uefi/grub.cfg"
SYNC_GRUB_CFG_ALIAS_NESTED="${SYNC_BOOT_ROOT}/ubuntu/uefi/grub/grub.cfg"
FETCH_LOG="${SYNC_RUN_DIR}/fetch.log"

(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  smoke_fetch_backend_asset() {
    local asset_path="$1"
    local destination_path="$2"
    mkdir -p "$(dirname "${destination_path}")"
    printf '%s\n' "${asset_path}" >>"${FETCH_LOG}"
    case "${asset_path}" in
      "ubuntu/uefi/grubx64.efi")
        printf 'shim-from-boopa\n' >"${destination_path}"
        ;;
      "ubuntu/uefi/grub.cfg")
        cat >"${destination_path}" <<'EOF'
set default=0
set timeout=2

menuentry "boopa ubuntu uefi smoke" {
    linux /ubuntu/uefi/kernel ip=dhcp console=ttyS0,115200n8 ---
    initrd /ubuntu/uefi/initrd
    boot
}
EOF
        ;;
      *)
        echo "unexpected backend fetch for ${asset_path}" >&2
        exit 1
        ;;
    esac
  }

  SMOKE_RUN_DIR="${SYNC_RUN_DIR}"
  SMOKE_SERVICE_DATA_DIR="${SYNC_RUN_DIR}/service-data"
  SMOKE_TFTP_ROOT="${SYNC_BOOT_ROOT}"
  SMOKE_LOG_DIR="${SYNC_RUN_DIR}/logs"

  smoke_prepare_workspace
  smoke_sync_boot_root_from_backend
)

[[ -f "${SYNC_GRUB_CFG}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG}" >&2; exit 1; }
[[ -f "${SYNC_GRUB_CFG_ALIAS}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG_ALIAS}" >&2; exit 1; }
[[ -f "${SYNC_UBUNTU_GRUB_CFG}" ]] || { echo "expected staged grub config at ${SYNC_UBUNTU_GRUB_CFG}" >&2; exit 1; }
[[ -f "${SYNC_GRUB_CFG_ALIAS_NESTED}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG_ALIAS_NESTED}" >&2; exit 1; }
[[ -f "${FETCH_LOG}" ]] || { echo "expected fetch log at ${FETCH_LOG}" >&2; exit 1; }

assert_contains "ubuntu/uefi/grubx64.efi" "${FETCH_LOG}"
assert_contains "ubuntu/uefi/grub.cfg" "${FETCH_LOG}"
assert_not_contains "ubuntu/uefi/kernel" "${FETCH_LOG}"
assert_not_contains "ubuntu/uefi/initrd" "${FETCH_LOG}"

assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG_ALIAS}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG_ALIAS}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_UBUNTU_GRUB_CFG}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_UBUNTU_GRUB_CFG}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG_ALIAS_NESTED}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG_ALIAS_NESTED}"

echo "smoke harness dry-run regression checks passed"
