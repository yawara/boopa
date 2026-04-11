## Why

The smoke test harness is hardcoded to Ubuntu UEFI in ~8 functions (asset sync, probe, evidence verification, markers, summary). This blocks end-to-end testing of Fedora UEFI, even though the Rust backend (boot recipes, GRUB config generation, kickstart generation, HTTP/TFTP serving) already fully supports it. Making the harness distro-generic now prevents per-distro forks of the same shell logic and unblocks Fedora UEFI acceptance.

## What Changes

- Make `scripts/smoke/lib.sh` distro-generic: introduce `SMOKE_DISTRO` / `SMOKE_MODE` variables and dispatch per-distro logic in `smoke_ensure_supported_target`, `smoke_configure_paths`, `smoke_refresh_backend_assets`, `smoke_sync_boot_root_from_backend`, `smoke_probe_assets`, `smoke_verify_guest_evidence`, `smoke_http_evidence_seen`, `smoke_print_summary`, `smoke_seed_dry_run_bootloader`, and marker defaults.
- Add `grubx64.efi` to the Fedora UEFI boot recipe (shim chain-loads it; currently missing).
- Add `url --url=...` to the generated Fedora kickstart so Anaconda can fetch packages without interactive prompts.
- Extend `scripts/smoke/test-harness.sh` with Fedora UEFI dry-run, sync, probe, and evidence regression assertions.
- Enable `boot-fedora-uefi.sh` to run end-to-end through the now-generic harness.

## Capabilities

### New Capabilities

- `smoke-harness-distro-generic`: Distro-generic smoke test harness that dispatches boot-root sync, asset probing, guest evidence verification, and serial markers based on the target distro and boot mode.

### Modified Capabilities

- `dhcp-server-runtime`: No requirement-level change — the DHCP runtime already serves distro-appropriate boot filenames. Only implementation detail: Fedora UEFI recipe gains a `grubx64.efi` asset and the kickstart gains a package repo URL.

## Impact

- **Shell**: `scripts/smoke/lib.sh`, `scripts/smoke/test-harness.sh` — main harness refactor.
- **Rust**: `crates/boot-recipe/src/lib.rs` — add Fedora UEFI `grubx64.efi` asset. `crates/boopa/src/boot_assets/mod.rs` — add `url` directive to kickstart, possibly add `EFI/fedora/grub.cfg` GRUB alias.
- **CI**: Fedora UEFI smoke runs will download ~300-400 MB of RPMs from Fedora mirrors; timeout may need increasing.
- **No API or frontend changes.**
