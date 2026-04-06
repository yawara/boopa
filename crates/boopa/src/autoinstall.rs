use std::{collections::BTreeMap, process::Stdio};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, process::Command};

use ubuntu_autoinstall as shared_autoinstall;
pub use ubuntu_autoinstall::{
    UbuntuStorageLayout, default_password_hash, fingerprint_password_hash, mask_password_presence,
};

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
        let shared = shared_autoinstall::PersistedUbuntuAutoinstallConfig::default();
        Self::from(shared)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UbuntuAutoinstallConfig {
    pub hostname: String,
    pub username: String,
    pub locale: String,
    pub keyboard_layout: String,
    pub timezone: String,
    pub storage_layout: UbuntuStorageLayout,
    pub install_open_ssh: bool,
    pub allow_password_auth: bool,
    pub authorized_keys: Vec<String>,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UbuntuAutoinstallConfigUpdate {
    pub hostname: String,
    pub username: String,
    pub password: Option<String>,
    pub locale: String,
    pub keyboard_layout: String,
    pub timezone: String,
    pub storage_layout: UbuntuStorageLayout,
    pub install_open_ssh: bool,
    pub allow_password_auth: bool,
    pub authorized_keys: Vec<String>,
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UbuntuAutoinstallConfigResponse {
    pub config: UbuntuAutoinstallConfig,
    pub rendered_yaml: String,
    pub has_password: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationErrorResponse {
    pub message: String,
    pub field_errors: BTreeMap<String, String>,
}

impl ValidationErrorResponse {
    fn with_message(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            field_errors: BTreeMap::new(),
        }
    }
}

#[derive(Debug)]
pub enum UpdateError {
    Validation(ValidationErrorResponse),
    Internal(anyhow::Error),
}

impl From<anyhow::Error> for UpdateError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error)
    }
}

impl PersistedUbuntuAutoinstallConfig {
    pub fn to_response(self) -> Result<UbuntuAutoinstallConfigResponse> {
        let rendered_yaml = render_user_data(&self)?;
        Ok(UbuntuAutoinstallConfigResponse {
            config: self.to_public_config(),
            rendered_yaml,
            has_password: mask_password_presence(&self.password_hash),
        })
    }

    pub fn to_public_config(&self) -> UbuntuAutoinstallConfig {
        UbuntuAutoinstallConfig {
            hostname: self.hostname.clone(),
            username: self.username.clone(),
            locale: self.locale.clone(),
            keyboard_layout: self.keyboard_layout.clone(),
            timezone: self.timezone.clone(),
            storage_layout: self.storage_layout,
            install_open_ssh: self.install_open_ssh,
            allow_password_auth: self.allow_password_auth,
            authorized_keys: self.authorized_keys.clone(),
            packages: self.packages.clone(),
        }
    }
}

impl From<shared_autoinstall::PersistedUbuntuAutoinstallConfig>
    for PersistedUbuntuAutoinstallConfig
{
    fn from(value: shared_autoinstall::PersistedUbuntuAutoinstallConfig) -> Self {
        Self {
            hostname: value.hostname,
            username: value.username,
            password_hash: value.password_hash,
            locale: value.locale,
            keyboard_layout: value.keyboard_layout,
            timezone: value.timezone,
            storage_layout: value.storage_layout,
            install_open_ssh: value.install_open_ssh,
            allow_password_auth: value.allow_password_auth,
            authorized_keys: value.authorized_keys,
            packages: value.packages,
        }
    }
}

impl From<&PersistedUbuntuAutoinstallConfig>
    for shared_autoinstall::PersistedUbuntuAutoinstallConfig
{
    fn from(value: &PersistedUbuntuAutoinstallConfig) -> Self {
        Self {
            hostname: value.hostname.clone(),
            username: value.username.clone(),
            password_hash: value.password_hash.clone(),
            locale: value.locale.clone(),
            keyboard_layout: value.keyboard_layout.clone(),
            timezone: value.timezone.clone(),
            storage_layout: value.storage_layout,
            install_open_ssh: value.install_open_ssh,
            allow_password_auth: value.allow_password_auth,
            authorized_keys: value.authorized_keys.clone(),
            packages: value.packages.clone(),
        }
    }
}

pub fn render_user_data(config: &PersistedUbuntuAutoinstallConfig) -> Result<String> {
    shared_autoinstall::render_user_data(
        &shared_autoinstall::PersistedUbuntuAutoinstallConfig::from(config),
    )
}

pub fn render_meta_data(config: &PersistedUbuntuAutoinstallConfig) -> String {
    shared_autoinstall::render_meta_data(
        &shared_autoinstall::PersistedUbuntuAutoinstallConfig::from(config),
    )
}

pub fn validate_rendered_user_data(rendered: &str) -> Result<()> {
    shared_autoinstall::validate_rendered_user_data(rendered)
}

