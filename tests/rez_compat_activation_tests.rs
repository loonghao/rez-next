//! Rez Compat — Context Activation Scripts, Weak Dependencies, PackageSerializer,
//! Rex script generation, and Version ordering tests
//!
//! Extracted from rez_compat_late_tests.rs (Cycle 75).
use rez_core::version::{Version, VersionRange};
use rez_next_package::PackageRequirement;
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

// ─── Context activation script E2E tests (296-300) ──────────────────────────

#[test]
fn test_context_activation_bash_sets_rez_env_vars() {
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    env.vars.insert(
        "REZ_USED_PACKAGES_PATH".to_string(),
        "/packages".to_string(),
    );
    env.vars
        .insert("PATH".to_string(), "/packages/python/3.9/bin".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);

    assert!(
        script.contains("REZ_RESOLVE"),
        "bash script must contain REZ_RESOLVE"
    );
    assert!(script.contains("PATH"), "bash script must contain PATH");
    assert!(
        script.contains("export") || script.contains("="),
        "bash script must have assignment syntax"
    );
}

#[test]
fn test_context_activation_powershell_syntax() {
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "maya-2024".to_string());
    env.vars.insert(
        "MAYA_LOCATION".to_string(),
        "C:\\Autodesk\\Maya2024".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::PowerShell);

    assert!(
        script.contains("$env:") || script.contains("REZ_RESOLVE"),
        "PowerShell activation script must use $env: syntax or contain var name, got: {}",
        &script[..script.len().min(300)]
    );
}

#[test]
fn test_context_activation_fish_set_syntax() {
    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_CONTEXT_FILE".to_string(),
        "/tmp/rez_context.rxt".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        !script.is_empty(),
        "fish activation script must not be empty"
    );
    assert!(
        script.contains("set") || script.contains("REZ_CONTEXT_FILE"),
        "fish script should use set syntax or contain var name"
    );
}

#[test]
fn test_context_activation_cmd_set_syntax() {
    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_PACKAGES_PATH".to_string(),
        "C:\\rez\\packages;D:\\rez\\packages".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Cmd);
    assert!(
        !script.is_empty(),
        "cmd activation script must not be empty"
    );
    assert!(
        script.to_uppercase().contains("SET") || script.contains("REZ_PACKAGES_PATH"),
        "cmd script should use SET command or contain var name"
    );
}

#[test]
fn test_context_activation_multiple_packages() {
    let mut env = RexEnvironment::new();
    env.vars.insert(
        "PYTHON_ROOT".to_string(),
        "/packages/python/3.9".to_string(),
    );
    env.vars
        .insert("MAYA_ROOT".to_string(), "/packages/maya/2024".to_string());
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 maya-2024".to_string(),
    );
    env.aliases.insert(
        "python".to_string(),
        "/packages/python/3.9/bin/python".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("PYTHON_ROOT"),
        "script must contain PYTHON_ROOT"
    );
    assert!(
        script.contains("MAYA_ROOT"),
        "script must contain MAYA_ROOT"
    );
    assert!(
        script.contains("REZ_RESOLVE"),
        "script must contain REZ_RESOLVE"
    );
}

// ─── Solver weak dependency (~pkg) tests (301-304) ──────────────────────────

#[test]
fn test_solver_weak_requirement_default_false() {
    let normal = PackageRequirement::parse("python").unwrap();
    assert!(
        !normal.weak,
        "Normal requirement 'python' should not be weak"
    );

    let with_ver = PackageRequirement::parse("python-3.9").unwrap();
    assert!(
        !with_ver.weak,
        "Versioned requirement 'python-3.9' should not be weak"
    );
}

#[test]
fn test_solver_weak_requirement_name_preserved() {
    let weak_req = PackageRequirement {
        name: "numpy".to_string(),
        version_spec: None,
        weak: true,
        conflict: false,
    };
    assert_eq!(weak_req.name(), "numpy");
    assert!(
        weak_req.weak,
        "Explicitly set weak=true should be preserved"
    );
}

#[test]
fn test_solver_weak_no_conflict_if_compatible() {
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "numpy".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Non-conflicting requirements should yield no conflicts"
    );
}

#[test]
fn test_solver_disjoint_ranges_produce_conflict() {
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            ">=4.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint requirements >=4.0 and <3.0 should produce conflict"
    );
}

// ─── PackageSerializer commands field tests (305-308) ───────────────────────

#[test]
fn test_serializer_package_with_commands_function() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'testpkg'
version = '2.0.0'
description = 'package with commands'
def commands():
    env.setenv('TESTPKG_ROOT', '{root}')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "testpkg");
    let has_commands = pkg.commands.is_some() || pkg.commands_function.is_some();
    assert!(
        has_commands,
        "Package with def commands() should have commands populated"
    );
}

#[test]
fn test_serializer_package_with_pre_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'prepkg'
version = '1.5.0'
def pre_commands():
    env.setenv('PREPKG_SETUP', '1')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "prepkg");
}

#[test]
fn test_serializer_package_with_post_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'postpkg'
version = '0.5.0'
def post_commands():
    env.setenv('POST_DONE', '1')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "postpkg");
}

