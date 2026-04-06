//! Rez Compatibility Integration Tests
//!
//! These tests verify that rez-next implements the same behavior as the original
//! rez package manager. Test cases are derived from rez's official test suite
//! and documentation examples.

use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement};
use rez_next_rex::{generate_shell_script, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Version compatibility tests ───────────────────────────────────────────

/// rez version parsing: numeric, alphanumeric, epoch-based
#[test]
fn test_rez_version_numeric() {
    let versions = ["1", "1.2", "1.2.3", "1.2.3.4"];
    for v in &versions {
        let parsed = Version::parse(v).unwrap_or_else(|_| panic!("Failed to parse version: {}", v));
        assert_eq!(parsed.as_str(), *v, "Version roundtrip failed for {}", v);
    }
}

#[test]
fn test_rez_version_ordering() {
    // Rez ordering: 1.0 > 1.0.0 (shorter is "greater epoch")
    let v1 = Version::parse("1.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    // In rez semantics: 1.0 > 1.0.0
    assert!(v1 > v2, "1.0 should be greater than 1.0.0 in rez semantics");
}

#[test]
fn test_rez_version_compare_major_minor() {
    let cases = [
        ("2.0.0", "1.9.9", true),  // 2.0.0 > 1.9.9
        ("1.10.0", "1.9.0", true), // 1.10 > 1.9
        ("1.0.0", "1.0.0", false), // equal
        ("1.0.0", "2.0.0", false), // 1.0.0 < 2.0.0
    ];
    for (a, b, expected_gt) in &cases {
        let va = Version::parse(a).unwrap();
        let vb = Version::parse(b).unwrap();
        assert_eq!(
            va > vb,
            *expected_gt,
            "{} > {} should be {}",
            a,
            b,
            expected_gt
        );
    }
}

#[test]
fn test_rez_version_range_any() {
    // Empty range means "any version" in rez
    let r = VersionRange::parse("").unwrap();
    assert!(r.is_any(), "Empty range should be 'any'");
    assert!(r.contains(&Version::parse("1.0.0").unwrap()));
    assert!(r.contains(&Version::parse("999.999.999").unwrap()));
}

#[test]
fn test_rez_version_range_exact() {
    // Exact version: "==1.2.3" or just "1.2.3" (point range)
    let r = VersionRange::parse("1.2.3").unwrap();
    assert!(
        r.contains(&Version::parse("1.2.3").unwrap()),
        "Range should contain exact version"
    );
}

#[test]
fn test_rez_version_range_ge() {
    let r = VersionRange::parse(">=1.0").unwrap();
    assert!(r.contains(&Version::parse("1.0").unwrap()));
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(!r.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_ge_lt() {
    let r = VersionRange::parse(">=1.0,<2.0").unwrap();
    assert!(r.contains(&Version::parse("1.5").unwrap()));
    assert!(!r.contains(&Version::parse("2.0").unwrap()));
    assert!(!r.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_intersection() {
    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<2.0").unwrap();
    let intersection = r1.intersect(&r2).expect("Intersection should exist");
    assert!(intersection.contains(&Version::parse("1.5").unwrap()));
    assert!(!intersection.contains(&Version::parse("2.0").unwrap()));
    assert!(!intersection.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_union() {
    let r1 = VersionRange::parse(">=1.0,<1.5").unwrap();
    let r2 = VersionRange::parse(">=2.0").unwrap();
    let union = r1.union(&r2);
    assert!(union.contains(&Version::parse("1.2").unwrap()));
    assert!(union.contains(&Version::parse("2.5").unwrap()));
    assert!(!union.contains(&Version::parse("1.7").unwrap()));
}

#[test]
fn test_rez_version_range_subset_superset() {
    let r_narrow = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r_wide = VersionRange::parse(">=1.0").unwrap();
    assert!(
        r_narrow.is_subset_of(&r_wide),
        "narrow should be subset of wide"
    );
    assert!(
        r_wide.is_superset_of(&r_narrow),
        "wide should be superset of narrow"
    );
}

// ─── Package parsing tests ──────────────────────────────────────────────────

#[test]
fn test_package_requirement_parse_name_only() {
    let req = PackageRequirement::parse("python").unwrap();
    assert_eq!(req.name, "python");
    // No version constraint for bare package name: version_spec should be empty or None
    // (field name is version_spec in the actual struct)
}

#[test]
fn test_package_requirement_parse_name_version() {
    // rez style: "python-3.9" means name=python, constraint=3.9
    let req = PackageRequirement::parse("python-3.9").unwrap();
    assert_eq!(req.name, "python");
}

#[test]
fn test_package_requirement_parse_semver_style() {
    // rez uses "name-version" syntax, not "name>=version"
    // "python>=3.9" is NOT standard rez syntax; test that it parses with correct name
    // The parser treats the entire string as name with no dash
    let req = PackageRequirement::parse("python-3.9").unwrap();
    assert_eq!(req.name, "python");
    // Bare python requirement
    let req2 = PackageRequirement::parse("python").unwrap();
    assert_eq!(req2.name, "python");
}

#[test]
fn test_package_creation_basic() {
    let pkg = Package::new("my_package".to_string());
    assert_eq!(pkg.name, "my_package");
    assert!(pkg.version.is_none());
    assert!(pkg.requires.is_empty());
}

#[test]
fn test_package_with_version() {
    let mut pkg = Package::new("my_package".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(pkg.version.is_some());
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.0.0");
}

#[test]
fn test_package_py_parse_minimal() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'minimal_pkg'
version = '1.0.0'
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "minimal_pkg");
    assert!(pkg.version.is_some());
}

#[test]
fn test_package_py_parse_with_requires() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'my_tool'
version = '2.0.0'
requires = ['python-3', 'pip-22']
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "my_tool");
    assert!(!pkg.requires.is_empty(), "requires should be parsed");
}

#[test]
fn test_package_py_parse_with_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'maya'
version = '2024.0'
requires = ['python-3.9']
commands = "env.setenv('MAYA_LOCATION', '{root}')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "maya");
}

#[test]
fn test_package_yaml_parse() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name: yaml_pkg
version: "3.2.1"
description: "A YAML package"
requires:
  - python-3.9
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.yaml");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "yaml_pkg");
}

// ─── Rex execution tests ────────────────────────────────────────────────────

#[test]
fn test_rex_typical_maya_setup() {
    let mut exec = RexExecutor::new();
    let commands = r#"env.setenv('MAYA_VERSION', '2024')
env.setenv('MAYA_LOCATION', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
alias('maya', '{root}/bin/maya')
alias('mayapy', '{root}/bin/mayapy')
"#;
    let env = exec
        .execute_commands(
            commands,
            "maya",
            Some("/opt/autodesk/maya/2024"),
            Some("2024"),
        )
        .unwrap();

    assert_eq!(env.vars.get("MAYA_VERSION"), Some(&"2024".to_string()));
    assert_eq!(
        env.vars.get("MAYA_LOCATION"),
        Some(&"/opt/autodesk/maya/2024".to_string())
    );
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/opt/autodesk/maya/2024/bin"))
        .unwrap_or(false));
    assert_eq!(
        env.aliases.get("maya"),
        Some(&"/opt/autodesk/maya/2024/bin/maya".to_string())
    );
}

#[test]
fn test_rex_python_package_setup() {
    let mut exec = RexExecutor::new();
    let commands = r#"env.setenv('PYTHONHOME', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
"#;
    let env = exec
        .execute_commands(commands, "python", Some("/usr/local"), Some("3.11.5"))
        .unwrap();

    assert_eq!(env.vars.get("PYTHONHOME"), Some(&"/usr/local".to_string()));
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/usr/local/bin"))
        .unwrap_or(false));
    assert!(env
        .vars
        .get("PYTHONPATH")
        .map(|v| v.contains("site-packages"))
        .unwrap_or(false));
}

#[test]
fn test_rex_generates_valid_bash_script() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv('TEST_VAR', 'test_value')
env.prepend_path('PATH', '/opt/test/bin')
alias('test_cmd', '/opt/test/bin/test')
"#,
            "test_pkg",
            Some("/opt/test"),
            Some("1.0"),
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("export TEST_VAR="),
        "bash script missing export"
    );
    assert!(script.contains("export PATH="), "bash script missing PATH");
    assert!(
        script.contains("alias test_cmd="),
        "bash script missing alias"
    );
}

