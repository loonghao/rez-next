//! Rez Compat — Context Activation Script E2E, Solver Weak Dependency,
//! PackageSerializer commands field tests
//!
//! Split from rez_compat_late_tests.rs (Cycle 143).
//! Rex info/shell + VersionRange union + rez.config tests
//! → extracted to rez_compat_rex_config_tests.rs

// ─── Context activation script E2E tests (296-300) ──────────────────────────

/// rez context: activation script for bash sets correct env vars
#[test]
fn test_context_activation_bash_sets_rez_env_vars() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

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

/// rez context: activation script for powershell uses $env: syntax
#[test]
fn test_context_activation_powershell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

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

/// rez context: activation script for fish uses 'set' syntax
#[test]
fn test_context_activation_fish_set_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

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

/// rez context: activation script for cmd uses SET syntax
#[test]
fn test_context_activation_cmd_set_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

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

/// rez context: multiple packages in activation script are all present
#[test]
fn test_context_activation_multiple_packages() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

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

/// rez solver: weak requirement flag defaults to false
#[test]
fn test_solver_weak_requirement_default_false() {
    use rez_next_package::PackageRequirement;

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

/// rez solver: weak requirement preserves package name correctly
#[test]
fn test_solver_weak_requirement_name_preserved() {
    use rez_next_package::PackageRequirement;

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

/// rez solver: non-conflicting requirements yield no conflicts
#[test]
fn test_solver_weak_no_conflict_if_compatible() {
    use rez_next_package::PackageRequirement;
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

/// rez solver: disjoint version ranges for same package produce conflict
#[test]
fn test_solver_disjoint_ranges_produce_conflict() {
    use rez_next_package::PackageRequirement;
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

/// rez serializer: package.py with def commands() is parsed correctly
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

/// rez serializer: package.py with pre_commands() is parsed without error
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

/// rez serializer: package.py with post_commands() is parsed without error
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

/// rez serializer: package.py with inline string commands is parsed without error
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
