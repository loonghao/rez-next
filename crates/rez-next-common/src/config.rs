//! Configuration management for rez-core

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// Configuration for rez-core components
#[cfg_attr(feature = "python-bindings", pyclass(name = "Config"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RezCoreConfig {
    /// Enable Rust version system
    pub use_rust_version: bool,

    /// Enable Rust solver
    pub use_rust_solver: bool,

    /// Enable Rust repository system
    pub use_rust_repository: bool,

    /// Fallback to Python on Rust errors
    pub rust_fallback: bool,

    /// Number of threads for parallel operations
    pub thread_count: Option<usize>,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Package search paths
    pub packages_path: Vec<String>,

    /// Local packages path
    pub local_packages_path: String,

    /// Release packages path
    pub release_packages_path: String,

    /// Default shell
    pub default_shell: String,

    /// Rez version
    pub version: String,

    /// Plugin paths
    pub plugin_path: Vec<String>,

    /// Package cache paths
    pub package_cache_path: Vec<String>,

    /// Temporary directory
    pub tmpdir: String,

    /// Editor command
    pub editor: String,

    /// Image viewer command
    pub image_viewer: String,

    /// Browser command
    pub browser: String,

    /// Diff program
    pub difftool: String,

    /// Terminal type
    pub terminal_emulator_command: String,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable memory cache
    pub enable_memory_cache: bool,

    /// Enable disk cache
    pub enable_disk_cache: bool,

    /// Memory cache size (number of entries)
    pub memory_cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl RezCoreConfig {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Config(use_rust_version={}, use_rust_solver={}, use_rust_repository={})",
            self.use_rust_version, self.use_rust_solver, self.use_rust_repository
        )
    }
}

#[cfg(not(feature = "python-bindings"))]
impl RezCoreConfig {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for RezCoreConfig {
    fn default() -> Self {
        Self {
            use_rust_version: true,
            use_rust_solver: true,
            use_rust_repository: true,
            rust_fallback: true,
            thread_count: None, // Use system default
            cache: CacheConfig::default(),
            packages_path: vec![
                "~/packages".to_string(),
                "~/.rez/packages/int".to_string(),
                "~/.rez/packages/ext".to_string(),
            ],
            local_packages_path: "~/packages".to_string(),
            release_packages_path: "~/.rez/packages/int".to_string(),
            default_shell: if cfg!(windows) { "cmd" } else { "bash" }.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_path: vec![],
            package_cache_path: vec!["~/.rez/cache".to_string()],
            tmpdir: std::env::temp_dir().to_string_lossy().to_string(),
            editor: if cfg!(windows) { "notepad" } else { "vi" }.to_string(),
            image_viewer: if cfg!(windows) { "mspaint" } else { "xdg-open" }.to_string(),
            browser: if cfg!(windows) { "start" } else { "xdg-open" }.to_string(),
            difftool: if cfg!(windows) { "fc" } else { "diff" }.to_string(),
            terminal_emulator_command: if cfg!(windows) {
                "cmd /c start cmd"
            } else {
                "xterm"
            }
            .to_string(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_memory_cache: true,
            enable_disk_cache: true,
            memory_cache_size: 1000,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}

impl RezCoreConfig {
    /// Get the list of configuration file paths that are searched
    pub fn get_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Built-in config (if exists)
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                paths.push(exe_dir.join("rezconfig.yaml"));
                paths.push(exe_dir.join("rezconfig.json"));
            }
        }

        // 2. Environment variable REZ_CONFIG_FILE
        if let Ok(config_file) = env::var("REZ_CONFIG_FILE") {
            for path in config_file.split(std::path::MAIN_SEPARATOR) {
                paths.push(PathBuf::from(path));
            }
        }

        // 3. System-wide config
        if cfg!(unix) {
            paths.push(PathBuf::from("/etc/rez/config.yaml"));
            paths.push(PathBuf::from("/usr/local/etc/rez/config.yaml"));
        } else if cfg!(windows) {
            if let Ok(program_data) = env::var("PROGRAMDATA") {
                paths.push(PathBuf::from(program_data).join("rez").join("config.yaml"));
            }
        }

        // 4. User home config (unless disabled)
        if env::var("REZ_DISABLE_HOME_CONFIG")
            .unwrap_or_default()
            .to_lowercase()
            != "1"
        {
            if let Ok(home) = env::var("HOME") {
                let home_path = PathBuf::from(&home);
                paths.push(home_path.join(".rezconfig"));
                paths.push(home_path.join(".rezconfig.yaml"));
                paths.push(home_path.join(".rez").join("config.yaml"));
            } else if cfg!(windows) {
                if let Ok(userprofile) = env::var("USERPROFILE") {
                    let user_path = PathBuf::from(&userprofile);
                    paths.push(user_path.join(".rezconfig"));
                    paths.push(user_path.join(".rezconfig.yaml"));
                    paths.push(user_path.join(".rez").join("config.yaml"));
                }
            }
        }

        paths
    }

    /// Get the list of configuration files that actually exist and are sourced
    pub fn get_sourced_paths() -> Vec<PathBuf> {
        Self::get_search_paths()
            .into_iter()
            .filter(|path| path.exists())
            .collect()
    }

