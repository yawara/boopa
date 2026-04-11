use anyhow::{Result, anyhow};
use boot_recipe::{BootMode, DistroId, get_recipe};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UbuntuIsoSource {
    pub relative_path: String,
    pub source_url: String,
}

pub fn canonical_ubuntu_uefi_iso_source() -> Result<UbuntuIsoSource> {
    let recipe = get_recipe(DistroId::Ubuntu, BootMode::Uefi)?;
    let asset = recipe
        .assets
        .into_iter()
        .find(|asset| asset.logical_name == "iso")
        .ok_or_else(|| anyhow!("ubuntu uefi recipe is missing its ISO asset"))?;

    Ok(UbuntuIsoSource {
        relative_path: asset.relative_path,
        source_url: asset.source_url,
    })
}