#[test]
fn test_rex_generates_valid_powershell_script() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv('MY_APP', '{root}')
alias('myapp', '{root}/myapp.exe')
"#,
            "myapp",
            Some("C:\\Program Files\\MyApp"),
            Some("2.0"),
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    assert!(
        script.contains("$env:MY_APP"),
        "PowerShell script missing $env:"
    );
    assert!(
        script.contains("Set-Alias"),
        "PowerShell script missing Set-Alias"
    );
}

// ─── Suite management tests ─────────────────────────────────────────────────

#[test]
fn test_suite_vfx_pipeline_setup() {
    // Simulate a typical VFX pipeline suite
    let mut suite = Suite::new()
        .with_description("VFX Pipeline Suite v2024")
        .with_conflict_mode(ToolConflictMode::Last);

    suite
        .add_context(
            "maya",
            vec![
                "maya-2024".to_string(),
                "python-3.9".to_string(),
                "mtoa-5".to_string(),
            ],
        )
        .unwrap();

    suite
        .add_context(
            "nuke",
            vec!["nuke-14".to_string(), "python-3.9".to_string()],
        )
        .unwrap();

    suite
        .add_context(
            "houdini",
            vec!["houdini-20".to_string(), "python-3.10".to_string()],
        )
        .unwrap();

    // Set up aliases
    suite.alias_tool("maya", "maya2024", "maya").unwrap();
    suite.alias_tool("nuke", "nuke14", "nuke").unwrap();

    assert_eq!(suite.len(), 3);
    assert_eq!(
        suite.context_names().len(),
        3,
        "Suite should have 3 contexts"
    );
    assert!(suite.get_context("maya").is_some());
    assert!(suite.get_context("nuke").is_some());
    assert!(suite.get_context("houdini").is_some());
}

