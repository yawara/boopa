use boot_recipe::{BootMode, DistroId, all_distros, get_recipe};

#[test]
fn exposes_all_supported_distros() {
    assert_eq!(
        all_distros(),
        vec![DistroId::Ubuntu, DistroId::Fedora, DistroId::Arch]
    );
}

#[test]
fn builds_non_empty_recipes_for_every_mode() {
    for distro in all_distros() {
        for mode in [BootMode::Bios, BootMode::Uefi] {
            let recipe = get_recipe(distro, mode).expect("recipe");
            assert!(!recipe.assets.is_empty());
            assert!(!recipe.dhcp.boot_filename.is_empty());
            assert_eq!(recipe.distro, distro);
            assert_eq!(recipe.boot_mode, mode);
        }
    }
}
