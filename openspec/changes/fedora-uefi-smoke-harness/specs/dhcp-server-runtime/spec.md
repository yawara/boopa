## MODIFIED Requirements

### Requirement: PXE boot options follow the active distro and client architecture
The system SHALL answer eligible IPv4 PXE DHCP requests from the configured pool and MUST include `next-server`, boot filename, and PXE-related options that match boopa's current selected distro and the requesting client's architecture. The Fedora UEFI boot recipe SHALL include both `shimx64.efi` and `grubx64.efi` assets so the shim-to-GRUB chain-load works over TFTP.

#### Scenario: BIOS client receives BIOS boot filename
- **WHEN** the selected distro is Ubuntu and a BIOS PXE client requests a lease
- **THEN** the DHCP response advertises boopa as `next-server` and includes the Ubuntu BIOS boot filename

#### Scenario: UEFI client receives UEFI boot filename
- **WHEN** the selected distro is Fedora and a UEFI PXE client requests a lease
- **THEN** the DHCP response advertises boopa as `next-server` and includes the Fedora UEFI boot filename

#### Scenario: Fedora UEFI recipe serves both shim and GRUB binaries
- **WHEN** a Fedora UEFI PXE client fetches boot assets from the TFTP server
- **THEN** both `fedora/uefi/shimx64.efi` and `fedora/uefi/grubx64.efi` are available

#### Scenario: Fedora UEFI kickstart includes a package repository URL
- **WHEN** the Anaconda installer fetches the generated kickstart from boopa
- **THEN** the kickstart contains a `url --url=` directive pointing to a Fedora package repository so the installer can fetch packages without interactive prompts