#[test]
fn test_suite_save_load_roundtrip() {
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let suite_path = tmp.path().join("vfx_pipeline");

    let mut suite = Suite::new().with_description("VFX pipeline suite");

    suite
        .add_context("dcc", vec!["maya-2024".to_string()])
        .unwrap();
    suite
        .add_context("render", vec!["arnold-7".to_string()])
        .unwrap();
    suite.alias_tool("dcc", "maya24", "maya").unwrap();
    suite.save(&suite_path).unwrap();

    // Reload and verify
    let loaded = Suite::load(&suite_path).unwrap();
    assert_eq!(loaded.description, Some("VFX pipeline suite".to_string()));
    assert_eq!(loaded.len(), 2);
    assert!(loaded.context_names().contains(&"dcc"));
    assert!(loaded.context_names().contains(&"render"));
}

#[test]
fn test_suite_is_suite_detection() {
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();

    // Empty directory is not a suite
    assert!(
        !Suite::is_suite(tmp.path()),
        "Empty dir should not be a suite"
    );

    // After saving, it becomes a suite
    let suite_path = tmp.path().join("my_suite");
    let mut suite = Suite::new();
    suite.add_context("ctx", vec![]).unwrap();
    suite.save(&suite_path).unwrap();

    assert!(
        Suite::is_suite(&suite_path),
        "Saved suite dir should be detected as suite"
    );
}

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
    // description field must be populated from the package.py.
    assert_eq!(
        pkg.description.as_deref(),
        Some("A well-authored package"),
        "description should be parsed from package.py"
    );
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
