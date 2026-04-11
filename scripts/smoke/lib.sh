#!/usr/bin/env bash
set -euo pipefail

smoke_die() {
  echo "smoke: $*" >&2
  exit 1
}

smoke_log() {
  echo "smoke: $*"
}

smoke_log_file_tail() {
  local label="$1"
  local file_path="$2"
  local lines="${3:-20}"

  if [[ -f "${file_path}" ]]; then
    smoke_log "${label} tail (${file_path}):"
    tail -n "${lines}" "${file_path}" | sed 's/^/  | /'
  else
    smoke_log "${label} log not created yet: ${file_path}"
  fi
}

smoke_file_size_bytes() {
  local file_path="$1"
  if [[ ! -f "${file_path}" ]]; then
    printf '0\n'
    return 0
  fi

  wc -c <"${file_path}" | tr -d '[:space:]'
}

smoke_last_nonempty_line() {
  local file_path="$1"
  if [[ ! -f "${file_path}" ]]; then
    return 0
  fi

  awk 'NF { line = $0 } END { if (line) print line }' "${file_path}"
}

smoke_repo_root() {
  local script_dir
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  cd "${script_dir}/../.." >/dev/null 2>&1 && pwd
}

smoke_require_command() {
  local command_name="$1"
  command -v "${command_name}" >/dev/null 2>&1 || smoke_die "missing required command: ${command_name}"
}

smoke_generate_uuid() {
  if command -v uuidgen >/dev/null 2>&1; then
    uuidgen | tr '[:upper:]' '[:lower:]'
    return 0
  fi

  smoke_die "missing required command: uuidgen"
}

smoke_resolve_firmware_file() {
  local file_name="$1"
  local qemu_prefix
  qemu_prefix="$(brew --prefix qemu 2>/dev/null || true)"
  if [[ -n "${qemu_prefix}" && -f "${qemu_prefix}/share/qemu/${file_name}" ]]; then
    printf '%s\n' "${qemu_prefix}/share/qemu/${file_name}"
    return 0
  fi

  local qemu_path
  qemu_path="$(command -v qemu-system-x86_64 || true)"
  if [[ -n "${qemu_path}" ]]; then
    local qemu_dir
    qemu_dir="$(cd "$(dirname "${qemu_path}")/../share/qemu" && pwd 2>/dev/null || true)"
    if [[ -n "${qemu_dir}" && -f "${qemu_dir}/${file_name}" ]]; then
      printf '%s\n' "${qemu_dir}/${file_name}"
      return 0
    fi
  fi

  smoke_die "unable to locate ${file_name}; set QEMU_FIRMWARE_CODE or QEMU_FIRMWARE_VARS explicitly"
}

smoke_ensure_supported_target() {
  local distro="$1"
  local mode="$2"
  case "${distro}/${mode}" in
    ubuntu/uefi|fedora/uefi) ;;
    *) smoke_die "unsupported target: ${distro} ${mode}" ;;
  esac
}

