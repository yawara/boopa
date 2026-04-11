# boopa

## WARNING

**EARLY STAGE SOFTWARE: DO NOT USE THIS IN PRODUCTION.**

This project is still in an early stage. Expect breaking changes, missing hardening, incomplete operational safeguards, and behavior that may change without notice.

![boopa](./boopa.png)

`boopa` is a single-service network boot controller for a trusted office LAN. It serves boot assets over HTTP and TFTP, can optionally act as an authoritative DHCPv4 server for a bounded PXE subnet, persists the currently selected distro, and exposes a small dashboard/API so an operator can inspect DHCP state and boot guidance in one place.

## Scope

- Rust backend with `actix-web` HTTP service and embedded TFTP service
- React + TypeScript + RTK Query dashboard
- Supported v1 distros: Ubuntu, Fedora, Arch Linux
- Supported boot modes: BIOS and UEFI
- Image distribution via cached official upstream assets
- Ubuntu custom image builds are supported in a bounded v1 build lane: Linux-host-only, root-required, and verified with a backendless Ubuntu UEFI smoke path
- `boopa` remains the network-boot controller; custom image builds are a separate build lane, not a runtime distribution path

Out of scope in the current release:

- Proxy-DHCP / DHCP assist mode
- Static reservations / MAC-pinned leases
- Auth or access control
- Post-install automation
- Non-Ubuntu custom image builds
- Non-Linux or rootless custom-image build support

## Runtime assumptions

- Deploy on a trusted LAN or behind a localhost-only tunnel/proxy.
- Point DHCP `next-server` and boot filename values at this service manually.
- Pre-build the frontend into `frontend/dist` or override `BOOPA_FRONTEND_DIR`.

## Environment

- `BOOPA_API_BIND` default: `127.0.0.1:8080`
- `BOOPA_TFTP_BIND` default: `0.0.0.0:6969`
- `BOOPA_TFTP_ADVERTISE_ADDR` default: the TFTP bind address when it is guest-usable, otherwise `127.0.0.1:<tftp-port>`
- `BOOPA_DHCP_MODE` default: `disabled`
- `BOOPA_DHCP_BIND` default: `0.0.0.0:67`
- `BOOPA_DHCP_SUBNET`: required when DHCP mode is `authoritative` (example `10.0.2.0/24`)
- `BOOPA_DHCP_POOL_START` / `BOOPA_DHCP_POOL_END`: required when DHCP mode is `authoritative`
- `BOOPA_DHCP_ROUTER`: optional IPv4 default gateway for leases
- `BOOPA_DHCP_DNS`: optional comma-separated IPv4 DNS servers for leases
- `BOOPA_DHCP_LEASE_SECS` default: `3600`
- `BOOPA_DATA_DIR` default: `var/boopa`
- `BOOPA_FRONTEND_DIR` default: `frontend/dist`

## API

- `GET /api/health`
- `GET /api/distros`
- `GET /api/dhcp` returns both manual BIOS/UEFI guidance and current DHCP runtime status
- `PUT /api/selection`
- `GET /api/cache`
- `POST /api/cache/refresh`

DHCP mode notes:

- DHCP is disabled by default.
- When `BOOPA_DHCP_MODE=authoritative`, boopa serves one IPv4 subnet with dynamic leases only.
- Proxy-DHCP and static reservations are intentionally deferred.

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

- `npm ci --prefix frontend`
- `npm run dev --prefix frontend`
- `npm test --prefix frontend`
- `npm run typecheck --prefix frontend`
- `npm run build --prefix frontend`
- `npx --prefix frontend playwright install chromium`
- `npm run test:e2e --prefix frontend`

DHCP verification for the current release:

- Packet-level DHCP tests remain the fast regression floor for the authoritative DHCP runtime.
- The working guest-path acceptance lane is the mac-host `SMOKE_NETWORK_MODE=vde` smoke path, where `boopa` keeps running directly on macOS and a user-space VDE helper bridges guest DHCP/TFTP/HTTP traffic back to the host process.
- `SMOKE_NETWORK_MODE=vmnet-host` remains an experimental fallback, but on this host/QEMU combination it fails to create the vmnet interface without extra privileges or entitlements.
- Do not treat the legacy `-netdev user` smoke path as proof of boopa-origin DHCP inside the guest network path.

Frontend dev proxy:

- The frontend npm package and lockfile both live under `frontend/`.
- `npm run dev --prefix frontend` proxies `/api` and `/boot` to `http://127.0.0.1:8080`
- Override the dev backend target with `BOOPA_DEV_BACKEND=http://host:port npm run dev --prefix frontend`

Frontend e2e:

- `frontend/playwright.config.ts` starts a real `boopa` backend plus a Vite dev server for browser tests.
- The browser lane uses an isolated `BOOPA_DATA_DIR` under the OS temp directory and never reuses the default `var/boopa`.
- First-wave browser coverage is intentionally narrow:
  - dashboard initial render against the live backend
  - Ubuntu autoinstall edit/save
  - persistence across reload for the saved autoinstall state
