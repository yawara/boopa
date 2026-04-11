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

assert_symlink_target() {
  local link_path="$1"
  local expected_target="$2"
  if [[ ! -L "${link_path}" ]]; then
    echo "expected ${link_path} to be a symlink" >&2
    exit 1
  fi

  local actual_target
  actual_target="$(readlink "${link_path}")"
  if [[ "${actual_target}" != "${expected_target}" ]]; then
    echo "expected ${link_path} -> ${expected_target}, got ${actual_target}" >&2
    exit 1
  fi
}

unsupported_log="${TMP_DIR}/unsupported.log"
if "${REPO_ROOT}/scripts/smoke/common.sh" arch bios >"${unsupported_log}" 2>&1; then
  echo "expected unsupported target to fail" >&2
  exit 1
fi
assert_contains "unsupported target" "${unsupported_log}"

SOURCE_DATA_DIR="${TMP_DIR}/source-data"
SOURCE_CACHE_DIR="${SOURCE_DATA_DIR}/cache/ubuntu/uefi"
mkdir -p "${SOURCE_CACHE_DIR}"
printf 'shim-bytes\n' >"${SOURCE_CACHE_DIR}/grubx64.efi"
printf 'kernel-bytes\n' >"${SOURCE_CACHE_DIR}/kernel"
printf 'initrd-bytes\n' >"${SOURCE_CACHE_DIR}/initrd"
printf 'iso-bytes\n' >"${SOURCE_CACHE_DIR}/live-server.iso"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170000Z" \
SMOKE_API_PORT=18080 \
SMOKE_TFTP_PORT=16969 \
SYSTEM_DISK_GB=48 \
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
SYSTEM_DISK_PATH="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/system-disk.qcow2"
SERVICE_CACHE_LINK="${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/service-data/cache"

[[ -f "${QEMU_CMD_LOG}" ]] || { echo "expected qemu command log at ${QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${BOOTX64}" ]] || { echo "expected bootloader at ${BOOTX64}" >&2; exit 1; }
[[ -f "${STARTUP_NSH}" ]] || { echo "expected startup.nsh at ${STARTUP_NSH}" >&2; exit 1; }
[[ -f "${SYSTEM_DISK_PATH}" ]] || { echo "expected installer disk at ${SYSTEM_DISK_PATH}" >&2; exit 1; }
assert_symlink_target "${SERVICE_CACHE_LINK}" "${SOURCE_DATA_DIR}/cache"
[[ ! -e "${GRUB_CFG}" ]] || { echo "did not expect locally staged grub config at ${GRUB_CFG}" >&2; exit 1; }
[[ ! -e "${KERNEL_PATH}" ]] || { echo "did not expect locally staged kernel at ${KERNEL_PATH}" >&2; exit 1; }
[[ ! -e "${INITRD_PATH}" ]] || { echo "did not expect locally staged initrd at ${INITRD_PATH}" >&2; exit 1; }

assert_contains "file=fat:rw:${TMP_DIR}/work/ubuntu-uefi-20260405T170000Z/boot-root" "${QEMU_CMD_LOG}"
assert_contains "-m 8192" "${QEMU_CMD_LOG}"
assert_contains "system-disk.qcow2" "${QEMU_CMD_LOG}"
assert_contains "format=qcow2" "${QEMU_CMD_LOG}"
assert_contains "if=virtio" "${QEMU_CMD_LOG}"
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

FEDORA_SOURCE_CACHE_DIR="${SOURCE_DATA_DIR}/cache/fedora/uefi"
mkdir -p "${FEDORA_SOURCE_CACHE_DIR}"
printf 'shim-bytes\n' >"${FEDORA_SOURCE_CACHE_DIR}/shimx64.efi"
printf 'grub-bytes\n' >"${FEDORA_SOURCE_CACHE_DIR}/grubx64.efi"
printf 'kernel-bytes\n' >"${FEDORA_SOURCE_CACHE_DIR}/kernel"
printf 'initrd-bytes\n' >"${FEDORA_SOURCE_CACHE_DIR}/initrd"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170010Z" \
SMOKE_API_PORT=18080 \
SMOKE_TFTP_PORT=16969 \
SYSTEM_DISK_GB=48 \
SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}" \
"${REPO_ROOT}/scripts/smoke/common.sh" fedora uefi >"${TMP_DIR}/fedora-dry-run.log"

