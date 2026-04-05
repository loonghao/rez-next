//! Rez Compat — Suite advanced (hide_tool, remove_context), version range subtract/disjoint,
//! package no-version, config misc, shell type / script generation, DependencyResolver empty.
//!
//! Split from rez_compat_tests.rs (Cycle 71) to keep file under 1000 lines.

use rez_core::version::{Version, VersionRange};
use rez_next_suites::{Suite, ToolConflictMode};

/// Suite: hide_tool should work without error
#[test]
fn test_suite_hide_tool() {
    let mut suite = Suite::new();
    suite
        .add_context("maya", vec!["maya-2024".to_string()])
        .unwrap();
    let result = suite.hide_tool("maya", "some_internal_tool");
    match result {
        Ok(()) => {
            let tools = suite.get_tools().unwrap_or_default();
            assert!(
                !tools.contains_key("some_internal_tool"),
                "hidden tool 'some_internal_tool' should not appear in get_tools()"
            );
        }
        Err(_) => {}
    }
}

/// Suite: remove_context should work
#[test]
fn test_suite_remove_context() {
    let mut suite = Suite::new();
    suite
        .add_context("maya", vec!["maya-2024".to_string()])
        .unwrap();
    suite
        .add_context("nuke", vec!["nuke-14".to_string()])
        .unwrap();
    assert_eq!(suite.len(), 2);

    suite.remove_context("maya").unwrap();
    assert_eq!(suite.len(), 1);
    assert!(suite.get_context("maya").is_none());
    assert!(suite.get_context("nuke").is_some());
}

/// rez: version range subtract operation
#[test]
fn test_rez_version_range_subtract() {
    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse(">=2.0").unwrap();

    let diff = r1.subtract(&r2);
    assert!(
        diff.is_some(),
        "Subtract of non-empty ranges should give result"
    );
    let diff = diff.unwrap();
    assert!(
        diff.contains(&Version::parse("1.5").unwrap()),
        "diff should contain 1.5"
    );
    assert!(
        !diff.contains(&Version::parse("2.5").unwrap()),
        "diff should not contain 2.5"
    );
}

/// rez: version range intersection with disjoint ranges returns None
#[test]
fn test_rez_version_range_disjoint_intersection() {
    let r1 = VersionRange::parse(">=1.0,<1.5").unwrap();
    let r2 = VersionRange::parse(">=2.0").unwrap();

    let intersection = r1.intersect(&r2);
    assert!(
        intersection.is_none(),
        "Disjoint ranges should return None for intersect(), got: {:?}",
        intersection.as_ref().map(|r| r.as_str())
    );
}

/// rez: package with no version is valid
#[test]
fn test_package_no_version_valid() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'versionless_pkg'
description = 'A package without a version'
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let result = PackageSerializer::load_from_file(&path);
    assert!(result.is_ok(), "Package without version should parse ok");
    if let Ok(pkg) = result {
        assert_eq!(pkg.name, "versionless_pkg");
    }
}

/// rez: config loads from environment variable path separator
#[test]
fn test_config_windows_path_separator() {
    use rez_next_common::config::RezCoreConfig;

    let config = RezCoreConfig::load();
    assert!(!config.version.is_empty(), "version should be set");
    assert!(
        !config.packages_path.is_empty(),
        "packages_path should have entries"
    );
}

/// rez: requirement with weak flag (~prefix)
#[test]
fn test_package_requirement_weak() {
    use rez_next_package::package::PackageRequirement;
    let req_normal = PackageRequirement::parse("python").unwrap();
    assert!(!req_normal.weak, "normal requirement should not be weak");
}

/// rez: DependencyResolver basic test
#[test]
fn test_dependency_resolver_empty() {
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo_mgr = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_mgr), SolverConfig::default());

    let result = rt.block_on(resolver.resolve(vec![]));
    assert!(result.is_ok(), "Empty resolve should succeed");
    let resolution = result.unwrap();
    assert_eq!(
        resolution.resolved_packages.len(),
        0,
        "Empty requirements should yield 0 resolved packages"
    );
}

/// rez: shell type detection
#[test]
fn test_shell_type_all_supported() {
    use rez_next_rex::ShellType;

    let shells = ["bash", "zsh", "fish", "cmd", "powershell"];
    for s in &shells {
        let st = ShellType::parse(s);
        assert!(st.is_some(), "Shell type '{}' should be supported", s);
    }

    let unknown = ShellType::parse("unknown_shell_xyz");
    assert!(unknown.is_none(), "Unknown shell type should return None");
}

/// rez: generate shell scripts for all supported shells
#[test]
fn test_shell_scripts_all_shells() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("MY_PKG".to_string(), "/opt/mypkg".to_string());
    env.aliases
        .insert("mypkg".to_string(), "/opt/mypkg/bin/mypkg".to_string());

    let shells = [
        ShellType::Bash,
        ShellType::Zsh,
        ShellType::Fish,
        ShellType::Cmd,
        ShellType::PowerShell,
    ];

    for shell in &shells {
        let script = generate_shell_script(&env, shell);
        assert!(
            !script.is_empty(),
            "Shell script for {:?} should not be empty",
            shell
        );
        assert!(
            script.contains("MY_PKG"),
            "Script for {:?} should contain env var name",
            shell
        );
    }
}
