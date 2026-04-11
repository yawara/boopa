## 1. Guest Network Harness

- [x] 1.1 Add a mac-host-compatible guest-path network backend to the smoke harness and fail fast when the required user-space helper or podman-assisted prerequisites are missing. <!-- oms:files=scripts/smoke/lib.sh,scripts/smoke/test-harness.sh -->
- [x] 1.2 Rewire the Ubuntu UEFI smoke lane so boopa stays directly on the mac host with one guest-visible address plan for DHCP, TFTP, and HTTP, and the guest boots without relying on the current DHCPless `-netdev user` acceptance path. <!-- oms:files=scripts/smoke/lib.sh,scripts/smoke/vde_host_helper.py,README.md -->

## 2. End-To-End Evidence

- [x] 2.1 Add stable evidence markers for DHCP lease metadata, TFTP asset serving, and HTTP `iso-url` access so the smoke lane can prove `DHCP -> TFTP -> grub -> kernel + initrd -> HTTP iso-url`. <!-- oms:files=crates/boopa/src/dhcp/mod.rs,crates/boopa/src/http/mod.rs,scripts/smoke/lib.sh,scripts/smoke/vde_host_helper.py -->
- [x] 2.2 Extend harness regression coverage so unsupported fallback paths fail, guest-path prerequisites are checked, and the authoritative Ubuntu UEFI smoke path is the documented acceptance lane. <!-- oms:files=scripts/smoke/test-harness.sh,README.md -->

## 3. Verification Boundary

- [x] 3.1 Keep fast packet-level DHCP tests as supporting coverage, but update docs and verification commands so guest-path E2E is required before the change is marked complete. <!-- oms:files=README.md -->