FEDORA_QEMU_CMD_LOG="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/logs/qemu-command.txt"
FEDORA_BOOTX64="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root/EFI/BOOT/BOOTX64.EFI"
FEDORA_STARTUP_NSH="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root/startup.nsh"
FEDORA_GRUB_CFG="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root/grub/grub.cfg"
FEDORA_KERNEL_PATH="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root/fedora/uefi/kernel"
FEDORA_INITRD_PATH="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root/fedora/uefi/initrd"
FEDORA_SYSTEM_DISK_PATH="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/system-disk.qcow2"
FEDORA_SERVICE_CACHE_LINK="${TMP_DIR}/work/fedora-uefi-20260405T170010Z/service-data/cache"

[[ -f "${FEDORA_QEMU_CMD_LOG}" ]] || { echo "expected fedora qemu command log at ${FEDORA_QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${FEDORA_BOOTX64}" ]] || { echo "expected fedora bootloader at ${FEDORA_BOOTX64}" >&2; exit 1; }
[[ -f "${FEDORA_STARTUP_NSH}" ]] || { echo "expected fedora startup.nsh at ${FEDORA_STARTUP_NSH}" >&2; exit 1; }
[[ -f "${FEDORA_SYSTEM_DISK_PATH}" ]] || { echo "expected fedora installer disk at ${FEDORA_SYSTEM_DISK_PATH}" >&2; exit 1; }
assert_symlink_target "${FEDORA_SERVICE_CACHE_LINK}" "${SOURCE_DATA_DIR}/cache"
assert_contains "file=fat:rw:${TMP_DIR}/work/fedora-uefi-20260405T170010Z/boot-root" "${FEDORA_QEMU_CMD_LOG}"
assert_contains "-m 8192" "${FEDORA_QEMU_CMD_LOG}"
assert_contains "system-disk.qcow2" "${FEDORA_QEMU_CMD_LOG}"
assert_contains "BOOTX64.EFI" "${FEDORA_STARTUP_NSH}"
assert_not_contains "-kernel" "${FEDORA_QEMU_CMD_LOG}"
assert_not_contains "-initrd" "${FEDORA_QEMU_CMD_LOG}"
assert_missing "${FEDORA_GRUB_CFG}"
assert_missing "${FEDORA_KERNEL_PATH}"
assert_missing "${FEDORA_INITRD_PATH}"

VMNET_DRY_RUN_LOG="${TMP_DIR}/vmnet-dry-run.log"
SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170050Z" \
SMOKE_NETWORK_MODE="vmnet-host" \
SMOKE_VMNET_NET_UUID="123e4567-e89b-12d3-a456-426614174000" \
SMOKE_DHCP_HELPER_MODE="podman-relay" \
SMOKE_DHCP_UPSTREAM_PORT="1067" \
SMOKE_DHCP_HELPER_IMAGE="docker.io/library/python:3.12-alpine" \
SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}" \
"${REPO_ROOT}/scripts/smoke/common.sh" ubuntu uefi >"${VMNET_DRY_RUN_LOG}"

VMNET_QEMU_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170050Z/logs/qemu-command.txt"
VMNET_DHCP_HELPER_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170050Z/logs/dhcp-helper-command.txt"
[[ -f "${VMNET_QEMU_CMD_LOG}" ]] || { echo "expected vmnet qemu command log at ${VMNET_QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${VMNET_DHCP_HELPER_CMD_LOG}" ]] || { echo "expected DHCP helper command log at ${VMNET_DHCP_HELPER_CMD_LOG}" >&2; exit 1; }
assert_contains "vmnet-host\\,id=net0" "${VMNET_QEMU_CMD_LOG}"
assert_contains "net-uuid=123e4567-e89b-12d3-a456-426614174000" "${VMNET_QEMU_CMD_LOG}"
assert_not_contains "user\\,id=net0" "${VMNET_QEMU_CMD_LOG}"
assert_contains "podman run --rm -d --name boopa-dhcp-relay-20260405T170050Z" "${VMNET_DHCP_HELPER_CMD_LOG}"
assert_contains "67:67/udp" "${VMNET_DHCP_HELPER_CMD_LOG}"
assert_contains "/relay.py --listen-port 67 --upstream-host host.containers.internal --upstream-port 1067" "${VMNET_DHCP_HELPER_CMD_LOG}"
assert_contains "Network mode: vmnet-host" "${VMNET_DRY_RUN_LOG}"
assert_contains "DHCP helper mode: podman-relay" "${VMNET_DRY_RUN_LOG}"