#[test]
fn test_serializer_package_commands_string_form() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'strpkg'
version = '3.0.0'
commands = "env.setenv('STRPKG_HOME', '{root}')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "strpkg");
}

// ─── Phase 136-143: Rex info messages, ShellType, env var CRUD ───────────────

#[test]
fn test_rex_info_messages_order_in_script() {
    let mut env = RexEnvironment::new();
    env.info_messages.push("first message".to_string());
    env.info_messages.push("second message".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    let pos1 = script.find("first message").unwrap();
    let pos2 = script.find("second message").unwrap();
    assert!(pos1 < pos2, "Info messages should appear in order");
}

#[test]
fn test_shell_type_parse_case_insensitive() {
    assert_eq!(ShellType::parse("BASH"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("bash"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("Bash"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("POWERSHELL"), Some(ShellType::PowerShell));
    assert_eq!(ShellType::parse("Fish"), Some(ShellType::Fish));
}

#[test]
fn test_rex_environment_env_var_crud() {
    let mut env = RexEnvironment::new();
    env.vars.insert("MY_VAR".to_string(), "initial".to_string());
    assert_eq!(env.vars.get("MY_VAR"), Some(&"initial".to_string()));

    env.vars.insert("MY_VAR".to_string(), "updated".to_string());
    assert_eq!(env.vars.get("MY_VAR"), Some(&"updated".to_string()));

    env.vars.remove("MY_VAR");
    assert!(!env.vars.contains_key("MY_VAR"));
}

#[test]
fn test_rex_zsh_identical_to_bash() {
    let mut env = RexEnvironment::new();
    env.vars
        .insert("PKG_ROOT".to_string(), "/opt/pkg".to_string());
    env.aliases
        .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());

    let bash = generate_shell_script(&env, &ShellType::Bash);
    let zsh = generate_shell_script(&env, &ShellType::Zsh);
    assert_eq!(bash, zsh, "Zsh script should be identical to bash script");
}

#[test]
fn test_rex_empty_env_has_header_all_shells() {
    let env = RexEnvironment::new();
    for shell in [ShellType::Bash, ShellType::Fish, ShellType::PowerShell] {
        let script = generate_shell_script(&env, &shell);
        assert!(
            !script.is_empty(),
            "Even empty env should produce non-empty script (header)"
        );
        assert!(
            script.contains("Generated by rez-next rex"),
            "Script should have generator header for {:?}",
            shell
        );
    }
}

#[test]
fn test_version_range_union_covers_both() {
    let r1 = VersionRange::parse("1.0+<2.0").unwrap();
    let r2 = VersionRange::parse("3.0+<4.0").unwrap();
    let union = r1.union(&r2);
    let v1 = Version::parse("1.5").unwrap();
    let v2 = Version::parse("3.5").unwrap();
    let v3 = Version::parse("2.5").unwrap();
    assert!(union.contains(&v1), "Union should contain 1.5");
    assert!(union.contains(&v2), "Union should contain 3.5");
    assert!(!union.contains(&v3), "Union should not contain 2.5");
}

#[test]
fn test_version_range_open_upper_contains_versions() {
    let open_range = VersionRange::parse("1.0+").unwrap();
    let v_lo = Version::parse("1.0").unwrap();
    let v_hi = Version::parse("99.99.99").unwrap();
    let v_below = Version::parse("0.9.9").unwrap();
    assert!(
        open_range.contains(&v_lo),
        "Open range should contain lower bound"
    );
    assert!(
        open_range.contains(&v_hi),
        "Open range should contain high version"
    );
    assert!(
        !open_range.contains(&v_below),
        "Open range should not contain version below"
    );
}

#[test]
fn test_requirement_weak_field_from_tilde_prefix() {
    let req = PackageRequirement::parse("~python").unwrap();
    assert!(
        req.weak,
        "Requirement starting with ~ should have weak=true"
    );
    assert_eq!(req.name, "python");

    let normal = PackageRequirement::parse("python").unwrap();
    assert!(!normal.weak, "Normal requirement should not be weak");
}

#[test]
fn test_requirement_standard_not_weak() {
    let req = PackageRequirement::parse("python").unwrap();
    assert!(!req.weak, "Standard requirement should have weak=false");
}

#[test]
fn test_package_variant_multi_req_roundtrip() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"
name = 'vartest'
version = '1.0.0'
variants = [
    ['python-3.7', 'maya-2022'],
    ['python-3.9', 'maya-2024'],
]
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "vartest");
    assert_eq!(
        pkg.variants.len(),
        2,
        "vartest should have 2 variant entries, got {}",
        pkg.variants.len()
    );
}

#[test]
fn test_rex_bash_alias_with_special_chars() {
    let mut env = RexEnvironment::new();
    env.aliases
        .insert("mypkg".to_string(), "/opt/my pkg/bin/run".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("mypkg"),
        "Alias name 'mypkg' should appear in bash script"
    );
}

#[test]
fn test_version_alphanumeric_ordering() {
    let v_alpha = Version::parse("1.0.alpha").unwrap();
    let v_zero = Version::parse("1.0.0").unwrap();
    assert!(
        v_alpha < v_zero,
        "rez ordering: '1.0.alpha' should be less than '1.0.0' (alpha < numeric)"
    );
}
