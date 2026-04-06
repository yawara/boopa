use std::{env, path::PathBuf, process::Command};

use anyhow::{Result, anyhow};

const REQUIRED_TOOLS: &[&str] = &[
    "apt-get",
    "bash",
    "chroot",
    "id",
    "md5sum",
    "mksquashfs",
    "mount",
    "umount",
    "unsquashfs",
    "xorriso",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPreflightReport {
    pub tools: Vec<PathBuf>,
}

pub fn preflight() -> Result<ToolPreflightReport> {
    ensure_linux_host()?;
    ensure_root()?;

    let mut tools = Vec::new();
    for tool in REQUIRED_TOOLS {
        tools.push(find_command(tool)?);
    }

    Ok(ToolPreflightReport { tools })
}

fn ensure_linux_host() -> Result<()> {
    if cfg!(target_os = "linux") {
        Ok(())
    } else {
        Err(anyhow!(
            "ubuntu-custom-image v1 only supports Linux hosts because it relies on chroot/mount tooling"
        ))
    }
}

fn ensure_root() -> Result<()> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .map_err(|error| anyhow!("failed to determine effective uid via id -u: {error}"))?;
    if !output.status.success() {
        return Err(anyhow!(
            "failed to determine effective uid: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if uid == "0" {
        Ok(())
    } else {
        Err(anyhow!(
            "ubuntu-custom-image v1 requires root because it mounts and chroots into the extracted Ubuntu rootfs"
        ))
    }
}

fn find_command(name: &str) -> Result<PathBuf> {
    let path = env::var_os("PATH").ok_or_else(|| anyhow!("PATH is not set"))?;
    for entry in env::split_paths(&path) {
        let candidate = entry.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(anyhow!("required tool not found in PATH: {name}"))
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_command() {
        let result = find_command("this-command-should-not-exist-12345");
        assert!(result.is_err());
    }
}