VDE_DRY_RUN_LOG="${TMP_DIR}/vde-dry-run.log"
SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170060Z" \
SMOKE_NETWORK_MODE="vde" \
SMOKE_DHCP_UPSTREAM_PORT="1067" \
SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}" \
"${REPO_ROOT}/scripts/smoke/common.sh" ubuntu uefi >"${VDE_DRY_RUN_LOG}"

VDE_QEMU_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170060Z/logs/qemu-command.txt"
VDE_HOST_HELPER_CMD_LOG="${TMP_DIR}/work/ubuntu-uefi-20260405T170060Z/logs/host-helper-command.txt"
[[ -f "${VDE_QEMU_CMD_LOG}" ]] || { echo "expected vde qemu command log at ${VDE_QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${VDE_HOST_HELPER_CMD_LOG}" ]] || { echo "expected VDE host helper command log at ${VDE_HOST_HELPER_CMD_LOG}" >&2; exit 1; }
assert_contains "vde\\,id=net0" "${VDE_QEMU_CMD_LOG}"
assert_contains "sock=${TMP_DIR}/work/ubuntu-uefi-20260405T170060Z/vde.ctl" "${VDE_QEMU_CMD_LOG}"
assert_contains "python3 ${REPO_ROOT}/scripts/smoke/vde_host_helper.py" "${VDE_HOST_HELPER_CMD_LOG}"
assert_contains "--switch-dir ${TMP_DIR}/work/ubuntu-uefi-20260405T170060Z/vde.ctl" "${VDE_HOST_HELPER_CMD_LOG}"
assert_contains "--tftp-upstream-host 127.0.0.1" "${VDE_HOST_HELPER_CMD_LOG}"
assert_contains "--http-upstream-host 127.0.0.1" "${VDE_HOST_HELPER_CMD_LOG}"
assert_contains "Network mode: vde" "${VDE_DRY_RUN_LOG}"

CUSTOM_BASE_ISO="${TMP_DIR}/custom/base.iso"
CUSTOM_MANIFEST="${TMP_DIR}/custom/manifest.yaml"
CUSTOM_EXISTING_OUTPUT_ISO="${TMP_DIR}/custom/existing-custom.iso"
mkdir -p "${TMP_DIR}/custom"
printf 'base-iso-bytes\n' >"${CUSTOM_BASE_ISO}"
cat >"${CUSTOM_MANIFEST}" <<'EOF'
packages:
  - openssh-server
  - git
EOF
printf 'custom-iso-bytes\n' >"${CUSTOM_EXISTING_OUTPUT_ISO}"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170100Z" \
CUSTOM_IMAGE_BASE_ISO="${CUSTOM_BASE_ISO}" \
CUSTOM_IMAGE_MANIFEST="${CUSTOM_MANIFEST}" \
CUSTOM_IMAGE_OUTPUT_ISO="${CUSTOM_EXISTING_OUTPUT_ISO}" \
"${REPO_ROOT}/scripts/smoke/boot-ubuntu-custom-image.sh" >"${TMP_DIR}/custom-existing.log"

CUSTOM_EXISTING_RUN_DIR="${TMP_DIR}/work/ubuntu-custom-image-20260405T170100Z"
CUSTOM_EXISTING_QEMU_CMD_LOG="${CUSTOM_EXISTING_RUN_DIR}/logs/qemu-command.txt"
CUSTOM_EXISTING_BUILD_CMD_LOG="${CUSTOM_EXISTING_RUN_DIR}/logs/custom-image-build-command.txt"
CUSTOM_EXISTING_BACKEND_LOG="${CUSTOM_EXISTING_RUN_DIR}/logs/backend.log"

