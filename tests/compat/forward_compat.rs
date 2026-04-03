use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Forward compatibility tests ────────────────────────────────────────────

/// rez forward: generate shell wrapper scripts
#[test]
fn test_forward_script_bash_contains_exec() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    // Simulate what a forward wrapper does: map a tool to a context env
    let mut env = RexEnvironment::new();
    env.aliases.insert(
        "maya".to_string(),
        "/packages/maya/2024/bin/maya".to_string(),
    );
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("maya"),
        "Bash script should reference the maya alias"
    );
}

#[test]
fn test_forward_script_powershell_contains_alias() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.aliases.insert(
        "houdini".to_string(),
        "/packages/houdini/20.0/bin/houdini".to_string(),
    );
    let script = generate_shell_script(&env, &ShellType::PowerShell);
    assert!(script.contains("houdini"));
}

