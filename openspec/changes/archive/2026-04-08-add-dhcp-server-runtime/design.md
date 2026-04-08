## Context

Today boopa serves HTTP and TFTP assets, persists the active distro selection, and shows the DHCP values that an external administrator must apply manually. The backend runtime only binds API and TFTP sockets, and the README explicitly treats DHCP management as out of scope in v1.

Adding DHCP server behavior changes boopa from a guidance tool into the LAN's PXE control plane. That introduces new runtime concerns: lease allocation, socket binding and privilege requirements, safety defaults, persistence across restart, and operational visibility when the selected distro changes.

## Goals / Non-Goals

**Goals:**
- Add an optional DHCP server that boopa can run alongside HTTP and TFTP.
- Keep DHCP responses tied to boopa's selected distro so BIOS and UEFI PXE clients get matching boot data.
- Persist enough DHCP state to survive restarts and support operator debugging.
- Preserve current deployments by keeping the feature disabled until explicitly configured.

**Non-Goals:**
- Becoming a full-featured enterprise DHCP appliance.
- IPv6 DHCP support, multi-subnet orchestration, or high-availability failover in the first slice.
- Managing an external DHCP server through writeback/relay APIs.
- Adding auth, tenancy, or general office-network management features outside PXE boot control.

## Decisions

### 1. Run DHCP in-process and keep it disabled by default

boopa should own DHCP directly in the same runtime rather than shelling out to a sidecar daemon. That keeps the active distro, TFTP advertise address, and DHCP boot options in one process and one deployment artifact.

Alternatives considered:
- External DHCP daemon or sidecar: rejected because it keeps the core drift problem and adds a second operational surface.
- DHCP writeback only: rejected because the user asked for DHCP server functionality, not another manual integration layer.

### 2. Scope the first implementation to a bounded IPv4 PXE use case

The initial lane should focus on a single-subnet IPv4 pool with PXE-centric options only. boopa already targets a trusted LAN and a narrow boot-controller role, so the DHCP server should optimize for that same bounded environment.

Alternatives considered:
- General-purpose DHCP feature parity: rejected because it expands the project far beyond its current operational scope.
- Proxy-DHCP-only first: deferred until deep-interview clarifies whether authoritative DHCP is acceptable for the intended deployment.

### 3. Reuse boopa's existing config/persistence patterns

DHCP enablement, bind/interface details, lease pool, and network options should live under new `BOOPA_DHCP_*` settings, with lease/runtime files stored under `BOOPA_DATA_DIR`. This matches the current pattern for selection and autoinstall persistence and keeps rollback simple.

Alternatives considered:
- Separate config file format or external state store: rejected because the project currently uses env-driven config plus local files.
- Mandatory new infrastructure dependency: rejected at the proposal stage because the repo prefers no new dependencies unless the need is justified explicitly.

### 4. Expose read-only DHCP runtime visibility through boopa

Operators need to confirm that DHCP is enabled, see which pool/config is active, and inspect current leases without logging into the host directly. A small read-only API surface fits the existing dashboard model better than logs alone.

Alternatives considered:
- Logs only: rejected because the project already exposes dashboard/API status surfaces for operators.
- Full lease management UI: deferred until deep-interview clarifies whether first-wave scope includes reservations or lease administration.

## Risks / Trade-offs

- [Protocol complexity] -> DHCP and PXE option handling are stateful and easy to get subtly wrong, so integration tests must exercise packet exchange rather than only unit-level helpers.
- [Operational conflicts] -> Running a second DHCP service on the same LAN is dangerous; boopa must default to disabled and document conflict risks clearly.
- [Privilege and bind requirements] -> Real DHCP often needs privileged ports or interface-specific binding; config validation and test-only port overrides should keep local verification practical.
- [State drift] -> Lease persistence can become stale after restart; storing expiration metadata and pruning expired leases on load reduces that risk.

## Migration Plan

1. Ship the feature behind explicit `BOOPA_DHCP_*` opt-in settings with current behavior unchanged by default.
2. Validate the service in an isolated LAN or test harness before enabling it on a shared segment.
3. Roll back by disabling the DHCP settings and restarting boopa, leaving existing HTTP/TFTP behavior intact.

## Open Questions

- Is the first slice expected to be an authoritative DHCP server, proxy-DHCP responder, or both?
- Do we need static reservations in the initial rollout, or is dynamic allocation enough?
- Is a new Rust DHCP protocol crate acceptable if implementing the wire protocol from scratch is too risky?
