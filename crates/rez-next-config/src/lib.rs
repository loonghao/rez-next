//! Configuration management for Rez package manager.
//!
//! This crate provides configuration loading, validation, and querying
//! for the Rez package manager. It supports multiple configuration
//! sources (system, user, project) and environment variable overrides.

use serde_json::Value as JsonValue;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub mod keys;

pub use keys::{KeyDef, KeyType, Origin};

// -----------------------------------------------------------------------------
// Error types
// -----------------------------------------------------------------------------

/// Errors that can occur during configuration operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Configuration file not found.
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    /// Failed to parse configuration file.
    #[error("Failed to parse configuration file '{file}': {error}")]
    ParseError {
        /// Configuration file path.
        file: String,
        /// Parse error message.
        error: String,
    },

    /// Configuration validation error.
    #[error("Configuration validation error: {0}")]
    ValidationError(String),

    /// Unsupported configuration format.
    #[error("Unsupported configuration format: {0}")]
    UnsupportedFormat(String),

    /// Environment variable error.
    #[error("Environment variable error: {0}")]
    EnvVarError(String),
}

// -----------------------------------------------------------------------------
// Configuration source types
// -----------------------------------------------------------------------------

/// Configuration source priority (lower number = higher priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigSource {
    /// Environment variable override (highest priority).
    Environment = 0,
    /// Project-level configuration.
    Project = 1,
    /// User-level configuration.
    User = 2,
    /// System-level configuration (lowest priority).
    System = 3,
}

// -----------------------------------------------------------------------------
// Main Config struct
// -----------------------------------------------------------------------------

/// Main configuration manager for Rez.
///
/// Supports loading configuration from multiple sources:
/// - System config (`/etc/rezconfig.toml` or similar)
/// - User config (`~/.rez/rezconfig.toml`)
/// - Project config (`<project>/.rez/rezconfig.toml`)
/// - Environment variables (`REZ_*` prefix)
#[derive(Debug, Clone)]
pub struct Config {
    /// Merged configuration data.
    data: JsonValue,
    /// Configuration sources that were loaded.
    sources: Vec<ConfigSource>,
    /// Whether configuration is locked (no env var overrides).
    locked: bool,
}

impl Config {
    /// Create a new empty configuration.
    pub fn new() -> Self {
        Self {
            data: JsonValue::Object(serde_json::Map::new()),
            sources: Vec::new(),
            locked: false,
        }
    }

    /// Load configuration from all standard sources.
    pub fn load() -> Result<Self, ConfigError> {
        let mut config = Self::new();

        // Load system config
        if let Some(system_config) = Self::find_system_config() {
            config.merge_file(&system_config, ConfigSource::System)?;
        }

        // Load user config
        if let Some(user_config) = Self::find_user_config() {
            config.merge_file(&user_config, ConfigSource::User)?;
        }

        // Load project config
        if let Some(project_config) = Self::find_project_config() {
            config.merge_file(&project_config, ConfigSource::Project)?;
        }

        // Apply environment variable overrides (unless locked)
        if !config.locked {
            config.apply_env_overrides()?;
        }

        Ok(config)
    }

    /// Get a configuration value by key (dot-separated path).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let config = Config::load()?;
    /// let value = config.get("packages.path");
    /// ```
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        let parts: Vec<&str> = key.split('.').collect();
        let mut current = &self.data;

        for part in parts {
            let value = current.get(part)?;
            current = value;
        }

