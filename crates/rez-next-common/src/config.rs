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

    /// Load configuration from files
    pub fn load() -> Self {
        // For now, return default config
        // TODO: Implement actual file loading with YAML/JSON parsing
        Self::default()
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
