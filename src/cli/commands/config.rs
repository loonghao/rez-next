//! # Config Command
//!
//! Implementation of the `rez config` command for viewing and managing configuration.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreConfig, RezCoreError};
use serde_json;

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

    /// Configuration field to display (e.g., packages_path)
    pub field: Option<String>,
}

/// Execute the config command
pub fn execute(args: ConfigArgs) -> RezCoreResult<()> {
    if args.search_list {
        return show_search_list();
    }

    if args.source_list {
        return show_source_list();
    }

    // Load configuration
    let config = RezCoreConfig::load();

    if let Some(field) = &args.field {
        show_config_field(&config, field, args.json)
    } else {
        show_full_config(&config, args.json)
    }
}

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

/// Show a specific configuration field
fn show_config_field(config: &RezCoreConfig, field: &str, json_output: bool) -> RezCoreResult<()> {
    // Use the new get_field method for dot-separated field access
    if let Some(value) = config.get_field(field) {
        if json_output {
            println!(
                "{}",
                serde_json::to_string(&value).map_err(RezCoreError::Serde)?
            );
        } else {
            // Format the output based on the value type
            match &value {
                serde_json::Value::Array(arr) => {
                    for item in arr {
                        if let serde_json::Value::String(s) = item {
                            println!("{}", s);
                        } else {
                            println!("{}", item);
                        }
                    }
                }
                serde_json::Value::String(s) => {
                    println!("{}", s);
                }
                serde_json::Value::Number(n) => {
                    println!("{}", n);
                }
                serde_json::Value::Bool(b) => {
                    println!("{}", b);
                }
                serde_json::Value::Object(_) => {
                    // For objects, use YAML-like format
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&value).map_err(RezCoreError::Serde)?
                    );
                }
                serde_json::Value::Null => {
                    println!("null");
                }
            }
        }
    } else {
        return Err(RezCoreError::RequirementParse(format!(
            "Unknown configuration field: '{}'",
            field
        )));
    }

    Ok(())
}

/// Show the full configuration
fn show_full_config(config: &RezCoreConfig, json_output: bool) -> RezCoreResult<()> {
    if json_output {
        // Serialize the actual config to JSON
        println!(
            "{}",
            serde_json::to_string_pretty(config).map_err(RezCoreError::Serde)?
        );
    } else {
        // Format the actual config in YAML-like format
        println!("# Rez Core Configuration");
        println!("version: {}", config.version);
        println!("use_rust_version: {}", config.use_rust_version);
        println!("use_rust_solver: {}", config.use_rust_solver);
        println!("use_rust_repository: {}", config.use_rust_repository);
        println!("rust_fallback: {}", config.rust_fallback);

        if let Some(thread_count) = config.thread_count {
            println!("thread_count: {}", thread_count);
        } else {
            println!("thread_count: null");
        }

        println!("packages_path:");
        for path in &config.packages_path {
            println!("  - {}", path);
        }

        println!("local_packages_path: {}", config.local_packages_path);
        println!("release_packages_path: {}", config.release_packages_path);
        println!("default_shell: {}", config.default_shell);

        println!("plugin_path:");
        for path in &config.plugin_path {
            println!("  - {}", path);
        }

        println!("package_cache_path:");
        for path in &config.package_cache_path {
            println!("  - {}", path);
        }

        println!("tmpdir: {}", config.tmpdir);
        println!("editor: {}", config.editor);
        println!("image_viewer: {}", config.image_viewer);
        println!("browser: {}", config.browser);
        println!("difftool: {}", config.difftool);
        println!(
            "terminal_emulator_command: {}",
            config.terminal_emulator_command
        );

        println!("cache:");
        println!(
            "  enable_memory_cache: {}",
            config.cache.enable_memory_cache
        );
        println!("  enable_disk_cache: {}", config.cache.enable_disk_cache);
        println!("  memory_cache_size: {}", config.cache.memory_cache_size);
        println!("  cache_ttl_seconds: {}", config.cache.cache_ttl_seconds);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_args_parsing() {
        // Test that the Args derive works correctly
        let args = ConfigArgs {
            json: true,
            search_list: false,
            source_list: false,
            field: Some("version".to_string()),
        };

        assert!(args.json);
        assert_eq!(args.field, Some("version".to_string()));
    }

    #[test]
    fn test_show_config_field() {
        let config = RezCoreConfig::default();

        // Test version field
        assert!(show_config_field(&config, "version", false).is_ok());
        assert!(show_config_field(&config, "version", true).is_ok());

        // Test packages_path field
        assert!(show_config_field(&config, "packages_path", false).is_ok());
        assert!(show_config_field(&config, "packages_path", true).is_ok());

        // Test nested field access
        assert!(show_config_field(&config, "cache.enable_memory_cache", false).is_ok());
        assert!(show_config_field(&config, "cache.memory_cache_size", true).is_ok());

        // Test unknown field
        assert!(show_config_field(&config, "unknown_field", false).is_err());
        assert!(show_config_field(&config, "cache.unknown_field", false).is_err());
    }

    #[test]
    fn test_config_search_paths() {
        let search_paths = RezCoreConfig::get_search_paths();
        assert!(!search_paths.is_empty());

        // Should include user home config paths
        let has_home_config = search_paths.iter().any(|p| {
            p.to_string_lossy().contains(".rezconfig") || p.to_string_lossy().contains(".rez")
        });
        assert!(has_home_config);
    }

    #[test]
    fn test_config_field_access() {
        let config = RezCoreConfig::default();

        // Test simple field access
        assert!(config.get_field("version").is_some());
        assert!(config.get_field("packages_path").is_some());

        // Test nested field access
        assert!(config.get_field("cache.enable_memory_cache").is_some());
        assert!(config.get_field("cache.memory_cache_size").is_some());

        // Test invalid field access
        assert!(config.get_field("nonexistent").is_none());
        assert!(config.get_field("cache.nonexistent").is_none());
    }
}