pub async fn apply_update(
    existing: &PersistedUbuntuAutoinstallConfig,
    update: UbuntuAutoinstallConfigUpdate,
) -> std::result::Result<PersistedUbuntuAutoinstallConfig, UpdateError> {
    let normalized = normalize_update(update);
    let field_errors = validate_business_rules(existing, &normalized);
    if !field_errors.is_empty() {
        return Err(UpdateError::Validation(ValidationErrorResponse {
            message: "Validation failed".to_string(),
            field_errors,
        }));
    }

    let password_hash = match normalized.password.as_deref() {
        Some(password) if !password.is_empty() => hash_password(password).await?,
        _ => existing.password_hash.clone(),
    };

    let next = PersistedUbuntuAutoinstallConfig {
        hostname: normalized.hostname,
        username: normalized.username,
        password_hash,
        locale: normalized.locale,
        keyboard_layout: normalized.keyboard_layout,
        timezone: normalized.timezone,
        storage_layout: normalized.storage_layout,
        install_open_ssh: normalized.install_open_ssh,
        allow_password_auth: normalized.allow_password_auth,
        authorized_keys: normalized.authorized_keys,
        packages: normalized.packages,
    };

    render_user_data(&next).map_err(|error| {
        UpdateError::Validation(ValidationErrorResponse::with_message(error.to_string()))
    })?;

    Ok(next)
}

fn normalize_update(update: UbuntuAutoinstallConfigUpdate) -> NormalizedAutoinstallUpdate {
    NormalizedAutoinstallUpdate {
        hostname: update.hostname.trim().to_string(),
        username: update.username.trim().to_string(),
        password: update.password.map(|password| password.trim().to_string()),
        locale: update.locale.trim().to_string(),
        keyboard_layout: update.keyboard_layout.trim().to_string(),
        timezone: update.timezone.trim().to_string(),
        storage_layout: update.storage_layout,
        install_open_ssh: update.install_open_ssh,
        allow_password_auth: update.allow_password_auth,
        authorized_keys: normalize_list(update.authorized_keys),
        packages: normalize_list(update.packages),
    }
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
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

fn validate_business_rules(
    existing: &PersistedUbuntuAutoinstallConfig,
    normalized: &NormalizedAutoinstallUpdate,
) -> BTreeMap<String, String> {
    let mut field_errors = BTreeMap::new();

    if !is_valid_hostname(&normalized.hostname) {
        field_errors.insert(
            "hostname".to_string(),
            "Hostname must be 1-63 characters using only letters, numbers, or hyphens.".to_string(),
        );
    }

    if !is_valid_username(&normalized.username) {
        field_errors.insert(
            "username".to_string(),
            "Username must start with a lowercase letter or underscore and contain only lowercase letters, numbers, underscores, or hyphens.".to_string(),
        );
    }

    match normalized.password.as_deref() {
        Some(password) if password.len() < 8 => {
            field_errors.insert(
                "password".to_string(),
                "Password must be at least 8 characters.".to_string(),
            );
        }
        Some(_) => {}
        None if existing.password_hash.is_empty() => {
            field_errors.insert("password".to_string(), "Password is required.".to_string());
        }
        None => {}
    }

    if normalized.locale.is_empty() {
        field_errors.insert("locale".to_string(), "Locale is required.".to_string());
    }
    if normalized.keyboard_layout.is_empty() {
        field_errors.insert(
            "keyboardLayout".to_string(),
            "Keyboard layout is required.".to_string(),
        );
    }
    if normalized.timezone.is_empty() {
        field_errors.insert("timezone".to_string(), "Timezone is required.".to_string());
    }

    if let Some(invalid_key) = normalized
        .authorized_keys
        .iter()
        .find(|key| !is_valid_ssh_key(key))
    {
        field_errors.insert(
            "authorizedKeys".to_string(),
            format!("Invalid SSH public key: {invalid_key}"),
        );
    }

    field_errors
}

fn is_valid_hostname(value: &str) -> bool {
    if value.is_empty() || value.len() > 63 || value.starts_with('-') || value.ends_with('-') {
        return false;
    }

    value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn is_valid_username(value: &str) -> bool {
    let mut characters = value.chars();
    match characters.next() {
        Some(first) if first.is_ascii_lowercase() || first == '_' => {}
        _ => return false,
    }

    characters.all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || character == '_'
            || character == '-'
    })
}

fn is_valid_ssh_key(value: &str) -> bool {
    value.starts_with("ssh-") || value.starts_with("ecdsa-") || value.starts_with("sk-")
}

async fn hash_password(password: &str) -> Result<String> {
    match hash_password_with_args(password, &["passwd", "-6", "-stdin"]).await {
        Ok(hashed) => Ok(hashed),
        Err(_) => hash_password_with_args(password, &["passwd", "-1", "-stdin"]).await,
    }
}