smoke_configure_paths() {
  local repo_root="$1"
  local distro="$2"
  local mode="$3"
  local target_name="${SMOKE_TARGET_NAME:-${distro}-${mode}}"

  SMOKE_QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
  SMOKE_RAM_MB="${RAM_MB:-8192}"
  SMOKE_SYSTEM_DISK_GB="${SYSTEM_DISK_GB:-32}"
  local default_timeout=180
  local default_ideal_markers="Reached target System Initialization|Ubuntu installer|Subiquity|Starting system log daemon"
  case "${distro}" in
    fedora)
      default_timeout=600
      default_ideal_markers="Reached target System Initialization|Starting Anaconda|anaconda:|Starting Kickstart|Installation complete"
      ;;
  esac
  SMOKE_TIMEOUT_SECS="${SMOKE_TIMEOUT_SECS:-${default_timeout}}"
  SMOKE_WORK_ROOT="${SMOKE_WORK_ROOT:-${repo_root}/var/smoke-work}"
  SMOKE_TIMESTAMP="${SMOKE_TIMESTAMP:-$(date -u +%Y%m%dT%H%M%SZ)}"
  SMOKE_TARGET_NAME="${target_name}"
  SMOKE_RUN_DIR="${SMOKE_WORK_ROOT}/${SMOKE_TARGET_NAME}-${SMOKE_TIMESTAMP}"
  SMOKE_SERVICE_DATA_DIR="${SMOKE_RUN_DIR}/service-data"
  SMOKE_TFTP_ROOT="${SMOKE_RUN_DIR}/boot-root"
  SMOKE_LOG_DIR="${SMOKE_RUN_DIR}/logs"
  SMOKE_SERIAL_LOG="${SMOKE_LOG_DIR}/serial.log"
  SMOKE_BACKEND_LOG="${SMOKE_LOG_DIR}/backend.log"
  SMOKE_CUSTOM_IMAGE_BUILD_LOG="${SMOKE_LOG_DIR}/custom-image-build.log"
  SMOKE_CUSTOM_IMAGE_BUILD_CMD_LOG="${SMOKE_LOG_DIR}/custom-image-build-command.txt"
  SMOKE_DEBUG_LOG="${SMOKE_LOG_DIR}/debugcon.log"
  SMOKE_QEMU_LOG="${SMOKE_LOG_DIR}/qemu.log"
  SMOKE_QEMU_CMD_LOG="${SMOKE_LOG_DIR}/qemu-command.txt"
  SMOKE_DHCP_HELPER_CMD_LOG="${SMOKE_LOG_DIR}/dhcp-helper-command.txt"
  SMOKE_HOST_HELPER_LOG="${SMOKE_LOG_DIR}/host-helper.log"
  SMOKE_HOST_HELPER_CMD_LOG="${SMOKE_LOG_DIR}/host-helper-command.txt"
  SMOKE_SYSTEM_DISK_PATH="${SMOKE_RUN_DIR}/system-disk.qcow2"
  SMOKE_API_HOST="${SMOKE_API_HOST:-127.0.0.1}"
  SMOKE_API_BIND_HOST="${SMOKE_API_BIND_HOST:-0.0.0.0}"
  SMOKE_TFTP_BIND_HOST="${SMOKE_TFTP_BIND_HOST:-0.0.0.0}"
  SMOKE_API_PORT="${SMOKE_API_PORT:-$((18080 + RANDOM % 2000))}"
  SMOKE_TFTP_PORT="${SMOKE_TFTP_PORT:-$((24000 + RANDOM % 2000))}"
  SMOKE_NETWORK_MODE="${SMOKE_NETWORK_MODE:-user}"
  if [[ -n "${SMOKE_SOURCE_DATA_DIR:-}" ]]; then
    SMOKE_SOURCE_DATA_DIR="${SMOKE_SOURCE_DATA_DIR}"
  elif [[ -d "${repo_root}/var/boopa" ]]; then
    SMOKE_SOURCE_DATA_DIR="${repo_root}/var/boopa"
  else
    SMOKE_SOURCE_DATA_DIR="${repo_root}/var/boopa"
  fi
  SMOKE_DISTRO="${distro}"
  SMOKE_MODE="${mode}"
  SMOKE_CACHE_SOURCE_DIR="${SMOKE_SOURCE_DATA_DIR}/cache/${distro}/${mode}"
  SMOKE_FRONTEND_DIR="${SMOKE_FRONTEND_DIR:-${repo_root}/frontend/dist}"
  SMOKE_DRY_RUN="${SMOKE_DRY_RUN:-0}"
  SMOKE_SKIP_DOWNLOADS="${SMOKE_SKIP_DOWNLOADS:-0}"
  SMOKE_QEMU_ACCEL="${SMOKE_QEMU_ACCEL:-tcg}"
  SMOKE_QEMU_DISPLAY="${SMOKE_QEMU_DISPLAY:-default}"
  SMOKE_IDEAL_MARKERS="${SMOKE_IDEAL_MARKERS:-${default_ideal_markers}}"
  SMOKE_FALLBACK_MARKERS="${SMOKE_FALLBACK_MARKERS:-Linux version|EFI stub:|Run /init as init process|Loading initial ramdisk|Freeing initrd memory}"

  case "${SMOKE_NETWORK_MODE}" in
    user)
      SMOKE_GUEST_HOST_IP="${SMOKE_GUEST_HOST_IP:-10.0.2.2}"
      ;;
    vmnet-host)
      SMOKE_GUEST_HOST_IP="${SMOKE_GUEST_HOST_IP:-192.168.127.1}"
      SMOKE_VMNET_NET_UUID="${SMOKE_VMNET_NET_UUID:-$(smoke_generate_uuid)}"
      SMOKE_VMNET_START_ADDRESS="${SMOKE_VMNET_START_ADDRESS:-192.168.127.10}"
      SMOKE_VMNET_END_ADDRESS="${SMOKE_VMNET_END_ADDRESS:-192.168.127.99}"
      SMOKE_VMNET_SUBNET_MASK="${SMOKE_VMNET_SUBNET_MASK:-255.255.255.0}"
      SMOKE_DHCP_HELPER_MODE="${SMOKE_DHCP_HELPER_MODE:-podman-relay}"
      SMOKE_DHCP_HELPER_IMAGE="${SMOKE_DHCP_HELPER_IMAGE:-docker.io/library/python:3.12-alpine}"
      SMOKE_DHCP_HELPER_NAME="${SMOKE_DHCP_HELPER_NAME:-boopa-dhcp-relay-${SMOKE_TIMESTAMP}}"
      SMOKE_DHCP_HOST_PORT="${SMOKE_DHCP_HOST_PORT:-67}"
      SMOKE_DHCP_UPSTREAM_PORT="${SMOKE_DHCP_UPSTREAM_PORT:-$((30000 + RANDOM % 10000))}"
      SMOKE_DHCP_SUBNET="${SMOKE_DHCP_SUBNET:-192.168.127.0/24}"
      SMOKE_DHCP_POOL_START="${SMOKE_DHCP_POOL_START:-192.168.127.50}"
      SMOKE_DHCP_POOL_END="${SMOKE_DHCP_POOL_END:-192.168.127.99}"
      SMOKE_DHCP_ROUTER="${SMOKE_DHCP_ROUTER:-${SMOKE_GUEST_HOST_IP}}"
      ;;
    vde)
      SMOKE_GUEST_HOST_IP="${SMOKE_GUEST_HOST_IP:-192.168.127.1}"
      SMOKE_VDE_SWITCH_DIR="${SMOKE_VDE_SWITCH_DIR:-${SMOKE_RUN_DIR}/vde.ctl}"
      SMOKE_VDE_SWITCH_PIDFILE="${SMOKE_VDE_SWITCH_PIDFILE:-${SMOKE_RUN_DIR}/vde-switch.pid}"
      SMOKE_DHCP_UPSTREAM_PORT="${SMOKE_DHCP_UPSTREAM_PORT:-$((30000 + RANDOM % 10000))}"
      SMOKE_DHCP_SUBNET="${SMOKE_DHCP_SUBNET:-192.168.127.0/24}"
      SMOKE_DHCP_POOL_START="${SMOKE_DHCP_POOL_START:-192.168.127.50}"
      SMOKE_DHCP_POOL_END="${SMOKE_DHCP_POOL_END:-192.168.127.99}"
      SMOKE_DHCP_ROUTER="${SMOKE_DHCP_ROUTER:-${SMOKE_GUEST_HOST_IP}}"
      SMOKE_VDE_HELPER_MODE="${SMOKE_VDE_HELPER_MODE:-python-host-helper}"
      ;;
    *)
      smoke_die "unsupported SMOKE_NETWORK_MODE: ${SMOKE_NETWORK_MODE}"
      ;;
  esac
}

smoke_prepare_workspace() {
  mkdir -p "${SMOKE_RUN_DIR}" "${SMOKE_SERVICE_DATA_DIR}" "${SMOKE_LOG_DIR}"

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    mkdir -p "${SMOKE_TFTP_ROOT}/${SMOKE_DISTRO}/${SMOKE_MODE}" "${SMOKE_TFTP_ROOT}/EFI/BOOT"

    if [[ "${SMOKE_SERVICE_DATA_DIR}/cache" == "${SMOKE_SOURCE_DATA_DIR}/cache" ]]; then
      mkdir -p "${SMOKE_SERVICE_DATA_DIR}/cache"
    else
      mkdir -p "${SMOKE_SOURCE_DATA_DIR}/cache"
      ln -s "${SMOKE_SOURCE_DATA_DIR}/cache" "${SMOKE_SERVICE_DATA_DIR}/cache"
      smoke_log "linked smoke cache ${SMOKE_SERVICE_DATA_DIR}/cache -> ${SMOKE_SOURCE_DATA_DIR}/cache"
    fi
  fi
}