    /// Load configuration from files (reads actual rezconfig files)
    pub fn load() -> Self {
        let mut config = Self::default();

        // Try to load from config files in priority order
        for path in Self::get_search_paths() {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    // Try YAML format
                    if let Ok(loaded) = serde_yaml::from_str::<RezCoreConfig>(&content) {
                        config = loaded;
                        break;
                    }
                    // Try JSON format
                    if let Ok(loaded) = serde_json::from_str::<RezCoreConfig>(&content) {
                        config = loaded;
                        break;
                    }
                }
            }
        }

        // Override with environment variables
        if let Ok(packages_path) = env::var("REZ_PACKAGES_PATH") {
            config.packages_path = packages_path.split(':').map(|s| s.to_string()).collect();
        }
        if let Ok(local_path) = env::var("REZ_LOCAL_PACKAGES_PATH") {
            config.local_packages_path = local_path;
        }
        if let Ok(release_path) = env::var("REZ_RELEASE_PACKAGES_PATH") {
            config.release_packages_path = release_path;
        }

        config
    }

    /// Get a configuration field by dot-separated path
    pub fn get_field(&self, field_path: &str) -> Option<serde_json::Value> {
        let parts: Vec<&str> = field_path.split('.').collect();

        // Convert config to JSON for easy field access
        let config_json = serde_json::to_value(self).ok()?;

        let mut current = &config_json;
        for part in parts {
            current = current.get(part)?;
        }

        Some(current.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_sensible_values() {
        let cfg = RezCoreConfig::default();
        assert!(cfg.use_rust_solver);
        assert!(cfg.use_rust_version);
        assert!(cfg.use_rust_repository);
        assert!(cfg.rust_fallback);
        assert!(!cfg.packages_path.is_empty());
        assert!(!cfg.local_packages_path.is_empty());
        assert!(!cfg.release_packages_path.is_empty());
        assert!(!cfg.version.is_empty());
    }

    #[test]
    fn test_default_cache_config() {
        let cfg = RezCoreConfig::default();
        assert!(cfg.cache.enable_memory_cache);
        assert!(cfg.cache.enable_disk_cache);
        assert!(cfg.cache.memory_cache_size > 0);
        assert!(cfg.cache.cache_ttl_seconds > 0);
    }

    #[test]
    fn test_get_field_simple() {
        let cfg = RezCoreConfig::default();
        let v = cfg.get_field("version");
        assert!(v.is_some());
        if let Some(serde_json::Value::String(s)) = v {
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn test_get_field_packages_path() {
        let cfg = RezCoreConfig::default();
        let v = cfg.get_field("packages_path");
        assert!(v.is_some());
        if let Some(serde_json::Value::Array(arr)) = v {
            assert!(!arr.is_empty());
        }
    }

    #[test]
    fn test_get_field_nested() {
        let cfg = RezCoreConfig::default();
        let v = cfg.get_field("cache.enable_memory_cache");
        assert!(v.is_some());
        assert_eq!(v, Some(serde_json::Value::Bool(true)));
    }

    #[test]
    fn test_get_field_nested_numeric() {
        let cfg = RezCoreConfig::default();
        let v = cfg.get_field("cache.memory_cache_size");
        assert!(v.is_some());
    }

    #[test]
    fn test_get_field_nonexistent() {
        let cfg = RezCoreConfig::default();
        assert!(cfg.get_field("nonexistent_field").is_none());
        assert!(cfg.get_field("cache.nonexistent").is_none());
    }

    #[test]
    fn test_get_search_paths_not_empty() {
        let paths = RezCoreConfig::get_search_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_get_search_paths_contain_home_config() {
        let paths = RezCoreConfig::get_search_paths();
        let has_home = paths.iter().any(|p| {
            p.to_string_lossy().contains(".rezconfig") || p.to_string_lossy().contains(".rez")
        });
        assert!(has_home);
    }

    #[test]
    fn test_load_returns_config() {
        // Should not panic, even if no config file exists
        let cfg = RezCoreConfig::load();
        assert!(!cfg.version.is_empty());
    }

    #[test]
    fn test_env_override_packages_path() {
        // Only safe to test if env var is not already set
        if std::env::var("REZ_PACKAGES_PATH").is_err() {
            std::env::set_var("REZ_PACKAGES_PATH", "/tmp/test_pkgs:/tmp/other_pkgs");
            let cfg = RezCoreConfig::load();
            assert!(cfg.packages_path.contains(&"/tmp/test_pkgs".to_string()));
            std::env::remove_var("REZ_PACKAGES_PATH");
        }
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let cfg = RezCoreConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let restored: RezCoreConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.version, restored.version);
        assert_eq!(cfg.packages_path, restored.packages_path);
        assert_eq!(
            cfg.cache.memory_cache_size,
            restored.cache.memory_cache_size
        );
    }

    #[test]
    fn test_config_clone() {
        let cfg = RezCoreConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.version, cloned.version);
        assert_eq!(cfg.packages_path, cloned.packages_path);
    }
}
