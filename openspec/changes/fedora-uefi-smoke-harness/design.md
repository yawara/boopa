## Context

The Rust backend already supports Fedora UEFI end-to-end: boot recipes define `shimx64.efi` + kernel + initrd assets, GRUB config generation produces `linuxefi`/`initrdefi` entries with a kickstart URL, and the kickstart is served over HTTP. However, the smoke test harness (`scripts/smoke/lib.sh`) hardcodes Ubuntu UEFI in ~8 functions, blocking any other distro from running through the acceptance pipeline.

The harness currently gates on `smoke_ensure_supported_target` (ubuntu/uefi only), uses fixed paths like `cache/ubuntu/uefi` and `ubuntu/uefi/grubx64.efi`, greps for Ubuntu-specific serial markers, and verifies Ubuntu-specific HTTP evidence (the live-server ISO download).

Two Rust-side gaps also exist: the Fedora UEFI recipe is missing a `grubx64.efi` asset (shim chain-loads it), and the generated kickstart lacks a `url --url=...` directive for the package repo.

## Goals / Non-Goals

**Goals:**
- Make the smoke harness dispatch per-distro logic via `SMOKE_DISTRO` / `SMOKE_MODE` variables instead of hardcoding Ubuntu paths.
- Complete the Fedora UEFI boot recipe (add `grubx64.efi`) and kickstart (add package repo URL).
- Add Fedora UEFI dry-run and evidence regression tests to `test-harness.sh`.
- Enable `boot-fedora-uefi.sh` to run a full end-to-end Fedora UEFI install via QEMU.

**Non-Goals:**
- BIOS boot support for any distro (future work).
- Arch Linux UEFI support (future work — same pattern applies).
- Fedora autoinstall configuration UI (kickstart is static for now).
- Serving a local Fedora package mirror (installer uses upstream mirrors).

## Decisions

### 1. Dispatch via case statements in existing functions

Rather than extracting distro-specific logic into separate files or a plugin system, each hardcoded function gains a `case "${SMOKE_DISTRO}"` block. This keeps changes localized and reviewable.

**Alternative considered:** Distro-specific shell scripts sourced by lib.sh (e.g., `lib-ubuntu.sh`, `lib-fedora.sh`). Rejected because the shared surface area is large and the per-distro logic is small — a few paths and grep patterns per function.

### 2. SMOKE_DISTRO / SMOKE_MODE as first-class variables

Set in `smoke_configure_paths` from the positional arguments already passed through `smoke_main`. Every dispatching function reads these instead of re-parsing arguments.

### 3. Fedora UEFI uses shimx64.efi → grubx64.efi chain

The DHCP-advertised boot file remains `fedora/uefi/shimx64.efi`. Shim chain-loads `grubx64.efi` from the same TFTP root. The recipe needs both assets. In the FAT boot volume, `shimx64.efi` is placed at `EFI/BOOT/BOOTX64.EFI` (same as Ubuntu's grub) because UEFI firmware loads from that well-known path.

**Alternative considered:** Skip shim and boot `grubx64.efi` directly. This works in QEMU (no Secure Boot) but diverges from real hardware PXE boot flow. Keeping shim maintains fidelity.

### 4. Kickstart gets `url --url=` pointing at Fedora mirror

The kickstart is the right place for the install source (vs. kernel args) because it's a single editable artifact. The URL points to the official Fedora 41 Server tree. The `inst.repo` kernel arg is not needed when `url` is in the kickstart.

### 5. Fedora HTTP evidence = kickstart fetch, not ISO

Ubuntu's guest-path evidence checks for the ISO download over HTTP. Fedora has no ISO — the equivalent HTTP evidence is the Anaconda installer fetching `fedora/uefi/kickstart/ks.cfg`.

### 6. Fedora serial markers

Default ideal markers for Fedora: `Starting Anaconda|anaconda:|Starting Kickstart|Installation complete`. Fallback markers remain generic (shared with Ubuntu): `Linux version|EFI stub:|Run /init`.

### 7. GRUB config aliases for Fedora

The Fedora shim→GRUB chain may look for `grub.cfg` at several paths. The boot-root sync copies the fetched GRUB config to: `grub/grub.cfg`, `boot/grub/grub.cfg`, `grub2/grub.cfg`, `boot/grub2/grub.cfg`, `fedora/uefi/grub.cfg`, `fedora/uefi/grub/grub.cfg`. This mirrors the alias list already defined in `FEDORA_UEFI_GRUB_CFG_ALIASES` on the Rust side.

## Risks / Trade-offs

**Fedora mirror speed** → Fedora installs download ~300-400 MB from upstream mirrors. Smoke timeout needs increasing for Fedora targets (600s+). Mitigated by making `SMOKE_TIMEOUT_SECS` a per-distro default that can still be overridden.

**Shim→GRUB config discovery** → If shim looks for GRUB config at an unexpected path, the boot stalls. Mitigated by the broad alias list and by the `startup.nsh` fallback that forces `EFI/BOOT/BOOTX64.EFI`.

**Fedora mirror availability** → CI depends on external Fedora mirrors. If mirrors are down, smoke tests fail. No mitigation in first wave — same risk Ubuntu has with its ISO download.
