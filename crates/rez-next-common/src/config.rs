//! Configuration management for rez-core

use crate::error::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;

/// Configuration for rez-core components
///
/// `#[serde(default)]` at container level is required: without it a user config
/// file that declares only a subset of keys fails to deserialize and is skipped,
/// silently discarding every setting the user actually wrote.
/// Not exhaustively constructible: modelling a new upstream setting adds a
/// field, and external struct literals would break on every addition.
/// Construct via [`RezCoreConfig::default`] and assign fields instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
#[allow(clippy::struct_excessive_bools)]
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

    /// Packages implicitly added to every resolve (`implicit_packages`)
    ///
    /// Deliberate divergence: upstream rez 3.4.0 defaults this to
    /// `["~platform=={system.platform}", "~arch=={system.arch}", "~os=={system.os}"]`,
    /// but those are implicit-package request strings that rez-next cannot
    /// parse yet. Defaulting to empty keeps the setting inert rather than
    /// feeding unparseable requests into the solver; align with upstream once
    /// that syntax is supported.
    pub implicit_packages: Vec<String>,

    /// Environment variables treated as path-like and joined rather than
    /// replaced (`parent_variables`)
    pub parent_variables: Vec<String>,

    /// Environment variables replaced rather than appended to
    /// (`resetting_variables`)
    pub resetting_variables: Vec<String>,

    /// Per-variable separator used when appending to a parent variable
    /// (`env_var_separators`)
    pub env_var_separators: BTreeMap<String, String>,

    /// Platform name overrides (`platform_map`)
    pub platform_map: BTreeMap<String, String>,

    /// Whether newly built packages are relocatable (`default_relocatable`)
    pub default_relocatable: bool,

    /// Directory packages are built in (`build_directory`)
    pub build_directory: String,

    /// Directory contexts are written to (`context_tmpdir`)
    pub context_tmpdir: String,
}

/// Nested keys accepted by [`CacheConfig`], used to reject unknown overrides
/// rather than silently ignoring them.
const CACHE_KEYS: [&str; 4] = [
    "enable_memory_cache",
    "enable_disk_cache",
    "memory_cache_size",
    "cache_ttl_seconds",
];

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

impl RezCoreConfig {
    #[must_use]
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
            // Upstream rez 3.4.0 defaults for the settings that were inert,
            // except implicit_packages - see the note on that field.
            implicit_packages: vec![],
            parent_variables: vec![],
            resetting_variables: vec![],
            env_var_separators: default_env_var_separators(),
            platform_map: BTreeMap::new(),
            default_relocatable: true,
            build_directory: "build".to_string(),
            context_tmpdir: std::env::temp_dir().to_string_lossy().to_string(),
        }
    }
}

