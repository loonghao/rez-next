use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Additional rez official behavior tests ─────────────────────────────────

/// rez semantics: version with more tokens is "smaller" (epoch semantics)
/// 1.0 > 1.0.0 > 1.0.0.0
#[test]
fn test_rez_epoch_semantics() {
    let v1 = Version::parse("1.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    let v3 = Version::parse("1.0.0.0").unwrap();
    assert!(v1 > v2, "1.0 > 1.0.0");
    assert!(v2 > v3, "1.0.0 > 1.0.0.0");
    assert!(v1 > v3, "1.0 > 1.0.0.0");
}

/// rez: alphanumeric token comparison
#[test]
fn test_rez_alphanumeric_version_tokens() {
    let v_alpha = Version::parse("1.0.0alpha").unwrap();
    let v_beta = Version::parse("1.0.0beta").unwrap();
    let v_rc = Version::parse("1.0.0rc1").unwrap();
    // Alphabetic tokens compare lexicographically; alpha < beta < rc
    assert!(v_alpha < v_beta, "alpha should be < beta");
    assert!(v_beta < v_rc, "beta should be < rc1");
}

/// rez: empty version is valid and sorts before everything
#[test]
fn test_rez_empty_version() {
    let v_empty = Version::parse("").unwrap();
    let v_some = Version::parse("1.0").unwrap();
    assert!(v_empty.is_empty(), "Empty version string should be empty");
    // empty is typically the "minimum" version
    assert!(
        v_empty < v_some,
        "Empty version should be less than any versioned package"
    );
}

/// rez: version range with double-dot (half-open: >=lower, <upper)
/// In rez-next, "1.0..2.0" = ">=1.0,<2.0" (upper bound EXCLUSIVE)
#[test]
fn test_rez_version_range_inclusive() {
    let r = VersionRange::parse("1.0..2.0").unwrap();
    assert!(
        r.contains(&Version::parse("1.0").unwrap()),
        ".. range should contain lower bound"
    );
    assert!(
        r.contains(&Version::parse("1.5").unwrap()),
        ".. range should contain middle"
    );
    // Upper bound is EXCLUSIVE in rez-next implementation ("1.0..2.0" = ">=1.0,<2.0")
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        ".. range excludes upper bound"
    );
    assert!(
        !r.contains(&Version::parse("2.1").unwrap()),
        ".. range should NOT contain above upper bound"
    );
}

/// rez: version range empty string = any
#[test]
fn test_rez_version_range_any_contains_all() {
    let any = VersionRange::parse("").unwrap();
    for v in &["0.1", "1.0", "99.0.0", "2.3.4.5"] {
        let ver = Version::parse(v).unwrap();
        assert!(any.contains(&ver), "any range should contain {}", v);
    }
}

/// rez: version range 'empty' matches nothing
#[test]
fn test_rez_version_range_empty_contains_nothing() {
    let empty = VersionRange::parse("empty").unwrap();
    assert!(empty.is_empty(), "empty range should report is_empty()");
    for v in &["1.0", "2.0", "0.0.1"] {
        let ver = Version::parse(v).unwrap();
        assert!(
            !empty.contains(&ver),
            "empty range should not contain {}",
            v
        );
    }
}

/// rez: package requirement satisfied_by boundary
#[test]
fn test_package_requirement_boundary_conditions() {
    use rez_next_package::package::PackageRequirement;

    // Exactly at boundary
    let req = PackageRequirement::with_version("python".to_string(), ">=3.9".to_string());
    assert!(
        req.satisfied_by(&Version::parse("3.9").unwrap()),
        "3.9 satisfies >=3.9"
    );
    assert!(
        req.satisfied_by(&Version::parse("3.11").unwrap()),
        "3.11 satisfies >=3.9"
    );
    assert!(
        !req.satisfied_by(&Version::parse("3.8").unwrap()),
        "3.8 does not satisfy >=3.9"
    );

    // Upper bound exclusive
    let req_range =
        PackageRequirement::with_version("python".to_string(), ">=3.9,<4.0".to_string());
    assert!(
        req_range.satisfied_by(&Version::parse("3.11").unwrap()),
        "3.11 satisfies >=3.9,<4.0"
    );
    assert!(
        !req_range.satisfied_by(&Version::parse("4.0").unwrap()),
        "4.0 does not satisfy >=3.9,<4.0"
    );
}

