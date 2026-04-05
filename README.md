# boopa

## WARNING

**EARLY STAGE SOFTWARE: DO NOT USE THIS IN PRODUCTION.**

This project is still in an early stage. Expect breaking changes, missing hardening, incomplete operational safeguards, and behavior that may change without notice.

![boopa](./boopa.png)

`boopa` is a single-service network boot controller for a trusted office LAN. It serves boot assets over HTTP and TFTP, persists the currently selected distro, and exposes a small dashboard/API so a DHCP administrator can switch the active distro and read the DHCP values they must configure manually.

## Scope

- Rust backend with embedded HTTP and TFTP services
- React + TypeScript + RTK Query dashboard
- Supported v1 distros: Ubuntu, Fedora, Arch Linux
- Supported boot modes: BIOS and UEFI
- Image distribution via cached official upstream assets

Out of scope in v1:

- DHCP writeback or DHCP server management
- Auth or access control
- Custom image builds
- Post-install automation

## Runtime assumptions

- Deploy on a trusted LAN or behind a localhost-only tunnel/proxy.
- Point DHCP `next-server` and boot filename values at this service manually.
- Pre-build the frontend into `frontend/dist` or override `BOOPA_FRONTEND_DIR`.

## Environment

- `BOOPA_API_BIND` default: `127.0.0.1:8080`
- `BOOPA_TFTP_BIND` default: `0.0.0.0:6969`
- `BOOPA_DATA_DIR` default: `var/boopa`
- `BOOPA_FRONTEND_DIR` default: `frontend/dist`
- Legacy aliases `NETWORK_BOOTD_API_BIND`, `NETWORK_BOOTD_TFTP_BIND`, `NETWORK_BOOTD_DATA_DIR`, and `NETWORK_BOOTD_FRONTEND_DIR` are still accepted during the rename transition.
- If no data-dir env var is set and `var/boopa` does not exist yet, `boopa` falls back to `var/network-bootd` when that legacy directory is present.

## API

- `GET /api/health`
- `GET /api/distros`
- `GET /api/dhcp`
- `PUT /api/selection`
- `GET /api/cache`
- `POST /api/cache/refresh`

## Verification

Backend:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace`

Frontend:

- `npm run dev --prefix frontend`
- `npm test --prefix frontend`
- `npm run typecheck --prefix frontend`
- `npm run build --prefix frontend`

Frontend dev proxy:

- `npm run dev --prefix frontend` proxies `/api` and `/boot` to `http://127.0.0.1:8080`
- Override the dev backend target with `BOOPA_DEV_BACKEND=http://host:port npm run dev --prefix frontend`
- Legacy alias `NETWORK_BOOTD_DEV_BACKEND` is also accepted.

Smoke scripts:

- `scripts/smoke/boot-ubuntu-bios.sh`
- `scripts/smoke/boot-ubuntu-uefi.sh`
- `scripts/smoke/boot-fedora-bios.sh`
- `scripts/smoke/boot-fedora-uefi.sh`
- `scripts/smoke/boot-arch-bios.sh`
- `scripts/smoke/boot-arch-uefi.sh`
- `scripts/smoke/test-harness.sh`

## Notes

The smoke scripts are structured entrypoints for a QEMU-based verification lane. They require local hypervisor tooling and network access to upstream boot assets, so treat them as operator/CI scripts rather than assumptions for every local build.

Current scope of the concrete harness:

- `scripts/smoke/boot-ubuntu-uefi.sh` is the only implemented target today.
- Other smoke entrypoints fail fast with a clear "not implemented" message.
- The harness stages real Ubuntu installer assets into a temporary data dir, starts `boopa`, then boots a QEMU UEFI guest.
- The first-stage EFI bootloader is staged onto a temporary FAT boot volume with a generated `grub.cfg`.
- Kernel and initrd are fetched from `boopa` over TFTP as `ubuntu/uefi/kernel` and `ubuntu/uefi/initrd`.
- The smoke harness picks random high unprivileged API/TFTP ports by default to avoid local port collisions.
- When launched from an interactive terminal, the harness attaches QEMU serial I/O to that terminal so prompts like `Press any key to continue...` accept input directly. Set `SMOKE_INTERACTIVE=0` to force headless mode.
- Success is log-based: ideal markers indicate installer/live progress, fallback markers indicate kernel/initrd handoff and boot continuation.

Typical local smoke verification:

```sh
scripts/smoke/boot-ubuntu-uefi.sh
```

Dry-run/regression verification for the harness itself:

```sh
scripts/smoke/test-harness.sh
```