/// rez 3.4.0's `env_var_separators` default (src/rez/rezconfig.py:528).
///
/// Path-style variables are *not* listed here: upstream falls back to
/// `os.pathsep` for any variable absent from this map, so only the two
/// overrides need declaring.
fn default_env_var_separators() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("CMAKE_MODULE_PATH".to_string(), ";".to_string()),
        ("DOXYGEN_TAGFILES".to_string(), " ".to_string()),
    ])
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
    #[must_use]
    pub fn get_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // 1. Built-in config (if exists)
        if let Ok(exe_path) = env::current_exe()
            && let Some(exe_dir) = exe_path.parent()
        {
            paths.push(exe_dir.join("rezconfig.yaml"));
            paths.push(exe_dir.join("rezconfig.json"));
        }

        // 2. Environment variable REZ_CONFIG_FILE
        if let Ok(config_file) = env::var("REZ_CONFIG_FILE") {
            for path in config_file.split(path_list_separator()) {
                paths.push(PathBuf::from(path));
            }
        }

        // 3. System-wide config
        if cfg!(unix) {
            paths.push(PathBuf::from("/etc/rez/config.yaml"));
            paths.push(PathBuf::from("/usr/local/etc/rez/config.yaml"));
        } else if cfg!(windows)
            && let Ok(program_data) = env::var("PROGRAMDATA")
        {
            paths.push(PathBuf::from(program_data).join("rez").join("config.yaml"));
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
            } else if cfg!(windows)
                && let Ok(userprofile) = env::var("USERPROFILE")
            {
                let user_path = PathBuf::from(&userprofile);
                paths.push(user_path.join(".rezconfig"));
                paths.push(user_path.join(".rezconfig.yaml"));
                paths.push(user_path.join(".rez").join("config.yaml"));
            }
        }

        paths
    }

    /// Get the list of configuration files that actually exist and are sourced
    #[must_use]
    pub fn get_sourced_paths() -> Vec<PathBuf> {
        Self::get_search_paths()
            .into_iter()
            .filter(|path| path.exists())
            .collect()
    }

    /// Report configuration keys that rez-next does not know about.
    ///
    /// The Rust binary previously accepted a config file, deserialized only the
    /// 19 fields it models, and dropped every other key without a word — so a
    /// studio migrating an upstream `rezconfig.py` saw most of it silently
    /// ignored. Validating against the shared key registry turns that silence
    /// into a warning that names the key.
    ///
    /// Unknown keys are reported but never fatal: a key rez-next has yet to
    /// implement should degrade to a warning, not break the whole config.
    /// Returns the keys that are absent from the shared key registry, so the
    /// caller can both report them and be asserted against in tests.
    fn unknown_keys(merged: &serde_json::Map<String, serde_json::Value>) -> Vec<String> {
        merged
            .keys()
            .filter(|key| rez_next_config::keys::find(key).is_none())
            .cloned()
            .collect()
    }

    /// Load configuration from files, reporting any parse failure.
    ///
    /// Configuration layers are applied in [`Self::get_search_paths`] order and
    /// **deep-merged** rather than replaced: a file declaring only
    /// `packages_path` leaves every other setting untouched. Within a merge,
    /// objects are merged recursively while scalars and arrays replace the
    /// previous value wholesale — the same rule rez applies to its own config
    /// layers.
    ///
    /// A file that exists but cannot be parsed is an **error**, not a reason to
    /// fall back to defaults: silently ignoring it would discard user settings
    /// without any signal.
    pub fn try_load() -> Result<Self, RezCoreError> {
        let mut merged = serde_json::Map::new();

        for path in Self::get_search_paths() {
            if !path.exists() {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(RezCoreError::Io)?;
            let layer = parse_config_layer(&path, &content)?;
            deep_merge_json_object(&mut merged, layer);
        }

        for key in Self::unknown_keys(&merged) {
            eprintln!("rez-next: ignoring unknown config key {key:?} (not in the key registry)");
        }

        let mut config: Self = serde_json::from_value(serde_json::Value::Object(merged))?;
        config.apply_env_overrides_and_report();
        Ok(config)
    }

    /// Load configuration from files (reads actual rezconfig files).
    ///
    /// Malformed configuration is reported on stderr rather than swallowed; see
    /// [`Self::try_load`] for the fallible variant.
    #[must_use]
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(config) => config,
            Err(e) => {
                eprintln!("rez-next: failed to load configuration: {e}");
                let mut config = Self::default();
                config.apply_env_overrides_and_report();
                config
            }
        }
    }

    /// Apply env overrides, printing any rejected override to stderr.
    ///
    /// A malformed override is a user-visible mistake and must not pass
    /// silently: rez raises `ConfigurationError` for the same condition
    /// (`src/rez/config.py`), and the config-file side of this loader already
    /// errors rather than falling back to defaults.
    fn apply_env_overrides_and_report(&mut self) {
        for message in self.apply_env_overrides() {
            eprintln!("rez-next: ignoring {message}");
        }
    }

    /// Apply `REZ_*` environment variable overrides in place.
    ///
    /// Implements rez's override rules (`src/rez/config.py`): a present
    /// `REZ_<KEY>` wins, including an empty value. `REZ_<KEY>_JSON` is parsed
    /// only when the plain variable is absent. Keys are **flat snake_case** — the
    /// underscores in a key such as `packages_path` are preserved, they do not
    /// denote nesting.
    ///
    /// Returns the list of overrides that could not be applied. A malformed
    /// override is **reported, never swallowed**: the field keeps the value it
    /// already had, so a bad environment variable cannot silently erase a
    /// setting the user wrote in a config file.
    fn apply_env_overrides(&mut self) -> Vec<String> {
        let mut rejected = Vec::new();
        // Collect all `REZ_*` variables. Lower-casing the suffix
        // yields the flat snake_case field name; underscores are preserved,
        // matching rez (`src/rez/config.py`) rather than implying nesting.
        let overrides: Vec<(String, String)> = env::vars()
            .filter_map(|(key, value)| {
                key.strip_prefix("REZ_")
                    .map(|suffix| (suffix.to_ascii_lowercase(), value))
            })
            .filter(|(suffix, _)| !suffix.is_empty())
            .collect();

        // Match Rez's presence check, not a truthiness check: even an empty
        // plain value masks its JSON counterpart. Do not parse masked JSON.
        let mut json_overrides: Vec<(String, serde_json::Value)> = Vec::new();
        for (suffix, raw_value) in &overrides {
            let Some(field) = suffix.strip_suffix("_json").filter(|f| !f.is_empty()) else {
                continue;
            };
            if overrides.iter().any(|(name, _)| name == field) {
                continue;
            }
            match serde_json::from_str::<serde_json::Value>(raw_value) {
                Ok(json) => json_overrides.push((field.to_string(), json)),
                Err(e) => rejected.push(format!(
                    "REZ_{}: invalid JSON ({e})",
                    suffix.to_ascii_uppercase()
                )),
            }
        }

        // Plain `REZ_<KEY>` overrides always take precedence.
        for (suffix, raw_value) in &overrides {
            if suffix.ends_with("_json") || suffix.is_empty() {
                continue;
            }
            if let Err(e) = self.apply_env_field(suffix, coerce_env_value(suffix, raw_value)) {
                rejected.push(format!("REZ_{}: {e}", suffix.to_ascii_uppercase()));
            }
        }

        // These fields have no plain counterpart, so JSON is the fallback.
        for (field, json) in json_overrides {
            if let Err(e) = self.apply_env_field(&field, json) {
                rejected.push(format!("REZ_{}_JSON: {e}", field.to_ascii_uppercase()));
            }
        }

        rejected
    }

    /// Apply a single parsed override to the matching field.
    ///
    /// Returns `Err` with a human-readable message when the value does not fit
    /// the field. The field is left untouched in that case, so a bad override
    /// can never erase a value that came from a config file.
    fn apply_env_field(&mut self, key: &str, value: serde_json::Value) -> Result<(), String> {
        match key {
            "use_rust_version" => assign(&mut self.use_rust_version, &value),
            "use_rust_solver" => assign(&mut self.use_rust_solver, &value),
            "use_rust_repository" => assign(&mut self.use_rust_repository, &value),
            "rust_fallback" => assign(&mut self.rust_fallback, &value),
            "thread_count" => assign(&mut self.thread_count, &value),
            "packages_path" => assign_path_list(&mut self.packages_path, &value),
            "local_packages_path" => assign(&mut self.local_packages_path, &value),
            "release_packages_path" => assign(&mut self.release_packages_path, &value),
            "default_shell" => assign(&mut self.default_shell, &value),
            "version" => assign(&mut self.version, &value),
            "plugin_path" => assign_path_list(&mut self.plugin_path, &value),
            "package_cache_path" => assign_path_list(&mut self.package_cache_path, &value),
            "tmpdir" => assign(&mut self.tmpdir, &value),
            "editor" => assign(&mut self.editor, &value),
            "image_viewer" => assign(&mut self.image_viewer, &value),
            "browser" => assign(&mut self.browser, &value),
            "difftool" => assign(&mut self.difftool, &value),
            "terminal_emulator_command" => assign(&mut self.terminal_emulator_command, &value),
            // Settings that were declared but never read; now overridable too.
            "implicit_packages" => assign_str_list(&mut self.implicit_packages, &value),
            "parent_variables" => assign_str_list(&mut self.parent_variables, &value),
            "resetting_variables" => assign_str_list(&mut self.resetting_variables, &value),
            "env_var_separators" => assign(&mut self.env_var_separators, &value),
            "platform_map" => assign(&mut self.platform_map, &value),
            "default_relocatable" => assign(&mut self.default_relocatable, &value),
            "build_directory" => assign(&mut self.build_directory, &value),
            "context_tmpdir" => assign(&mut self.context_tmpdir, &value),
            // Nested cache.* keys: merge into the existing cache so sibling
            // keys set by a config file survive a partial env object.
            "cache" => assign_cache(&mut self.cache, &value),
            other => {
                // An override for a key rez-next does not model is as invisible
                // as one in a config file, so report it the same way rather
                // than letting REZ_<KEY> fail silently.
                if rez_next_config::keys::find(other).is_none() {
                    eprintln!(
                        "rez-next: ignoring unknown config key {other:?} \
                         (not in the key registry)"
                    );
                }
                Ok(())
            }
        }
        .map_err(|e| format!("{key}: {e}"))
    }

    /// Get a configuration field by dot-separated path
    #[must_use]
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

