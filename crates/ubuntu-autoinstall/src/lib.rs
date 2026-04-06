use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UbuntuStorageLayout {
    Direct,
    Lvm,
}

impl UbuntuStorageLayout {
    fn as_autoinstall_name(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Lvm => "lvm",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedUbuntuAutoinstallConfig {
    pub hostname: String,
    pub username: String,
    pub password_hash: String,
    pub locale: String,
    pub keyboard_layout: String,
    pub timezone: String,
    pub storage_layout: UbuntuStorageLayout,
    pub install_open_ssh: bool,
    pub allow_password_auth: bool,
    pub authorized_keys: Vec<String>,
    pub packages: Vec<String>,
}

impl Default for PersistedUbuntuAutoinstallConfig {
    fn default() -> Self {
        Self {
            hostname: "boopa-ubuntu".to_string(),
            username: "ubuntu".to_string(),
            password_hash: "$1$DYoAfNpV$3G/NIgG/dz0XHgRX0/.MN.".to_string(),
            locale: "en_US.UTF-8".to_string(),
            keyboard_layout: "us".to_string(),
            timezone: "UTC".to_string(),
            storage_layout: UbuntuStorageLayout::Direct,
            install_open_ssh: true,
            allow_password_auth: true,
            authorized_keys: Vec::new(),
            packages: Vec::new(),
        }
    }
}

pub fn render_user_data(config: &PersistedUbuntuAutoinstallConfig) -> Result<String> {
    let payload = UbuntuAutoinstallCloudConfig {
        autoinstall: UbuntuAutoinstallDocument {
            version: 1,
            locale: config.locale.clone(),
            keyboard: UbuntuKeyboard {
                layout: config.keyboard_layout.clone(),
            },
            timezone: config.timezone.clone(),
            storage: UbuntuStorage {
                layout: UbuntuStorageLayoutDocument {
                    name: config.storage_layout.as_autoinstall_name().to_string(),
                },
            },
            identity: UbuntuIdentity {
                hostname: config.hostname.clone(),
                username: config.username.clone(),
                password: config.password_hash.clone(),
            },
            ssh: UbuntuSsh {
                install_server: config.install_open_ssh,
                allow_pw: config.allow_password_auth,
                authorized_keys: config.authorized_keys.clone(),
            },
            packages: config.packages.clone(),
        },
    };
    let yaml = serde_yaml::to_string(&payload)?;
    let rendered = format!("#cloud-config\n{yaml}");
    validate_rendered_user_data(&rendered)?;
    Ok(rendered)
}

pub fn render_meta_data(config: &PersistedUbuntuAutoinstallConfig) -> String {
    format!(
        "instance-id: boopa-ubuntu-uefi-autoinstall\nlocal-hostname: {}\n",
        config.hostname
    )
}

pub fn validate_rendered_user_data(rendered: &str) -> Result<()> {
    serde_yaml::from_str::<UbuntuAutoinstallCloudConfig>(rendered)
        .map(|_| ())
        .map_err(|error| anyhow!("generated autoinstall YAML is invalid: {error}"))
}

pub fn default_password_hash() -> String {
    PersistedUbuntuAutoinstallConfig::default().password_hash
}

pub fn mask_password_presence(password_hash: &str) -> bool {
    !password_hash.trim().is_empty()
}

pub fn fingerprint_password_hash(password_hash: &str) -> String {
    Sha256::digest(password_hash.as_bytes())
        .as_slice()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuAutoinstallCloudConfig {
    autoinstall: UbuntuAutoinstallDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuAutoinstallDocument {
    version: u8,
    locale: String,
    keyboard: UbuntuKeyboard,
    timezone: String,
    storage: UbuntuStorage,
    identity: UbuntuIdentity,
    ssh: UbuntuSsh,
    packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuKeyboard {
    layout: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuStorage {
    layout: UbuntuStorageLayoutDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuStorageLayoutDocument {
    name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct UbuntuIdentity {
    hostname: String,
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct UbuntuSsh {
    install_server: bool,
    allow_pw: bool,
    authorized_keys: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_renders_parseable_yaml() {
        let rendered =
            render_user_data(&PersistedUbuntuAutoinstallConfig::default()).expect("yaml");
        validate_rendered_user_data(&rendered).expect("valid yaml");
        assert!(rendered.contains("autoinstall:"));
        assert!(rendered.contains("identity:"));
    }

    #[test]
    fn invalid_rendered_yaml_is_rejected() {
        let result = validate_rendered_user_data("#cloud-config\nautoinstall: [");
        assert!(result.is_err());
    }

    #[test]
    fn password_hash_helpers_are_stable() {
        let hash = default_password_hash();
        assert!(mask_password_presence(&hash));
        assert_eq!(fingerprint_password_hash(&hash).len(), 64);
    }
}
