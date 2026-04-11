use ubuntu_custom_image::source::canonical_ubuntu_uefi_iso_source;

#[test]
fn resolves_canonical_ubuntu_uefi_iso_source() {
    let source = canonical_ubuntu_uefi_iso_source().expect("source");
    assert_eq!(source.relative_path, "ubuntu/uefi/live-server.iso");
    assert!(
        source
            .source_url
            .contains("ubuntu-24.04.4-live-server-amd64.iso")
    );
}
