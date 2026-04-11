pub mod cache;
pub mod manifest;
pub mod pipeline;
pub mod source;
pub mod tools;

use std::path::PathBuf;

use anyhow::Result;
use cache::BuildMetadata;
use manifest::CustomImageManifest;
use pipeline::{BuildOutcome, SystemCommandRunner, execute_build};
use source::canonical_ubuntu_uefi_iso_source;
use tools::preflight;

#[derive(Debug, Clone)]
pub struct BuildRequest {
    pub base_iso_path: PathBuf,
    pub manifest_path: PathBuf,
    pub output_path: PathBuf,
    pub work_dir: Option<PathBuf>,
}

pub fn build(request: &BuildRequest) -> Result<BuildOutcome> {
    preflight()?;
    let manifest = CustomImageManifest::load(&request.manifest_path)?;
    let source = canonical_ubuntu_uefi_iso_source()?;
    let mut runner = SystemCommandRunner;
    execute_build(request, &manifest, &source, &mut runner)
}

pub fn build_metadata(request: &BuildRequest) -> Result<BuildMetadata> {
    let manifest = CustomImageManifest::load(&request.manifest_path)?;
    let source = canonical_ubuntu_uefi_iso_source()?;
    BuildMetadata::from_inputs(request, &manifest, &source, None)
}