fn split_env_paths(paths: &str) -> Vec<String> {
    paths
        .split(path_list_separator())
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Split a plain environment value for a rez `StrList` setting.
///
/// Upstream rez replaces commas with spaces, then uses Python's `str.split()`;
/// only `PathList` splits on `os.pathsep`. Keep `:` and `;` inside each element.
#[must_use]
fn split_env_str_list(value: &str) -> Vec<String> {
    // Python also treats the four C0 information separators as whitespace.
    value
        .split(|ch: char| ch == ',' || ch.is_whitespace() || matches!(ch, '\u{1c}'..='\u{1f}'))
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Platform path-list separator, matching rez's use of `os.pathsep`.
#[must_use]
pub const fn path_list_separator() -> char {
    if cfg!(windows) { ';' } else { ':' }
}

/// Parse a single config file body (YAML or JSON) into a JSON object.
fn parse_config_layer(
    path: &std::path::Path,
    content: &str,
) -> Result<serde_json::Value, RezCoreError> {
    let display = path.display().to_string();

    // YAML is a superset of JSON, so one parse handles both formats.
    let value = serde_yaml::from_str::<serde_json::Value>(content)
        .map_err(|e| RezCoreError::ConfigError(format!("{display}: {e}")))?;

    if value.is_null() {
        // An empty file is valid and simply contributes nothing.
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }

    if !value.is_object() {
        return Err(RezCoreError::ConfigError(format!(
            "{display}: expected a mapping at the top level, got {}",
            value_kind_name(&value)
        )));
    }

    Ok(value)
}

/// Deep-merge `patch` into `base`: objects merge recursively, everything else
/// (scalars and arrays) replaces the previous value.
fn deep_merge_json_object(
    base: &mut serde_json::Map<String, serde_json::Value>,
    patch: serde_json::Value,
) {
    let Some(patch_obj) = patch.as_object() else {
        return;
    };

    for (key, patch_value) in patch_obj {
        match base.get_mut(key) {
            Some(base_value) if base_value.is_object() && patch_value.is_object() => {
                let Some(base_obj) = base_value.as_object_mut() else {
                    continue;
                };
                deep_merge_json_object(base_obj, patch_value.clone());
            }
            _ => {
                base.insert(key.clone(), patch_value.clone());
            }
        }
    }
}

/// Human-readable name for a JSON value kind, used in error messages.
fn value_kind_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "mapping",
    }
}

