## ADDED Requirements

### Requirement: Smoke harness dispatches boot-root sync per distro
The smoke harness SHALL select the correct bootloader asset, GRUB config source path, and GRUB config alias directories based on the target distro when syncing the firmware-carrier boot volume from the backend.

#### Scenario: Ubuntu UEFI sync fetches grubx64.efi and Ubuntu GRUB aliases
- **WHEN** the smoke target is ubuntu/uefi
- **THEN** the harness fetches `ubuntu/uefi/grubx64.efi` as `EFI/BOOT/BOOTX64.EFI` and copies the GRUB config to `grub/grub.cfg`, `boot/grub/grub.cfg`, `ubuntu/uefi/grub.cfg`, and `ubuntu/uefi/grub/grub.cfg`

#### Scenario: Fedora UEFI sync fetches shimx64.efi and Fedora GRUB aliases
- **WHEN** the smoke target is fedora/uefi
- **THEN** the harness fetches `fedora/uefi/shimx64.efi` as `EFI/BOOT/BOOTX64.EFI` and copies the GRUB config to `grub/grub.cfg`, `boot/grub/grub.cfg`, `grub2/grub.cfg`, `boot/grub2/grub.cfg`, `fedora/uefi/grub.cfg`, and `fedora/uefi/grub/grub.cfg`

### Requirement: Smoke harness probes distro-appropriate backend assets before guest boot
The smoke harness SHALL verify that all boot-critical assets for the target distro are reachable from the backend's HTTP endpoint before launching QEMU.

#### Scenario: Ubuntu UEFI probes include ISO
- **WHEN** the smoke target is ubuntu/uefi
- **THEN** the harness probes `ubuntu/uefi/grubx64.efi`, `ubuntu/uefi/grub.cfg`, `ubuntu/uefi/kernel`, `ubuntu/uefi/initrd`, and `ubuntu/uefi/live-server.iso`

#### Scenario: Fedora UEFI probes include kickstart
- **WHEN** the smoke target is fedora/uefi
- **THEN** the harness probes `fedora/uefi/shimx64.efi`, `fedora/uefi/grub.cfg`, `fedora/uefi/kernel`, `fedora/uefi/initrd`, and `fedora/uefi/kickstart/ks.cfg`

### Requirement: Smoke harness verifies distro-appropriate guest-path evidence
The smoke harness SHALL verify that the guest actually fetched the expected TFTP and HTTP assets by checking backend or host-helper logs for distro-specific paths.

#### Scenario: Ubuntu UEFI evidence includes ISO download
- **WHEN** the smoke target is ubuntu/uefi and the guest has booted
- **THEN** the harness verifies DHCP lease evidence, TFTP evidence for `ubuntu/uefi/kernel` and `ubuntu/uefi/initrd`, and HTTP evidence for `ubuntu/uefi/live-server.iso`

#### Scenario: Fedora UEFI evidence includes kickstart download
- **WHEN** the smoke target is fedora/uefi and the guest has booted
- **THEN** the harness verifies DHCP lease evidence, TFTP evidence for `fedora/uefi/kernel` and `fedora/uefi/initrd`, and HTTP evidence for `fedora/uefi/kickstart/ks.cfg`

### Requirement: Smoke harness uses distro-appropriate serial success markers
The smoke harness SHALL default to distro-specific ideal serial markers while keeping generic fallback markers shared across distros.

#### Scenario: Ubuntu UEFI uses Ubuntu-specific ideal markers
- **WHEN** the smoke target is ubuntu/uefi and SMOKE_IDEAL_MARKERS is not overridden
- **THEN** the harness watches for Ubuntu-specific markers including "Ubuntu installer" and "Subiquity"

#### Scenario: Fedora UEFI uses Anaconda-specific ideal markers
- **WHEN** the smoke target is fedora/uefi and SMOKE_IDEAL_MARKERS is not overridden
- **THEN** the harness watches for Fedora-specific markers including "Anaconda" and "Kickstart"

### Requirement: Smoke harness refreshes cache for the active distro
The smoke harness SHALL send the target distro identifier when requesting a backend cache refresh, not a hardcoded distro name.

#### Scenario: Fedora UEFI cache refresh targets fedora
- **WHEN** the smoke target is fedora/uefi
- **THEN** the harness sends `{"distro":"fedora"}` to the cache refresh endpoint

### Requirement: Smoke harness accepts both ubuntu/uefi and fedora/uefi targets
The smoke harness SHALL allow both ubuntu/uefi and fedora/uefi as supported targets without dying at the gate check.

#### Scenario: Fedora UEFI passes the supported-target gate
- **WHEN** `common.sh` is invoked with arguments `fedora uefi`
- **THEN** the harness proceeds without error

#### Scenario: Unsupported targets are still rejected
- **WHEN** `common.sh` is invoked with arguments `arch bios`
- **THEN** the harness exits with an error indicating the target is unsupported

### Requirement: Test harness includes Fedora UEFI dry-run regression assertions
The test harness (`test-harness.sh`) SHALL include dry-run regression assertions for Fedora UEFI that verify QEMU command generation, boot volume layout, and GRUB alias placement, parallel to the existing Ubuntu UEFI assertions.

#### Scenario: Fedora UEFI dry-run produces correct QEMU command and boot volume
- **WHEN** `test-harness.sh` runs the Fedora UEFI dry-run block
- **THEN** the QEMU command log contains the FAT boot-root drive, the BOOTX64.EFI file exists, and startup.nsh references BOOTX64.EFI
