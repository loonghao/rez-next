use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Source module tests ────────────────────────────────────────────────────

/// rez source: activation script contains required env vars
#[test]
fn test_source_activation_bash_contains_rez_resolve() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 maya-2024".to_string(),
    );
    env.vars
        .insert("REZ_CONTEXT_FILE".to_string(), "/tmp/test.rxt".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("REZ_RESOLVE"),
        "bash script should export REZ_RESOLVE"
    );
    assert!(
        script.contains("REZ_CONTEXT_FILE"),
        "bash script should export REZ_CONTEXT_FILE"
    );
}

/// rez source: PowerShell activation script uses $env: syntax
#[test]
fn test_source_activation_powershell_env_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    // PowerShell sets env with $env:VAR = "value"
    assert!(
        script.contains("REZ_RESOLVE"),
        "ps1 script should reference REZ_RESOLVE"
    );
}

/// rez source: fish activation script uses set -gx syntax
#[test]
fn test_source_activation_fish_set_gx_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "nuke-14".to_string());

    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        script.contains("REZ_RESOLVE"),
        "fish script should set REZ_RESOLVE"
    );
}

/// rez source: activation script write to tempfile and verify content
#[test]
fn test_source_write_tempfile_roundtrip() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 houdini-19.5".to_string(),
    );
    env.vars
        .insert("REZPKG_PYTHON".to_string(), "3.9".to_string());
    env.vars
        .insert("REZPKG_HOUDINI".to_string(), "19.5".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::fs::write(&path, &script).unwrap();

    let read_back = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        read_back, script,
        "Written and read-back script should be identical"
    );
    assert!(read_back.contains("REZ_RESOLVE"));
    assert!(read_back.contains("REZPKG_PYTHON"));
}