[[ -f "${CUSTOM_EXISTING_QEMU_CMD_LOG}" ]] || { echo "expected custom-image qemu command log at ${CUSTOM_EXISTING_QEMU_CMD_LOG}" >&2; exit 1; }
assert_missing "${CUSTOM_EXISTING_BUILD_CMD_LOG}"
assert_missing "${CUSTOM_EXISTING_BACKEND_LOG}"
assert_contains "order=d\\,menu=off" "${CUSTOM_EXISTING_QEMU_CMD_LOG}"
assert_contains "file=${CUSTOM_EXISTING_OUTPUT_ISO}\\,media=cdrom\\,if=ide\\,index=0" "${CUSTOM_EXISTING_QEMU_CMD_LOG}"
assert_not_contains "file=fat:rw:" "${CUSTOM_EXISTING_QEMU_CMD_LOG}"
assert_not_contains "http://" "${CUSTOM_EXISTING_QEMU_CMD_LOG}"
assert_not_contains "/boot/ubuntu/uefi/" "${CUSTOM_EXISTING_QEMU_CMD_LOG}"

CUSTOM_MISSING_OUTPUT_ISO="${TMP_DIR}/custom/generated/custom.iso"
assert_missing "${CUSTOM_MISSING_OUTPUT_ISO}"

SMOKE_DRY_RUN=1 \
SMOKE_WORK_ROOT="${TMP_DIR}/work" \
SMOKE_TIMESTAMP="20260405T170200Z" \
CUSTOM_IMAGE_BASE_ISO="${CUSTOM_BASE_ISO}" \
CUSTOM_IMAGE_MANIFEST="${CUSTOM_MANIFEST}" \
CUSTOM_IMAGE_OUTPUT_ISO="${CUSTOM_MISSING_OUTPUT_ISO}" \
"${REPO_ROOT}/scripts/smoke/boot-ubuntu-custom-image.sh" >"${TMP_DIR}/custom-build.log"

CUSTOM_BUILD_RUN_DIR="${TMP_DIR}/work/ubuntu-custom-image-20260405T170200Z"
CUSTOM_BUILD_QEMU_CMD_LOG="${CUSTOM_BUILD_RUN_DIR}/logs/qemu-command.txt"
CUSTOM_BUILD_CMD_LOG="${CUSTOM_BUILD_RUN_DIR}/logs/custom-image-build-command.txt"
CUSTOM_BUILD_BACKEND_LOG="${CUSTOM_BUILD_RUN_DIR}/logs/backend.log"

[[ -f "${CUSTOM_BUILD_QEMU_CMD_LOG}" ]] || { echo "expected custom-image qemu command log at ${CUSTOM_BUILD_QEMU_CMD_LOG}" >&2; exit 1; }
[[ -f "${CUSTOM_BUILD_CMD_LOG}" ]] || { echo "expected custom-image build command log at ${CUSTOM_BUILD_CMD_LOG}" >&2; exit 1; }
assert_missing "${CUSTOM_BUILD_BACKEND_LOG}"
assert_missing "${CUSTOM_MISSING_OUTPUT_ISO}"
assert_contains "cargo run -p ubuntu-custom-image -- build --base-iso ${CUSTOM_BASE_ISO} --manifest ${CUSTOM_MANIFEST} --output ${CUSTOM_MISSING_OUTPUT_ISO}" "${CUSTOM_BUILD_CMD_LOG}"
assert_contains "order=d\\,menu=off" "${CUSTOM_BUILD_QEMU_CMD_LOG}"
assert_contains "file=${CUSTOM_MISSING_OUTPUT_ISO}\\,media=cdrom\\,if=ide\\,index=0" "${CUSTOM_BUILD_QEMU_CMD_LOG}"
assert_not_contains "file=fat:rw:" "${CUSTOM_BUILD_QEMU_CMD_LOG}"
assert_not_contains "http://" "${CUSTOM_BUILD_QEMU_CMD_LOG}"

CUSTOM_NO_BACKEND_RUN_DIR="${TMP_DIR}/work/ubuntu-custom-image-20260405T170300Z"
CUSTOM_NO_BACKEND_SENTINEL="${TMP_DIR}/custom-no-backend.log"

