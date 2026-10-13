//! # Config Command
//!
//! Implementation of the `rez config` command for viewing and managing configuration.
//!
//! `where`, `schema` and `validate` are **rez-next extensions**: upstream rez
//! exposes no equivalent machine-readable surface. They add a way to inspect
//! and check configuration without changing how rez itself resolves it.

use clap::{Args, Subcommand};
use rez_next_common::config::{LocatedValue, RezCoreConfig};
use rez_next_common::{RezCoreError, error::RezCoreResult};
use rez_next_config::keys::{KeyDef, registry};
use serde_json::Value as JsonValue;
use std::path::PathBuf;

/// Arguments for the config command
#[derive(Args, Clone)]
pub struct ConfigArgs {
    /// Output dict/list field values as JSON
    #[arg(long)]
    pub json: bool,

    /// List the config files searched
    #[arg(long)]
    pub search_list: bool,

    /// List the config files sourced
    #[arg(long)]
    pub source_list: bool,

    /// Machine-readable subcommands (rez-next extensions)
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,

    /// Configuration field to display (e.g., packages_path)
    pub field: Option<String>,
}

/// Machine-readable configuration subcommands.
///
/// All three are rez-next extensions; upstream rez defines none of them.
#[derive(Subcommand, Clone)]
pub enum ConfigCommand {
    /// Show where a key's effective value comes from (rez-next extension)
    #[command(name = "where")]
    Where {
        /// Configuration key to locate (e.g., packages_path)
        key: String,
    },

    /// Export the configuration JSON Schema (rez-next extension)
    Schema {
        /// Emit the schema as JSON (default)
        #[arg(long)]
        json: bool,
    },

    /// Validate configuration files (rez-next extension)
    Validate {
        /// Validate a specific file instead of every sourced config file
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

/// Execute the config command
pub fn execute(args: ConfigArgs) -> RezCoreResult<()> {
    if args.search_list {
        return show_search_list();
    }

    if args.source_list {
        return show_source_list();
    }

    match &args.command {
        Some(ConfigCommand::Where { key }) => show_where(key, args.json),
        Some(ConfigCommand::Schema { .. }) => show_schema(args.json),
        Some(ConfigCommand::Validate { file }) => run_validate(file.clone()),
        None => show_config(args),
    }
}

// ── Existing behaviour ───────────────────────────────────────────────────────

/// Show the list of configuration files that are searched
fn show_search_list() -> RezCoreResult<()> {
    let search_paths = RezCoreConfig::get_search_paths();

    for path in search_paths {
        println!("{}", path.display());
    }

    Ok(())
}

/// Show the list of configuration files that are actually sourced
fn show_source_list() -> RezCoreResult<()> {
    let sourced_paths = RezCoreConfig::get_sourced_paths();

    for path in sourced_paths {
        println!("{}", path.display());
    }

    Ok(())
}

/// Show a specific configuration field or the whole configuration.
fn show_config(args: ConfigArgs) -> RezCoreResult<()> {
    // Load configuration. A malformed config file is a hard error here so the
    // command exits non-zero instead of reporting defaults the user never set.
    let config = RezCoreConfig::try_load()
        .map_err(|e| RezCoreError::ConfigError(format!("failed to load configuration: {e}")))?;

    if let Some(field) = &args.field {
        show_config_field(&config, field, args.json)
    } else {
        show_full_config(&config, args.json)
    }
}

/// Show a specific configuration field
fn show_config_field(config: &RezCoreConfig, field: &str, json_output: bool) -> RezCoreResult<()> {
    if let Some(value) = config.get_field(field) {
        if json_output {
            println!(
                "{}",
                serde_json::to_string(&value).map_err(RezCoreError::Serde)?
            );
        } else {
            print_value(&value, 0);
        }
        return Ok(());
    }

    // Fall back to the key registry: a key rez-next knows about but that
    // `RezCoreConfig` does not model as a field still has a value to report.
    if let Some(def) = rez_next_config::keys::find(field)
        && let Some(value) = registry_value(config, def)
    {
        if json_output {
            println!(
                "{}",
                serde_json::to_string(&value).map_err(RezCoreError::Serde)?
            );
        } else {
            print_value(&value, 0);
        }
        return Ok(());
    }

    Err(RezCoreError::UsageError(format!(
        "Unknown configuration field: '{field}'"
    )))
}

// ── Whole-configuration output ───────────────────────────────────────────────

/// Show the full configuration.
///
/// The output walks the **key registry**, not a hand-written field list. A
/// hard-coded list silently omitted every key added after it was written, which
/// read as "complete" while actually being partial — the worst failure mode for
/// a caller that enumerates settings.
fn show_full_config(config: &RezCoreConfig, json_output: bool) -> RezCoreResult<()> {
    let values = all_values(config);

    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(&values).map_err(RezCoreError::Serde)?
        );
        return Ok(());
    }