smoke_prepare_firmware() {
  local vars_copy="${SMOKE_RUN_DIR}/edk2-vars.fd"
  cp "${QEMU_FIRMWARE_VARS}" "${vars_copy}"
  SMOKE_QEMU_VARS_COPY="${vars_copy}"
  smoke_log "prepared writable firmware vars copy at ${SMOKE_QEMU_VARS_COPY}"
}

smoke_prepare_system_disk() {
  smoke_log "preparing ${SMOKE_SYSTEM_DISK_GB}G installer disk at ${SMOKE_SYSTEM_DISK_PATH}"
  rm -f "${SMOKE_SYSTEM_DISK_PATH}"
  qemu-img create -f qcow2 "${SMOKE_SYSTEM_DISK_PATH}" "${SMOKE_SYSTEM_DISK_GB}G" >/dev/null
}

smoke_configure_interactive_mode() {
  local interactive_setting="${SMOKE_INTERACTIVE:-auto}"
  case "${interactive_setting}" in
    auto)
      if [[ "${SMOKE_DRY_RUN}" != "1" && -t 0 && -t 1 ]]; then
        SMOKE_INTERACTIVE=1
      else
        SMOKE_INTERACTIVE=0
      fi
      ;;
    1|true|yes)
      SMOKE_INTERACTIVE=1
      ;;
    0|false|no)
      SMOKE_INTERACTIVE=0
      ;;
    *)
      smoke_die "SMOKE_INTERACTIVE must be auto, 0, or 1"
      ;;
  esac
}

smoke_prepare_boot_root() {
  smoke_log "preparing minimal UEFI firmware carrier under ${SMOKE_RUN_DIR}"
  cat >"${SMOKE_TFTP_ROOT}/startup.nsh" <<'EOF'
fs0:\EFI\BOOT\BOOTX64.EFI
fs1:\EFI\BOOT\BOOTX64.EFI
EOF
  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    smoke_seed_dry_run_bootloader
  fi
  smoke_log "boot volume prepared at ${SMOKE_TFTP_ROOT}"
}

smoke_seed_dry_run_bootloader() {
  local source_file
  case "${SMOKE_DISTRO}" in
    ubuntu) source_file="grubx64.efi" ;;
    fedora) source_file="shimx64.efi" ;;
    *) source_file="grubx64.efi" ;;
  esac
  local source_path="${SMOKE_CACHE_SOURCE_DIR}/${source_file}"
  local destination_path="${SMOKE_TFTP_ROOT}/EFI/BOOT/BOOTX64.EFI"

  if [[ -f "${source_path}" ]]; then
    cp "${source_path}" "${destination_path}"
  else
    printf 'dry-run placeholder bootloader\n' >"${destination_path}"
  fi
}

smoke_preflight() {
  smoke_require_command "${SMOKE_QEMU_BIN}"
  smoke_require_command qemu-img

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    smoke_require_command cargo
    smoke_require_command curl
  elif [[ ! -f "${CUSTOM_IMAGE_OUTPUT_ISO:-}" ]]; then
    smoke_require_command cargo
  fi

  if [[ -z "${QEMU_FIRMWARE_CODE:-}" ]]; then
    QEMU_FIRMWARE_CODE="$(smoke_resolve_firmware_file "edk2-x86_64-code.fd")"
  fi
  if [[ -z "${QEMU_FIRMWARE_VARS:-}" ]]; then
    QEMU_FIRMWARE_VARS="$(smoke_resolve_firmware_file "edk2-i386-vars.fd")"
  fi
  [[ -f "${QEMU_FIRMWARE_CODE}" ]] || smoke_die "firmware code image not found: ${QEMU_FIRMWARE_CODE}"
  [[ -f "${QEMU_FIRMWARE_VARS}" ]] || smoke_die "firmware vars image not found: ${QEMU_FIRMWARE_VARS}"

  if [[ "${SMOKE_NETWORK_MODE}" == "vmnet-host" ]]; then
    if ! "${SMOKE_QEMU_BIN}" --help 2>&1 | grep -q "vmnet-host"; then
      smoke_die "qemu does not advertise vmnet-host support"
    fi
    if [[ "${SMOKE_DHCP_HELPER_MODE}" == "podman-relay" ]]; then
      smoke_require_command podman
      [[ -f "${SMOKE_REPO_ROOT}/scripts/smoke/dhcp-relay.py" ]] || smoke_die "missing DHCP relay helper script"
    fi
  elif [[ "${SMOKE_NETWORK_MODE}" == "vde" ]]; then
    smoke_require_command vde_switch
    smoke_require_command vde_plug
    smoke_require_command python3
    [[ -f "${SMOKE_REPO_ROOT}/scripts/smoke/vde_host_helper.py" ]] || smoke_die "missing VDE host helper script"
  fi
}

smoke_cleanup() {
  local exit_code="$1"
  if [[ -n "${SMOKE_QEMU_PID:-}" ]]; then
    kill "${SMOKE_QEMU_PID}" >/dev/null 2>&1 || true
    wait "${SMOKE_QEMU_PID}" 2>/dev/null || true
  fi
  if [[ -n "${SMOKE_BACKEND_PID:-}" ]]; then
    kill "${SMOKE_BACKEND_PID}" >/dev/null 2>&1 || true
    wait "${SMOKE_BACKEND_PID}" 2>/dev/null || true
  fi
  if [[ -n "${SMOKE_HOST_HELPER_PID:-}" ]]; then
    kill "${SMOKE_HOST_HELPER_PID}" >/dev/null 2>&1 || true
    wait "${SMOKE_HOST_HELPER_PID}" 2>/dev/null || true
  fi
  if [[ -f "${SMOKE_VDE_SWITCH_PIDFILE:-}" ]]; then
    kill "$(cat "${SMOKE_VDE_SWITCH_PIDFILE}")" >/dev/null 2>&1 || true
  fi
  if [[ -n "${SMOKE_DHCP_HELPER_NAME:-}" && "${SMOKE_DHCP_HELPER_STARTED:-0}" == "1" ]]; then
    podman stop "${SMOKE_DHCP_HELPER_NAME}" >/dev/null 2>&1 || true
  fi
  if [[ "${exit_code}" -ne 0 ]]; then
    smoke_log_file_tail "serial" "${SMOKE_SERIAL_LOG}" 30
    if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
      smoke_log_file_tail "backend" "${SMOKE_BACKEND_LOG}" 30
    else
      smoke_log_file_tail "custom image build" "${SMOKE_CUSTOM_IMAGE_BUILD_LOG}" 30
    fi
    smoke_log_file_tail "qemu" "${SMOKE_QEMU_LOG}" 30
    echo "smoke logs preserved at ${SMOKE_RUN_DIR}" >&2
  fi
}

