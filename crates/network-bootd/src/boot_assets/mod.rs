use std::path::{Path, PathBuf};

use boot_recipe::{BootMode, DistroId, get_recipe};

pub fn resolve_asset(cache_root: &Path, distro: DistroId, requested_path: &str) -> Option<PathBuf> {
    let normalized = requested_path.trim_start_matches('/');

    [BootMode::Bios, BootMode::Uefi]
        .into_iter()
        .filter_map(|mode| get_recipe(distro, mode).ok())
        .flat_map(|recipe| recipe.assets.into_iter())
        .find_map(|asset| {
            if asset.relative_path == normalized {
                Some(cache_root.join(normalized))
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use boot_recipe::DistroId;

    use super::resolve_asset;

    #[test]
    fn resolves_known_asset_path() {
        let resolved = resolve_asset(
            Path::new("/tmp/cache"),
            DistroId::Ubuntu,
            "ubuntu/bios/kernel",
        );
        assert_eq!(
            resolved,
            Some(Path::new("/tmp/cache/ubuntu/bios/kernel").to_path_buf())
        );
    }
}
