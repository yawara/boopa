use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Context, anyhow, bail};

#[derive(Clone, Debug)]
pub struct Config {
    pub api_bind: SocketAddr,
    pub tftp_bind: SocketAddr,
    pub tftp_advertise_addr: SocketAddr,
    pub dhcp: DhcpConfig,
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let tftp_bind = env_var("BOOPA_TFTP_BIND")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 6969)));
        let tftp_advertise_addr = env_var("BOOPA_TFTP_ADVERTISE_ADDR")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| default_tftp_advertise_addr(tftp_bind));

        let config = Self {
            api_bind: env_var("BOOPA_API_BIND")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080))),
            tftp_bind,
            tftp_advertise_addr,
            dhcp: DhcpConfig::from_env(tftp_advertise_addr)?,
            data_dir: data_dir_from_env(),
            frontend_dir: env_var("BOOPA_FRONTEND_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("frontend/dist")),
        };

        Ok(config)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    pub fn state_path(&self) -> PathBuf {
        self.data_dir.join("selection.json")
    }

    pub fn ubuntu_autoinstall_path(&self) -> PathBuf {
        self.data_dir.join("ubuntu-autoinstall.json")
    }

    pub fn dhcp_leases_path(&self) -> PathBuf {
        self.data_dir.join("dhcp-leases.json")
    }

    pub fn guest_http_base_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.tftp_advertise_addr.ip(),
            self.api_bind.port()
        )
    }

    pub fn guest_boot_url(&self, relative_path: &str) -> String {
        format!(
            "{}/boot/{}",
            self.guest_http_base_url(),
            relative_path.trim_start_matches('/')
        )
    }

    pub fn ubuntu_uefi_iso_url(&self) -> String {
        self.guest_boot_url("ubuntu/uefi/live-server.iso")
    }

    pub fn ubuntu_uefi_autoinstall_seed_url(&self) -> String {
        format!("{}/", self.guest_boot_url("ubuntu/uefi/autoinstall"))
    }

    pub fn fedora_uefi_kickstart_url(&self) -> String {
        self.guest_boot_url("fedora/uefi/kickstart/ks.cfg")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DhcpConfig {
    pub mode: DhcpMode,
    pub bind: SocketAddr,
    pub subnet: Option<DhcpSubnetConfig>,
}

impl DhcpConfig {
    fn from_env(tftp_advertise_addr: SocketAddr) -> anyhow::Result<Self> {
        let mode = match env_var("BOOPA_DHCP_MODE") {
            Ok(value) => value.parse()?,
            Err(std::env::VarError::NotPresent) => DhcpMode::Disabled,
            Err(error) => return Err(error).context("failed to read BOOPA_DHCP_MODE"),
        };
        let bind = env_var("BOOPA_DHCP_BIND")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 67)));

        if mode == DhcpMode::Disabled {
            return Ok(Self {
                mode,
                bind,
                subnet: None,
            });
        }

        let server_ip = match tftp_advertise_addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => {
                bail!(
                    "DHCP authoritative mode requires an IPv4 BOOPA_TFTP_ADVERTISE_ADDR, got {}",
                    ip
                )
            }
        };
        let subnet = DhcpSubnetConfig::from_env(server_ip)?;

        Ok(Self {
            mode,
            bind,
            subnet: Some(subnet),
        })
    }

    pub fn enabled(&self) -> bool {
        self.mode != DhcpMode::Disabled
    }

    pub fn authoritative_subnet(&self) -> Option<&DhcpSubnetConfig> {
        self.subnet.as_ref()
    }
}