smoke_start_backend() {
  smoke_log "starting boopa on API ${SMOKE_API_BIND_HOST}:${SMOKE_API_PORT} and TFTP ${SMOKE_TFTP_BIND_HOST}:${SMOKE_TFTP_PORT}"
  (
    cd "${SMOKE_REPO_ROOT}"
    if [[ "${SMOKE_NETWORK_MODE}" == "vmnet-host" || "${SMOKE_NETWORK_MODE}" == "vde" ]]; then
      BOOPA_API_BIND="${SMOKE_API_BIND_HOST}:${SMOKE_API_PORT}" \
      BOOPA_TFTP_BIND="${SMOKE_TFTP_BIND_HOST}:${SMOKE_TFTP_PORT}" \
      BOOPA_TFTP_ADVERTISE_ADDR="${SMOKE_GUEST_HOST_IP}:${SMOKE_TFTP_PORT}" \
      BOOPA_DHCP_MODE="authoritative" \
      BOOPA_DHCP_BIND="127.0.0.1:${SMOKE_DHCP_UPSTREAM_PORT}" \
      BOOPA_DHCP_SUBNET="${SMOKE_DHCP_SUBNET}" \
      BOOPA_DHCP_POOL_START="${SMOKE_DHCP_POOL_START}" \
      BOOPA_DHCP_POOL_END="${SMOKE_DHCP_POOL_END}" \
      BOOPA_DHCP_ROUTER="${SMOKE_DHCP_ROUTER}" \
      BOOPA_DATA_DIR="${SMOKE_SERVICE_DATA_DIR}" \
      BOOPA_FRONTEND_DIR="${SMOKE_FRONTEND_DIR}" \
      cargo run -p boopa --quiet
    else
      BOOPA_API_BIND="${SMOKE_API_BIND_HOST}:${SMOKE_API_PORT}" \
      BOOPA_TFTP_BIND="${SMOKE_TFTP_BIND_HOST}:${SMOKE_TFTP_PORT}" \
      BOOPA_TFTP_ADVERTISE_ADDR="${SMOKE_GUEST_HOST_IP}:${SMOKE_TFTP_PORT}" \
      BOOPA_DATA_DIR="${SMOKE_SERVICE_DATA_DIR}" \
      BOOPA_FRONTEND_DIR="${SMOKE_FRONTEND_DIR}" \
      cargo run -p boopa --quiet
    fi
  ) >"${SMOKE_BACKEND_LOG}" 2>&1 &
  SMOKE_BACKEND_PID=$!
}

smoke_start_network_backend() {
  if [[ "${SMOKE_NETWORK_MODE}" != "vde" ]]; then
    return 0
  fi

  mkdir -p "${SMOKE_VDE_SWITCH_DIR}"
  smoke_log "starting VDE switch at ${SMOKE_VDE_SWITCH_DIR}"

  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    return 0
  fi

  vde_switch -s "${SMOKE_VDE_SWITCH_DIR}" -d -p "${SMOKE_VDE_SWITCH_PIDFILE}"
  sleep 1
  [[ -S "${SMOKE_VDE_SWITCH_DIR}/ctl" ]] || smoke_die "vde_switch did not create ${SMOKE_VDE_SWITCH_DIR}/ctl"
}

smoke_wait_for_backend() {
  local attempts=0
  smoke_log "waiting for backend health endpoint"
  while [[ "${attempts}" -lt 60 ]]; do
    if [[ -n "${SMOKE_BACKEND_PID:-}" ]] && ! kill -0 "${SMOKE_BACKEND_PID}" >/dev/null 2>&1; then
      smoke_die "boopa exited before becoming healthy; see ${SMOKE_BACKEND_LOG}"
    fi
    if curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/api/health" >/dev/null 2>&1; then
      smoke_log "backend is healthy"
      return 0
    fi
    attempts=$((attempts + 1))
    if (( attempts % 5 == 0 )); then
      smoke_log "backend still starting after ${attempts}s"
    fi
    sleep 1
  done

  smoke_die "boopa did not become healthy; see ${SMOKE_BACKEND_LOG}"
}

smoke_refresh_backend_assets() {
  smoke_log "selecting ${SMOKE_DISTRO} distro through boopa"
  curl -fsS \
    -X PUT \
    -H 'content-type: application/json' \
    --data "{\"distro\":\"${SMOKE_DISTRO}\"}" \
    "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/api/selection" \
    -o /dev/null
  smoke_log "refreshing ${SMOKE_DISTRO} ${SMOKE_MODE} assets through boopa"
  curl -fsS \
    -X POST \
    -H 'content-type: application/json' \
    --data "{\"distro\":\"${SMOKE_DISTRO}\",\"mode\":\"${SMOKE_MODE}\"}" \
    "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/api/cache/refresh" \
    -o /dev/null
}

smoke_fetch_backend_asset() {
  local asset_path="$1"
  local destination_path="$2"

  mkdir -p "$(dirname "${destination_path}")"
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/${asset_path}" -o "${destination_path}"
}

