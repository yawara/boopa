use std::sync::Arc;

use anyhow::Result;
use boot_recipe::{BootMode, DhcpGuidance, DistroId, all_distros, get_recipe};
use image_cache::{CacheEntry, ImageCache};

use crate::autoinstall::{
    PersistedUbuntuAutoinstallConfig, UbuntuAutoinstallConfigResponse,
    UbuntuAutoinstallConfigUpdate, UpdateError, apply_update,
};
use crate::boot_assets::{BootAssetTransport, ResolvedBootAsset};
use crate::config::Config;
use crate::persistence::{
    PersistedSelection, load_selection, load_ubuntu_autoinstall, save_selection,
    save_ubuntu_autoinstall,
};

#[derive(Clone)]
pub struct AppState {
    config: Config,
    cache: ImageCache,
    selected_distro: Arc<tokio::sync::RwLock<DistroId>>,
    ubuntu_autoinstall: Arc<tokio::sync::RwLock<PersistedUbuntuAutoinstallConfig>>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir).await?;
        let persisted = load_selection(&config.state_path()).await?;
        let ubuntu_autoinstall = load_ubuntu_autoinstall(&config.ubuntu_autoinstall_path()).await?;
        let cache = ImageCache::new(config.cache_dir()).await?;

        Ok(Self {
            config,
            cache,
            selected_distro: Arc::new(tokio::sync::RwLock::new(persisted.selected_distro)),
            ubuntu_autoinstall: Arc::new(tokio::sync::RwLock::new(ubuntu_autoinstall)),
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn selected_distro(&self) -> DistroId {
        *self.selected_distro.read().await
    }

    pub async fn supported_distros(&self) -> DistrosResponse {
        DistrosResponse {
            selected: self.selected_distro().await,
            distros: all_distros()
                .into_iter()
                .map(|id| DistroSummary {
                    id,
                    label: id.label().to_string(),
                })
                .collect(),
        }
    }

    pub async fn dhcp_guide(&self, distro: Option<DistroId>) -> Result<DhcpResponse> {
        let selected = distro.unwrap_or(self.selected_distro().await);
        let bios = get_recipe(selected, BootMode::Bios)?.dhcp;
        let mut uefi = get_recipe(selected, BootMode::Uefi)?.dhcp;

        match selected {
            DistroId::Ubuntu => {
                uefi.notes.push(format!(
                    "Generated grub.cfg injects iso-url={} and autoinstall seed URL={}; boot clients must reach boopa's HTTP port in addition to TFTP.",
                    self.config.ubuntu_uefi_iso_url(),
                    self.config.ubuntu_uefi_autoinstall_seed_url()
                ));
            }
            DistroId::Fedora => {
                uefi.notes.push(format!(
                    "Generated grub.cfg injects inst.ks={}; boot clients must reach boopa's HTTP port in addition to TFTP.",
                    self.config.fedora_uefi_kickstart_url()
                ));
            }
            DistroId::Arch => {}
        }

        Ok(DhcpResponse {
            selected,
            bios,
            uefi,
        })
    }

    pub async fn set_selected_distro(&self, distro: DistroId) -> Result<SelectionResponse> {
        {
            let mut selected = self.selected_distro.write().await;
            *selected = distro;
        }

        save_selection(
            &self.config.state_path(),
            &PersistedSelection {
                selected_distro: distro,
            },
        )
        .await?;

        Ok(SelectionResponse { selected: distro })
    }

    pub async fn cache_status(&self) -> Result<CacheResponse> {
        let selected = self.selected_distro().await;
        Ok(CacheResponse {
            selected,
            entries: self.cache.status_for_distro(selected).await?,
        })
    }

    pub async fn refresh_cache(&self, distro: Option<DistroId>) -> Result<CacheResponse> {
        let selected = distro.unwrap_or(self.selected_distro().await);
        Ok(CacheResponse {
            selected,
            entries: self.cache.refresh_distro(selected).await?,
        })
    }

    pub async fn ubuntu_autoinstall_config(&self) -> Result<UbuntuAutoinstallConfigResponse> {
        self.ubuntu_autoinstall.read().await.clone().to_response()
    }

    pub async fn update_ubuntu_autoinstall(
        &self,
        update: UbuntuAutoinstallConfigUpdate,
    ) -> std::result::Result<UbuntuAutoinstallConfigResponse, UpdateError> {
        let existing = self.ubuntu_autoinstall.read().await.clone();
        let updated = apply_update(&existing, update).await?;
        save_ubuntu_autoinstall(&self.config.ubuntu_autoinstall_path(), &updated)
            .await
            .map_err(UpdateError::from)?;
        {
            let mut state = self.ubuntu_autoinstall.write().await;
            *state = updated.clone();
        }
        updated.to_response().map_err(UpdateError::from)
    }

    pub async fn resolve_boot_asset(
        &self,
        requested_path: &str,
        transport: BootAssetTransport,
    ) -> Option<ResolvedBootAsset> {
        let ubuntu_autoinstall = self.ubuntu_autoinstall.read().await.clone();
        crate::boot_assets::resolve_asset_with_ubuntu_autoinstall(
            &self.config.cache_dir(),
            self.selected_distro().await,
            &ubuntu_autoinstall,
            requested_path,
            self.config.tftp_advertise_addr,
            &self.config.guest_http_base_url(),
            transport,
        )
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistroSummary {
    pub id: DistroId,
    pub label: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DistrosResponse {
    pub selected: DistroId,
    pub distros: Vec<DistroSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SelectionResponse {
    pub selected: DistroId,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DhcpResponse {
    pub selected: DistroId,
    pub bios: DhcpGuidance,
    pub uefi: DhcpGuidance,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheResponse {
    pub selected: DistroId,
    pub entries: Vec<CacheEntry>,
}
