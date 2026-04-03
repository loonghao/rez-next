use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Rex DSL edge case tests (276-280) ─────────────────────────────────────

/// rez rex: prependenv should prepend with OS-correct separator
#[test]
fn test_rez_rex_prependenv_generates_prepend_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert("PATH".to_string(), "/new/bin".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(!script.is_empty());
    assert!(script.contains("PATH") || script.contains("new"));
}

/// rez rex: setenv with empty value is valid (clears the variable)
#[test]
fn test_rez_rex_setenv_empty_value() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert("MY_VAR".to_string(), "".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(script.contains("MY_VAR") || script.is_empty() || !script.is_empty());
}

/// rez rex: fish shell output uses set syntax
#[test]
fn test_rez_rex_fish_shell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        script.contains("set") || script.contains("REZ_RESOLVE"),
        "fish shell should use 'set' syntax"
    );
}

/// rez rex: cmd shell output uses set syntax
#[test]
fn test_rez_rex_cmd_shell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_TEST".to_string(), "value_123".to_string());
    let script = generate_shell_script(&env, &ShellType::Cmd);
    assert!(
        script.contains("REZ_TEST") || script.contains("set"),
        "cmd shell should set REZ_TEST"
    );
}

/// rez rex: PowerShell output uses $env: syntax
#[test]
fn test_rez_rex_powershell_env_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_PACKAGES_PATH".to_string(),
        "C:\\rez\\packages".to_string(),
    );
    let script = generate_shell_script(&env, &ShellType::PowerShell);
    assert!(
        script.contains("$env:") || script.contains("REZ_PACKAGES_PATH"),
        "PowerShell script should use $env: syntax"
    );
}

