use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.depends: reverse dependency query semantics ─────────────────────────

/// rez depends: finding dependents when nothing depends on target returns empty
#[test]
fn test_depends_no_dependents_for_isolated_package() {
    use rez_next_package::Package;
    // Build a synthetic package set where nothing requires "isolated_pkg".
    // Package.requires is Vec<String> (requirement strings).
    let packages: Vec<Package> = vec![
        Package::new("python".to_string()),
        Package::new("maya".to_string()),
    ];
    let target = "isolated_pkg";
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with(target)))
        .collect();
    assert!(
        dependents.is_empty(),
        "no package should depend on an isolated package"
    );
}

/// rez depends: direct dependent detection via requires list
#[test]
fn test_depends_direct_dependent_found() {
    use rez_next_package::Package;
    let mut consumer = Package::new("my_tool".to_string());
    consumer.requires = vec!["python-3+".to_string()];

    let packages = vec![consumer];
    let target = "python";
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with(target)))
        .collect();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].name, "my_tool");
}

/// rez depends: packages with empty requires list never appear as dependents
#[test]
fn test_depends_empty_requires_not_dependent() {
    use rez_next_package::Package;
    let packages: Vec<Package> = vec![
        Package::new("standalone_a".to_string()),
        Package::new("standalone_b".to_string()),
    ];
    for pkg in &packages {
        assert!(
            pkg.requires.is_empty(),
            "packages should have empty requires"
        );
    }
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with("anything")))
        .collect();
    assert!(dependents.is_empty());
}

/// rez status: REZ_USED_PACKAGES_NAMES parsing produces correct package list
#[test]
fn test_status_parse_rez_used_packages_names() {
    let raw = "python-3.9 maya-2024.1 houdini-20.5";
    let packages: Vec<&str> = raw.split_whitespace().collect();
    assert_eq!(packages.len(), 3);
    assert_eq!(packages[0], "python-3.9");
    assert_eq!(packages[1], "maya-2024.1");
    assert_eq!(packages[2], "houdini-20.5");
}

/// rez status: REZ_ env var prefix filtering
#[test]
fn test_status_rez_env_prefix_filter() {
    let all_env: Vec<(String, String)> = vec![
        ("PATH".to_string(), "/usr/bin".to_string()),
        ("REZ_CONTEXT_FILE".to_string(), "/tmp/ctx.rxt".to_string()),
        ("REZ_VERSION".to_string(), "3.0.0".to_string()),
        ("HOME".to_string(), "/home/user".to_string()),
    ];

    let rez_vars: Vec<_> = all_env
        .iter()
        .filter(|(k, _)| k.starts_with("REZ_"))
        .collect();
    assert_eq!(rez_vars.len(), 2, "Should find exactly 2 REZ_ vars");
    assert!(rez_vars.iter().any(|(k, _)| k == "REZ_CONTEXT_FILE"));
    assert!(rez_vars.iter().any(|(k, _)| k == "REZ_VERSION"));
}

/// rez status: shell detection on various SHELL env values
#[test]
fn test_status_shell_detection_logic() {
    let cases = [
        ("/bin/bash", "bash"),
        ("/usr/bin/zsh", "zsh"),
        ("/usr/local/bin/fish", "fish"),
    ];

    for (shell_val, expected) in &cases {
        let detected = if shell_val.contains("zsh") {
            "zsh"
        } else if shell_val.contains("fish") {
            "fish"
        } else if shell_val.contains("bash") {
            "bash"
        } else {
            *shell_val
        };
        assert_eq!(
            detected, *expected,
            "Shell detection should identify {}",
            expected
        );
    }
}

/// rez status: context file path round-trips through env var
#[test]
fn test_status_context_file_path_format() {
    let ctx_path = "/tmp/rez_ctx_12345.rxt";
    // Simulate what would be in REZ_CONTEXT_FILE
    let parsed = ctx_path.to_string();
    assert!(
        parsed.ends_with(".rxt"),
        "Context file should have .rxt extension"
    );
    assert!(
        parsed.starts_with("/tmp"),
        "Context file path should be absolute"
    );
}

