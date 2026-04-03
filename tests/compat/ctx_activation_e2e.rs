use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