- First-wave browser coverage intentionally excludes distro-switch e2e.
- First-wave browser coverage does not validate backend-served static assets in-browser; it exercises the Vite dev server plus live backend path.

Typical local frontend e2e verification:

```sh
npx --prefix frontend playwright install chromium
npm run test:e2e --prefix frontend
```

Smoke CLI:

- `python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi`
- `python3 -m scripts.smoke run --distro ubuntu --boot-mode bios`
- `python3 -m scripts.smoke run --distro fedora --boot-mode uefi`
- `python3 -m scripts.smoke run --distro fedora --boot-mode bios`
- `python3 -m scripts.smoke custom-image`
- `python3 -m scripts.smoke.test_harness`

## Notes

The smoke scripts are structured entrypoints for a QEMU-based verification lane. They require local hypervisor tooling and network access to upstream boot assets, so treat them as operator/CI scripts rather than assumptions for every local build.

Current scope of the concrete harness:

- The canonical surface is `python3 -m scripts.smoke`; legacy shell entrypoints have been removed.
- Formal target coverage is `Ubuntu/Fedora x UEFI/BIOS`, with `custom-image` retained as an Ubuntu UEFI-only lane.
- `Arch` is not part of the supported matrix.
- The harness emits a structured execution plan (`logs/plan.json`) during dry-runs so reviewers can inspect commands, helper processes, and side effects without reading shell internals.
- The guest-path backend is selected with `SMOKE_NETWORK_MODE`.
  - `user` remains the legacy debug/support path and is not DHCP acceptance.
  - `vde` is the current mac-host acceptance path. It starts a user-space `vde_switch` plus a host helper process and keeps `boopa` directly on the host.
  - `vmnet-host` is still available as an experimental backend with `SMOKE_DHCP_HELPER_MODE=podman-relay`, but some host/QEMU combinations reject vmnet interface creation without extra privileges or entitlements.
- BIOS targets are modeled explicitly in the support matrix and planner, but live execution on this host remains unverified until a representative BIOS smoke lane is exercised end to end.
- During smoke runs, `BOOPA_DATA_DIR/cache` is symlinked to `var/boopa/cache` (or `SMOKE_SOURCE_DATA_DIR/cache`) so cached assets and `manifest.json` are reused across runs.
- If a temporary FAT boot volume is needed for the first-stage firmware handoff, it is limited to firmware-carrier files plus `boopa`-served copies of the bootloader and GRUB config.
- `boopa` now generates and serves the Ubuntu UEFI `grub.cfg`; kernel and initrd are fetched from `boopa` over TFTP as `ubuntu/uefi/kernel` and `ubuntu/uefi/initrd`, while the generated `iso-url` points clients at `/boot/ubuntu/uefi/live-server.iso` over HTTP.
- Ubuntu UEFI clients must reach both the advertised TFTP endpoint and `http://<boopa-host>:<api-port>/boot/ubuntu/uefi/live-server.iso`.
- For the mac-host guest-path lane, `boopa` binds DHCP on an unprivileged localhost port and the selected helper backend bridges guest traffic back to that host process; this keeps the workflow under ordinary user permissions without moving `boopa` into a VM or container.
- The Ubuntu UEFI smoke path defaults to `RAM_MB=8192` and provisions a `SYSTEM_DISK_GB=32` qcow2 installer disk because the live installer downloads a multi-gigabyte ISO before pivoting to the live filesystem.
- The smoke harness picks random high unprivileged API/TFTP ports by default to avoid local port collisions.
- When launched from an interactive terminal, the harness attaches QEMU serial I/O to that terminal and enables a QEMU display window by default so VGA/installer output is visible. Set `SMOKE_INTERACTIVE=0` to force headless mode, or override the interactive display backend with `SMOKE_QEMU_DISPLAY` if `default` is not suitable on your host.
- Success is log-based: ideal markers indicate installer/live progress, fallback markers indicate kernel/initrd handoff and boot continuation.

Canonical custom-image smoke shape:

- set `CUSTOM_IMAGE_BASE_ISO`, `CUSTOM_IMAGE_MANIFEST`, and `CUSTOM_IMAGE_OUTPUT_ISO`
- run `python3 -m scripts.smoke custom-image`
- the lane builds the ISO if needed, then boots the generated Ubuntu UEFI image without starting `boopa`

Typical local smoke verification:

```sh
python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi
```

Typical mac-host guest-path smoke shape:

```sh
python3 -m scripts.smoke plan --distro ubuntu --boot-mode uefi --network-mode vde --format json
python3 -m scripts.smoke run --distro ubuntu --boot-mode uefi --network-mode vde
```

Dry-run/regression verification for the harness itself:

```sh
python3 -m scripts.smoke.test_harness
```