(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  smoke_preflight() {
    QEMU_FIRMWARE_CODE="${TMP_DIR}/custom-firmware-code.fd"
    QEMU_FIRMWARE_VARS="${TMP_DIR}/custom-firmware-vars.fd"
    printf 'code\n' >"${QEMU_FIRMWARE_CODE}"
    printf 'vars\n' >"${QEMU_FIRMWARE_VARS}"
  }

  smoke_prepare_firmware() {
    SMOKE_QEMU_VARS_COPY="${CUSTOM_NO_BACKEND_RUN_DIR}/edk2-vars.fd"
    mkdir -p "$(dirname "${SMOKE_QEMU_VARS_COPY}")"
    printf 'vars\n' >"${SMOKE_QEMU_VARS_COPY}"
  }

  smoke_prepare_system_disk() {
    mkdir -p "$(dirname "${SMOKE_SYSTEM_DISK_PATH}")"
    printf 'qcow2-placeholder\n' >"${SMOKE_SYSTEM_DISK_PATH}"
  }

  smoke_start_backend() {
    printf 'start_backend\n' >>"${CUSTOM_NO_BACKEND_SENTINEL}"
  }

  smoke_wait_for_backend() {
    printf 'wait_for_backend\n' >>"${CUSTOM_NO_BACKEND_SENTINEL}"
  }

  smoke_refresh_backend_assets() {
    printf 'refresh_backend_assets\n' >>"${CUSTOM_NO_BACKEND_SENTINEL}"
  }

  smoke_sync_boot_root_from_backend() {
    printf 'sync_boot_root_from_backend\n' >>"${CUSTOM_NO_BACKEND_SENTINEL}"
  }

  smoke_probe_assets() {
    printf 'probe_assets\n' >>"${CUSTOM_NO_BACKEND_SENTINEL}"
  }

  smoke_start_qemu() {
    printf 'start_qemu\n' >>"${TMP_DIR}/custom-flow.log"
  }

  smoke_wait_for_markers() {
    printf 'wait_for_markers\n' >>"${TMP_DIR}/custom-flow.log"
  }

  SMOKE_LANE=custom-image
  SMOKE_WORK_ROOT="${TMP_DIR}/work"
  SMOKE_TIMESTAMP="20260405T170300Z"
  SMOKE_INTERACTIVE=0
  CUSTOM_IMAGE_BASE_ISO="${CUSTOM_BASE_ISO}"
  CUSTOM_IMAGE_MANIFEST="${CUSTOM_MANIFEST}"
  CUSTOM_IMAGE_OUTPUT_ISO="${CUSTOM_EXISTING_OUTPUT_ISO}"
  smoke_main ubuntu uefi
)

assert_missing "${CUSTOM_NO_BACKEND_SENTINEL}"

SYNC_RUN_DIR="${TMP_DIR}/sync/ubuntu-uefi-20260405T170500Z"
SYNC_BOOT_ROOT="${SYNC_RUN_DIR}/boot-root"
SYNC_GRUB_CFG="${SYNC_BOOT_ROOT}/grub/grub.cfg"
SYNC_GRUB_CFG_ALIAS="${SYNC_BOOT_ROOT}/boot/grub/grub.cfg"
SYNC_UBUNTU_GRUB_CFG="${SYNC_BOOT_ROOT}/ubuntu/uefi/grub.cfg"
SYNC_GRUB_CFG_ALIAS_NESTED="${SYNC_BOOT_ROOT}/ubuntu/uefi/grub/grub.cfg"
FETCH_LOG="${SYNC_RUN_DIR}/fetch.log"
SYNC_SERVICE_CACHE_LINK="${SYNC_RUN_DIR}/service-data/cache"

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
    linux /ubuntu/uefi/kernel ip=dhcp boot=casper iso-url=http://10.0.2.2:18080/boot/ubuntu/uefi/live-server.iso console=ttyS0,115200n8 ---
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

  SMOKE_DISTRO="ubuntu"
  SMOKE_MODE="uefi"
  SMOKE_RUN_DIR="${SYNC_RUN_DIR}"
  SMOKE_SERVICE_DATA_DIR="${SYNC_RUN_DIR}/service-data"
  SMOKE_TFTP_ROOT="${SYNC_BOOT_ROOT}"
  SMOKE_LOG_DIR="${SYNC_RUN_DIR}/logs"
  SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}"

  smoke_prepare_workspace
  smoke_sync_boot_root_from_backend
)