impl Default for DhcpConfig {
    fn default() -> Self {
        Self {
            mode: DhcpMode::Disabled,
            bind: SocketAddr::from(([0, 0, 0, 0], 67)),
            subnet: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DhcpMode {
    Disabled,
    Authoritative,
}

impl std::str::FromStr for DhcpMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "disabled" => Ok(Self::Disabled),
            "authoritative" => Ok(Self::Authoritative),
            other => Err(anyhow!(
                "unsupported BOOPA_DHCP_MODE '{}'; expected 'disabled' or 'authoritative'",
                other
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DhcpSubnetConfig {
    pub subnet: Ipv4Subnet,
    pub pool_start: Ipv4Addr,
    pub pool_end: Ipv4Addr,
    pub router: Option<Ipv4Addr>,
    pub dns_servers: Vec<Ipv4Addr>,
    pub lease_duration_secs: u32,
    pub server_ip: Ipv4Addr,
}

impl DhcpSubnetConfig {
    fn from_env(server_ip: Ipv4Addr) -> anyhow::Result<Self> {
        let subnet = required_env("BOOPA_DHCP_SUBNET")?
            .parse::<Ipv4Subnet>()
            .context("failed to parse BOOPA_DHCP_SUBNET")?;
        let pool_start = required_env("BOOPA_DHCP_POOL_START")?
            .parse::<Ipv4Addr>()
            .context("failed to parse BOOPA_DHCP_POOL_START")?;
        let pool_end = required_env("BOOPA_DHCP_POOL_END")?
            .parse::<Ipv4Addr>()
            .context("failed to parse BOOPA_DHCP_POOL_END")?;
        let router = match env_var("BOOPA_DHCP_ROUTER") {
            Ok(value) => Some(
                value
                    .parse::<Ipv4Addr>()
                    .context("failed to parse BOOPA_DHCP_ROUTER")?,
            ),
            Err(std::env::VarError::NotPresent) => None,
            Err(error) => return Err(error).context("failed to read BOOPA_DHCP_ROUTER"),
        };
        let dns_servers = env_var("BOOPA_DHCP_DNS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(parse_ipv4_csv)
            .transpose()?
            .unwrap_or_default();
        let lease_duration_secs = env_var("BOOPA_DHCP_LEASE_SECS")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .map(|value| {
                value
                    .parse::<u32>()
                    .context("failed to parse BOOPA_DHCP_LEASE_SECS")
            })
            .transpose()?
            .unwrap_or(3600);

        if !subnet.contains(pool_start) {
            bail!("BOOPA_DHCP_POOL_START must be inside {}", subnet);
        }
        if !subnet.contains(pool_end) {
            bail!("BOOPA_DHCP_POOL_END must be inside {}", subnet);
        }
        if u32::from(pool_start) > u32::from(pool_end) {
            bail!("BOOPA_DHCP_POOL_START must be <= BOOPA_DHCP_POOL_END");
        }
        if let Some(router) = router
            && !subnet.contains(router)
        {
            bail!("BOOPA_DHCP_ROUTER must be inside {}", subnet);
        }
        for dns in &dns_servers {
            if !subnet.contains(*dns) {
                bail!("BOOPA_DHCP_DNS entry {} must be inside {}", dns, subnet);
            }
        }
        if lease_duration_secs == 0 {
            bail!("BOOPA_DHCP_LEASE_SECS must be > 0");
        }

        Ok(Self {
            subnet,
            pool_start,
            pool_end,
            router,
            dns_servers,
            lease_duration_secs,
            server_ip,
        })
    }

    pub fn pool_label(&self) -> String {
        format!("{} - {}", self.pool_start, self.pool_end)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ipv4Subnet {
    pub network: Ipv4Addr,
    pub prefix_len: u8,
}

impl Ipv4Subnet {
    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        let mask = self.mask();
        (u32::from(ip) & mask) == u32::from(self.network)
    }

    pub fn netmask(&self) -> Ipv4Addr {
        Ipv4Addr::from(self.mask())
    }

    fn mask(&self) -> u32 {
        if self.prefix_len == 0 {
            0
        } else {
            u32::MAX << (32 - self.prefix_len)
        }
    }
}

impl fmt::Display for Ipv4Subnet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.network, self.prefix_len)
    }
}

impl std::str::FromStr for Ipv4Subnet {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (network, prefix_len) = value
            .split_once('/')
            .ok_or_else(|| anyhow!("expected CIDR notation like 10.0.0.0/24"))?;
        let network = network
            .parse::<Ipv4Addr>()
            .context("failed to parse subnet network address")?;
        let prefix_len = prefix_len
            .parse::<u8>()
            .context("failed to parse subnet prefix length")?;
        if prefix_len > 32 {
            bail!("subnet prefix length must be <= 32");
        }

        let subnet = Self {
            network,
            prefix_len,
        };
        if subnet.network != Ipv4Addr::from(u32::from(network) & subnet.mask()) {
            bail!("BOOPA_DHCP_SUBNET network address must be normalized for its prefix");
        }

        Ok(subnet)
    }
}

fn env_var(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)
}

fn required_env(name: &str) -> anyhow::Result<String> {
    env_var(name).with_context(|| format!("missing required environment variable {}", name))
}

fn parse_ipv4_csv(value: String) -> anyhow::Result<Vec<Ipv4Addr>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            segment
                .parse::<Ipv4Addr>()
                .with_context(|| format!("failed to parse IPv4 address '{}'", segment))
        })
        .collect()
}

