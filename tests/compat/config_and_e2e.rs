use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Config compatibility tests ─────────────────────────────────────────────

#[test]
fn test_config_packages_path_default() {
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::default();
    assert!(
        !config.packages_path.is_empty(),
        "packages_path should have defaults"
    );
    assert!(
        !config.local_packages_path.is_empty(),
        "local_packages_path should be set"
    );
}

#[test]
fn test_config_env_override() {
    use rez_next_common::config::RezCoreConfig;

    // Set custom packages path via env var
    std::env::set_var("REZ_PACKAGES_PATH", "/custom/packages:/another/path");
    let config = RezCoreConfig::load();
    // On POSIX the split is ':', on Windows it might be ';'
    // Just ensure it's non-empty and contains our paths
    let joined = config.packages_path.join(":");
    assert!(
        joined.contains("/custom/packages"),
        "Env override should set packages path"
    );
    std::env::remove_var("REZ_PACKAGES_PATH");
}

// ─── End-to-end workflow tests ──────────────────────────────────────────────

/// Simulate a complete "create package → serialize → deserialize" workflow
#[test]
fn test_e2e_package_create_serialize_deserialize() {
    use rez_next_package::serialization::PackageSerializer;

    let mut pkg = Package::new("my_tool".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    pkg.description = Some("My test tool".to_string());
    pkg.authors = vec!["Author Name".to_string()];
    pkg.requires = vec!["python-3.9".to_string()];
    pkg.tools = vec!["mytool".to_string()];

    // Serialize to YAML string
    let yaml_str = PackageSerializer::save_to_yaml(&pkg).unwrap();
    assert!(!yaml_str.is_empty(), "YAML string should not be empty");

    // Deserialize back
    let loaded = PackageSerializer::load_from_yaml(&yaml_str).unwrap();
    assert_eq!(loaded.name, "my_tool");
    assert_eq!(loaded.description.as_deref(), Some("My test tool"));
}

/// Simulate rex commands → environment → shell script workflow
#[test]
fn test_e2e_rex_to_shell_script() {
    use rez_next_rex::ShellType;

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv('PKG_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
alias('myapp', '{root}/bin/myapp')
"#,
            "myapp",
            Some("/opt/myapp/2.0"),
            Some("2.0"),
        )
        .unwrap();

    // Generate scripts for all shells
    for shell in [
        ShellType::Bash,
        ShellType::PowerShell,
        ShellType::Fish,
        ShellType::Cmd,
    ] {
        let script = generate_shell_script(&env, &shell);
        assert!(
            !script.is_empty(),
            "Script for {:?} should not be empty",
            shell
        );
        assert!(
            script.len() > 20,
            "Script for {:?} should have content",
            shell
        );
    }
}

/// Verify version range operations match rez's expected behavior
#[test]
fn test_rez_version_range_rez_syntax() {
    // rez uses '+' to mean "up to and including this version's epoch"
    // "1+" means ">=1, <2" in some rez contexts, but primarily ">=1.0"
    let r = VersionRange::parse("1.0+").unwrap();
    assert!(
        r.contains(&Version::parse("1.5").unwrap()),
        "1.0+ should contain 1.5"
    );
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "1.0+ should contain 2.0"
    );
    assert!(
        !r.contains(&Version::parse("0.9").unwrap()),
        "1.0+ should not contain 0.9"
    );
}

#[test]
fn test_rez_version_range_lt_syntax() {
    let r = VersionRange::parse("<2.0").unwrap();
    assert!(
        r.contains(&Version::parse("1.9").unwrap()),
        "<2.0 should contain 1.9"
    );
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        "<2.0 should not contain 2.0"
    );
}