[[ -f "${SYNC_GRUB_CFG}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG}" >&2; exit 1; }
[[ -f "${SYNC_GRUB_CFG_ALIAS}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG_ALIAS}" >&2; exit 1; }
[[ -f "${SYNC_UBUNTU_GRUB_CFG}" ]] || { echo "expected staged grub config at ${SYNC_UBUNTU_GRUB_CFG}" >&2; exit 1; }
[[ -f "${SYNC_GRUB_CFG_ALIAS_NESTED}" ]] || { echo "expected staged grub config at ${SYNC_GRUB_CFG_ALIAS_NESTED}" >&2; exit 1; }
[[ -f "${FETCH_LOG}" ]] || { echo "expected fetch log at ${FETCH_LOG}" >&2; exit 1; }
assert_symlink_target "${SYNC_SERVICE_CACHE_LINK}" "${SOURCE_DATA_DIR}/cache"

assert_contains "ubuntu/uefi/grubx64.efi" "${FETCH_LOG}"
assert_contains "ubuntu/uefi/grub.cfg" "${FETCH_LOG}"
assert_not_contains "ubuntu/uefi/kernel" "${FETCH_LOG}"
assert_not_contains "ubuntu/uefi/initrd" "${FETCH_LOG}"
assert_not_contains "ubuntu/uefi/live-server.iso" "${FETCH_LOG}"

assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG}"
assert_contains "boot=casper" "${SYNC_GRUB_CFG}"
assert_contains "iso-url=http://10.0.2.2:18080/boot/ubuntu/uefi/live-server.iso" "${SYNC_GRUB_CFG}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG_ALIAS}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG_ALIAS}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_UBUNTU_GRUB_CFG}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_UBUNTU_GRUB_CFG}"
assert_contains "linux /ubuntu/uefi/kernel" "${SYNC_GRUB_CFG_ALIAS_NESTED}"
assert_contains "initrd /ubuntu/uefi/initrd" "${SYNC_GRUB_CFG_ALIAS_NESTED}"

PROBE_LOG="${TMP_DIR}/probe.log"
(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  curl() {
    printf '%s\n' "$*" >>"${PROBE_LOG}"
  }

  SMOKE_DISTRO="ubuntu"
  SMOKE_MODE="uefi"
  SMOKE_API_HOST="127.0.0.1"
  SMOKE_API_PORT="18080"
  smoke_probe_assets
)

assert_contains "/boot/ubuntu/uefi/live-server.iso" "${PROBE_LOG}"

GUEST_EVIDENCE_DIR="${TMP_DIR}/guest-evidence"
mkdir -p "${GUEST_EVIDENCE_DIR}"
GUEST_BACKEND_LOG="${GUEST_EVIDENCE_DIR}/backend.log"
printf 'pre-qemu probe line\n' >"${GUEST_BACKEND_LOG}"
(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  SMOKE_DISTRO="ubuntu"
  SMOKE_MODE="uefi"
  SMOKE_LANE="backend"
  SMOKE_NETWORK_MODE="vmnet-host"
  SMOKE_LOG_DIR="${GUEST_EVIDENCE_DIR}"
  SMOKE_BACKEND_LOG="${GUEST_BACKEND_LOG}"
  smoke_mark_guest_evidence_start
  cat >>"${GUEST_BACKEND_LOG}" <<'EOF'
dhcp lease response
tftp serving asset served_path = ubuntu/uefi/kernel
tftp serving asset served_path = ubuntu/uefi/initrd
http serving cached boot asset requested_path = ubuntu/uefi/live-server.iso
EOF
  smoke_verify_guest_evidence
)
assert_contains "dhcp lease response" "${GUEST_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "served_path = ubuntu/uefi/kernel" "${GUEST_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "served_path = ubuntu/uefi/initrd" "${GUEST_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "requested_path = ubuntu/uefi/live-server.iso" "${GUEST_EVIDENCE_DIR}/backend-guest-evidence.log"

VDE_EVIDENCE_DIR="${TMP_DIR}/vde-evidence"
mkdir -p "${VDE_EVIDENCE_DIR}"
cat >"${VDE_EVIDENCE_DIR}/host-helper.log" <<'EOF'
dhcp relay xid=deadbeef guest_mac=52:54:00:12:34:56
tftp rrq path=ubuntu/uefi/kernel
tftp rrq path=ubuntu/uefi/initrd
http request GET /boot/ubuntu/uefi/live-server.iso HTTP/1.1
EOF
(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  SMOKE_DISTRO="ubuntu"
  SMOKE_MODE="uefi"
  SMOKE_LANE="backend"
  SMOKE_NETWORK_MODE="vde"
  SMOKE_LOG_DIR="${VDE_EVIDENCE_DIR}"
  SMOKE_HOST_HELPER_LOG="${VDE_EVIDENCE_DIR}/host-helper.log"
  smoke_verify_guest_evidence
)

FEDORA_SYNC_RUN_DIR="${TMP_DIR}/sync/fedora-uefi-20260405T170600Z"
FEDORA_SYNC_BOOT_ROOT="${FEDORA_SYNC_RUN_DIR}/boot-root"
FEDORA_SYNC_GRUB_CFG="${FEDORA_SYNC_BOOT_ROOT}/grub/grub.cfg"
FEDORA_SYNC_GRUB_CFG_ALIAS="${FEDORA_SYNC_BOOT_ROOT}/boot/grub/grub.cfg"
FEDORA_SYNC_GRUB2_CFG="${FEDORA_SYNC_BOOT_ROOT}/grub2/grub.cfg"
FEDORA_SYNC_GRUB2_CFG_ALIAS="${FEDORA_SYNC_BOOT_ROOT}/boot/grub2/grub.cfg"
FEDORA_SYNC_FEDORA_GRUB_CFG="${FEDORA_SYNC_BOOT_ROOT}/fedora/uefi/grub.cfg"
FEDORA_SYNC_FEDORA_GRUB_CFG_NESTED="${FEDORA_SYNC_BOOT_ROOT}/fedora/uefi/grub/grub.cfg"
FEDORA_FETCH_LOG="${FEDORA_SYNC_RUN_DIR}/fetch.log"
FEDORA_SYNC_SERVICE_CACHE_LINK="${FEDORA_SYNC_RUN_DIR}/service-data/cache"

(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  smoke_fetch_backend_asset() {
    local asset_path="$1"
    local destination_path="$2"
    mkdir -p "$(dirname "${destination_path}")"
    printf '%s\n' "${asset_path}" >>"${FEDORA_FETCH_LOG}"
    case "${asset_path}" in
      "fedora/uefi/shimx64.efi")
        printf 'shim-from-boopa\n' >"${destination_path}"
        ;;
      "fedora/uefi/grubx64.efi")
        printf 'grub-from-boopa\n' >"${destination_path}"
        ;;
      "fedora/uefi/grub.cfg")
        cat >"${destination_path}" <<'EOF'
set default=0
set timeout=2

menuentry "boopa fedora uefi smoke" {
    linuxefi /fedora/uefi/kernel ip=dhcp inst.ks=http://10.0.2.2:18080/boot/fedora/uefi/kickstart/ks.cfg console=ttyS0,115200n8
    initrdefi /fedora/uefi/initrd
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

  SMOKE_DISTRO="fedora"
  SMOKE_MODE="uefi"
  SMOKE_RUN_DIR="${FEDORA_SYNC_RUN_DIR}"
  SMOKE_SERVICE_DATA_DIR="${FEDORA_SYNC_RUN_DIR}/service-data"
  SMOKE_TFTP_ROOT="${FEDORA_SYNC_BOOT_ROOT}"
  SMOKE_LOG_DIR="${FEDORA_SYNC_RUN_DIR}/logs"
  SMOKE_SOURCE_DATA_DIR="${SOURCE_DATA_DIR}"

  smoke_prepare_workspace
  smoke_sync_boot_root_from_backend
)

[[ -f "${FEDORA_SYNC_GRUB_CFG}" ]] || { echo "expected staged grub config at ${FEDORA_SYNC_GRUB_CFG}" >&2; exit 1; }
[[ -f "${FEDORA_SYNC_GRUB_CFG_ALIAS}" ]] || { echo "expected staged grub config at ${FEDORA_SYNC_GRUB_CFG_ALIAS}" >&2; exit 1; }
[[ -f "${FEDORA_SYNC_GRUB2_CFG}" ]] || { echo "expected staged grub2 config at ${FEDORA_SYNC_GRUB2_CFG}" >&2; exit 1; }
[[ -f "${FEDORA_SYNC_GRUB2_CFG_ALIAS}" ]] || { echo "expected staged grub2 config at ${FEDORA_SYNC_GRUB2_CFG_ALIAS}" >&2; exit 1; }
[[ -f "${FEDORA_SYNC_FEDORA_GRUB_CFG}" ]] || { echo "expected staged grub config at ${FEDORA_SYNC_FEDORA_GRUB_CFG}" >&2; exit 1; }
[[ -f "${FEDORA_SYNC_FEDORA_GRUB_CFG_NESTED}" ]] || { echo "expected staged grub config at ${FEDORA_SYNC_FEDORA_GRUB_CFG_NESTED}" >&2; exit 1; }
[[ -f "${FEDORA_FETCH_LOG}" ]] || { echo "expected fetch log at ${FEDORA_FETCH_LOG}" >&2; exit 1; }
assert_symlink_target "${FEDORA_SYNC_SERVICE_CACHE_LINK}" "${SOURCE_DATA_DIR}/cache"

assert_contains "fedora/uefi/shimx64.efi" "${FEDORA_FETCH_LOG}"
assert_contains "fedora/uefi/grubx64.efi" "${FEDORA_FETCH_LOG}"
assert_contains "fedora/uefi/grub.cfg" "${FEDORA_FETCH_LOG}"
assert_not_contains "fedora/uefi/kernel" "${FEDORA_FETCH_LOG}"
assert_not_contains "fedora/uefi/initrd" "${FEDORA_FETCH_LOG}"

assert_contains "linuxefi /fedora/uefi/kernel" "${FEDORA_SYNC_GRUB_CFG}"
assert_contains "inst.ks=http://10.0.2.2:18080/boot/fedora/uefi/kickstart/ks.cfg" "${FEDORA_SYNC_GRUB_CFG}"
assert_contains "initrdefi /fedora/uefi/initrd" "${FEDORA_SYNC_GRUB_CFG}"
assert_contains "linuxefi /fedora/uefi/kernel" "${FEDORA_SYNC_GRUB2_CFG}"
assert_contains "linuxefi /fedora/uefi/kernel" "${FEDORA_SYNC_FEDORA_GRUB_CFG}"
assert_contains "linuxefi /fedora/uefi/kernel" "${FEDORA_SYNC_FEDORA_GRUB_CFG_NESTED}"

FEDORA_PROBE_LOG="${TMP_DIR}/fedora-probe.log"
(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  curl() {
    printf '%s\n' "$*" >>"${FEDORA_PROBE_LOG}"
  }

  SMOKE_DISTRO="fedora"
  SMOKE_MODE="uefi"
  SMOKE_API_HOST="127.0.0.1"
  SMOKE_API_PORT="18080"
  smoke_probe_assets
)

assert_contains "/boot/fedora/uefi/shimx64.efi" "${FEDORA_PROBE_LOG}"
assert_contains "/boot/fedora/uefi/kickstart/ks.cfg" "${FEDORA_PROBE_LOG}"
assert_not_contains "/boot/ubuntu/" "${FEDORA_PROBE_LOG}"

FEDORA_EVIDENCE_DIR="${TMP_DIR}/fedora-evidence"
mkdir -p "${FEDORA_EVIDENCE_DIR}"
FEDORA_EVIDENCE_BACKEND_LOG="${FEDORA_EVIDENCE_DIR}/backend.log"
printf 'pre-qemu probe line\n' >"${FEDORA_EVIDENCE_BACKEND_LOG}"
(
  set -euo pipefail
  # shellcheck source=scripts/smoke/lib.sh
  source "${REPO_ROOT}/scripts/smoke/lib.sh"

  SMOKE_DISTRO="fedora"
  SMOKE_MODE="uefi"
  SMOKE_LANE="backend"
  SMOKE_NETWORK_MODE="vmnet-host"
  SMOKE_LOG_DIR="${FEDORA_EVIDENCE_DIR}"
  SMOKE_BACKEND_LOG="${FEDORA_EVIDENCE_BACKEND_LOG}"
  smoke_mark_guest_evidence_start
  cat >>"${FEDORA_EVIDENCE_BACKEND_LOG}" <<'EOF'
dhcp lease response
tftp serving asset served_path = fedora/uefi/kernel
tftp serving asset served_path = fedora/uefi/initrd
http serving cached boot asset requested_path = fedora/uefi/kickstart/ks.cfg
EOF
  smoke_verify_guest_evidence
)
assert_contains "dhcp lease response" "${FEDORA_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "served_path = fedora/uefi/kernel" "${FEDORA_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "served_path = fedora/uefi/initrd" "${FEDORA_EVIDENCE_DIR}/backend-guest-evidence.log"
assert_contains "requested_path = fedora/uefi/kickstart/ks.cfg" "${FEDORA_EVIDENCE_DIR}/backend-guest-evidence.log"

echo "smoke harness dry-run regression checks passed"
