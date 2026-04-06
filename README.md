# boopa

## WARNING

**EARLY STAGE SOFTWARE: DO NOT USE THIS IN PRODUCTION.**

This project is still in an early stage. Expect breaking changes, missing hardening, incomplete operational safeguards, and behavior that may change without notice.

![boopa](./boopa.png)

`boopa` is a single-service network boot controller for a trusted office LAN. It serves boot assets over HTTP and TFTP, persists the currently selected distro, and exposes a small dashboard/API so a DHCP administrator can switch the active distro and read the DHCP values they must configure manually.

## Scope

- Rust backend with `actix-web` HTTP service and embedded TFTP service
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
- `BOOPA_TFTP_ADVERTISE_ADDR` default: the TFTP bind address when it is guest-usable, otherwise `127.0.0.1:<tftp-port>`
- `BOOPA_DATA_DIR` default: `var/boopa`
- `BOOPA_FRONTEND_DIR` default: `frontend/dist`

## API

- `GET /api/health`
- `GET /api/distros`
- `GET /api/dhcp`
- `PUT /api/selection`
- `GET /api/cache`
- `POST /api/cache/refresh`

Cache refresh behavior:

- Cache refresh is manual only; assets are refreshed when `POST /api/cache/refresh` is called.
- `boopa` persists asset hashes in `BOOPA_DATA_DIR/cache/manifest.json`.
- If a recipe asset file already exists and its stored SHA-256 plus `source_url` still match, refresh skips re-downloading that asset.
- If the file is missing, the hash differs, or the recipe `source_url` changed, refresh downloads the asset again and updates the manifest.
- `force` refresh is not implemented yet.

## Verification

Backend:

- `cargo fmt --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo build --workspace`

Frontend:

- `npm ci`
- `npm run dev --workspace frontend`
- `npm run test --workspace frontend`
- `npm run typecheck --workspace frontend`
- `npm run build --workspace frontend`

Frontend dev proxy:

- The npm lockfile lives at the repository root as `package-lock.json`, and `frontend` is installed as an npm workspace.
- `npm run dev --workspace frontend` proxies `/api` and `/boot` to `http://127.0.0.1:8080`
- Override the dev backend target with `BOOPA_DEV_BACKEND=http://host:port npm run dev --workspace frontend`

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
- The harness starts `boopa`, refreshes the Ubuntu cache through `POST /api/cache/refresh`, and treats `boopa` as the only source of Ubuntu UEFI boot assets.
- During smoke runs, `BOOPA_DATA_DIR/cache` is symlinked to `var/boopa/cache` (or `SMOKE_SOURCE_DATA_DIR/cache`) so cached assets and `manifest.json` are reused across runs.
- If a temporary FAT boot volume is needed for the first-stage firmware handoff, it is limited to firmware-carrier files plus `boopa`-served copies of the bootloader and GRUB config.
- `boopa` now generates and serves the Ubuntu UEFI `grub.cfg`; kernel and initrd are fetched from `boopa` over TFTP as `ubuntu/uefi/kernel` and `ubuntu/uefi/initrd`, while the generated `iso-url` points clients at `/boot/ubuntu/uefi/live-server.iso` over HTTP.
- Ubuntu UEFI clients must reach both the advertised TFTP endpoint and `http://<boopa-host>:<api-port>/boot/ubuntu/uefi/live-server.iso`.
- The Ubuntu UEFI smoke path defaults to `RAM_MB=8192` and provisions a `SYSTEM_DISK_GB=32` qcow2 installer disk because the live installer downloads a multi-gigabyte ISO before pivoting to the live filesystem.
- The smoke harness picks random high unprivileged API/TFTP ports by default to avoid local port collisions.
- When launched from an interactive terminal, the harness attaches QEMU serial I/O to that terminal and enables a QEMU display window by default so VGA/installer output is visible. Set `SMOKE_INTERACTIVE=0` to force headless mode, or override the interactive display backend with `SMOKE_QEMU_DISPLAY` if `default` is not suitable on your host.
- Success is log-based: ideal markers indicate installer/live progress, fallback markers indicate kernel/initrd handoff and boot continuation.

Typical local smoke verification:

```sh
scripts/smoke/boot-ubuntu-uefi.sh
```

Dry-run/regression verification for the harness itself:

```sh
scripts/smoke/test-harness.sh
```
