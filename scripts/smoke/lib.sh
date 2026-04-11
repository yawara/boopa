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
  if [[ "${distro}" != "ubuntu" || "${mode}" != "uefi" ]]; then
    smoke_die "only ubuntu uefi is implemented right now; got ${distro} ${mode}"
  fi
}

smoke_configure_paths() {
  local repo_root="$1"
  local distro="$2"
  local mode="$3"
  local target_name="${SMOKE_TARGET_NAME:-${distro}-${mode}}"

  SMOKE_QEMU_BIN="${QEMU_BIN:-qemu-system-x86_64}"
  SMOKE_RAM_MB="${RAM_MB:-8192}"
  SMOKE_SYSTEM_DISK_GB="${SYSTEM_DISK_GB:-32}"
  SMOKE_TIMEOUT_SECS="${SMOKE_TIMEOUT_SECS:-180}"
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
  SMOKE_SYSTEM_DISK_PATH="${SMOKE_RUN_DIR}/system-disk.qcow2"
  SMOKE_API_HOST="${SMOKE_API_HOST:-127.0.0.1}"
  SMOKE_API_BIND_HOST="${SMOKE_API_BIND_HOST:-0.0.0.0}"
  SMOKE_TFTP_BIND_HOST="${SMOKE_TFTP_BIND_HOST:-0.0.0.0}"
  SMOKE_API_PORT="${SMOKE_API_PORT:-$((18080 + RANDOM % 2000))}"
  SMOKE_TFTP_PORT="${SMOKE_TFTP_PORT:-$((24000 + RANDOM % 2000))}"
  SMOKE_GUEST_HOST_IP="${SMOKE_GUEST_HOST_IP:-10.0.2.2}"
  if [[ -n "${SMOKE_SOURCE_DATA_DIR:-}" ]]; then
    SMOKE_SOURCE_DATA_DIR="${SMOKE_SOURCE_DATA_DIR}"
  elif [[ -d "${repo_root}/var/boopa" ]]; then
    SMOKE_SOURCE_DATA_DIR="${repo_root}/var/boopa"
  else
    SMOKE_SOURCE_DATA_DIR="${repo_root}/var/boopa"
  fi
  SMOKE_CACHE_SOURCE_DIR="${SMOKE_SOURCE_DATA_DIR}/cache/ubuntu/uefi"
  SMOKE_FRONTEND_DIR="${SMOKE_FRONTEND_DIR:-${repo_root}/frontend/dist}"
  SMOKE_DRY_RUN="${SMOKE_DRY_RUN:-0}"
  SMOKE_SKIP_DOWNLOADS="${SMOKE_SKIP_DOWNLOADS:-0}"
  SMOKE_QEMU_ACCEL="${SMOKE_QEMU_ACCEL:-tcg}"
  SMOKE_QEMU_DISPLAY="${SMOKE_QEMU_DISPLAY:-default}"
  SMOKE_IDEAL_MARKERS="${SMOKE_IDEAL_MARKERS:-Reached target System Initialization|Ubuntu installer|Subiquity|Starting system log daemon}"
  SMOKE_FALLBACK_MARKERS="${SMOKE_FALLBACK_MARKERS:-Linux version|EFI stub:|Run /init as init process|Loading initial ramdisk|Freeing initrd memory}"
}

smoke_prepare_workspace() {
  mkdir -p "${SMOKE_RUN_DIR}" "${SMOKE_SERVICE_DATA_DIR}" "${SMOKE_LOG_DIR}"

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    mkdir -p "${SMOKE_TFTP_ROOT}/ubuntu/uefi" "${SMOKE_TFTP_ROOT}/EFI/BOOT"

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
  local source_path="${SMOKE_CACHE_SOURCE_DIR}/grubx64.efi"
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
    BOOPA_API_BIND="${SMOKE_API_BIND_HOST}:${SMOKE_API_PORT}" \
    BOOPA_TFTP_BIND="${SMOKE_TFTP_BIND_HOST}:${SMOKE_TFTP_PORT}" \
    BOOPA_TFTP_ADVERTISE_ADDR="${SMOKE_GUEST_HOST_IP}:${SMOKE_TFTP_PORT}" \
    BOOPA_DATA_DIR="${SMOKE_SERVICE_DATA_DIR}" \
    BOOPA_FRONTEND_DIR="${SMOKE_FRONTEND_DIR}" \
    cargo run -p boopa --quiet
  ) >"${SMOKE_BACKEND_LOG}" 2>&1 &
  SMOKE_BACKEND_PID=$!
}