smoke_sync_boot_root_from_backend() {
  local fetched_grub_cfg="${SMOKE_RUN_DIR}/grub.cfg"

  smoke_log "syncing firmware-carrier assets from boopa for ${SMOKE_DISTRO}/${SMOKE_MODE}"
  case "${SMOKE_DISTRO}" in
    ubuntu)
      smoke_fetch_backend_asset "ubuntu/uefi/grubx64.efi" "${SMOKE_TFTP_ROOT}/EFI/BOOT/BOOTX64.EFI"
      smoke_fetch_backend_asset "ubuntu/uefi/grub.cfg" "${fetched_grub_cfg}"
      mkdir -p \
        "${SMOKE_TFTP_ROOT}/grub" \
        "${SMOKE_TFTP_ROOT}/boot/grub" \
        "${SMOKE_TFTP_ROOT}/ubuntu/uefi/grub"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/grub/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/boot/grub/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/ubuntu/uefi/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/ubuntu/uefi/grub/grub.cfg"
      ;;
    fedora)
      smoke_fetch_backend_asset "fedora/uefi/shimx64.efi" "${SMOKE_TFTP_ROOT}/EFI/BOOT/BOOTX64.EFI"
      smoke_fetch_backend_asset "fedora/uefi/grubx64.efi" "${SMOKE_TFTP_ROOT}/EFI/BOOT/grubx64.efi"
      smoke_fetch_backend_asset "fedora/uefi/grub.cfg" "${fetched_grub_cfg}"
      mkdir -p \
        "${SMOKE_TFTP_ROOT}/grub" \
        "${SMOKE_TFTP_ROOT}/boot/grub" \
        "${SMOKE_TFTP_ROOT}/grub2" \
        "${SMOKE_TFTP_ROOT}/boot/grub2" \
        "${SMOKE_TFTP_ROOT}/EFI/fedora" \
        "${SMOKE_TFTP_ROOT}/fedora/uefi/grub"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/EFI/BOOT/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/EFI/fedora/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/grub/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/boot/grub/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/grub2/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/boot/grub2/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/fedora/uefi/grub.cfg"
      cp "${fetched_grub_cfg}" "${SMOKE_TFTP_ROOT}/fedora/uefi/grub/grub.cfg"
      ;;
  esac
  rm -f "${fetched_grub_cfg}"
}

smoke_probe_assets() {
  smoke_log "probing backend boot asset endpoints before guest boot"
  local base="http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot"
  case "${SMOKE_DISTRO}" in
    ubuntu)
      curl -fsS "${base}/ubuntu/uefi/grubx64.efi" -o /dev/null
      curl -fsS "${base}/ubuntu/uefi/grub.cfg" -o /dev/null
      curl -fsS "${base}/ubuntu/uefi/kernel" -o /dev/null
      curl -fsS "${base}/ubuntu/uefi/initrd" -o /dev/null
      curl -fsS "${base}/ubuntu/uefi/live-server.iso" -o /dev/null
      ;;
    fedora)
      curl -fsS "${base}/fedora/uefi/shimx64.efi" -o /dev/null
      curl -fsS "${base}/fedora/uefi/grub.cfg" -o /dev/null
      curl -fsS "${base}/fedora/uefi/kernel" -o /dev/null
      curl -fsS "${base}/fedora/uefi/initrd" -o /dev/null
      curl -fsS "${base}/fedora/uefi/kickstart/ks.cfg" -o /dev/null
      ;;
  esac
  smoke_log "backend asset probes succeeded"
}

smoke_mark_guest_evidence_start() {
  if [[ "${SMOKE_LANE:-backend}" != "backend" || "${SMOKE_NETWORK_MODE}" != "vmnet-host" ]]; then
    return 0
  fi

  SMOKE_BACKEND_EVIDENCE_OFFSET="$(smoke_file_size_bytes "${SMOKE_BACKEND_LOG}")"
}

smoke_verify_guest_evidence() {
  if [[ "${SMOKE_LANE:-backend}" != "backend" ]]; then
    return 0
  fi

  local distro_prefix="${SMOKE_DISTRO}/${SMOKE_MODE}"
  local http_evidence_path
  case "${SMOKE_DISTRO}" in
    ubuntu) http_evidence_path="ubuntu/uefi/live-server.iso" ;;
    fedora) http_evidence_path="fedora/uefi/kickstart/ks.cfg" ;;
  esac

  case "${SMOKE_NETWORK_MODE}" in
    vmnet-host)
      local offset="${SMOKE_BACKEND_EVIDENCE_OFFSET:-0}"
      local guest_evidence_log="${SMOKE_LOG_DIR}/backend-guest-evidence.log"

      if [[ ! -f "${SMOKE_BACKEND_LOG}" ]]; then
        smoke_die "backend log missing for guest evidence verification"
      fi

      tail -c "+$((offset + 1))" "${SMOKE_BACKEND_LOG}" >"${guest_evidence_log}" || true

      grep -q "dhcp lease response" "${guest_evidence_log}" || smoke_die "guest-path run lacked DHCP lease evidence after QEMU start"
      grep -q "tftp serving asset" "${guest_evidence_log}" || smoke_die "guest-path run lacked TFTP asset evidence after QEMU start"
      grep -q "served_path = ${distro_prefix}/kernel" "${guest_evidence_log}" || smoke_die "guest-path run lacked kernel TFTP evidence after QEMU start"
      grep -q "served_path = ${distro_prefix}/initrd" "${guest_evidence_log}" || smoke_die "guest-path run lacked initrd TFTP evidence after QEMU start"
      grep -q "requested_path = ${http_evidence_path}" "${guest_evidence_log}" || smoke_die "guest-path run lacked HTTP evidence for ${http_evidence_path} after QEMU start"
      ;;
    vde)
      local helper_evidence_log="${SMOKE_HOST_HELPER_LOG}"
      [[ -f "${helper_evidence_log}" ]] || smoke_die "host helper log missing for VDE guest evidence verification"
      grep -q "dhcp relay" "${helper_evidence_log}" || smoke_die "vde guest-path run lacked DHCP relay evidence"
      grep -Eq "tftp rrq path=/${distro_prefix}/kernel|tftp rrq path=${distro_prefix}/kernel" "${helper_evidence_log}" || smoke_die "vde guest-path run lacked kernel TFTP evidence"
      if ! grep -Eq "tftp rrq path=/${distro_prefix}/initrd|tftp rrq path=${distro_prefix}/initrd" "${helper_evidence_log}"; then
        grep -q "Trying to unpack rootfs image as initramfs" "${SMOKE_SERIAL_LOG}" || smoke_die "vde guest-path run lacked initrd handoff evidence"
      fi
      grep -q "http request GET /boot/${http_evidence_path}" "${helper_evidence_log}" || smoke_die "vde guest-path run lacked HTTP evidence for ${http_evidence_path}"
      ;;
    *)
      return 0
      ;;
  esac

  smoke_log "guest-path backend evidence matched"
}

