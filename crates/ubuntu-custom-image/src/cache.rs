use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    BuildRequest,
    manifest::{CustomImageManifest, resolve_source_path},
    source::UbuntuIsoSource,
};

fn tree_label(tree: crate::manifest::TargetTree) -> &'static str {
    match tree {
        crate::manifest::TargetTree::Rootfs => "rootfs",
        crate::manifest::TargetTree::Iso => "iso",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub base_iso_path: String,
    pub source_iso_sha256: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    pub normalized_build_record_sha256: String,
    pub output_path: String,
    pub observed_output_sha256: Option<String>,
    pub ubuntu_source_url: String,
}

impl BuildMetadata {
    pub fn from_inputs(
        request: &BuildRequest,
        manifest: &CustomImageManifest,
        source: &UbuntuIsoSource,
        observed_output_sha256: Option<String>,
    ) -> Result<Self> {
        let source_iso_sha256 = sha256_file(&request.base_iso_path)?;
        let manifest_sha256 = sha256_file(&request.manifest_path)?;
        let normalized_build_record_sha256 =
            normalized_build_record_sha256(&request.manifest_path, manifest)?;
        Ok(Self {
            base_iso_path: request.base_iso_path.display().to_string(),
            source_iso_sha256,
            manifest_path: request.manifest_path.display().to_string(),
            manifest_sha256,
            normalized_build_record_sha256,
            output_path: request.output_path.display().to_string(),
            observed_output_sha256,
            ubuntu_source_url: source.source_url.clone(),
        })
    }

    pub fn write_to_path(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let payload = serde_json::to_vec_pretty(self)?;
        fs::write(path, payload).with_context(|| format!("failed to write {}", path.display()))
    }
}

pub fn metadata_path_for_output(output_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.build.json", output_path.display()))
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .as_slice()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn normalized_build_record_sha256(
    manifest_path: &Path,
    manifest: &CustomImageManifest,
) -> Result<String> {
    #[derive(Serialize)]
    struct NormalizedFileRecord<'a> {
        source: String,
        source_sha256: String,
        target: &'a str,
        tree: &'a str,
        mode: Option<&'a str>,
    }

    #[derive(Serialize)]
    struct NormalizedConfigRecord<'a> {
        target: &'a str,
        tree: &'a str,
        mode: Option<&'a str>,
        content_sha256: String,
    }

    #[derive(Serialize)]
    struct NormalizedAutoinstallRecord<'a> {
        seed_dir: &'a str,
        config_sha256: String,
    }

    #[derive(Serialize)]
    struct NormalizedRecord<'a> {
        packages: &'a [String],
        files: Vec<NormalizedFileRecord<'a>>,
        config: Vec<NormalizedConfigRecord<'a>>,
        autoinstall: Option<NormalizedAutoinstallRecord<'a>>,
    }

    let payload = NormalizedRecord {
        packages: &manifest.packages,
        files: manifest
            .files
            .iter()
            .map(|entry| {
                let source_path = resolve_source_path(manifest_path, &entry.source);
                Ok(NormalizedFileRecord {
                    source: entry.source.display().to_string(),
                    source_sha256: sha256_file(&source_path)?,
                    target: entry.target.as_str(),
                    tree: tree_label(entry.tree),
                    mode: entry.mode.as_deref(),
                })
            })
            .collect::<Result<Vec<_>>>()?,
        config: manifest
            .config
            .iter()
            .map(|entry| NormalizedConfigRecord {
                target: entry.target.as_str(),
                tree: tree_label(entry.tree),
                mode: entry.mode.as_deref(),
                content_sha256: sha256_bytes(entry.content.as_bytes()),
            })
            .collect(),
        autoinstall: manifest.autoinstall.as_ref().map(|entry| {
            let bytes = serde_json::to_vec(&entry.config).expect("autoinstall config serializes");
            NormalizedAutoinstallRecord {
                seed_dir: entry.seed_dir.as_str(),
                config_sha256: sha256_bytes(&bytes),
            }
        }),
    };
    let bytes = serde_json::to_vec(&payload)?;
    Ok(sha256_bytes(&bytes))
}