smoke_wait_for_backend() {
  local attempts=0
  smoke_log "waiting for backend health endpoint"
  while [[ "${attempts}" -lt 60 ]]; do
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
  smoke_log "refreshing Ubuntu assets through boopa"
  curl -fsS \
    -X POST \
    -H 'content-type: application/json' \
    --data '{"distro":"ubuntu"}' \
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

  smoke_log "syncing firmware-carrier assets from boopa"
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
  rm -f "${fetched_grub_cfg}"
}

smoke_probe_assets() {
  smoke_log "probing backend boot asset endpoints before guest boot"
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/ubuntu/uefi/grubx64.efi" -o /dev/null
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/ubuntu/uefi/grub.cfg" -o /dev/null
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/ubuntu/uefi/kernel" -o /dev/null
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/ubuntu/uefi/initrd" -o /dev/null
  curl -fsS "http://${SMOKE_API_HOST}:${SMOKE_API_PORT}/boot/ubuntu/uefi/live-server.iso" -o /dev/null
  smoke_log "backend asset probes succeeded"
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

smoke_start_qemu() {
  local boot_media=()
  while IFS= read -r -d '' boot_arg; do
    boot_media+=("${boot_arg}")
  done < <(smoke_boot_media_args)

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
    -netdev "user,id=net0,ipv6=off"
    -device "e1000,netdev=net0"
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
      -netdev "user,id=net0,ipv6=off"
      -device "e1000,netdev=net0"
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
  deadline=$((SECONDS + SMOKE_TIMEOUT_SECS))
  next_heartbeat=$((SECONDS + heartbeat_interval))
  smoke_log "watching serial log for success markers for up to ${SMOKE_TIMEOUT_SECS}s"

  while [[ "${SECONDS}" -lt "${deadline}" ]]; do
    if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_IDEAL_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
      smoke_log "ideal marker matched"
      return 0
    fi
    if [[ -f "${SMOKE_SERIAL_LOG}" ]] && grep -E -q "${SMOKE_FALLBACK_MARKERS}" "${SMOKE_SERIAL_LOG}"; then
      smoke_log "fallback marker matched"
      return 0
    fi

    if [[ -n "${SMOKE_QEMU_PID:-}" ]] && ! kill -0 "${SMOKE_QEMU_PID}" >/dev/null 2>&1; then
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
Smoke target: ubuntu uefi
Run dir: ${SMOKE_RUN_DIR}
API base: http://${SMOKE_API_HOST}:${SMOKE_API_PORT}
TFTP endpoint: ${SMOKE_GUEST_HOST_IP}:${SMOKE_TFTP_PORT}
Ubuntu ISO URL: http://${SMOKE_GUEST_HOST_IP}:${SMOKE_API_PORT}/boot/ubuntu/uefi/live-server.iso
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

  trap 'smoke_cleanup $?' EXIT

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    smoke_prepare_boot_root
  else
    smoke_prepare_custom_image_iso
  fi
  smoke_print_summary

  if [[ "${SMOKE_DRY_RUN}" == "1" ]]; then
    smoke_start_qemu
    return 0
  fi

  if [[ "${SMOKE_LANE:-backend}" == "backend" ]]; then
    smoke_start_backend
    smoke_wait_for_backend
    smoke_refresh_backend_assets
    smoke_sync_boot_root_from_backend
    smoke_probe_assets
  fi
  smoke_start_qemu
  if [[ "${SMOKE_INTERACTIVE}" == "1" ]]; then
    smoke_verify_markers_post_run
  else
    smoke_wait_for_markers
  fi
}