smoke_http_evidence_seen() {
  local http_evidence_path
  case "${SMOKE_DISTRO}" in
    ubuntu) http_evidence_path="ubuntu/uefi/live-server.iso" ;;
    fedora) http_evidence_path="fedora/uefi/kickstart/ks.cfg" ;;
  esac

  case "${SMOKE_NETWORK_MODE}" in
    vmnet-host)
      [[ -f "${SMOKE_BACKEND_LOG}" ]] && grep -q "requested_path = ${http_evidence_path}" "${SMOKE_BACKEND_LOG}"
      ;;
    vde)
      [[ -f "${SMOKE_HOST_HELPER_LOG}" ]] && grep -q "http request GET /boot/${http_evidence_path}" "${SMOKE_HOST_HELPER_LOG}"
      ;;
    *)
      return 0
      ;;
  esac
}

smoke_write_qemu_command_log() {
  smoke_write_command_log "${SMOKE_QEMU_CMD_LOG}" "$@"
}

smoke_write_command_log() {
  local log_path="$1"
  shift
  local cmd=("$@")
  printf '%q ' "${cmd[@]}" >"${log_path}"
  printf '\n' >>"${log_path}"
}

smoke_require_existing_file() {
  local label="$1"
  local file_path="$2"

  [[ -n "${file_path}" ]] || smoke_die "${label} must be set"
  [[ -f "${file_path}" ]] || smoke_die "${label} not found: ${file_path}"
}

smoke_prepare_custom_image_iso() {
  smoke_require_existing_file "CUSTOM_IMAGE_BASE_ISO" "${CUSTOM_IMAGE_BASE_ISO:-}"
  smoke_require_existing_file "CUSTOM_IMAGE_MANIFEST" "${CUSTOM_IMAGE_MANIFEST:-}"

  SMOKE_CUSTOM_IMAGE_OUTPUT_ISO="${CUSTOM_IMAGE_OUTPUT_ISO:-}"
  [[ -n "${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}" ]] || smoke_die "CUSTOM_IMAGE_OUTPUT_ISO must be set"

  if [[ -f "${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}" ]]; then
    smoke_log "using existing custom image ISO ${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}"
    return 0
  fi

  mkdir -p "$(dirname "${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}")"

  local build_cmd=(
    cargo
    run
    -p
    ubuntu-custom-image
    --
    build
    --base-iso
    "${CUSTOM_IMAGE_BASE_ISO}"
    --manifest
    "${CUSTOM_IMAGE_MANIFEST}"
    --output
    "${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}"
  )

  smoke_write_command_log "${SMOKE_CUSTOM_IMAGE_BUILD_CMD_LOG}" "${build_cmd[@]}"

  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    smoke_log "dry-run custom image build command prepared at ${SMOKE_CUSTOM_IMAGE_BUILD_CMD_LOG}"
    return 0
  fi

  smoke_log "building custom image ISO at ${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}"
  (
    cd "${SMOKE_REPO_ROOT}"
    "${build_cmd[@]}"
  ) >"${SMOKE_CUSTOM_IMAGE_BUILD_LOG}" 2>&1

  [[ -f "${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}" ]] || smoke_die "custom image build did not create ${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}"
}

smoke_boot_media_args() {
  if [[ "${SMOKE_LANE:-backend}" == "custom-image" ]]; then
    printf '%s\0' \
      "-boot" "order=d,menu=off" \
      "-drive" "file=${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO},media=cdrom,if=ide,index=0"
    return 0
  fi

  printf '%s\0' \
    "-boot" "order=c,menu=off" \
    "-drive" "file=fat:rw:${SMOKE_TFTP_ROOT},format=raw,if=ide,index=0"
}

smoke_network_args() {
  case "${SMOKE_NETWORK_MODE}" in
    user)
      printf '%s\0' \
        "-netdev" "user,id=net0,ipv6=off" \
        "-device" "e1000,netdev=net0"
      ;;
    vmnet-host)
      printf '%s\0' \
        "-netdev" "vmnet-host,id=net0,isolated=on,net-uuid=${SMOKE_VMNET_NET_UUID},start-address=${SMOKE_VMNET_START_ADDRESS},end-address=${SMOKE_VMNET_END_ADDRESS},subnet-mask=${SMOKE_VMNET_SUBNET_MASK}" \
        "-device" "e1000,netdev=net0"
      ;;
    vde)
      printf '%s\0' \
        "-netdev" "vde,id=net0,sock=${SMOKE_VDE_SWITCH_DIR}" \
        "-device" "e1000,netdev=net0"
      ;;
    *)
      smoke_die "unsupported SMOKE_NETWORK_MODE: ${SMOKE_NETWORK_MODE}"
      ;;
  esac
}

smoke_start_dhcp_helper() {
  if [[ "${SMOKE_NETWORK_MODE}" != "vmnet-host" ]]; then
    return 0
  fi

  case "${SMOKE_DHCP_HELPER_MODE}" in
    podman-relay)
      local helper_cmd=(
        podman
        run
        --rm
        -d
        --name
        "${SMOKE_DHCP_HELPER_NAME}"
        -p
        "${SMOKE_DHCP_HOST_PORT}:67/udp"
        -v
        "${SMOKE_REPO_ROOT}/scripts/smoke/dhcp-relay.py:/relay.py:ro"
        "${SMOKE_DHCP_HELPER_IMAGE}"
        python3
        /relay.py
        --listen-port
        "67"
        --upstream-host
        "host.containers.internal"
        --upstream-port
        "${SMOKE_DHCP_UPSTREAM_PORT}"
      )
      smoke_write_command_log "${SMOKE_DHCP_HELPER_CMD_LOG}" "${helper_cmd[@]}"

      if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
        smoke_log "dry-run DHCP helper command prepared at ${SMOKE_DHCP_HELPER_CMD_LOG}"
        return 0
      fi

      smoke_log "starting DHCP relay helper ${SMOKE_DHCP_HELPER_NAME}"
      "${helper_cmd[@]}" >/dev/null
      SMOKE_DHCP_HELPER_STARTED=1
      ;;
    none)
      smoke_die "SMOKE_NETWORK_MODE=vmnet-host requires a DHCP helper; set SMOKE_DHCP_HELPER_MODE=podman-relay or provide another supported mode"
      ;;
    *)
      smoke_die "unsupported SMOKE_DHCP_HELPER_MODE: ${SMOKE_DHCP_HELPER_MODE}"
      ;;
  esac
}

