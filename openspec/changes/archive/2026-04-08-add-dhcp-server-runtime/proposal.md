## Why

boopa currently stops at HTTP/TFTP and asks an operator to configure DHCP manually. That leaves the selected distro and the actual PXE boot path split across two systems, which creates drift and prevents boopa from acting as a single-box network-boot controller on a trusted LAN.

## What Changes

- Add an optional in-process DHCP runtime to boopa so it can answer PXE-oriented IPv4 DHCP requests for a configured subnet and lease pool.
- Generate DHCP `next-server`, boot filename, and related PXE boot values from boopa's active distro selection and advertised boot endpoints.
- Persist DHCP lease/runtime state under `BOOPA_DATA_DIR` and expose bounded operational status through boopa's existing API/dashboard surfaces.
- Document the deployment model, configuration, and verification path for running boopa as the DHCP authority for a small trusted LAN.

## Capabilities

### New Capabilities
- `dhcp-server-runtime`: Serve bounded PXE-focused DHCP responses, track leases, and derive boot options from boopa's selected distro.

### Modified Capabilities
- None.

## Impact

- `crates/boopa` runtime startup, config parsing, state management, persistence, and HTTP routes
- Potential DHCP protocol implementation or dependency evaluation in the Rust backend
- Frontend/dashboard DHCP surfaces if runtime status is exposed there
- README, operator guidance, and test/smoke coverage for DHCP + TFTP boot flow
