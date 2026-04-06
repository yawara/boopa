use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use ubuntu_autoinstall::PersistedUbuntuAutoinstallConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TargetTree {
    #[default]
    Rootfs,
    Iso,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileInjection {
    pub source: PathBuf,
    pub target: String,
    #[serde(default)]
    pub tree: TargetTree,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedConfig {
    pub target: String,
    pub content: String,
    #[serde(default)]
    pub tree: TargetTree,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoinstallSection {
    #[serde(default = "default_seed_dir")]
    pub seed_dir: String,
    #[serde(flatten)]
    pub config: PersistedUbuntuAutoinstallConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomImageManifest {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default)]
    pub files: Vec<FileInjection>,
    #[serde(default)]
    pub config: Vec<ManagedConfig>,
    #[serde(default)]
    pub autoinstall: Option<AutoinstallSection>,
}

impl CustomImageManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = fs::read(path)
            .with_context(|| format!("failed to read manifest at {}", path.display()))?;
        let mut manifest: Self = serde_yaml::from_slice(&bytes)
            .with_context(|| format!("failed to parse YAML manifest at {}", path.display()))?;
        manifest.normalize()?;
        Ok(manifest)
    }

    pub fn normalize(&mut self) -> Result<()> {
        self.packages = normalize_string_list(std::mem::take(&mut self.packages));
        for entry in &self.files {
            normalized_target_relative_path(&entry.target)?;
        }
        for entry in &self.config {
            normalized_target_relative_path(&entry.target)?;
        }
        if let Some(autoinstall) = &self.autoinstall {
            normalized_target_relative_path(&autoinstall.seed_dir)?;
        }
        Ok(())
    }
}

pub fn parse_mode(mode: Option<&str>) -> Result<Option<u32>> {
    match mode {
        Some(mode) => {
            let trimmed = mode.trim_start_matches('0');
            let value = u32::from_str_radix(if trimmed.is_empty() { "0" } else { trimmed }, 8)
                .map_err(|error| anyhow!("invalid file mode {mode}: {error}"))?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

pub fn resolve_source_path(manifest_path: &Path, source: &Path) -> PathBuf {
    if source.is_absolute() {
        source.to_path_buf()
    } else {
        manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(source)
    }
}

pub fn normalized_target_relative_path(target: &str) -> Result<PathBuf> {
    if !target.starts_with('/') {
        return Err(anyhow!("manifest target paths must be absolute: {target}"));
    }

    let mut normalized = PathBuf::new();
    for component in Path::new(target).components() {
        match component {
            Component::RootDir | Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir | Component::Prefix(_) => {
                return Err(anyhow!(
                    "manifest target paths must stay within the staged rootfs/ISO tree: {target}"
                ));
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err(anyhow!(
            "manifest target path must not resolve to the root of the staged tree: {target}"
        ));
    }

    Ok(normalized)
}

fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || normalized.iter().any(|existing| existing == trimmed) {
            continue;
        }
        normalized.push(trimmed.to_string());
    }
    normalized
}

fn default_seed_dir() -> String {
    "/autoinstall".to_string()
}