smoke_start_host_helper() {
  if [[ "${SMOKE_NETWORK_MODE}" != "vde" ]]; then
    return 0
  fi

  case "${SMOKE_VDE_HELPER_MODE}" in
    python-host-helper)
      local helper_cmd=(
        python3
        "${SMOKE_REPO_ROOT}/scripts/smoke/vde_host_helper.py"
        --switch-dir
        "${SMOKE_VDE_SWITCH_DIR}"
        --host-ip
        "${SMOKE_GUEST_HOST_IP}"
        --dhcp-upstream-host
        "127.0.0.1"
        --dhcp-upstream-port
        "${SMOKE_DHCP_UPSTREAM_PORT}"
        --tftp-upstream-host
        "127.0.0.1"
        --tftp-upstream-port
        "${SMOKE_TFTP_PORT}"
        --http-upstream-host
        "127.0.0.1"
        --http-upstream-port
        "${SMOKE_API_PORT}"
      )
      smoke_write_command_log "${SMOKE_HOST_HELPER_CMD_LOG}" "${helper_cmd[@]}"

      if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
        smoke_log "dry-run host helper command prepared at ${SMOKE_HOST_HELPER_CMD_LOG}"
        return 0
      fi

      smoke_log "starting VDE host helper"
      "${helper_cmd[@]}" >"${SMOKE_HOST_HELPER_LOG}" 2>&1 &
      SMOKE_HOST_HELPER_PID=$!
      sleep 1
      kill -0 "${SMOKE_HOST_HELPER_PID}" >/dev/null 2>&1 || smoke_die "VDE host helper exited during startup; see ${SMOKE_HOST_HELPER_LOG}"
      ;;
    *)
      smoke_die "unsupported SMOKE_VDE_HELPER_MODE: ${SMOKE_VDE_HELPER_MODE}"
      ;;
  esac
}