        Some(current)
    }

    /// Get a string value by key.
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).and_then(|v| v.as_str()).map(String::from)
    }

    /// Get a boolean value by key.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).and_then(|v| v.as_bool())
    }

    /// Get an integer value by key.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.as_i64())
    }

    /// Get a float value by key.
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.get(key).and_then(|v| v.as_f64())
    }

    /// Get an array value by key.
    pub fn get_array(&self, key: &str) -> Option<&Vec<JsonValue>> {
        self.get(key).and_then(|v| v.as_array())
    }

    /// Get an object value by key.
    pub fn get_object(&self, key: &str) -> Option<&serde_json::Map<String, JsonValue>> {
        self.get(key).and_then(|v| v.as_object())
    }

    /// Check if a configuration key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Merge configuration from a file.
    fn merge_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        source: ConfigSource,
    ) -> Result<(), ConfigError> {
        let path = path.as_ref();
        let path_str = path.display().to_string();
        let content =
            fs::read_to_string(path).map_err(|_| ConfigError::FileNotFound(path_str.clone()))?;

        let value: JsonValue = if path.extension().map(|e| e == "json").unwrap_or(false) {
            serde_json::from_str(&content).map_err(|e| ConfigError::ParseError {
                file: path_str.clone(),
                error: e.to_string(),
            })?
        } else if path
            .extension()
            .map(|e| e == "yaml" || e == "yml")
            .unwrap_or(false)
        {
            serde_yaml::from_str(&content).map_err(|e| ConfigError::ParseError {
                file: path_str.clone(),
                error: e.to_string(),
            })?
        } else if path.extension().map(|e| e == "toml").unwrap_or(false) {
            let toml_value: toml::Value =
                toml::from_str(&content).map_err(|e| ConfigError::ParseError {
                    file: path_str.clone(),
                    error: e.to_string(),
                })?;
            let toml_json_str =
                json5::to_string(&toml_value).map_err(|e| ConfigError::ParseError {
                    file: path_str.clone(),
                    error: format!("TOML → JSON5 conversion failed: {e}"),
                })?;
            json5::from_str(&toml_json_str).map_err(|e| ConfigError::ParseError {
                file: path_str,
                error: format!("JSON5 parse failed: {e}"),
            })?
        } else {
            return Err(ConfigError::UnsupportedFormat(
                path.extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ));
        };

        self.merge_value(value);
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }

        Ok(())
    }

    /// Merge a JSON value into the configuration.
    fn merge_value(&mut self, new_value: JsonValue) {
        if self.data.is_null() {
            self.data = new_value;
            return;
        }

        if let (Some(obj), Some(new_obj)) = (self.data.as_object(), new_value.as_object()) {
            let mut merged = obj.clone();
            for (key, value) in new_obj {
                if let Some(existing) = merged.get(key) {
                    if existing.is_object() && value.is_object() {
                        let mut config = Self::new();
                        config.data = existing.clone();
                        config.merge_value(value.clone());
                        merged.insert(key.clone(), config.data);
                    } else {
                        merged.insert(key.clone(), value.clone());
                    }
                } else {
                    merged.insert(key.clone(), value.clone());
                }
            }
            self.data = JsonValue::Object(merged);
        } else {
            self.data = new_value;
        }
    }

    /// Apply environment variable overrides (`REZ_*`).
    ///
    /// Keys are **flat snake_case**, matching rez (`src/rez/config.py`): the
    /// underscores in `REZ_PACKAGES_PATH` are part of the key name, not nesting,
    /// so it maps to `packages_path`. Mapping underscores to dots made
    /// nearly every real key resolve to a path that no config reads.
    ///
    /// `REZ_<KEY>_JSON` supplies a structured value and takes precedence over
    /// the plain `REZ_<KEY>` spelling; otherwise the raw string is used.
    fn apply_env_overrides(&mut self) -> Result<(), ConfigError> {
        let mut count = 0usize;

        let vars: Vec<(String, String)> = env::vars()
            .filter_map(|(key, value)| {
                key.strip_prefix("REZ_")
                    .map(|suffix| (suffix.to_ascii_lowercase(), value))
            })
            .filter(|(suffix, _)| !suffix.is_empty())
            .collect();

        let mut json_keys: Vec<String> = Vec::new();
        for (suffix, raw) in &vars {
            let Some(field) = suffix.strip_suffix("_json").filter(|f| !f.is_empty()) else {
                continue;
            };
            match serde_json::from_str::<JsonValue>(raw) {
                Ok(parsed) => {
                    tracing::debug!("REZ override (json): {}={}", field, parsed);
                    set_flat_value(&mut self.data, field, parsed);
                    json_keys.push(field.to_string());
                    count += 1;
                }
                Err(e) => {
                    return Err(ConfigError::EnvVarError(format!(
                        "REZ_{}: invalid JSON ({e})",
                        suffix.to_ascii_uppercase()
                    )));
                }
            }
        }

        for (suffix, raw) in &vars {
            if suffix.ends_with("_json") || json_keys.contains(suffix) {
                continue;
            }
            let parsed: JsonValue =
                serde_json::from_str(raw).unwrap_or_else(|_| JsonValue::String(raw.clone()));
            tracing::debug!("REZ override: {}={}", suffix, parsed);
            // `plugins.*` is the one family rez addresses by dotted path; every
            // other key is flat snake_case.
            if suffix.starts_with("plugins.") {
                set_nested_value(&mut self.data, suffix, parsed);
            } else {
                set_flat_value(&mut self.data, suffix, parsed);
            }
            count += 1;
        }

        if count > 0 {
            tracing::info!("Applied {} REZ_* environment override(s)", count);
        }
        Ok(())
    }

    /// Find system-level configuration file.
    fn find_system_config() -> Option<PathBuf> {
        // Check common system config locations
        let locations = vec![
            PathBuf::from("/etc/rez/rezconfig.toml"),
            PathBuf::from("/etc/rezconfig.toml"),
            PathBuf::from("C:\\ProgramData\\rez\\rezconfig.toml"),
        ];

        locations.into_iter().find(|loc| loc.exists())
    }

    /// Find user-level configuration file.
    fn find_user_config() -> Option<PathBuf> {
        if let Some(config_dir) = dirs::config_dir() {
            let rez_config = config_dir.join("rez").join("rezconfig.toml");
            if rez_config.exists() {
                return Some(rez_config);
            }
        }

        // Fallback to home directory
        if let Some(home) = dirs::home_dir() {
            let rez_config = home.join(".rez").join("rezconfig.toml");
            if rez_config.exists() {
                return Some(rez_config);
            }
        }

        None
    }

    /// Find project-level configuration file.
    fn find_project_config() -> Option<PathBuf> {
        // TODO: Walk up from current directory to find .rez/rezconfig.toml
        let current = env::current_dir().ok()?;

        let project_config = current.join(".rez").join("rezconfig.toml");
        if project_config.exists() {
            return Some(project_config);
        }

        None
    }

    /// Get all loaded configuration sources.
    pub fn sources(&self) -> &[ConfigSource] {
        &self.sources
    }

    /// Check if configuration is locked.
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Set locked state (disables environment variable overrides).
    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked;
    }

    /// Get the internal data as JSON value (for Python bindings).
    pub fn data(&self) -> &JsonValue {
        &self.data
    }
}

