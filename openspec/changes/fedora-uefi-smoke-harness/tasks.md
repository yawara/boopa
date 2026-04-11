## 1. Rust-side Fedora UEFI recipe and kickstart fixes

- [x] 1.1 Add `grubx64.efi` asset to the Fedora UEFI recipe in `crates/boot-recipe/src/lib.rs` (shim chain-loads it)
- [x] 1.2 Add `url --url=https://download.fedoraproject.org/pub/fedora/linux/releases/41/Server/x86_64/os/` directive to the generated kickstart in `crates/boopa/src/boot_assets/mod.rs`
- [x] 1.3 Update existing Rust unit tests to account for the new grubx64.efi asset and kickstart url directive

## 2. Make smoke harness distro-generic (`scripts/smoke/lib.sh`)

- [x] 2.1 Widen `smoke_ensure_supported_target` to accept `fedora/uefi` in addition to `ubuntu/uefi`
- [x] 2.2 Add `SMOKE_DISTRO` and `SMOKE_MODE` variables in `smoke_configure_paths`; make `SMOKE_CACHE_SOURCE_DIR` use `${distro}/${mode}` instead of hardcoded `ubuntu/uefi`
- [x] 2.3 Set per-distro default `SMOKE_IDEAL_MARKERS` (Anaconda/Kickstart for Fedora) while keeping generic `SMOKE_FALLBACK_MARKERS`; set per-distro default `SMOKE_TIMEOUT_SECS` (600s for Fedora)
- [x] 2.4 Make `smoke_refresh_backend_assets` use `SMOKE_DISTRO` instead of hardcoded `"ubuntu"`
- [x] 2.5 Refactor `smoke_sync_boot_root_from_backend` to dispatch per distro: Ubuntu fetches `grubx64.efi` + Ubuntu grub aliases; Fedora fetches `shimx64.efi` + Fedora grub/grub2 aliases
- [x] 2.6 Make `smoke_seed_dry_run_bootloader` select the correct source bootloader filename per distro
- [x] 2.7 Refactor `smoke_probe_assets` to dispatch per distro: Ubuntu probes ISO, Fedora probes kickstart
- [x] 2.8 Refactor `smoke_verify_guest_evidence` to dispatch per distro: Ubuntu checks ISO HTTP evidence, Fedora checks kickstart HTTP evidence; TFTP evidence uses distro-specific kernel/initrd paths
- [x] 2.9 Refactor `smoke_http_evidence_seen` to dispatch per distro
- [x] 2.10 Make `smoke_print_summary` use `SMOKE_DISTRO`/`SMOKE_MODE` instead of hardcoded "ubuntu uefi" and Ubuntu ISO URL

## 3. Extend test harness (`scripts/smoke/test-harness.sh`)

- [x] 3.1 Replace the "unsupported target" assertion for `fedora uefi` with a new assertion that the target is now accepted
- [x] 3.2 Add Fedora UEFI dry-run block: create stub cache files (`shimx64.efi`, `grubx64.efi`, `kernel`, `initrd`), run `common.sh fedora uefi` in dry-run, assert QEMU command log, BOOTX64.EFI, startup.nsh
- [x] 3.3 Add Fedora UEFI boot-root sync test: mock `smoke_fetch_backend_asset` for `fedora/uefi/shimx64.efi` and `fedora/uefi/grub.cfg`, verify grub2 alias directories are created
- [x] 3.4 Add Fedora UEFI evidence verification test: verify `smoke_verify_guest_evidence` checks for `fedora/uefi/kernel`, `fedora/uefi/initrd`, and `fedora/uefi/kickstart/ks.cfg`

## 4. Validate end-to-end

- [x] 4.1 Run `scripts/smoke/test-harness.sh` and verify all dry-run regression assertions pass (both Ubuntu and Fedora)
- [x] 4.2 Run `cargo test -p boopa` to verify Rust unit tests pass
- [ ] 4.3 Run a Fedora UEFI interactive smoke boot (`SMOKE_INTERACTIVE=1 ./scripts/smoke/boot-fedora-uefi.sh`) and confirm Anaconda reaches the kickstart installation phase
