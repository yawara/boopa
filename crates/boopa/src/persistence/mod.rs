use std::net::Ipv4Addr;
use std::path::Path;

use anyhow::Context;
use boot_recipe::DistroId;
use serde::{Deserialize, Serialize};

use crate::autoinstall::PersistedUbuntuAutoinstallConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSelection {
    pub selected_distro: DistroId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedDhcpLease {
    pub ip_address: Ipv4Addr,
    pub client_key: String,
    pub client_mac: String,
    pub expires_at_unix_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedDhcpLeases {
    pub leases: Vec<PersistedDhcpLease>,
}

impl Default for PersistedSelection {
    fn default() -> Self {
        Self {
            selected_distro: DistroId::Ubuntu,
        }
    }
}

pub async fn load_selection(path: &Path) -> anyhow::Result<PersistedSelection> {
    match tokio::fs::read(path).await {
        Ok(contents) => serde_json::from_slice(&contents)
            .with_context(|| format!("failed to parse persisted selection at {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedSelection::default())
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to read persisted selection at {}", path.display())),
    }
}

pub async fn save_selection(path: &Path, selection: &PersistedSelection) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = serde_json::to_vec_pretty(selection)?;
    tokio::fs::write(path, payload)
        .await
        .with_context(|| format!("failed to write {}", path.display()))
}

pub async fn load_ubuntu_autoinstall(
    path: &Path,
) -> anyhow::Result<PersistedUbuntuAutoinstallConfig> {
    match tokio::fs::read(path).await {
        Ok(contents) => serde_json::from_slice(&contents)
            .with_context(|| format!("failed to parse autoinstall settings at {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedUbuntuAutoinstallConfig::default())
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to read autoinstall settings at {}", path.display())),
    }
}

pub async fn save_ubuntu_autoinstall(
    path: &Path,
    config: &PersistedUbuntuAutoinstallConfig,
) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = serde_json::to_vec_pretty(config)?;
    tokio::fs::write(path, payload)
        .await
        .with_context(|| format!("failed to write {}", path.display()))
}

pub async fn load_dhcp_leases(path: &Path) -> anyhow::Result<PersistedDhcpLeases> {
    match tokio::fs::read(path).await {
        Ok(contents) => serde_json::from_slice(&contents)
            .with_context(|| format!("failed to parse DHCP leases at {}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedDhcpLeases::default())
        }
        Err(error) => {
            Err(error).with_context(|| format!("failed to read DHCP leases at {}", path.display()))
        }
    }
}

pub async fn save_dhcp_leases(path: &Path, leases: &PersistedDhcpLeases) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let payload = serde_json::to_vec_pretty(leases)?;
    tokio::fs::write(path, payload)
        .await
        .with_context(|| format!("failed to write {}", path.display()))
}