    println!("# Rez Core Configuration");
    let mut sorted: Vec<(&String, &JsonValue)> = values.iter().collect();
    sorted.sort_by_key(|(key, _)| *key);
    for (key, value) in sorted {
        if matches!(value, JsonValue::Array(_) | JsonValue::Object(_)) {
            println!("{key}:");
            print_value(value, 1);
        } else {
            println!("{key}: {}", scalar_text(value));
        }
    }

    Ok(())
}

/// Every registry key with its current effective value.
///
/// Keys `RezCoreConfig` models are read from the loaded configuration; the rest
/// fall back to their registry default. This is what makes
/// `config --json` cover exactly the key surface rez-next accepts.
fn all_values(config: &RezCoreConfig) -> serde_json::Map<String, JsonValue> {
    let mut values = serde_json::Map::new();

    for def in registry() {
        let value = registry_value(config, def)
            .or_else(|| def.default.clone())
            .unwrap_or(JsonValue::Null);
        values.insert(def.name.to_string(), value);
    }

    values
}

/// The current value of a registry key, when the loaded config supplies one.
fn registry_value(config: &RezCoreConfig, def: &KeyDef) -> Option<JsonValue> {
    // Nested keys such as `cache.enable_memory_cache` are addressed by their
    // dotted name, which `get_field` already resolves.
    config.get_field(def.name)
}

// ── rez-next extensions: where / schema / validate ───────────────────────────

/// Show where a key's effective value comes from.
///
/// Reports the value, the layer that supplies it and the file it was read from,
/// plus any lower-priority definitions it overrides. Knowing only the value is
/// not enough to change it: the same value can come from four different files.
fn show_where(key: &str, json_output: bool) -> RezCoreResult<()> {
    // A key no config file sets still has an answer: its registry default.
    // Reporting "default" is more useful than refusing, and it makes `where`
    // total over the same key surface `config --json` exposes.
    let located = match RezCoreConfig::locate(key) {
        Some(located) => located,
        None => match rez_next_config::keys::find(key).and_then(|def| def.default.clone()) {
            Some(value) => LocatedValue {
                key: key.to_string(),
                value,
                source: rez_next_common::config::LayerSource::Builtin,
                path: None,
            },
            None => {
                return Err(RezCoreError::UsageError(format!(
                    "Unknown configuration key: '{key}'"
                )));
            }
        },
    };

    if json_output {
        let payload = serde_json::json!({
            "key": located.key,
            "value": located.value,
            "source": located.source.to_string(),
            "file": located.path.as_ref().map(|p| p.display().to_string()),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).map_err(RezCoreError::Serde)?
        );
        return Ok(());
    }

    println!("key: {}", located.key);
    println!("value: {}", scalar_text(&located.value));
    println!("source: {}", located.source);
    match &located.path {
        Some(path) => println!("file: {}", path.display()),
        None => println!("file: (none — not set by any config file)"),
    }

    let overridden = overridden_by(key, &located);
    if overridden.is_empty() {
        println!("overridden: (none)");
    } else {
        println!("overridden:");
        for entry in overridden {
            println!("  - {}: {}", entry.source, scalar_text(&entry.value));
        }
    }

    Ok(())
}

/// Lower-priority definitions of `key` that the effective value shadows.
fn overridden_by(key: &str, located: &LocatedValue) -> Vec<LocatedValue> {
    let Ok(layers) = RezCoreConfig::load_layers() else {
        return Vec::new();
    };

    let mut shadowed = Vec::new();
    for layer in layers.iter().rev() {
        // Skip the winning layer and anything above it.
        if layer.source >= located.source {
            continue;
        }
        let Some(value) = layer
            .data
            .get(key)
            .or_else(|| lookup_nested(&layer.data, key))
        else {
            continue;
        };
        shadowed.push(LocatedValue {
            key: key.to_string(),
            value: value.clone(),
            source: layer.source,
            path: layer.path.clone(),
        });
    }

    shadowed
}

