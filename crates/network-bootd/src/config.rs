use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub api_bind: SocketAddr,
    pub tftp_bind: SocketAddr,
    pub data_dir: PathBuf,
    pub frontend_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            api_bind: env_var("BOOPA_API_BIND", "NETWORK_BOOTD_API_BIND")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 8080))),
            tftp_bind: env_var("BOOPA_TFTP_BIND", "NETWORK_BOOTD_TFTP_BIND")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 6969))),
            data_dir: data_dir_from_env(),
            frontend_dir: env_var("BOOPA_FRONTEND_DIR", "NETWORK_BOOTD_FRONTEND_DIR")
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

fn env_var(primary: &str, legacy: &str) -> Result<String, std::env::VarError> {
    std::env::var(primary).or_else(|_| std::env::var(legacy))
}

fn data_dir_from_env() -> PathBuf {
    if let Ok(value) = env_var("BOOPA_DATA_DIR", "NETWORK_BOOTD_DATA_DIR") {
        return PathBuf::from(value);
    }

    let canonical = PathBuf::from("var/boopa");
    if canonical.exists() {
        return canonical;
    }

    let legacy = PathBuf::from("var/network-bootd");
    if legacy.exists() {
        tracing::warn!(
            legacy_path = %legacy.display(),
            canonical_path = %canonical.display(),
            "using legacy data directory fallback; migrate to BOOPA_DATA_DIR or var/boopa"
        );
        return legacy;
    }

    canonical
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    const BOOPA_ENV_VARS: [&str; 4] = [
        "BOOPA_API_BIND",
        "BOOPA_TFTP_BIND",
        "BOOPA_DATA_DIR",
        "BOOPA_FRONTEND_DIR",
    ];
    const LEGACY_ENV_VARS: [&str; 4] = [
        "NETWORK_BOOTD_API_BIND",
        "NETWORK_BOOTD_TFTP_BIND",
        "NETWORK_BOOTD_DATA_DIR",
        "NETWORK_BOOTD_FRONTEND_DIR",
    ];

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("env lock")
    }

    fn clear_env() {
        for key in BOOPA_ENV_VARS.into_iter().chain(LEGACY_ENV_VARS) {
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
    fn boopa_env_vars_take_precedence_over_legacy_aliases() {
        let _lock = env_lock();
        clear_env();

        set_env_vars(&[
            ("BOOPA_API_BIND", "127.0.0.1:18080"),
            ("NETWORK_BOOTD_API_BIND", "127.0.0.1:28080"),
            ("BOOPA_TFTP_BIND", "127.0.0.1:16969"),
            ("NETWORK_BOOTD_TFTP_BIND", "127.0.0.1:26969"),
            ("BOOPA_DATA_DIR", "/tmp/boopa-data"),
            ("NETWORK_BOOTD_DATA_DIR", "/tmp/network-bootd-data"),
            ("BOOPA_FRONTEND_DIR", "/tmp/boopa-frontend"),
            ("NETWORK_BOOTD_FRONTEND_DIR", "/tmp/network-bootd-frontend"),
        ]);

        let config = Config::from_env();
        assert_eq!(config.api_bind, SocketAddr::from(([127, 0, 0, 1], 18080)));
        assert_eq!(config.tftp_bind, SocketAddr::from(([127, 0, 0, 1], 16969)));
        assert_eq!(config.data_dir, PathBuf::from("/tmp/boopa-data"));
        assert_eq!(config.frontend_dir, PathBuf::from("/tmp/boopa-frontend"));

        clear_env();
    }

    #[test]
    fn legacy_env_vars_still_work_when_primary_names_are_absent() {
        let _lock = env_lock();
        clear_env();

        set_env_vars(&[
            ("NETWORK_BOOTD_API_BIND", "127.0.0.1:28080"),
            ("NETWORK_BOOTD_TFTP_BIND", "127.0.0.1:26969"),
            ("NETWORK_BOOTD_DATA_DIR", "/tmp/network-bootd-data"),
            ("NETWORK_BOOTD_FRONTEND_DIR", "/tmp/network-bootd-frontend"),
        ]);

        let config = Config::from_env();
        assert_eq!(config.api_bind, SocketAddr::from(([127, 0, 0, 1], 28080)));
        assert_eq!(config.tftp_bind, SocketAddr::from(([127, 0, 0, 1], 26969)));
        assert_eq!(config.data_dir, PathBuf::from("/tmp/network-bootd-data"));
        assert_eq!(
            config.frontend_dir,
            PathBuf::from("/tmp/network-bootd-frontend")
        );

        clear_env();
    }

    #[test]
    fn defaults_to_boopa_data_dir_when_no_legacy_path_exists() {
        let _lock = env_lock();
        clear_env();

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let _dir_guard = CurrentDirGuard::change_to(temp_dir.path());

        let config = Config::from_env();
        assert_eq!(config.data_dir, PathBuf::from("var/boopa"));

        clear_env();
    }

    #[test]
    fn falls_back_to_legacy_data_dir_when_canonical_path_is_missing() {
        let _lock = env_lock();
        clear_env();

        let temp_dir = tempfile::tempdir().expect("tempdir");
        let _dir_guard = CurrentDirGuard::change_to(temp_dir.path());
        std::fs::create_dir_all(temp_dir.path().join("var/network-bootd")).expect("legacy dir");

        let config = Config::from_env();
        assert_eq!(config.data_dir, PathBuf::from("var/network-bootd"));

        clear_env();
    }
}
