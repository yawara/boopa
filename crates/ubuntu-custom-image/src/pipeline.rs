use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, anyhow};
use ubuntu_autoinstall::{render_meta_data, render_user_data};

use crate::{
    BuildRequest,
    cache::{BuildMetadata, metadata_path_for_output, sha256_file},
    manifest::{
        AutoinstallSection, CustomImageManifest, FileInjection, ManagedConfig, TargetTree,
        normalized_target_relative_path, parse_mode, resolve_source_path,
    },
    source::UbuntuIsoSource,
};

const CASPER_SQUASHFS_PATH: &str = "casper/filesystem.squashfs";

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
}

pub trait CommandRunner {
    fn run(&mut self, spec: &CommandSpec) -> Result<()>;
}

#[derive(Debug, Default)]
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&mut self, spec: &CommandSpec) -> Result<()> {
        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        for (key, value) in &spec.env {
            command.env(key, value);
        }

        let output = command.output().with_context(|| {
            format!(
                "failed to spawn command: {} {}",
                spec.program,
                spec.args.join(" ")
            )
        })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(anyhow!(
                "command failed: {} {}\nstdout: {}\nstderr: {}",
                spec.program,
                spec.args.join(" "),
                String::from_utf8_lossy(&output.stdout).trim(),
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildOutcome {
    pub output_path: PathBuf,
    pub metadata_path: PathBuf,
}

#[derive(Debug)]
struct PipelinePaths {
    work_root: PathBuf,
    iso_root: PathBuf,
    rootfs_root: PathBuf,
}

pub fn execute_build(
    request: &BuildRequest,
    manifest: &CustomImageManifest,
    source: &UbuntuIsoSource,
    runner: &mut dyn CommandRunner,
) -> Result<BuildOutcome> {
    let temp_dir = if request.work_dir.is_none() {
        Some(tempfile::tempdir().context("failed to allocate temporary work dir")?)
    } else {
        None
    };
    let resolved_work_root = request
        .work_dir
        .clone()
        .unwrap_or_else(|| temp_dir.as_ref().expect("tempdir").path().to_path_buf());
    let paths = PipelinePaths {
        work_root: resolved_work_root.clone(),
        iso_root: resolved_work_root.join("iso-root"),
        rootfs_root: resolved_work_root.join("rootfs"),
    };

    prepare_workspace(&paths, &request.output_path)?;
    extract_iso(&request.base_iso_path, &paths.iso_root, runner)?;
    extract_rootfs(&paths.iso_root, &paths.rootfs_root, runner)?;
    install_packages(&paths.rootfs_root, &manifest.packages, runner)?;
    apply_manifest_layers(&request.manifest_path, manifest, &paths)?;
    rebuild_rootfs(&paths.rootfs_root, &paths.iso_root, runner)?;
    update_filesystem_size(&paths.rootfs_root, &paths.iso_root)?;
    update_md5sums(&paths.iso_root, runner)?;
    rebuild_iso(
        &request.base_iso_path,
        &paths.iso_root,
        &request.output_path,
        runner,
    )?;

    let observed_output_sha256 = sha256_file(&request.output_path).ok();
    let metadata = BuildMetadata::from_inputs(request, manifest, source, observed_output_sha256)?;
    let metadata_path = metadata_path_for_output(&request.output_path);
    metadata.write_to_path(&metadata_path)?;

    drop(temp_dir);

    Ok(BuildOutcome {
        output_path: request.output_path.clone(),
        metadata_path,
    })
}

fn prepare_workspace(paths: &PipelinePaths, output_path: &Path) -> Result<()> {
    fs::create_dir_all(&paths.work_root)
        .with_context(|| format!("failed to create {}", paths.work_root.display()))?;
    fs::create_dir_all(&paths.iso_root)
        .with_context(|| format!("failed to create {}", paths.iso_root.display()))?;
    fs::create_dir_all(&paths.rootfs_root)
        .with_context(|| format!("failed to create {}", paths.rootfs_root.display()))?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::create_dir_all(paths.iso_root.join("casper")).with_context(|| {
        format!(
            "failed to create {}",
            paths.iso_root.join("casper").display()
        )
    })?;
    Ok(())
}

fn extract_iso(
    base_iso_path: &Path,
    iso_root: &Path,
    runner: &mut dyn CommandRunner,
) -> Result<()> {
    runner.run(&CommandSpec {
        program: "xorriso".to_string(),
        args: vec![
            "-osirrox".to_string(),
            "on".to_string(),
            "-indev".to_string(),
            base_iso_path.display().to_string(),
            "-extract".to_string(),
            "/".to_string(),
            iso_root.display().to_string(),
        ],
        env: Vec::new(),
        cwd: None,
    })
}

fn extract_rootfs(
    iso_root: &Path,
    rootfs_root: &Path,
    runner: &mut dyn CommandRunner,
) -> Result<()> {
    let squashfs = iso_root.join(CASPER_SQUASHFS_PATH);
    runner.run(&CommandSpec {
        program: "unsquashfs".to_string(),
        args: vec![
            "-d".to_string(),
            rootfs_root.display().to_string(),
            squashfs.display().to_string(),
        ],
        env: Vec::new(),
        cwd: None,
    })
}

fn install_packages(
    rootfs_root: &Path,
    packages: &[String],
    runner: &mut dyn CommandRunner,
) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let etc_dir = rootfs_root.join("etc");
    fs::create_dir_all(&etc_dir)
        .with_context(|| format!("failed to create {}", etc_dir.display()))?;
    copy_if_present(Path::new("/etc/resolv.conf"), &etc_dir.join("resolv.conf"))?;

    let mounts = [
        (
            "--bind".to_string(),
            "/dev".to_string(),
            rootfs_root.join("dev"),
        ),
        (
            "-t".to_string(),
            "proc".to_string(),
            rootfs_root.join("proc"),
        ),
        (
            "-t".to_string(),
            "sysfs".to_string(),
            rootfs_root.join("sys"),
        ),
    ];

    for (_, _, target) in &mounts {
        fs::create_dir_all(target)
            .with_context(|| format!("failed to create {}", target.display()))?;
    }

    let mount_result = (|| -> Result<()> {
        runner.run(&CommandSpec {
            program: "mount".to_string(),
            args: vec![
                "--bind".to_string(),
                "/dev".to_string(),
                rootfs_root.join("dev").display().to_string(),
            ],
            env: Vec::new(),
            cwd: None,
        })?;
        runner.run(&CommandSpec {
            program: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "proc".to_string(),
                "proc".to_string(),
                rootfs_root.join("proc").display().to_string(),
            ],
            env: Vec::new(),
            cwd: None,
        })?;
        runner.run(&CommandSpec {
            program: "mount".to_string(),
            args: vec![
                "-t".to_string(),
                "sysfs".to_string(),
                "sys".to_string(),
                rootfs_root.join("sys").display().to_string(),
            ],
            env: Vec::new(),
            cwd: None,
        })?;
        runner.run(&CommandSpec {
            program: "chroot".to_string(),
            args: vec![
                rootfs_root.display().to_string(),
                "apt-get".to_string(),
                "update".to_string(),
            ],
            env: vec![("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string())],
            cwd: None,
        })?;

        let mut install_args = vec![
            rootfs_root.display().to_string(),
            "apt-get".to_string(),
            "install".to_string(),
            "-y".to_string(),
        ];
        install_args.extend(packages.iter().cloned());
        runner.run(&CommandSpec {
            program: "chroot".to_string(),
            args: install_args,
            env: vec![("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string())],
            cwd: None,
        })
    })();

    let cleanup_result = cleanup_mounts(rootfs_root, runner);
    mount_result?;
    cleanup_result?;
    Ok(())
}

fn cleanup_mounts(rootfs_root: &Path, runner: &mut dyn CommandRunner) -> Result<()> {
    for target in ["sys", "proc", "dev"] {
        let path = rootfs_root.join(target);
        match runner.run(&CommandSpec {
            program: "umount".to_string(),
            args: vec![path.display().to_string()],
            env: Vec::new(),
            cwd: None,
        }) {
            Ok(()) => {}
            Err(error) if target == "sys" || target == "proc" || target == "dev" => {
                return Err(error);
            }
            Err(_) => {}
        }
    }
    Ok(())
}

fn apply_manifest_layers(
    manifest_path: &Path,
    manifest: &CustomImageManifest,
    paths: &PipelinePaths,
) -> Result<()> {
    for entry in &manifest.files {
        apply_file_entry(manifest_path, entry, paths)?;
    }
    for entry in &manifest.config {
        apply_config_entry(entry, paths)?;
    }
    if let Some(autoinstall) = &manifest.autoinstall {
        apply_autoinstall_section(autoinstall, paths)?;
    }
    Ok(())
}

fn apply_file_entry(
    manifest_path: &Path,
    entry: &FileInjection,
    paths: &PipelinePaths,
) -> Result<()> {
    let source = resolve_source_path(manifest_path, &entry.source);
    let destination = resolve_target_path(paths, entry.tree, &entry.target)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::copy(&source, &destination).with_context(|| {
        format!(
            "failed to copy file injection from {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    if let Some(mode) = parse_mode(entry.mode.as_deref())? {
        let permissions = fs::Permissions::from_mode(mode);
        fs::set_permissions(&destination, permissions)
            .with_context(|| format!("failed to chmod {}", destination.display()))?;
    }
    Ok(())
}

fn apply_config_entry(entry: &ManagedConfig, paths: &PipelinePaths) -> Result<()> {
    let destination = resolve_target_path(paths, entry.tree, &entry.target)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&destination, &entry.content)
        .with_context(|| format!("failed to write config {}", destination.display()))?;
    if let Some(mode) = parse_mode(entry.mode.as_deref())? {
        let permissions = fs::Permissions::from_mode(mode);
        fs::set_permissions(&destination, permissions)
            .with_context(|| format!("failed to chmod {}", destination.display()))?;
    }
    Ok(())
}

fn apply_autoinstall_section(
    autoinstall: &AutoinstallSection,
    paths: &PipelinePaths,
) -> Result<()> {
    let seed_root = resolve_target_path(paths, TargetTree::Iso, &autoinstall.seed_dir)?;
    fs::create_dir_all(&seed_root)
        .with_context(|| format!("failed to create {}", seed_root.display()))?;
    fs::write(
        seed_root.join("user-data"),
        render_user_data(&autoinstall.config)?,
    )
    .with_context(|| format!("failed to write {}", seed_root.join("user-data").display()))?;
    fs::write(
        seed_root.join("meta-data"),
        render_meta_data(&autoinstall.config),
    )
    .with_context(|| format!("failed to write {}", seed_root.join("meta-data").display()))?;
    patch_grub_cfgs(&paths.iso_root, &autoinstall.seed_dir)?;
    Ok(())
}

fn patch_grub_cfgs(iso_root: &Path, seed_dir: &str) -> Result<()> {
    for path in find_grub_cfgs(iso_root)? {
        let original = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut changed = false;
        let rendered = original
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                if (trimmed.starts_with("linux ") || trimmed.starts_with("linuxefi "))
                    && !line.contains("autoinstall")
                {
                    changed = true;
                    inject_autoinstall_args(line, seed_dir)
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        if changed {
            fs::write(&path, format!("{rendered}\n"))
                .with_context(|| format!("failed to patch {}", path.display()))?;
        }
    }
    Ok(())
}

fn inject_autoinstall_args(line: &str, seed_dir: &str) -> String {
    let seed = format!("ds=nocloud;s=/cdrom{}/", seed_dir.trim_end_matches('/'));
    if let Some((prefix, suffix)) = line.split_once(" ---") {
        format!("{prefix} autoinstall '{seed}' ---{suffix}")
    } else {
        format!("{line} autoinstall '{seed}'")
    }
}

fn rebuild_rootfs(
    rootfs_root: &Path,
    iso_root: &Path,
    runner: &mut dyn CommandRunner,
) -> Result<()> {
    let squashfs = iso_root.join(CASPER_SQUASHFS_PATH);
    if squashfs.exists() {
        fs::remove_file(&squashfs)
            .with_context(|| format!("failed to remove {}", squashfs.display()))?;
    }
    runner.run(&CommandSpec {
        program: "mksquashfs".to_string(),
        args: vec![
            rootfs_root.display().to_string(),
            squashfs.display().to_string(),
            "-noappend".to_string(),
            "-comp".to_string(),
            "xz".to_string(),
        ],
        env: Vec::new(),
        cwd: None,
    })
}

fn update_filesystem_size(rootfs_root: &Path, iso_root: &Path) -> Result<()> {
    let size = directory_size(rootfs_root)?;
    let size_path = iso_root.join("casper/filesystem.size");
    fs::write(&size_path, format!("{size}\n"))
        .with_context(|| format!("failed to write {}", size_path.display()))
}

fn update_md5sums(iso_root: &Path, runner: &mut dyn CommandRunner) -> Result<()> {
    runner.run(&CommandSpec {
        program: "bash".to_string(),
        args: vec![
            "-lc".to_string(),
            "find . -type f ! -name md5sum.txt -print0 | sort -z | xargs -0 md5sum > md5sum.txt"
                .to_string(),
        ],
        env: Vec::new(),
        cwd: Some(iso_root.to_path_buf()),
    })
}

fn rebuild_iso(
    base_iso_path: &Path,
    iso_root: &Path,
    output_path: &Path,
    runner: &mut dyn CommandRunner,
) -> Result<()> {
    runner.run(&CommandSpec {
        program: "xorriso".to_string(),
        args: vec![
            "-indev".to_string(),
            base_iso_path.display().to_string(),
            "-outdev".to_string(),
            output_path.display().to_string(),
            "-map".to_string(),
            iso_root.display().to_string(),
            "/".to_string(),
            "-boot_image".to_string(),
            "any".to_string(),
            "replay".to_string(),
        ],
        env: Vec::new(),
        cwd: None,
    })
}

fn resolve_target_path(paths: &PipelinePaths, tree: TargetTree, target: &str) -> Result<PathBuf> {
    let stripped = normalized_target_relative_path(target)?;
    match tree {
        TargetTree::Rootfs => Ok(paths.rootfs_root.join(stripped)),
        TargetTree::Iso => Ok(paths.iso_root.join(stripped)),
    }
}

fn copy_if_present(source: &Path, destination: &Path) -> Result<()> {
    match fs::copy(source, destination) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| {
            format!(
                "failed to copy {} to {}",
                source.display(),
                destination.display()
            )
        }),
    }
}

fn directory_size(root: &Path) -> Result<u64> {
    let mut total = 0_u64;
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total += directory_size(&path)?;
        } else if metadata.is_file() {
            total += metadata.len();
        }
    }
    Ok(total)
}

fn find_grub_cfgs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_grub_cfgs(root, &mut paths)?;
    Ok(paths)
}

fn collect_grub_cfgs(root: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry?;
        let path = entry.path();
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_grub_cfgs(&path, paths)?;
        } else if metadata.is_file() && path.file_name().is_some_and(|name| name == "grub.cfg") {
            paths.push(path);
        }
    }
    Ok(())
}
