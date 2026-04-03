use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Rex DSL completeness tests ───────────────────────────────────────────────

/// Rex: unsetenv should remove a previously set variable
#[test]
fn test_rex_unsetenv_removes_var() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = "env.setenv('TEMP_VAR', 'temp_value')\nenv.unsetenv('TEMP_VAR')";
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    // After unsetenv, the variable should not be present or be empty
    let val = env.vars.get("TEMP_VAR");
    assert!(
        val.is_none() || val.map(|s| s.is_empty()).unwrap_or(false),
        "TEMP_VAR should be unset after unsetenv"
    );
}

/// Rex: multiple path prepends should accumulate correctly
#[test]
fn test_rex_multiple_prepend_path_order() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"env.prepend_path('MYPATH', '/first')
env.prepend_path('MYPATH', '/second')
"#;
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    let path_val = env.vars.get("MYPATH").cloned().unwrap_or_default();
    // /second should come before /first (last prepend wins front position)
    let second_pos = path_val.find("/second");
    let first_pos = path_val.find("/first");
    assert!(
        second_pos.is_some() && first_pos.is_some(),
        "Both paths should be present"
    );
    assert!(
        second_pos.unwrap() <= first_pos.unwrap(),
        "/second (last prepended) should appear before /first"
    );
}

/// Rex: shell script generation for bash contains expected variable export
#[test]
fn test_rex_bash_script_contains_export() {
    use rez_next_rex::{generate_shell_script, RexExecutor, ShellType};

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            "env.setenv('REZ_TEST_VAR', 'hello_bash')",
            "testpkg",
            None,
            None,
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("REZ_TEST_VAR"),
        "Bash script should contain variable name"
    );
    assert!(
        script.contains("hello_bash"),
        "Bash script should contain variable value"
    );
}

