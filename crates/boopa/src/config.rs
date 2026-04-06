use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_bind: SocketAddr,
    pub tftp_bind: SocketAddr,
    pub tftp_advertise_addr: SocketAddr,
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Self {
        let tftp_bind = env_var("BOOPA_TFTP_BIND")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 6969)));

        Self {
            api_bind: env_var("BOOPA_API_BIND")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080))),
            tftp_bind,
            tftp_advertise_addr: env_var("BOOPA_TFTP_ADVERTISE_ADDR")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| default_tftp_advertise_addr(tftp_bind)),
            data_dir: data_dir_from_env(),
            frontend_dir: env_var("BOOPA_FRONTEND_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("frontend/dist")),
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    pub fn state_path(&self) -> PathBuf {
        self.data_dir.join("selection.json")
    }
}

fn env_var(name: &str) -> Result<String, std::env::VarError> {
    std::env::var(name)
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

    const BOOPA_ENV_VARS: [&str; 5] = [
        "BOOPA_API_BIND",
        "BOOPA_TFTP_BIND",
        "BOOPA_TFTP_ADVERTISE_ADDR",
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

        let config = Config::from_env();
        assert_eq!(config.api_bind, SocketAddr::from(([127, 0, 0, 1], 18080)));
        assert_eq!(config.tftp_bind, SocketAddr::from(([127, 0, 0, 1], 16969)));
        assert_eq!(
            config.tftp_advertise_addr,
            SocketAddr::from(([10, 0, 2, 2], 16969))
        );
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

        let config = Config::from_env();
        assert_eq!(config.data_dir, PathBuf::from("var/boopa"));

        clear_env();
    }

    #[test]
    fn wildcard_tftp_bind_defaults_advertise_addr_to_loopback() {
        let _lock = env_lock();
        clear_env();
        set_env_vars(&[("BOOPA_TFTP_BIND", "0.0.0.0:16969")]);

        let config = Config::from_env();
        assert_eq!(config.tftp_bind, SocketAddr::from(([0, 0, 0, 0], 16969)));
        assert_eq!(
            config.tftp_advertise_addr,
            SocketAddr::from(([127, 0, 0, 1], 16969))
        );

        clear_env();
    }
}
