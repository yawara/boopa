## ADDED Requirements

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
