use std::sync::Arc;

use anyhow::Result;
use boot_recipe::{BootMode, DhcpGuidance, DistroId, all_distros, get_recipe};
use image_cache::{CacheEntry, ImageCache};

use crate::boot_assets::ResolvedBootAsset;
use crate::config::Config;
use crate::persistence::{PersistedSelection, load_selection, save_selection};

#[derive(Clone)]
pub struct AppState {
    config: Config,
    cache: ImageCache,
    selected_distro: Arc<tokio::sync::RwLock<DistroId>>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir).await?;
        let persisted = load_selection(&config.state_path()).await?;
        let cache = ImageCache::new(config.cache_dir()).await?;

        Ok(Self {
            config,
            cache,
            selected_distro: Arc::new(tokio::sync::RwLock::new(persisted.selected_distro)),
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
        let uefi = get_recipe(selected, BootMode::Uefi)?.dhcp;

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

    pub async fn resolve_boot_asset(&self, requested_path: &str) -> Option<ResolvedBootAsset> {
        crate::boot_assets::resolve_asset(
            &self.config.cache_dir(),
            self.selected_distro().await,
            requested_path,
            self.config.tftp_advertise_addr,
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
