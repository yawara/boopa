# dhcp-server-runtime Specification

## Purpose
TBD - created by archiving change add-dhcp-server-runtime. Update Purpose after archive.
## Requirements
### Requirement: Optional DHCP server lifecycle
The system SHALL keep the DHCP server disabled unless the operator explicitly enables it with a complete DHCP configuration. When DHCP is enabled with invalid or incomplete settings, boopa MUST refuse to start the DHCP runtime and return an actionable error.

#### Scenario: DHCP remains disabled by default
- **WHEN** boopa starts without DHCP enablement configured
- **THEN** boopa does not bind a DHCP socket and continues serving its existing HTTP and TFTP functionality

#### Scenario: Invalid DHCP configuration is rejected
- **WHEN** boopa starts with DHCP enabled but the configured subnet or lease pool is invalid
- **THEN** boopa does not start the DHCP runtime and reports which configuration value is invalid

### Requirement: PXE boot options follow the active distro and client architecture
The system SHALL answer eligible IPv4 PXE DHCP requests from the configured pool and MUST include `next-server`, boot filename, and PXE-related options that match boopa's current selected distro and the requesting client's architecture.

#### Scenario: BIOS client receives BIOS boot filename
- **WHEN** the selected distro is Ubuntu and a BIOS PXE client requests a lease
- **THEN** the DHCP response advertises boopa as `next-server` and includes the Ubuntu BIOS boot filename

#### Scenario: UEFI client receives UEFI boot filename
- **WHEN** the selected distro is Fedora and a UEFI PXE client requests a lease
- **THEN** the DHCP response advertises boopa as `next-server` and includes the Fedora UEFI boot filename

### Requirement: Active distro changes apply to future DHCP responses without a restart
The system SHALL use the latest persisted distro selection for all new DHCP offers and acknowledgements without requiring the operator to restart boopa or the DHCP runtime.

#### Scenario: Distro switch changes subsequent offers
- **WHEN** the operator changes the selected distro from Ubuntu to Arch through boopa's existing selection flow
- **THEN** the next eligible DHCP response uses Arch boot metadata instead of Ubuntu boot metadata

### Requirement: DHCP leases are persisted and observable
The system SHALL record active DHCP lease assignments with enough metadata to recover unexpired leases after restart and MUST expose read-only runtime status and lease information through boopa's API surface.

#### Scenario: Lease survives restart
- **WHEN** boopa restarts while a previously assigned lease is still unexpired
- **THEN** the lease remains reserved for the same client identifier after startup completes

#### Scenario: Operator can inspect runtime status
- **WHEN** the operator requests DHCP runtime status through boopa's API
- **THEN** the response includes whether DHCP is enabled, the active address pool, and the current active leases

### Requirement: First-wave scope is bounded to one IPv4 PXE subnet
The system SHALL support one configured IPv4 subnet and one bounded lease pool in the first release and MUST reject unsupported DHCP topologies that fall outside that scope.

#### Scenario: Unsupported topology is rejected
- **WHEN** the operator attempts to configure multiple lease pools or an IPv6-only DHCP deployment
- **THEN** boopa rejects the configuration and explains that only one IPv4 PXE subnet is supported in the first release

### Requirement: Guest-path smoke harness proves the authoritative DHCP boot chain
The system SHALL provide a supported guest-path verification lane for the authoritative DHCP runtime, and that lane MUST prove that a PXE guest boots through boopa-owned DHCP, TFTP, and HTTP stages without silently falling back to a DHCPless network path.

#### Scenario: Ubuntu UEFI guest boots through boopa-origin DHCP
- **WHEN** an operator runs the supported Ubuntu UEFI guest-path smoke lane against `boopa` with authoritative DHCP enabled
- **THEN** the guest receives its lease and PXE boot metadata from boopa
- **AND** the guest fetches GRUB, kernel, and initrd through boopa-managed TFTP paths
- **AND** the boot flow reaches boopa's HTTP `iso-url` for the Ubuntu live installer payload
- **AND** `boopa` itself remains running directly on the mac host rather than inside a container or VM

#### Scenario: Guest-path prerequisites are missing
- **WHEN** the supported mac-host guest network backend or user-space helper path is unavailable or misconfigured
- **THEN** the smoke lane fails before guest boot with an actionable prerequisite error
- **AND** the harness does not downgrade to `-netdev user` or any other DHCPless fallback while reporting success

#### Scenario: User-permission boundary is preserved
- **WHEN** an operator runs the supported guest-path smoke lane on macOS
- **THEN** `boopa` runs directly on the mac host under ordinary user permissions
- **AND** any helper process used by the lane also runs without requiring `sudo`

