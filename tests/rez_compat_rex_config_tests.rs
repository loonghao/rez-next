//! Rez Compat — rez.config Compatibility Tests
//!
//! Rex/VersionRange/Requirement tests that were previously here were exact
//! duplicates of rez_compat_activation_tests.rs (lines 289-451) and have been
//! removed in cleanup Cycle 44.

// ─── rez.config compatibility tests ─────────────────────────────────────────

/// rez.config: default packages_path is a non-empty list of paths
/// Mirrors rez.config.packages_path default behavior (defaults include ~/packages).
#[test]
fn test_config_packages_path_default_is_list() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.packages_path.is_empty(),
        "default packages_path should be non-empty"
    );
}

/// rez.config: local_packages_path is a non-empty string
#[test]
fn test_config_local_packages_path_is_string() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.local_packages_path.is_empty(),
        "local_packages_path must be non-empty"
    );
}

/// rez.config: release_packages_path is a non-empty string
#[test]
fn test_config_release_packages_path_is_string() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.release_packages_path.is_empty(),
        "release_packages_path must be non-empty"
    );
}

/// rez.config: packages_path can be overridden by direct field assignment
#[test]
fn test_config_override_packages_path_direct() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig {
        packages_path: vec!["/tmp/pkgs".to_string(), "/opt/pkgs".to_string()],
        ..RezCoreConfig::default()
    };
    assert_eq!(
        cfg.packages_path.len(),
        2,
        "overridden packages_path should have 2 entries"
    );
    assert!(cfg.packages_path.contains(&"/tmp/pkgs".to_string()));
    assert!(cfg.packages_path.contains(&"/opt/pkgs".to_string()));
}

/// rez.config: get_field accessor returns packages_path as JSON array
#[test]
fn test_config_get_field_packages_path() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let value = cfg.get_field("packages_path");
    assert!(
        value.is_some(),
        "get_field('packages_path') should return Some"
    );
    if let Some(serde_json::Value::Array(arr)) = value {
        assert!(
            !arr.is_empty(),
            "packages_path field should be non-empty array"
        );
    }
}

/// rez.config: get_field for nested cache config returns correct bool
#[test]
fn test_config_get_field_cache_nested() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let mem = cfg.get_field("cache.enable_memory_cache");
    assert_eq!(
        mem,
        Some(serde_json::Value::Bool(true)),
        "cache.enable_memory_cache should default to true"
    );
    let disk = cfg.get_field("cache.enable_disk_cache");
    assert_eq!(
        disk,
        Some(serde_json::Value::Bool(true)),
        "cache.enable_disk_cache should default to true"
    );
}

/// rez.config: default_shell is platform-appropriate (cmd on Windows, bash on Unix)
#[test]
fn test_config_default_shell_platform_appropriate() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.default_shell.is_empty(),
        "default_shell must not be empty"
    );
    #[cfg(windows)]
    assert_eq!(
        cfg.default_shell, "cmd",
        "on Windows default_shell should be 'cmd'"
    );
    #[cfg(not(windows))]
    assert_eq!(
        cfg.default_shell, "bash",
        "on Unix default_shell should be 'bash'"
    );
}

/// rez.config: version field matches CARGO_PKG_VERSION (non-empty semver string)
#[test]
fn test_config_version_non_empty() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(!cfg.version.is_empty(), "config version must be non-empty");
    // Should look like a semver: contains a dot separator
    assert!(
        cfg.version.contains('.'),
        "config version should contain '.' (semver format)"
    );
}

/// rez.config: RezCoreConfig serializes to valid JSON and roundtrips correctly
#[test]
fn test_config_serialization_json_roundtrip_compat() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let json = serde_json::to_string(&cfg).expect("config must serialize to JSON");
    let restored: RezCoreConfig =
        serde_json::from_str(&json).expect("config must deserialize from JSON");
    assert_eq!(
        cfg.packages_path, restored.packages_path,
        "packages_path must survive JSON roundtrip"
    );
    assert_eq!(
        cfg.local_packages_path, restored.local_packages_path,
        "local_packages_path must survive JSON roundtrip"
    );
    assert_eq!(
        cfg.default_shell, restored.default_shell,
        "default_shell must survive JSON roundtrip"
    );
}