smoke_start_qemu() {
  local boot_media=()
  local network_args=()
  while IFS= read -r -d '' boot_arg; do
    boot_media+=("${boot_arg}")
  done < <(smoke_boot_media_args)
  while IFS= read -r -d '' network_arg; do
    network_args+=("${network_arg}")
  done < <(smoke_network_args)

  local qemu_cmd=(
    "${SMOKE_QEMU_BIN}"
    -machine q35
    -accel "${SMOKE_QEMU_ACCEL}"
    -m "${SMOKE_RAM_MB}"
    -display none
    -monitor none
    -serial "file:${SMOKE_SERIAL_LOG}"
    -debugcon "file:${SMOKE_DEBUG_LOG}"
    -global isa-debugcon.iobase=0x402
    -drive "if=pflash,format=raw,readonly=on,file=${QEMU_FIRMWARE_CODE}"
    -drive "if=pflash,format=raw,file=${SMOKE_QEMU_VARS_COPY}"
    "${boot_media[@]}"
    -drive "file=${SMOKE_SYSTEM_DISK_PATH},format=qcow2,if=virtio"
    "${network_args[@]}"
    -no-reboot
  )

  if [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then
    qemu_cmd=(
      "${SMOKE_QEMU_BIN}"
      -machine q35
      -accel "${SMOKE_QEMU_ACCEL}"
      -m "${SMOKE_RAM_MB}"
      -display "${SMOKE_QEMU_DISPLAY}"
      -monitor none
      -serial stdio
      -debugcon "file:${SMOKE_DEBUG_LOG}"
      -global isa-debugcon.iobase=0x402
      -drive "if=pflash,format=raw,readonly=on,file=${QEMU_FIRMWARE_CODE}"
      -drive "if=pflash,format=raw,file=${SMOKE_QEMU_VARS_COPY}"
      "${boot_media[@]}"
      -drive "file=${SMOKE_SYSTEM_DISK_PATH},format=qcow2,if=virtio"
      "${network_args[@]}"
      -no-reboot
    )
  fi

  smoke_write_qemu_command_log "${qemu_cmd[@]}"

  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    smoke_log "dry-run qemu command prepared at ${SMOKE_QEMU_CMD_LOG}"
    return 0
  fi

  if [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then
    smoke_log "starting qemu in interactive mode; command saved to ${SMOKE_QEMU_CMD_LOG}"
    smoke_log "input is attached to this terminal; press keys here when firmware or GRUB asks"
    smoke_log "serial session is being recorded to ${SMOKE_SERIAL_LOG}"
    smoke_log "QEMU display mode: ${SMOKE_QEMU_DISPLAY}"
    script -q -F "${SMOKE_SERIAL_LOG}" "${qemu_cmd[@]}"
  else
    smoke_log "starting qemu; command saved to ${SMOKE_QEMU_CMD_LOG}"
    "${qemu_cmd[@]}" >"${SMOKE_QEMU_LOG}" 2>&1 &
    SMOKE_QEMU_PID=$!
    smoke_log "qemu pid ${SMOKE_QEMU_PID}"
  fi
}

smoke_verify_markers_post_run() {
  if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_IDEAL_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
    smoke_log "ideal marker matched"
    return 0
  fi
  if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_FALLBACK_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
    smoke_log "fallback marker matched"
    return 0
  fi

  smoke_die "qemu session ended without matching success markers; inspect ${SMOKE_SERIAL_LOG} and ${SMOKE_QEMU_LOG}"
}

smoke_wait_for_markers() {
  local deadline
  local heartbeat_interval=10
  local next_heartbeat
  local waiting_for_http_evidence=0
  deadline=$((SECONDS + SMOKE_TIMEOUT_SECS))
  next_heartbeat=$((SECONDS + heartbeat_interval))
  smoke_log "watching serial log for success markers for up to ${SMOKE_TIMEOUT_SECS}s"

  while [[ "${SECONDS}" -lt "${deadline}" ]]; do
    if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_IDEAL_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
      smoke_log "ideal marker matched"
      return 0
    fi
    if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_FALLBACK_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
      if [[ "${SMOKE_NETWORK_MODE}" == "vmnet-host" || "${SMOKE_NETWORK_MODE}" == "vde" ]]; then
        if smoke_http_evidence_seen; then
          smoke_log "fallback marker matched"
          return 0
        fi
        if [[ "${waiting_for_http_evidence}" == "0" ]]; then
          smoke_log "fallback marker matched; continuing to wait for HTTP guest evidence"
          waiting_for_http_evidence=1
        fi
      else
        smoke_log "fallback marker matched"
        return 0
      fi
    fi

    if [[ -n "${SMOKE_QEMU_PID:-}" ]] && ! kill -0 "${SMOKE_QEMU_PID}" >/dev/null 2>&1; then
      if [[ -f "${SMOKE_QEMU_LOG}" ]] && grep -q "cannot create vmnet interface" "${SMOKE_QEMU_LOG}"; then
        smoke_die "qemu vmnet-host backend failed; this host/qemu combination does not permit creating the vmnet interface without extra privileges or entitlements"
      fi
      smoke_log "qemu exited before success markers were observed"
      break
    fi

    if [[ "${SECONDS}" -ge "${next_heartbeat}" ]]; then
      local serial_line
      local backend_line
      serial_line="$(smoke_last_nonempty_line "${SMOKE_SERIAL_LOG}")"
      backend_line="$(smoke_last_nonempty_line "${SMOKE_BACKEND_LOG}")"
      smoke_log "still waiting at ${SECONDS}s/${SMOKE_TIMEOUT_SECS}s"
      if [[ -n "${serial_line}" ]]; then
        smoke_log "latest serial: ${serial_line}"
      fi
      if [[ -n "${backend_line}" ]]; then
        smoke_log "latest backend: ${backend_line}"
      fi
      next_heartbeat=$((SECONDS + heartbeat_interval))
    fi
    sleep 2
  done

  smoke_die "no success markers matched before timeout; inspect ${SMOKE_SERIAL_LOG} and ${SMOKE_QEMU_LOG}"
}

smoke_print_summary() {
  if [[ "${SMOKE_LANE:-backend}" == "custom-image" ]]; then
    cat <<EOF
Smoke target: ${SMOKE_TARGET_NAME}
Run dir: ${SMOKE_RUN_DIR}
Base ISO: ${CUSTOM_IMAGE_BASE_ISO}
Manifest: ${CUSTOM_IMAGE_MANIFEST}
Output ISO: ${SMOKE_CUSTOM_IMAGE_OUTPUT_ISO}
Guest RAM: ${SMOKE_RAM_MB} MiB
Installer disk: ${SMOKE_SYSTEM_DISK_PATH} (${SMOKE_SYSTEM_DISK_GB}G)
Interactive display: ${SMOKE_QEMU_DISPLAY}
QEMU: ${SMOKE_QEMU_BIN}
Firmware code: ${QEMU_FIRMWARE_CODE}
Firmware vars: ${QEMU_FIRMWARE_VARS}
Mode: $(if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then printf '%s' dry-run; elif [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then printf '%s' interactive; else printf '%s' headless; fi)
EOF
    return 0
  fi

  cat <<EOF
Smoke target: ${SMOKE_DISTRO} ${SMOKE_MODE}
Run dir: ${SMOKE_RUN_DIR}
API base: http://${SMOKE_API_HOST}:${SMOKE_API_PORT}
TFTP endpoint: ${SMOKE_GUEST_HOST_IP}:${SMOKE_TFTP_PORT}
Network mode: ${SMOKE_NETWORK_MODE}
DHCP helper mode: ${SMOKE_DHCP_HELPER_MODE:-none}
Guest RAM: ${SMOKE_RAM_MB} MiB
Installer disk: ${SMOKE_SYSTEM_DISK_PATH} (${SMOKE_SYSTEM_DISK_GB}G)
Interactive display: ${SMOKE_QEMU_DISPLAY}
QEMU: ${SMOKE_QEMU_BIN}
Firmware code: ${QEMU_FIRMWARE_CODE}
Firmware vars: ${QEMU_FIRMWARE_VARS}
Mode: $(if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then printf '%s' dry-run; elif [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then printf '%s' interactive; else printf '%s' headless; fi)
EOF
}

smoke_main() {
  if [[ $# -ne 2 ]]; then
    smoke_die "usage: $0 <distro> <mode>"
  fi

  local distro="$1"
  local mode="$2"
  SMOKE_LANE="${SMOKE_LANE:-backend}"
  SMOKE_REPO_ROOT="$(smoke_repo_root)"

  case "${SMOKE_LANE}" in
    backend)
      ;;
    custom-image)
      SMOKE_TARGET_NAME="${SMOKE_TARGET_NAME:-ubuntu-custom-image}"
      ;;
    *)
      smoke_die "unsupported smoke lane: ${SMOKE_LANE}"
      ;;
  esac

  smoke_ensure_supported_target "${distro}" "${mode}"
  smoke_configure_paths "${SMOKE_REPO_ROOT}" "${distro}" "${mode}"
  smoke_configure_interactive_mode
  smoke_preflight
  smoke_prepare_workspace
  smoke_prepare_firmware
  smoke_prepare_system_disk
  smoke_start_network_backend

  trap 'smoke_cleanup $?' EXIT

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    smoke_prepare_boot_root
  else
    smoke_prepare_custom_image_iso
  fi
  smoke_print_summary

  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
      smoke_start_dhcp_helper
      smoke_start_host_helper
    fi
    smoke_start_qemu
    return 0
  fi

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    smoke_start_backend
    smoke_wait_for_backend
    smoke_refresh_backend_assets
    smoke_sync_boot_root_from_backend
    smoke_probe_assets
    smoke_start_dhcp_helper
    smoke_start_host_helper
    smoke_mark_guest_evidence_start
  fi
  smoke_start_qemu
  if [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then
    smoke_verify_markers_post_run
    smoke_verify_guest_evidence
  else
    smoke_wait_for_markers
    smoke_verify_guest_evidence
  fi
}