/// Coerce a plain (non-JSON) environment value to the target field's type.
///
/// Booleans accept rez's `0`/`1`/`true`/`false`; path-list fields are split on
/// the platform path separator; everything else stays a string.
fn coerce_env_value(key: &str, raw: &str) -> serde_json::Value {
    match key {
        "packages_path" | "plugin_path" | "package_cache_path" => serde_json::Value::Array(
            split_env_paths(raw)
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
        // These are rez's `StrList`, not `PathList`: upstream separates them
        // with commas and whitespace; only `PathList` splits on `os.pathsep`.
        // Package requests may contain `:`, which path splitting would break apart.
        "implicit_packages" | "parent_variables" | "resetting_variables" => {
            serde_json::Value::Array(
                split_env_str_list(raw)
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            )
        }
        "default_relocatable" => parse_env_bool(raw),
        "use_rust_version" | "use_rust_solver" | "use_rust_repository" | "rust_fallback" => {
            parse_env_bool(raw)
        }
        // An unparsable count stays a string so `assign` rejects it. Folding it
        // into `Null` would deserialize successfully as `None`, silently
        // clearing a `thread_count` the user set in a config file.
        "thread_count" => raw.parse::<usize>().map_or_else(
            |_| serde_json::Value::String(raw.to_string()),
            serde_json::Value::from,
        ),
        _ => serde_json::Value::String(raw.to_string()),
    }
}

/// Parse a boolean environment value, accepting the forms rez accepts.
fn parse_env_bool(raw: &str) -> serde_json::Value {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" | "t" => serde_json::Value::Bool(true),
        "0" | "false" | "no" | "off" | "f" => serde_json::Value::Bool(false),
        _ => serde_json::Value::String(raw.to_string()),
    }
}

/// Assign `value` to `target` when the value is compatible with the field type.
///
/// Returns `Err` when the value cannot be assigned, so the caller can report a
/// malformed override instead of silently keeping (or worse, clearing) the
/// value that came from a config file.
fn assign<T: serde::de::DeserializeOwned>(
    target: &mut T,
    value: &serde_json::Value,
) -> Result<(), String> {
    let parsed = serde_json::from_value::<T>(value.clone())
        .map_err(|e| format!("invalid {} ({e})", value_kind_name(value)))?;
    *target = parsed;
    Ok(())
}

/// Assign a plain string list, honouring the registry's `StrList` type.
///
/// `assign_path_list` splits bare strings on `os.pathsep`, which is correct for
/// `packages_path` but wrong for lists whose elements may legitimately contain
/// a separator. Implicit-package requests (`~os=={system.os}`) and unquoted
/// variant syntax can contain `:`, and `implicit_packages` /
/// `parent_variables` / `resetting_variables` are all registered as `StrList`,
/// not path lists. A bare string for those is treated as a single element.
fn assign_str_list(target: &mut Vec<String>, value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Array(items) => {
            let entries: Vec<String> = items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect();
            if entries.len() != items.len() {
                return Err("expected every list entry to be a string".to_string());
            }
            *target = entries;
            Ok(())
        }
        serde_json::Value::String(raw) => {
            *target = vec![raw.clone()];
            Ok(())
        }
        other => Err(format!(
            "expected a string list, got {}",
            value_kind_name(other)
        )),
    }
}

/// Assign an override to the nested `cache` object, merging it into the
/// existing value.
///
/// A partial object such as `{"memory_cache_size": 999}` must leave the sibling
/// keys untouched. Deserializing straight into `CacheConfig` would instead let
/// `#[serde(default)]` fill every unlisted key with the *struct* default,
/// discarding what a config file had set. Merging first keeps the file's value
/// for any key the override does not name.
fn assign_cache(target: &mut CacheConfig, value: &serde_json::Value) -> Result<(), String> {
    let existing = serde_json::to_value(&*target)
        .map_err(|e| format!("could not read current cache config ({e})"))?;
    let mut merged = existing;

    match value {
        serde_json::Value::Object(patch) => {
            let Some(base) = merged.as_object_mut() else {
                return Err("existing cache config is not an object".to_string());
            };
            for (key, patch_value) in patch {
                if !base.contains_key(key) {
                    return Err(format!(
                        "unknown cache key '{key}'; expected one of {}",
                        CACHE_KEYS.join(", ")
                    ));
                }
                base.insert(key.clone(), patch_value.clone());
            }
        }
        // A non-object replaces the whole block, mirroring scalar assignment.
        other => merged = other.clone(),
    }

    *target = serde_json::from_value::<CacheConfig>(merged)
        .map_err(|e| format!("invalid cache override ({e})"))?;
    Ok(())
}