/// Look up a dotted key path in a config mapping.
fn lookup_nested<'v>(
    data: &'v serde_json::Map<String, JsonValue>,
    key: &str,
) -> Option<&'v JsonValue> {
    let segments: Vec<&str> = key.split('.').collect();
    let mut current = data.get(*segments.first()?)?;
    for segment in &segments[1..] {
        current = current.as_object()?.get(*segment)?;
    }
    Some(current)
}

/// Export the configuration JSON Schema (draft 2020-12).
fn show_schema(json_output: bool) -> RezCoreResult<()> {
    let schema = rez_next_config::json_schema();

    // The schema is JSON by definition; `--json` is accepted so the flag reads
    // consistently with the rest of the command.
    let _ = json_output;
    println!(
        "{}",
        serde_json::to_string_pretty(&schema).map_err(RezCoreError::Serde)?
    );
    Ok(())
}

/// Validate configuration files, reporting key, expected type, file and line.
fn run_validate(file: Option<PathBuf>) -> RezCoreResult<()> {
    let targets: Vec<PathBuf> = match file {
        Some(path) => {
            if !path.exists() {
                return Err(RezCoreError::UsageError(format!(
                    "configuration file not found: {}",
                    path.display()
                )));
            }
            vec![path]
        }
        None => Vec::new(),
    };

    let issues = rez_next_config::validate_all(&targets);

    if issues.is_empty() {
        println!("ok: no configuration problems found");
        return Ok(());
    }

    for issue in &issues {
        eprintln!("{issue}");
    }

    // Report the first issue as the error so the process exits 3; the full set
    // has already been printed, so nothing is lost.
    let first = issues
        .into_iter()
        .next()
        .expect("issues is non-empty: just iterated it");
    Err(RezCoreError::ValidationError(first.to_string()))
}

// ── Value rendering ──────────────────────────────────────────────────────────

/// Print a value, indenting nested structures.
fn print_value(value: &JsonValue, depth: usize) {
    let indent = "  ".repeat(depth);

    match value {
        JsonValue::Array(items) => {
            for item in items {
                match item {
                    JsonValue::Array(_) | JsonValue::Object(_) => {
                        println!("{indent}-");
                        print_value(item, depth + 1);
                    }
                    _ => println!("{indent}- {}", scalar_text(item)),
                }
            }
        }
        JsonValue::Object(map) => {
            let mut sorted: Vec<(&String, &JsonValue)> = map.iter().collect();
            sorted.sort_by_key(|(key, _)| *key);
            for (key, item) in sorted {
                match item {
                    JsonValue::Array(_) | JsonValue::Object(_) => {
                        println!("{indent}{key}:");
                        print_value(item, depth + 1);
                    }
                    _ => println!("{indent}{key}: {}", scalar_text(item)),
                }
            }
        }
        _ => println!("{indent}{}", scalar_text(value)),
    }
}