fn default_tftp_advertise_addr(bind: SocketAddr) -> SocketAddr {
    match bind.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), bind.port())
        }
        IpAddr::V6(ip) if ip.is_unspecified() => {
            SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), bind.port())
        }
        _ => bind,
    }
}

fn data_dir_from_env() -> PathBuf {
    if let Ok(value) = env_var("BOOPA_DATA_DIR") {
        return PathBuf::from(value);
    }

    PathBuf::from("var/boopa")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    const BOOPA_ENV_VARS: [&str; 13] = [
        "BOOPA_API_BIND",
        "BOOPA_TFTP_BIND",
        "BOOPA_TFTP_ADVERTISE_ADDR",
        "BOOPA_DHCP_MODE",
        "BOOPA_DHCP_BIND",
        "BOOPA_DHCP_SUBNET",
        "BOOPA_DHCP_POOL_START",
        "BOOPA_DHCP_POOL_END",
        "BOOPA_DHCP_ROUTER",
        "BOOPA_DHCP_DNS",
        "BOOPA_DHCP_LEASE_SECS",
        "BOOPA_DATA_DIR",
        "BOOPA_FRONTEND_DIR",
    ];

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn clear_env() {
        for key in BOOPA_ENV_VARS {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    fn set_env_vars(pairs: &[(&str, &str)]) {
        for (key, value) in pairs {
            unsafe {
                std::env::set_var(key, value);
            }
        }
    }

    struct CurrentDirGuard {
        original_dir: PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &Path) -> Self {
            let original_dir = std::env::current_dir().expect("current dir");
            std::env::set_current_dir(path).expect("set current dir");
            Self { original_dir }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.original_dir).expect("restore current dir");
        }
    }

    #[test]
    fn boopa_env_vars_override_defaults() {
        let _lock = env_lock();
        clear_env();

        set_env_vars(&[
            ("BOOPA_API_BIND", "127.0.0.1:18080"),
            ("BOOPA_TFTP_BIND", "127.0.0.1:16969"),
            ("BOOPA_TFTP_ADVERTISE_ADDR", "10.0.2.2:16969"),
            ("BOOPA_DATA_DIR", "/tmp/boopa-data"),
            ("BOOPA_FRONTEND_DIR", "/tmp/boopa-frontend"),
        ]);

        let config = Config::from_env().expect("config");
        assert_eq!(config.api_bind, SocketAddr::from(([127, 0, 0, 1], 18080)));
        assert_eq!(config.tftp_bind, SocketAddr::from(([127, 0, 0, 1], 16969)));
        assert_eq!(
            config.tftp_advertise_addr,
            SocketAddr::from(([10, 0, 2, 2], 16969))
        );
        assert_eq!(config.dhcp.mode, DhcpMode::Disabled);
        assert_eq!(config.data_dir, PathBuf::from("/tmp/boopa-data"));
        assert_eq!(config.frontend_dir, PathBuf::from("/tmp/boopa-frontend"));

        clear_env();
    }

    #[test]
    fn defaults_to_boopa_data_dir_when_env_var_is_absent() {
        let _lock = env_lock();
        clear_env();

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let _dir_guard = CurrentDirGuard::change_to(temp_dir.path());

        let config = Config::from_env().expect("config");
        assert_eq!(config.data_dir, PathBuf::from("var/boopa"));

        clear_env();
    }

    #[test]
    fn wildcard_tftp_bind_defaults_advertise_addr_to_loopback() {
        let _lock = env_lock();
        clear_env();
        set_env_vars(&[("BOOPA_TFTP_BIND", "0.0.0.0:16969")]);

        let config = Config::from_env().expect("config");
        assert_eq!(config.tftp_bind, SocketAddr::from(([0, 0, 0, 0], 16969)));
        assert_eq!(
            config.tftp_advertise_addr,
            SocketAddr::from(([127, 0, 0, 1], 16969))
        );

        clear_env();
    }

    #[test]
    fn parses_authoritative_dhcp_config_from_env() {
        let _lock = env_lock();
        clear_env();
        set_env_vars(&[
            ("BOOPA_TFTP_ADVERTISE_ADDR", "10.0.2.2:16969"),
            ("BOOPA_DHCP_MODE", "authoritative"),
            ("BOOPA_DHCP_BIND", "127.0.0.1:1067"),
            ("BOOPA_DHCP_SUBNET", "10.0.2.0/24"),
            ("BOOPA_DHCP_POOL_START", "10.0.2.50"),
            ("BOOPA_DHCP_POOL_END", "10.0.2.99"),
            ("BOOPA_DHCP_ROUTER", "10.0.2.1"),
            ("BOOPA_DHCP_DNS", "10.0.2.2,10.0.2.3"),
            ("BOOPA_DHCP_LEASE_SECS", "7200"),
        ]);

        let config = Config::from_env().expect("config");
        let dhcp = config
            .dhcp
            .authoritative_subnet()
            .expect("authoritative subnet");
        assert_eq!(config.dhcp.mode, DhcpMode::Authoritative);
        assert_eq!(config.dhcp.bind, SocketAddr::from(([127, 0, 0, 1], 1067)));
        assert_eq!(dhcp.subnet, "10.0.2.0/24".parse().expect("subnet"));
        assert_eq!(dhcp.pool_start, Ipv4Addr::new(10, 0, 2, 50));
        assert_eq!(dhcp.pool_end, Ipv4Addr::new(10, 0, 2, 99));
        assert_eq!(dhcp.router, Some(Ipv4Addr::new(10, 0, 2, 1)));
        assert_eq!(
            dhcp.dns_servers,
            vec![Ipv4Addr::new(10, 0, 2, 2), Ipv4Addr::new(10, 0, 2, 3)]
        );
        assert_eq!(dhcp.lease_duration_secs, 7200);
        assert_eq!(dhcp.server_ip, Ipv4Addr::new(10, 0, 2, 2));

        clear_env();
    }

    #[test]
    fn rejects_invalid_authoritative_dhcp_pool() {
        let _lock = env_lock();
        clear_env();
        set_env_vars(&[
            ("BOOPA_TFTP_ADVERTISE_ADDR", "10.0.2.2:16969"),
            ("BOOPA_DHCP_MODE", "authoritative"),
            ("BOOPA_DHCP_SUBNET", "10.0.2.0/24"),
            ("BOOPA_DHCP_POOL_START", "10.0.3.10"),
            ("BOOPA_DHCP_POOL_END", "10.0.2.99"),
        ]);

        let error = Config::from_env().expect_err("invalid config");
        assert!(error.to_string().contains("BOOPA_DHCP_POOL_START"));

        clear_env();
    }
}
