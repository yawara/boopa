use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tempfile::tempdir;
use ubuntu_custom_image::{
    BuildRequest, build_metadata,
    manifest::CustomImageManifest,
    pipeline::{CommandRunner, CommandSpec, execute_build},
    source::canonical_ubuntu_uefi_iso_source,
};

#[derive(Default)]
struct FakeRunner {
    commands: Vec<CommandSpec>,
}

impl CommandRunner for FakeRunner {
    fn run(&mut self, spec: &CommandSpec) -> Result<()> {
        if spec.program == "xorriso"
            && let Some(index) = spec.args.iter().position(|arg| arg == "-outdev")
            && let Some(path) = spec.args.get(index + 1)
        {
            fs::write(path, b"fake-custom-iso")?;
        }
        if spec.program == "mksquashfs" && spec.args.len() >= 2 {
            fs::write(&spec.args[1], b"fake-squashfs")?;
        }
        self.commands.push(spec.clone());
        Ok(())
    }
}

#[test]
fn build_pipeline_records_artifact_and_applies_manifest_layers() {
    let temp_dir = tempdir().expect("temp dir");
    let base_iso = temp_dir.path().join("ubuntu-base.iso");
    fs::write(&base_iso, b"base-iso").expect("base iso");

    let manifest_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/ubuntu-custom-image.yaml");
    let manifest = CustomImageManifest::load(&manifest_path).expect("manifest");
    let output_path = temp_dir.path().join("custom.iso");
    let work_dir = temp_dir.path().join("work");

    fs::create_dir_all(work_dir.join("iso-root/boot/grub")).expect("iso dir");
    fs::write(
        work_dir.join("iso-root/boot/grub/grub.cfg"),
        "menuentry \"Ubuntu\" {\n linux /casper/vmlinuz ---\n}\n",
    )
    .expect("grub cfg");

    let request = BuildRequest {
        base_iso_path: base_iso,
        manifest_path: manifest_path.clone(),
        output_path: output_path.clone(),
        work_dir: Some(work_dir.clone()),
    };

    let mut runner = FakeRunner::default();
    let source = canonical_ubuntu_uefi_iso_source().expect("source");
    let outcome = execute_build(&request, &manifest, &source, &mut runner).expect("build");

    assert!(outcome.output_path.exists());
    assert!(outcome.metadata_path.exists());
    assert_file_contains(
        &work_dir.join("rootfs/usr/local/share/boopa/motd.txt"),
        "boopa custom image\n",
    );
    assert_file_contains(
        &work_dir.join("rootfs/etc/boopa/custom-image.conf"),
        "mirror=http://archive.ubuntu.com/ubuntu\n",
    );
    assert!(work_dir.join("iso-root/autoinstall/user-data").exists());
    assert!(work_dir.join("iso-root/autoinstall/meta-data").exists());
    assert_file_contains(
        &work_dir.join("iso-root/boot/grub/grub.cfg"),
        "autoinstall 'ds=nocloud;s=/cdrom/autoinstall/'",
    );
    assert!(
        runner
            .commands
            .iter()
            .any(|spec| spec.program == "chroot" && spec.args.iter().any(|arg| arg == "apt-get"))
    );
}

#[test]
fn build_metadata_hash_changes_when_injected_file_bytes_change() {
    let temp_dir = tempdir().expect("temp dir");
    let base_iso = temp_dir.path().join("ubuntu-base.iso");
    fs::write(&base_iso, b"base-iso").expect("base iso");

    let manifest_dir = temp_dir.path().join("manifest");
    fs::create_dir_all(manifest_dir.join("files")).expect("manifest dir");
    let source_file = manifest_dir.join("files/motd.txt");
    fs::write(&source_file, b"first").expect("source");

    let manifest_path = manifest_dir.join("custom.yaml");
    fs::write(
        &manifest_path,
        r#"
packages: []
files:
  - source: files/motd.txt
    target: /usr/local/share/boopa/motd.txt
    tree: rootfs
config: []
"#,
    )
    .expect("manifest");

    let request = BuildRequest {
        base_iso_path: base_iso.clone(),
        manifest_path: manifest_path.clone(),
        output_path: temp_dir.path().join("custom.iso"),
        work_dir: Some(temp_dir.path().join("work")),
    };

    let before = build_metadata(&request).expect("metadata");
    fs::write(&source_file, b"second").expect("updated source");
    let after = build_metadata(&request).expect("metadata");

    assert_ne!(
        before.normalized_build_record_sha256,
        after.normalized_build_record_sha256
    );
}

fn assert_file_contains(path: &Path, needle: &str) {
    let content = fs::read_to_string(path).expect("content");
    assert!(
        content.contains(needle),
        "expected {needle:?} in {}",
        path.display()
    );
}
