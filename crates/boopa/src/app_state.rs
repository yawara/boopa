use std::net::Ipv4Addr;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use boot_recipe::{BootMode, DhcpGuidance, DistroId, all_distros, get_recipe};
use image_cache::{CacheEntry, ImageCache};

use crate::autoinstall::{
    PersistedUbuntuAutoinstallConfig, UbuntuAutoinstallConfigResponse,
    UbuntuAutoinstallConfigUpdate, UpdateError, apply_update,
};
use crate::boot_assets::{BootAssetTransport, ResolvedBootAsset};
use crate::config::Config;
use crate::dhcp::now_unix_secs;
use crate::persistence::{
    PersistedDhcpLease, PersistedDhcpLeases, PersistedSelection, load_dhcp_leases, load_selection,
    load_ubuntu_autoinstall, save_dhcp_leases, save_selection, save_ubuntu_autoinstall,
};

#[derive(Clone)]
pub struct AppState {
    config: Config,
    cache: ImageCache,
    selected_distro: Arc<tokio::sync::RwLock<DistroId>>,
    ubuntu_autoinstall: Arc<tokio::sync::RwLock<PersistedUbuntuAutoinstallConfig>>,
    dhcp_leases: Arc<tokio::sync::RwLock<PersistedDhcpLeases>>,
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self> {
        tokio::fs::create_dir_all(&config.data_dir).await?;
        let persisted = load_selection(&config.state_path()).await?;
        let ubuntu_autoinstall = load_ubuntu_autoinstall(&config.ubuntu_autoinstall_path()).await?;
        let mut dhcp_leases = load_dhcp_leases(&config.dhcp_leases_path()).await?;
        let now = now_unix_secs();
        dhcp_leases
            .leases
            .retain(|lease| lease.expires_at_unix_secs > now);
        save_dhcp_leases(&config.dhcp_leases_path(), &dhcp_leases).await?;
        let cache = ImageCache::new(config.cache_dir()).await?;

        Ok(Self {
            config,
            cache,
            selected_distro: Arc::new(tokio::sync::RwLock::new(persisted.selected_distro)),
            ubuntu_autoinstall: Arc::new(tokio::sync::RwLock::new(ubuntu_autoinstall)),
            dhcp_leases: Arc::new(tokio::sync::RwLock::new(dhcp_leases)),
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
            runtime: self.dhcp_runtime_status().await,
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

    pub async fn refresh_cache(
        &self,
        distro: Option<DistroId>,
        mode: Option<boot_recipe::BootMode>,
    ) -> Result<CacheResponse> {
        let selected = distro.unwrap_or(self.selected_distro().await);
        let entries = match mode {
            Some(m) => self.cache.refresh_distro_mode(selected, m).await?,
            None => self.cache.refresh_distro(selected).await?,
        };
        Ok(CacheResponse {
            selected,
            entries,
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

    pub async fn dhcp_runtime_status(&self) -> DhcpRuntimeStatusResponse {
        let active_leases = self.active_dhcp_leases().await;
        let authoritative = self.config.dhcp.authoritative_subnet();

        DhcpRuntimeStatusResponse {
            enabled: self.config.dhcp.enabled(),
            mode: match self.config.dhcp.mode {
                crate::config::DhcpMode::Disabled => "disabled".to_string(),
                crate::config::DhcpMode::Authoritative => "authoritative".to_string(),
            },
            bind_address: self.config.dhcp.bind.to_string(),
            subnet: authoritative.map(|subnet| subnet.subnet.to_string()),
            pool_start: authoritative.map(|subnet| subnet.pool_start.to_string()),
            pool_end: authoritative.map(|subnet| subnet.pool_end.to_string()),
            router: authoritative.and_then(|subnet| subnet.router.map(|ip| ip.to_string())),
            dns_servers: authoritative
                .map(|subnet| subnet.dns_servers.iter().map(ToString::to_string).collect())
                .unwrap_or_default(),
            lease_duration_secs: authoritative.map(|subnet| subnet.lease_duration_secs),
            active_lease_count: active_leases.len(),
            active_leases: active_leases
                .into_iter()
                .map(|lease| DhcpLeaseSummary {
                    ip_address: lease.ip_address.to_string(),
                    client_key: lease.client_key,
                    client_mac: lease.client_mac,
                    expires_at_unix_secs: lease.expires_at_unix_secs,
                })
                .collect(),
        }
    }

    pub async fn allocate_dhcp_lease(
        &self,
        client_key: String,
        client_mac: String,
        requested_ip: Option<Ipv4Addr>,
    ) -> Result<PersistedDhcpLease> {
        let authoritative = self
            .config
            .dhcp
            .authoritative_subnet()
            .context("DHCP authoritative mode is not configured")?;
        let now = now_unix_secs();
        let mut leases = self.dhcp_leases.write().await;
        leases
            .leases
            .retain(|lease| lease.expires_at_unix_secs > now);

        if let Some(existing) = leases
            .leases
            .iter_mut()
            .find(|lease| lease.client_key == client_key)
        {
            existing.client_mac = client_mac;
            existing.expires_at_unix_secs = now + authoritative.lease_duration_secs as u64;
            let lease = existing.clone();
            save_dhcp_leases(&self.config.dhcp_leases_path(), &leases).await?;
            return Ok(lease);
        }

        let chosen_ip = requested_ip
            .filter(|ip| self.ip_is_available(&leases, authoritative, *ip))
            .unwrap_or_else(|| {
                first_available_ip(&leases, authoritative).unwrap_or(Ipv4Addr::UNSPECIFIED)
            });
        if chosen_ip == Ipv4Addr::UNSPECIFIED {
            bail!(
                "no free DHCP leases remain in {}",
                authoritative.pool_label()
            );
        }

        let lease = PersistedDhcpLease {
            ip_address: chosen_ip,
            client_key,
            client_mac,
            expires_at_unix_secs: now + authoritative.lease_duration_secs as u64,
        };
        leases.leases.push(lease.clone());
        save_dhcp_leases(&self.config.dhcp_leases_path(), &leases).await?;
        Ok(lease)
    }

    async fn active_dhcp_leases(&self) -> Vec<PersistedDhcpLease> {
        let now = now_unix_secs();
        let leases = self.dhcp_leases.read().await;
        let mut active = leases
            .leases
            .iter()
            .filter(|lease| lease.expires_at_unix_secs > now)
            .cloned()
            .collect::<Vec<_>>();
        active.sort_by_key(|lease| u32::from(lease.ip_address));
        active
    }

    fn ip_is_available(
        &self,
        leases: &PersistedDhcpLeases,
        authoritative: &crate::config::DhcpSubnetConfig,
        ip: Ipv4Addr,
    ) -> bool {
        if !authoritative.subnet.contains(ip) {
            return false;
        }
        if u32::from(ip) < u32::from(authoritative.pool_start)
            || u32::from(ip) > u32::from(authoritative.pool_end)
        {
            return false;
        }

        !leases.leases.iter().any(|lease| lease.ip_address == ip)
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
    pub runtime: DhcpRuntimeStatusResponse,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheResponse {
    pub selected: DistroId,
    pub entries: Vec<CacheEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpLeaseSummary {
    pub ip_address: String,
    pub client_key: String,
    pub client_mac: String,
    pub expires_at_unix_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpRuntimeStatusResponse {
    pub enabled: bool,
    pub mode: String,
    pub bind_address: String,
    pub subnet: Option<String>,
    pub pool_start: Option<String>,
    pub pool_end: Option<String>,
    pub router: Option<String>,
    pub dns_servers: Vec<String>,
    pub lease_duration_secs: Option<u32>,
    pub active_lease_count: usize,
    pub active_leases: Vec<DhcpLeaseSummary>,
}

fn first_available_ip(
    leases: &PersistedDhcpLeases,
    authoritative: &crate::config::DhcpSubnetConfig,
) -> Option<Ipv4Addr> {
    let mut candidate = u32::from(authoritative.pool_start);
    let end = u32::from(authoritative.pool_end);
    while candidate <= end {
        let ip = Ipv4Addr::from(candidate);
        if !leases.leases.iter().any(|lease| lease.ip_address == ip) {
            return Some(ip);
        }
        candidate += 1;
    }

    None
}