/// Set a flat, `snake_case` key on a JSON object (e.g. `packages_path`).
///
/// Underscores are part of the key name; only an explicit `.` in a key such as
/// the `plugins.*` family denotes nesting.
fn set_flat_value(root: &mut JsonValue, key: &str, value: JsonValue) {
    if !root.is_object() {
        *root = JsonValue::Object(serde_json::Map::new());
    }
    if let Some(obj) = root.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
}

/// Set a nested JSON value given a dotted key path (e.g. "plugins.release_vcs.git").
fn set_nested_value(root: &mut JsonValue, dotted_key: &str, value: JsonValue) {
    let segments: Vec<&str> = dotted_key.split('.').collect();
    if segments.is_empty() {
        return;
    }
    let target = get_or_create_nested(root, &segments[..segments.len() - 1]);
    if let Some(obj) = target.as_object_mut() {
        obj.insert(segments.last().unwrap().to_string(), value);
    }
}

/// Drill into a JSON object, creating intermediate objects as needed.
fn get_or_create_nested<'v>(root: &'v mut JsonValue, segments: &[&str]) -> &'v mut JsonValue {
    let mut current = root;
    for seg in segments {
        // Ensure current is an object
        if current.is_null() || !current.is_object() {
            *current = JsonValue::Object(serde_json::Map::new());
        }
        let obj = current.as_object_mut().expect("just set to object");
        current = obj
            .entry(seg.to_string())
            .or_insert(JsonValue::Object(serde_json::Map::new()));
    }
    current
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    /// Guards every test that mutates `REZ_*` environment variables: they are
    /// process-global, so such tests must not run concurrently.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert!(!config.is_locked());
        assert!(config.sources().is_empty());
    }

    #[test]
    fn test_config_get_string() {
        let mut config = Config::new();
        config.data = json!({
            "packages": {
                "path": "/usr/local/packages"
            },
            "resolution": {
                "warn_on_lib_conflict": true
            }
        });

        assert_eq!(
            config.get_string("packages.path"),
            Some("/usr/local/packages".to_string())
        );
        assert_eq!(
            config.get_bool("resolution.warn_on_lib_conflict"),
            Some(true)
        );
        assert_eq!(config.get("nonexistent"), None);
    }

    #[test]
    fn test_config_merge_value() {
        let mut config = Config::new();
        config.data = json!({
            "a": 1,
            "b": {
                "c": 2,
                "d": 3
            }
        });

        config.merge_value(json!({
            "b": {
                "c": 4,
                "e": 5
            },
            "f": 6
        }));

        assert_eq!(config.get("a"), Some(&json!(1)));
        assert_eq!(config.get("b.c"), Some(&json!(4)));
        assert_eq!(config.get("b.d"), Some(&json!(3)));
        assert_eq!(config.get("b.e"), Some(&json!(5)));
        assert_eq!(config.get("f"), Some(&json!(6)));
    }

    #[test]
    fn test_env_override_uses_flat_snake_case_key() {
        let _guard = env_lock();
        // Regression: underscores were rewritten to dots, so REZ_PACKAGES_PATH
        // landed at packages.path and no reader ever saw it.
        let mut config = Config::new();
        config.data = json!({});
        // SAFETY: single-threaded test, value restored immediately after.
        unsafe { env::set_var("REZ_PACKAGES_PATH", "/env/pkgs") };
        let result = config.apply_env_overrides();
        unsafe { env::remove_var("REZ_PACKAGES_PATH") };

        assert!(result.is_ok());
        assert_eq!(config.get_string("packages_path"), Some("/env/pkgs".into()));
        // The old dotted form must not be produced.
        assert!(config.get("packages").is_none());
    }

    #[test]
    fn test_env_json_override_overrides_plain_form() {
        let _guard = env_lock();
        let mut config = Config::new();
        config.data = json!({});
        // SAFETY: single-threaded test, values restored immediately after.
        unsafe {
            env::set_var("REZ_IMPLICIT_PACKAGES", "ignored");
            env::set_var("REZ_IMPLICIT_PACKAGES_JSON", r#"["a", "b"]"#);
        };
        let result = config.apply_env_overrides();
        unsafe {
            env::remove_var("REZ_IMPLICIT_PACKAGES");
            env::remove_var("REZ_IMPLICIT_PACKAGES_JSON");
        };

        assert!(result.is_ok());
        assert_eq!(
            config.get_array("implicit_packages"),
            Some(&vec![json!("a"), json!("b")])
        );
    }

    #[test]
    fn test_env_json_override_invalid_json_errors() {
        let _guard = env_lock();
        let mut config = Config::new();
        config.data = json!({});
        // SAFETY: single-threaded test, value restored immediately after.
        unsafe { env::set_var("REZ_PACKAGES_PATH_JSON", "[not json") };
        let result = config.apply_env_overrides();
        unsafe { env::remove_var("REZ_PACKAGES_PATH_JSON") };

        assert!(result.is_err(), "invalid JSON must not be silently ignored");
    }

    #[test]
    fn test_config_contains_key() {
        let mut config = Config::new();
        config.data = json!({
            "packages": {
                "path": "/packages"
            }
        });

        assert!(config.contains_key("packages.path"));
        assert!(!config.contains_key("nonexistent"));
    }
}