/// Render a scalar for human-readable output.
fn scalar_text(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => "null".to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(items) => {
            let rendered: Vec<String> = items.iter().map(scalar_text).collect();
            format!("[{}]", rendered.join(", "))
        }
        JsonValue::Object(_) => serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_args_parsing() {
        let args = ConfigArgs {
            json: true,
            search_list: false,
            source_list: false,
            command: None,
            field: Some("version".to_string()),
        };

        assert!(args.json);
        assert_eq!(args.field, Some("version".to_string()));
    }

    #[test]
    fn test_show_config_field() {
        let config = RezCoreConfig::default();

        assert!(show_config_field(&config, "version", false).is_ok());
        assert!(show_config_field(&config, "version", true).is_ok());
        assert!(show_config_field(&config, "packages_path", false).is_ok());
        assert!(show_config_field(&config, "packages_path", true).is_ok());

        // Nested field access
        assert!(show_config_field(&config, "cache.enable_memory_cache", false).is_ok());
        assert!(show_config_field(&config, "cache.memory_cache_size", true).is_ok());
    }

    #[test]
    fn test_show_config_field_unknown_key_is_a_usage_error() {
        // A mistyped key is the caller's mistake, not a rez-next failure, so it
        // must exit 2 rather than 1.
        let config = RezCoreConfig::default();
        let error = show_config_field(&config, "unknown_field", false).expect_err("must error");
        assert_eq!(error.exit_code(), 2);
        assert!(error.to_string().contains("unknown_field"));
    }

    #[test]
    fn test_config_search_paths() {
        let search_paths = RezCoreConfig::get_search_paths();
        assert!(!search_paths.is_empty());

        let has_home_config = search_paths.iter().any(|p| {
            p.to_string_lossy().contains(".rezconfig") || p.to_string_lossy().contains(".rez")
        });
        assert!(has_home_config);
    }

    #[test]
    fn test_config_field_access() {
        let config = RezCoreConfig::default();

        assert!(config.get_field("version").is_some());
        assert!(config.get_field("packages_path").is_some());
        assert!(config.get_field("cache.enable_memory_cache").is_some());
        assert!(config.get_field("cache.memory_cache_size").is_some());

        assert!(config.get_field("nonexistent").is_none());
        assert!(config.get_field("cache.nonexistent").is_none());
    }

    #[test]
    fn test_all_values_covers_every_registry_key() {
        // Acceptance: `config --json` must expose the whole key surface. A
        // hand-written field list could not guarantee this.
        let config = RezCoreConfig::default();
        let values = all_values(&config);
        assert_eq!(
            values.len(),
            registry().len(),
            "config --json must cover every registry key"
        );
        for def in registry() {
            assert!(
                values.contains_key(def.name),
                "registry key {} missing from config output",
                def.name
            );
        }
    }

    #[test]
    fn test_all_values_reflects_loaded_config() {
        // `RezCoreConfig` is `#[non_exhaustive]`, so it cannot be built with a
        // struct literal from another crate; start from the default instead.
        let mut config = RezCoreConfig::default();
        config.packages_path = vec!["/from/config".to_string()];
        let values = all_values(&config);
        assert_eq!(
            values["packages_path"],
            serde_json::json!(["/from/config"]),
            "the loaded value must win over the registry default"
        );
    }

    #[test]
    fn test_show_full_config_emits_every_key() {
        // Regression: the non-JSON branch used to print a hard-coded subset, so
        // keys added later never appeared.
        let config = RezCoreConfig::default();
        let values = all_values(&config);

        for key in values.keys() {
            assert!(
                serde_json::to_string(key).is_ok(),
                "every key must be printable"
            );
        }
        assert!(values.contains_key("packages_path"));
        assert!(values.contains_key("cache.enable_memory_cache"));
    }

    #[test]
    fn test_scalar_text_renders_every_json_kind() {
        assert_eq!(scalar_text(&JsonValue::Null), "null");
        assert_eq!(scalar_text(&JsonValue::Bool(true)), "true");
        assert_eq!(scalar_text(&serde_json::json!(42)), "42");
        assert_eq!(scalar_text(&JsonValue::String("x".into())), "x");
        assert_eq!(
            scalar_text(&serde_json::json!(["a", "b"])),
            "[a, b]".to_string()
        );
    }

    #[test]
    fn test_where_unknown_key_is_a_usage_error() {
        let error = show_where("definitely_not_a_key", false).expect_err("must error");
        assert_eq!(error.exit_code(), 2);
    }

    #[test]
    fn test_where_reports_a_known_key() {
        // `packages_path` has a registry default, so `where` must resolve it
        // even with no config file present.
        assert!(show_where("packages_path", false).is_ok());
        assert!(show_where("packages_path", true).is_ok());
    }

    #[test]
    fn test_schema_output_declares_draft_2020_12() {
        let schema = rez_next_config::json_schema();
        assert_eq!(
            schema["$schema"],
            serde_json::json!(rez_next_config::schema::SCHEMA_DIALECT)
        );
        assert!(schema["properties"].is_object());
    }

    #[test]
    fn test_validate_missing_file_is_a_usage_error() {
        let error = run_validate(Some(PathBuf::from("/definitely/not/here.yaml")))
            .expect_err("must report a missing file");
        assert_eq!(error.exit_code(), 2);
    }
}