/// Assign a value to a path-list field, accepting either a JSON array or a
/// separator-delimited string.
fn assign_path_list(target: &mut Vec<String>, value: &serde_json::Value) -> Result<(), String> {
    match value {
        serde_json::Value::Array(items) => {
            let paths: Vec<String> = items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect();
            if paths.len() != items.len() {
                return Err("expected every path entry to be a string".to_string());
            }
            *target = paths;
            Ok(())
        }
        serde_json::Value::String(raw) => {
            *target = split_env_paths(raw);
            Ok(())
        }
        other => Err(format!(
            "expected a path list, got {}",
            value_kind_name(other)
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Guards every test that mutates `REZ_*` environment variables. These are
    /// process-global, so tests that set them must not run concurrently.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Run `body` with `REZ_CONFIG_FILE` pointing at `files`.
    fn with_config_files<F: FnOnce()>(files: &[&std::path::Path], body: F) {
        let _guard = env_lock();

        let joined = files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(&path_list_separator().to_string());

        // SAFETY: guarded by ENV_LOCK; no other test reads these vars concurrently.
        unsafe { env::set_var("REZ_CONFIG_FILE", &joined) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        unsafe { env::remove_var("REZ_CONFIG_FILE") };

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
    }

    fn write_config(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, body).expect("write config fixture");
        path
    }

    // --- P2 regressions: malformed / partial env overrides must not silently
    // --- clobber values that came from config files.

    #[test]
    fn test_malformed_env_override_preserves_config_file_value() {
        let _guard = env_lock();

        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "c.yaml", "thread_count: 8\n");

        // SAFETY: guarded by ENV_LOCK.
        unsafe {
            env::set_var("REZ_CONFIG_FILE", file.to_string_lossy().to_string());
            env::set_var("REZ_THREAD_COUNT", "abc");
        };
        let cfg = RezCoreConfig::try_load().expect("load");
        unsafe {
            env::remove_var("REZ_THREAD_COUNT");
            env::remove_var("REZ_CONFIG_FILE");
        };

        assert_eq!(
            cfg.thread_count,
            Some(8),
            "an unparsable REZ_THREAD_COUNT must not erase the file value"
        );
    }

    #[test]
    fn test_malformed_env_override_reports_instead_of_silently_ignoring() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_USE_RUST_SOLVER", "maybe") };
        let mut cfg = RezCoreConfig::default();
        let warnings = cfg.apply_env_overrides();
        unsafe { env::remove_var("REZ_USE_RUST_SOLVER") };

        assert!(
            warnings.iter().any(|m| m.contains("REZ_USE_RUST_SOLVER")),
            "malformed env override must be reported, got: {warnings:?}"
        );
    }

    #[test]
    fn test_env_cache_json_merges_into_config_file_cache() {
        let _guard = env_lock();

        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(
            dir.path(),
            "c.yaml",
            "cache:\n  enable_disk_cache: false\n  memory_cache_size: 500\n",
        );

        // SAFETY: guarded by ENV_LOCK.
        unsafe {
            env::set_var("REZ_CONFIG_FILE", file.to_string_lossy().to_string());
            env::set_var("REZ_CACHE_JSON", r#"{"memory_cache_size": 999}"#);
        };
        let cfg = RezCoreConfig::try_load().expect("load");
        unsafe {
            env::remove_var("REZ_CACHE_JSON");
            env::remove_var("REZ_CONFIG_FILE");
        };

        assert_eq!(cfg.cache.memory_cache_size, 999, "env value must apply");
        assert!(
            !cfg.cache.enable_disk_cache,
            "sibling cache keys from the file must survive a partial env object"
        );
    }

    #[test]
    fn test_partial_config_file_is_not_discarded() {
        // Regression: a file declaring a single key used to fail deserialization
        // and be silently skipped, so the user's setting never took effect.
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(
            dir.path(),
            "rezconfig.yaml",
            "packages_path:\n  - /custom/pkgs\n",
        );

        with_config_files(&[&file], || {
            let cfg = RezCoreConfig::try_load().expect("partial config must load");
            assert_eq!(cfg.packages_path, vec!["/custom/pkgs".to_string()]);
            // Untouched keys keep their defaults instead of being dropped.
            assert!(cfg.use_rust_solver);
            assert_eq!(cfg.local_packages_path, "~/packages");
        });
    }

    #[test]
    fn test_config_layers_deep_merge_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lower = write_config(
            dir.path(),
            "lower.yaml",
            "packages_path:\n  - /lower\nlocal_packages_path: /lower-local\ncache:\n  memory_cache_size: 111\n  enable_disk_cache: false\n",
        );
        let upper = write_config(
            dir.path(),
            "upper.yaml",
            "packages_path:\n  - /upper\ncache:\n  memory_cache_size: 222\n",
        );

        with_config_files(&[&lower, &upper], || {
            let cfg = RezCoreConfig::try_load().expect("layered config must load");
            // Arrays replace wholesale rather than concatenating.
            assert_eq!(cfg.packages_path, vec!["/upper".to_string()]);
            // Keys absent from the upper layer survive from the lower one.
            assert_eq!(cfg.local_packages_path, "/lower-local");
            // Nested objects merge key by key, not by whole-object replacement.
            assert_eq!(cfg.cache.memory_cache_size, 222);
            assert!(!cfg.cache.enable_disk_cache);
        });
    }

    #[test]
    fn test_multiple_entries_in_rez_config_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = write_config(dir.path(), "a.yaml", "editor: /bin/first\n");
        let second = write_config(dir.path(), "b.yaml", "browser: /bin/second\n");

        with_config_files(&[&first, &second], || {
            let cfg = RezCoreConfig::try_load().expect("multi-file config must load");
            assert_eq!(cfg.editor, "/bin/first");
            assert_eq!(cfg.browser, "/bin/second");
        });
    }

    #[test]
    fn test_malformed_config_reports_error_instead_of_defaults() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "bad.yaml", "packages_path: [unterminated\n");

        with_config_files(&[&file], || {
            let err = RezCoreConfig::try_load().expect_err("malformed config must error");
            let msg = err.to_string();
            assert!(
                msg.contains("bad.yaml"),
                "error must name the offending file, got: {msg}"
            );
        });
    }

    #[test]
    fn test_non_mapping_config_reports_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "list.yaml", "- just\n- a\n- list\n");

        with_config_files(&[&file], || {
            let err = RezCoreConfig::try_load().expect_err("non-mapping config must error");
            assert!(err.to_string().contains("list.yaml"));
        });
    }

    #[test]
    fn test_empty_config_file_is_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "empty.yaml", "");

        with_config_files(&[&file], || {
            let cfg = RezCoreConfig::try_load().expect("empty config must load");
            assert_eq!(cfg.packages_path, RezCoreConfig::default().packages_path);
        });
    }

    #[test]
    fn test_snake_case_env_override_applies() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_LOCAL_PACKAGES_PATH", "/env/local") };
        let cfg = RezCoreConfig::load();
        unsafe { env::remove_var("REZ_LOCAL_PACKAGES_PATH") };

        assert_eq!(cfg.local_packages_path, "/env/local");
    }

    #[test]
    fn test_env_override_wins_over_config_file() {
        let _guard = env_lock();

        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "c.yaml", "local_packages_path: /from-file\n");

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_CONFIG_FILE", file.to_string_lossy().to_string()) };
        unsafe { env::set_var("REZ_LOCAL_PACKAGES_PATH", "/from-env") };
        let cfg = RezCoreConfig::try_load().expect("load");
        unsafe {
            env::remove_var("REZ_LOCAL_PACKAGES_PATH");
            env::remove_var("REZ_CONFIG_FILE");
        }

        assert_eq!(cfg.local_packages_path, "/from-env");
    }

    #[test]
    fn test_env_json_override_parses_structured_values() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_PACKAGES_PATH_JSON", r#"["/a","/b"]"#) };
        let cfg = RezCoreConfig::load();
        unsafe { env::remove_var("REZ_PACKAGES_PATH_JSON") };

        assert_eq!(cfg.packages_path, vec!["/a".to_string(), "/b".to_string()]);
    }

    #[test]
    fn test_env_bool_override_accepts_rez_forms() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_USE_RUST_SOLVER", "0") };
        let cfg = RezCoreConfig::load();
        unsafe { env::remove_var("REZ_USE_RUST_SOLVER") };

        assert!(!cfg.use_rust_solver);
    }

    #[test]
    fn test_path_list_separator_matches_platform() {
        let expected = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(path_list_separator(), expected);
    }

    #[test]
    fn test_deep_merge_replaces_arrays_wholesale() {
        let mut base = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
            r#"{"a": [1, 2, 3]}"#,
        )
        .expect("base");
        deep_merge_json_object(&mut base, serde_json::json!({"a": [9]}));
        assert_eq!(base["a"], serde_json::json!([9]));
    }

    #[test]
    fn test_deep_merge_merges_nested_objects() {
        let mut base = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
            r#"{"o": {"x": 1, "y": 2}}"#,
        )
        .expect("base");
        deep_merge_json_object(&mut base, serde_json::json!({"o": {"y": 3, "z": 4}}));
        assert_eq!(base["o"]["x"], serde_json::json!(1));
        assert_eq!(base["o"]["y"], serde_json::json!(3));
        assert_eq!(base["o"]["z"], serde_json::json!(4));
    }

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
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK; restored before the guard drops.
        unsafe {
            let separator = path_list_separator();
            env::set_var(
                "REZ_PACKAGES_PATH",
                format!("/tmp/test_pkgs{separator}/tmp/other_pkgs"),
            );
        };
        let cfg = RezCoreConfig::load();
        unsafe {
            env::remove_var("REZ_PACKAGES_PATH");
        };
        assert!(cfg.packages_path.contains(&"/tmp/test_pkgs".to_string()));
    }

    /// `implicit_packages`, `parent_variables` and `resetting_variables` are
    /// registered as `StrList`, not path lists, so a bare string must stay a
    /// single element.
    ///
    /// The token carries both separators on purpose. `split_env_paths` splits on
    /// `;`, which only Windows uses; it splits on `:`, which only unix uses. A
    /// token containing just one of them is invisible to the other platform's
    /// lane, so this test would go green on Windows even with the bug present.
    #[test]
    fn test_str_list_settings_are_not_path_split() {
        for (key, field) in [
            ("implicit_packages", "implicit_packages"),
            ("parent_variables", "parent_variables"),
            ("resetting_variables", "resetting_variables"),
        ] {
            let mut cfg = RezCoreConfig::new();
            cfg.apply_env_field(key, serde_json::json!("python:3.9;maya"))
                .unwrap_or_else(|e| panic!("{key} must accept a string list ({e})"));

            let value = match field {
                "implicit_packages" => &cfg.implicit_packages,
                "parent_variables" => &cfg.parent_variables,
                _ => &cfg.resetting_variables,
            };
            assert_eq!(
                value,
                &vec!["python:3.9;maya".to_string()],
                "{key} must keep the whole request as one element"
            );
        }
    }

    /// The environment channel is a separate code path from config files: plain
    /// `REZ_*` values are coerced before `assign_str_list` ever sees them, so a
    /// fix confined to the assignment helper leaves this reachable route broken.
    #[test]
    fn test_env_str_list_uses_comma_not_path_separator() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK; removed before the guard drops.
        unsafe {
            env::set_var("REZ_IMPLICIT_PACKAGES", "python:3.9,maya;2024\thoudini-20");
        }
        let cfg = RezCoreConfig::load();
        unsafe {
            env::remove_var("REZ_IMPLICIT_PACKAGES");
        }

        // Commas and whitespace split values; literal path separators do not.
        assert_eq!(
            cfg.implicit_packages,
            vec![
                "python:3.9".to_string(),
                "maya;2024".to_string(),
                "houdini-20".to_string()
            ]
        );
    }

    #[test]
    fn test_env_str_list_single_value_is_not_split() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK; removed before the guard drops.
        unsafe { env::set_var("REZ_IMPLICIT_PACKAGES", "~os=={system.os}") };
        let cfg = RezCoreConfig::load();
        unsafe { env::remove_var("REZ_IMPLICIT_PACKAGES") };

        assert_eq!(cfg.implicit_packages, vec!["~os=={system.os}".to_string()]);
    }

    #[test]
    fn test_str_list_settings_accept_yaml_arrays() {
        let mut cfg = RezCoreConfig::new();
        cfg.apply_env_field(
            "implicit_packages",
            serde_json::json!(["python:3.9", "~os=={system.os}"]),
        )
        .expect("arrays must load");

        assert_eq!(
            cfg.implicit_packages,
            vec!["python:3.9".to_string(), "~os=={system.os}".to_string()]
        );
    }

    #[test]
    fn test_split_env_paths_uses_platform_separator() {
        let separator = if cfg!(windows) { ';' } else { ':' };
        let result = split_env_paths(&format!("first{separator}second"));
        assert_eq!(result, vec!["first".to_string(), "second".to_string()]);
    }

    /// Upstream rez's `StrList` separates on commas and whitespace. Path
    /// separators stay inside the values, on every platform.
    #[test]
    fn test_split_env_str_list_uses_upstream_separators() {
        assert_eq!(
            split_env_str_list("python:3.9,maya-2024"),
            vec!["python:3.9".to_string(), "maya-2024".to_string()]
        );
        assert_eq!(
            split_env_str_list("first;second"),
            vec!["first;second".to_string()],
            "semicolons are not a StrList separator, even on unix"
        );
        assert_eq!(
            split_env_str_list("~os=={system.os}"),
            vec!["~os=={system.os}".to_string()]
        );
        assert!(split_env_str_list(" , , ").is_empty());
        assert_eq!(
            split_env_str_list(" python:3.9\tmaya;2024,houdini-20\n~os=={system.os} "),
            vec!["python:3.9", "maya;2024", "houdini-20", "~os=={system.os}"]
        );
        assert_eq!(
            split_env_str_list("first\u{a0}second\u{1c}third"),
            vec!["first", "second", "third"]
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_split_env_paths_preserves_windows_drive_letters() {
        let result = split_env_paths(r"C:\packages\rez;D:\local\packages");
        assert_eq!(
            result,
            vec![
                r"C:\packages\rez".to_string(),
                r"D:\local\packages".to_string()
            ]
        );
    }

    #[test]
    fn test_registry_backed_keys_are_accepted_not_dropped() {
        // Regression for #368: keys that rez defines and the registry knows
        // used to be deserialized away silently, so a migrated rezconfig.py
        // appeared to load while most of it was discarded.
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(
            dir.path(),
            "rezconfig.yaml",
            "implicit_packages:\n  - python\n\
             parent_variables:\n  - PYTHONPATH\n\
             resetting_variables:\n  - REZ_USED\n\
             default_relocatable: true\n\
             build_directory: _build\n\
             context_tmpdir: /tmp/ctx\n\
             platform_map:\n  centos: linux\n\
             env_var_separators:\n  MYVAR: \";\"\n",
        );

        with_config_files(&[&file], || {
            let cfg = RezCoreConfig::try_load().expect("registry-backed config must load");
            assert_eq!(cfg.implicit_packages, vec!["python".to_string()]);
            assert_eq!(cfg.parent_variables, vec!["PYTHONPATH".to_string()]);
            assert_eq!(cfg.resetting_variables, vec!["REZ_USED".to_string()]);
            assert!(cfg.default_relocatable);
            assert_eq!(cfg.build_directory, "_build");
            assert_eq!(cfg.context_tmpdir, "/tmp/ctx");
            assert_eq!(
                cfg.platform_map.get("centos").map(String::as_str),
                Some("linux")
            );
            assert_eq!(
                cfg.env_var_separators.get("MYVAR").map(String::as_str),
                Some(";")
            );
        });
    }

    #[test]
    fn test_previously_inert_settings_have_upstream_defaults() {
        let cfg = RezCoreConfig::default();
        // implicit_packages stays empty by design; see the field's doc comment.
        assert!(cfg.implicit_packages.is_empty());
        assert!(cfg.parent_variables.is_empty());
        assert!(cfg.resetting_variables.is_empty());
        assert!(cfg.platform_map.is_empty());
        assert!(cfg.default_relocatable);
        assert_eq!(cfg.build_directory, "build");
        assert_eq!(
            cfg.context_tmpdir,
            std::env::temp_dir().to_string_lossy().to_string()
        );

        // rez 3.4.0 declares exactly these two separator overrides; every other
        // variable falls back to os.pathsep at the shell layer.
        assert_eq!(cfg.env_var_separators.len(), 2);
        assert_eq!(
            cfg.env_var_separators
                .get("CMAKE_MODULE_PATH")
                .map(String::as_str),
            Some(";")
        );
        assert_eq!(
            cfg.env_var_separators
                .get("DOXYGEN_TAGFILES")
                .map(String::as_str),
            Some(" ")
        );
    }

    #[test]
    fn test_env_overrides_apply_to_previously_inert_settings() {
        let _guard = env_lock();

        // `implicit_packages` is a rez `StrList`, which upstream separates with
        // a comma; only `PathList` splits on `os.pathsep`.
        // SAFETY: guarded by ENV_LOCK; removed before the guard drops.
        unsafe {
            env::set_var("REZ_IMPLICIT_PACKAGES", "python,maya");
            env::set_var("REZ_DEFAULT_RELOCATABLE", "1");
            env::set_var("REZ_BUILD_DIRECTORY", "_builddir");
        }
        let cfg = RezCoreConfig::load();
        unsafe {
            env::remove_var("REZ_IMPLICIT_PACKAGES");
            env::remove_var("REZ_DEFAULT_RELOCATABLE");
            env::remove_var("REZ_BUILD_DIRECTORY");
        }

        assert_eq!(cfg.implicit_packages, vec!["python", "maya"]);
        assert!(cfg.default_relocatable);
        assert_eq!(cfg.build_directory, "_builddir");
    }

    #[test]
    fn test_env_var_separators_accept_json_object() {
        let _guard = env_lock();

        // SAFETY: guarded by ENV_LOCK.
        unsafe { env::set_var("REZ_ENV_VAR_SEPARATORS_JSON", r#"{"MYVAR": "|"}"#) };
        let cfg = RezCoreConfig::load();
        unsafe { env::remove_var("REZ_ENV_VAR_SEPARATORS_JSON") };

        assert_eq!(
            cfg.env_var_separators.get("MYVAR").map(String::as_str),
            Some("|")
        );
    }

    #[test]
    fn test_unknown_key_is_reported_not_silently_dropped() {
        // The registry is the shared contract: a key neither rez nor rez-next
        // defines must not vanish without a word.
        let dir = tempfile::tempdir().expect("tempdir");
        let file = write_config(dir.path(), "c.yaml", "totally_made_up_key: 1\n");

        with_config_files(&[&file], || {
            // Unknown keys warn but never fail the load.
            let cfg = RezCoreConfig::try_load().expect("unknown key must not be fatal");
            assert!(!cfg.packages_path.is_empty());
        });
    }

    #[test]
    fn test_unknown_keys_are_identified_distinctly_from_known_ones() {
        // Guards the reporting itself: this fails if unknown key detection is
        // removed, not merely if the load breaks.
        let mut merged = serde_json::Map::new();
        merged.insert("packages_path".to_string(), serde_json::json!(["/tmp"]));
        merged.insert("default_shell".to_string(), serde_json::json!("bash"));
        merged.insert("totally_made_up_key".to_string(), serde_json::json!(1));
        merged.insert("another_unknown".to_string(), serde_json::json!(2));

        let mut unknown = RezCoreConfig::unknown_keys(&merged);
        // The report order follows the merged map's iteration order. That is
        // already sorted today because `serde_json::Map` is a `BTreeMap`, so
        // this sort is currently a no-op; it is kept so the assertion states
        // that it checks *which* keys are reported and stays correct if the map
        // ever stops preserving insertion or key order.
        unknown.sort();

        assert_eq!(
            unknown,
            vec![
                "another_unknown".to_string(),
                "totally_made_up_key".to_string()
            ],
            "only registry-backed keys may be excluded from the report"
        );
    }

    #[test]
    fn test_known_keys_produce_no_unknown_key_report() {
        let cfg = RezCoreConfig::default();
        let merged =
            serde_json::to_value(&cfg).expect("config must serialize")["packages_path"].clone();
        let mut map = serde_json::Map::new();
        map.insert("packages_path".to_string(), merged);

        assert!(
            RezCoreConfig::unknown_keys(&map).is_empty(),
            "keys the binary models are registered and must not be reported"
        );
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
