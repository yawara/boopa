use std::path::PathBuf;

use ubuntu_custom_image::manifest::{
    CustomImageManifest, normalized_target_relative_path, parse_mode,
};

#[test]
fn example_manifest_parses() {
    let manifest_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/ubuntu-custom-image.yaml");
    let manifest = CustomImageManifest::load(&manifest_path).expect("manifest");

    assert!(manifest.packages.contains(&"openssh-server".to_string()));
    assert!(!manifest.files.is_empty());
    assert!(!manifest.config.is_empty());
    assert!(manifest.autoinstall.is_some());
}

#[test]
fn mode_parser_accepts_octal_strings() {
    assert_eq!(parse_mode(Some("0644")).expect("mode"), Some(0o644));
}

#[test]
fn rejects_parent_directory_target_escapes() {
    let error = normalized_target_relative_path("/../../etc/passwd").expect_err("invalid target");
    assert!(error.to_string().contains("staged rootfs/ISO tree"));
}
