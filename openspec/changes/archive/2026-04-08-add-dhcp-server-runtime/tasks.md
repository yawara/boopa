## 1. Configuration And Lifecycle

- [x] 1.1 Extend boopa configuration with opt-in DHCP settings, validation rules, and persistence paths for lease runtime state. <!-- oms:files=crates/boopa/src/config.rs,crates/boopa/src/persistence/mod.rs,crates/boopa/src/app_state.rs;verify=openspec-validate,cargo-check,cargo-test -->
- [x] 1.2 Define the startup and shutdown lifecycle for the DHCP runtime so it can run beside the existing HTTP and TFTP services without changing default behavior. <!-- oms:files=crates/boopa/src/lib.rs,crates/boopa/src/main.rs -->

## 2. DHCP Runtime

- [x] 2.1 Implement the bounded DHCP server loop for one IPv4 PXE subnet and configured lease pool. <!-- oms:files=crates/boopa/src/dhcp/mod.rs -->
- [x] 2.2 Generate BIOS and UEFI PXE boot options from boopa's selected distro and advertised boot endpoints. <!-- oms:files=crates/boopa/src/dhcp/mod.rs,crates/boopa/src/app_state.rs -->
- [x] 2.3 Persist active lease state and reload unexpired leases on restart. <!-- oms:files=crates/boopa/src/persistence/mod.rs,crates/boopa/src/app_state.rs,crates/boopa/tests/dhcp_runtime.rs -->

## 3. Operator Surface And Verification

- [x] 3.1 Add a read-only API surface for DHCP runtime status and current leases, and wire dashboard support if the chosen scope requires it. <!-- oms:files=crates/boopa/src/app_state.rs,frontend/src/services/api.ts,frontend/src/components/DhcpGuideCard.tsx,frontend/src/pages/DashboardPage.tsx -->
- [x] 3.2 Add backend tests for configuration validation, lease allocation, distro-switch propagation, and persisted-lease recovery. <!-- oms:files=crates/boopa/tests/dhcp_runtime.rs,crates/boopa/tests/api.rs,crates/boopa/src/config.rs;verify=cargo-test -->
- [x] 3.3 Add an isolated verification path covering DHCP plus existing TFTP and boot behavior, and document how operators should enable and validate the feature. <!-- oms:files=README.md,crates/boopa/tests/dhcp_runtime.rs,crates/boopa/tests/http_boot.rs,crates/boopa/tests/tftp_boot.rs;verify=openspec-validate,cargo-test,frontend-test,frontend-typecheck,frontend-build -->