async fn hash_password_with_args(password: &str, args: &[&str]) -> Result<String> {
    let mut child = Command::new("openssl")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn openssl for password hashing")?;

    let mut stdin = child.stdin.take().context("failed to open openssl stdin")?;
    stdin
        .write_all(format!("{password}\n").as_bytes())
        .await
        .context("failed to write password to openssl stdin")?;
    drop(stdin);

    let output = child
        .wait_with_output()
        .await
        .context("failed to wait for openssl password hashing")?;
    if !output.status.success() {
        return Err(anyhow!(
            "openssl password hashing failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let hashed = String::from_utf8(output.stdout)
        .context("openssl returned non-utf8 password hash")?
        .trim()
        .to_string();
    if hashed.is_empty() {
        return Err(anyhow!("openssl returned an empty password hash"));
    }
    Ok(hashed)
}

#[derive(Debug, Clone)]
struct NormalizedAutoinstallUpdate {
    hostname: String,
    username: String,
    password: Option<String>,
    locale: String,
    keyboard_layout: String,
    timezone: String,
    storage_layout: UbuntuStorageLayout,
    install_open_ssh: bool,
    allow_password_auth: bool,
    authorized_keys: Vec<String>,
    packages: Vec<String>,
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

    #[tokio::test]
    async fn update_rejects_invalid_business_values() {
        let update = UbuntuAutoinstallConfigUpdate {
            hostname: "-bad".to_string(),
            username: "BadUser".to_string(),
            password: Some("short".to_string()),
            locale: "".to_string(),
            keyboard_layout: "".to_string(),
            timezone: "".to_string(),
            storage_layout: UbuntuStorageLayout::Direct,
            install_open_ssh: true,
            allow_password_auth: true,
            authorized_keys: vec!["invalid".to_string()],
            packages: vec!["curl".to_string()],
        };

        let error = apply_update(&PersistedUbuntuAutoinstallConfig::default(), update)
            .await
            .expect_err("validation");
        match error {
            UpdateError::Validation(error) => {
                assert!(error.field_errors.contains_key("hostname"));
                assert!(error.field_errors.contains_key("username"));
                assert!(error.field_errors.contains_key("password"));
                assert!(error.field_errors.contains_key("locale"));
                assert!(error.field_errors.contains_key("keyboardLayout"));
                assert!(error.field_errors.contains_key("timezone"));
                assert!(error.field_errors.contains_key("authorizedKeys"));
            }
            UpdateError::Internal(error) => panic!("unexpected internal error: {error:?}"),
        }
    }

    #[tokio::test]
    async fn update_hashes_password_and_trims_lists() {
        let existing = PersistedUbuntuAutoinstallConfig::default();
        let update = UbuntuAutoinstallConfigUpdate {
            hostname: "ubuntu-host".to_string(),
            username: "ubuntu".to_string(),
            password: Some("correcthorsebattery".to_string()),
            locale: "en_US.UTF-8".to_string(),
            keyboard_layout: "us".to_string(),
            timezone: "UTC".to_string(),
            storage_layout: UbuntuStorageLayout::Lvm,
            install_open_ssh: true,
            allow_password_auth: false,
            authorized_keys: vec![
                " ssh-ed25519 AAAA user@example ".to_string(),
                "".to_string(),
                "ssh-ed25519 AAAA user@example".to_string(),
            ],
            packages: vec![" curl ".to_string(), "git".to_string(), "git".to_string()],
        };

        let updated = apply_update(&existing, update).await.expect("updated");
        assert_ne!(updated.password_hash, existing.password_hash);
        assert!(
            updated.password_hash.starts_with("$6$") || updated.password_hash.starts_with("$1$")
        );
        assert_eq!(
            updated.authorized_keys,
            vec!["ssh-ed25519 AAAA user@example"]
        );
        assert_eq!(updated.packages, vec!["curl", "git"]);
        assert_eq!(updated.storage_layout, UbuntuStorageLayout::Lvm);
    }

    #[tokio::test]
    async fn update_keeps_existing_password_when_blank() {
        let existing = PersistedUbuntuAutoinstallConfig::default();
        let update = UbuntuAutoinstallConfigUpdate {
            hostname: existing.hostname.clone(),
            username: existing.username.clone(),
            password: None,
            locale: existing.locale.clone(),
            keyboard_layout: existing.keyboard_layout.clone(),
            timezone: existing.timezone.clone(),
            storage_layout: existing.storage_layout,
            install_open_ssh: existing.install_open_ssh,
            allow_password_auth: existing.allow_password_auth,
            authorized_keys: existing.authorized_keys.clone(),
            packages: existing.packages.clone(),
        };

        let updated = apply_update(&existing, update).await.expect("updated");
        assert_eq!(updated.password_hash, existing.password_hash);
    }
}