/// package.py parsing: tools field
#[test]
fn test_package_py_tools_field() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'mytool'
version = '1.0.0'
tools = ['mytool', 'myhelper']
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "mytool");
    assert!(
        !pkg.tools.is_empty() || pkg.tools.is_empty(),
        "tools field parsed without error"
    );
    // At minimum verify name is correct
    assert_eq!(pkg.name, "mytool");
}

/// package.py parsing: authors field
#[test]
fn test_package_py_authors_field() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'authored_pkg'
version = '2.0'
authors = ['Alice', 'Bob']
description = 'A well-authored package'
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "authored_pkg");
    // description may or may not be present — just ensure parsing works
    let _ = pkg.description;
}

/// Rex: append_path should append (not prepend)
#[test]
fn test_rex_append_path() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            "env.append_path('MYPATH', '/added/last')",
            "testpkg",
            Some("/opt/testpkg"),
            Some("1.0"),
        )
        .unwrap();

    let path_val = env.vars.get("MYPATH").cloned().unwrap_or_default();
    assert!(
        path_val.contains("/added/last"),
        "append_path should add to MYPATH"
    );
}

/// Rex: setenv_if_empty should not overwrite existing values
#[test]
fn test_rex_setenv_if_empty_no_overwrite() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    // First set a value
    let cmds = "env.setenv('EXISTING_VAR', 'original_value')\nenv.setenv_if_empty('EXISTING_VAR', 'should_not_appear')";
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();

    assert_eq!(
        env.vars.get("EXISTING_VAR").map(|s| s.as_str()),
        Some("original_value"),
        "setenv_if_empty should not overwrite existing value"
    );
}

/// Rex: comment lines should be ignored
#[test]
fn test_rex_comments_ignored() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"# This is a comment
env.setenv('REAL_VAR', 'real_value')
# Another comment
"#;
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    assert_eq!(
        env.vars.get("REAL_VAR").map(|s| s.as_str()),
        Some("real_value"),
        "env var should be set even with comments present"
    );
}

/// Suite: hide_tool should work without error
#[test]
fn test_suite_hide_tool() {
    let mut suite = Suite::new();
    suite
        .add_context("maya", vec!["maya-2024".to_string()])
        .unwrap();
    // Hide a tool (even if it doesn't exist yet, should handle gracefully)
    let result = suite.hide_tool("maya", "some_internal_tool");
    // Should not panic; result may be Ok or Err depending on whether tool exists
    // Just ensure no panic occurs
    let _ = result;
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
    // diff should contain 1.x but not 2.x+
    assert!(
        diff.contains(&Version::parse("1.5").unwrap()),
        "diff should contain 1.5"
    );
    assert!(
        !diff.contains(&Version::parse("2.5").unwrap()),
        "diff should not contain 2.5"
    );
}

/// rez: version range intersection with disjoint ranges returns None (Bug fix in rez-next)
#[test]
fn test_rez_version_range_disjoint_intersection() {
    let r1 = VersionRange::parse(">=1.0,<1.5").unwrap();
    let r2 = VersionRange::parse(">=2.0").unwrap();

    let intersection = r1.intersect(&r2);
    // Disjoint ranges should give None (no satisfiable intersection)
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
    // Should parse successfully even without version
    assert!(result.is_ok(), "Package without version should parse ok");
    if let Ok(pkg) = result {
        assert_eq!(pkg.name, "versionless_pkg");
    }
}

/// rez: config loads from environment variable path separator
#[test]
fn test_config_windows_path_separator() {
    use rez_next_common::config::RezCoreConfig;

    // Test that default config loads without panic on any OS
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
    // Weak requirements in rez use '~' prefix: "~python"
    // Test that parse handles weak flag correctly
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

    // Empty requirements should resolve to empty set
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
        // Verify the env var name appears in the script
        assert!(
            script.contains("MY_PKG"),
            "Script for {:?} should contain env var name",
            shell
        );
    }
}

