use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Rex DSL boundary tests (additional) ──────────────────────────────────────

/// Rex: executing empty commands block returns empty env, does not error
#[test]
fn test_rex_empty_commands_no_error() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let result = exec.execute_commands("", "empty_pkg", None, None);
    assert!(
        result.is_ok(),
        "Empty commands block should not produce an error"
    );
}

/// Rex: setenv then prepend_path on same var accumulates correctly
#[test]
fn test_rex_setenv_then_prepend_path() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"env.setenv('MYPATH', '/base')
env.prepend_path('MYPATH', '/extra')
"#;
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    let val = env.vars.get("MYPATH").cloned().unwrap_or_default();
    assert!(
        val.contains("/extra"),
        "MYPATH should contain /extra after prepend"
    );
    assert!(
        val.contains("/base"),
        "MYPATH should still contain /base after prepend"
    );
}

/// Rex: alias command produces correct name → path mapping
#[test]
fn test_rex_alias_name_path_mapping() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = "alias('mytool', '/opt/mytool/1.0/bin/mytool')";
    let env = exec
        .execute_commands(cmds, "mytoolpkg", None, None)
        .unwrap();
    assert_eq!(
        env.aliases.get("mytool"),
        Some(&"/opt/mytool/1.0/bin/mytool".to_string()),
        "alias should map 'mytool' → '/opt/mytool/1.0/bin/mytool'"
    );
}

