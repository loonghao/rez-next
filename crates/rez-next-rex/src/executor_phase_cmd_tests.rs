//! Tests for RexExecutor — pre/post command phases and command() variable expansion.
//! Split from executor_tests.rs (Cycle 145) to keep file size ≤400 lines.

use crate::executor::RexExecutor;

// ── Phase 91: pre_commands / post_commands execution order ──────────────────

/// pre_commands sets a var, main commands uses it (simulation via sequential execution)
#[test]
fn test_pre_commands_then_commands_sequential() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(
        r#"env.setenv('STAGE', 'pre')"#,
        "mypkg",
        Some("/opt/mypkg/1.0"),
        Some("1.0"),
    )
    .unwrap();

    let env = exec
        .execute_commands(
            r#"env.setenv('STAGE', 'main')"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            Some("1.0"),
        )
        .unwrap();

    assert_eq!(env.vars.get("STAGE"), Some(&"main".to_string()));
}

/// post_commands runs after main — verify it overrides main var
#[test]
fn test_post_commands_overrides_main() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(r#"env.setenv('LOG_LEVEL', 'info')"#, "mypkg", None, None)
        .unwrap();

    let env = exec
        .execute_commands(r#"env.setenv('LOG_LEVEL', 'debug')"#, "mypkg", None, None)
        .unwrap();

    assert_eq!(env.vars.get("LOG_LEVEL"), Some(&"debug".to_string()));
}

/// pre_commands accumulates PATH entries; main commands adds more
#[test]
fn test_pre_and_main_commands_accumulate_path() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(
        r#"env.prepend_path('LD_LIBRARY_PATH', '/opt/common/lib')"#,
        "common",
        None,
        None,
    )
    .unwrap();

    let env = exec
        .execute_commands(
            r#"env.prepend_path('LD_LIBRARY_PATH', '/opt/mypkg/1.0/lib')"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            Some("1.0"),
        )
        .unwrap();

    let ldpath = env.vars.get("LD_LIBRARY_PATH").cloned().unwrap_or_default();
    assert!(
        ldpath.contains("/opt/common/lib"),
        "common lib path should be in LD_LIBRARY_PATH"
    );
    assert!(
        ldpath.contains("/opt/mypkg/1.0/lib"),
        "pkg lib path should be in LD_LIBRARY_PATH"
    );
}

/// pre_build_commands: verify env setup before build (setenv_if_empty semantics)
#[test]
fn test_pre_build_commands_setenv_if_empty() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(
        r#"env.setenv_if_empty('BUILD_TYPE', 'Release')"#,
        "mypkg",
        None,
        None,
    )
    .unwrap();

    let env = exec
        .execute_commands(
            r#"env.setenv_if_empty('BUILD_TYPE', 'Debug')"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();

    assert_eq!(
        env.vars.get("BUILD_TYPE"),
        Some(&"Release".to_string()),
        "setenv_if_empty should not overwrite existing value"
    );
}

/// Verify all actions from pre+main+post recorded with correct source_package
#[test]
fn test_multi_phase_actions_source_tracking() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(r#"env.setenv('PRE_VAR', '1')"#, "pkg_pre", None, None)
        .unwrap();
    exec.execute_commands(r#"env.setenv('MAIN_VAR', '2')"#, "pkg_main", None, None)
        .unwrap();
    exec.execute_commands(r#"env.setenv('POST_VAR', '3')"#, "pkg_post", None, None)
        .unwrap();

    let actions = exec.get_actions();
    assert_eq!(actions.len(), 3, "Should have exactly 3 actions");

    let sources: Vec<_> = actions
        .iter()
        .map(|a| a.source_package.as_deref().unwrap_or(""))
        .collect();
    assert!(sources.contains(&"pkg_pre"), "pkg_pre should be in sources");
    assert!(
        sources.contains(&"pkg_main"),
        "pkg_main should be in sources"
    );
    assert!(
        sources.contains(&"pkg_post"),
        "pkg_post should be in sources"
    );
}

// ── Phase 105: command() variable expansion + multi-command ordering ──────────

/// command() with {root} variable expansion
#[test]
fn test_command_with_root_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"command("{root}/bin/setup.sh")"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            None,
        )
        .unwrap();
    assert!(
        !env.startup_commands.is_empty(),
        "startup_commands should not be empty"
    );
    let cmd = &env.startup_commands[0];
    assert!(
        cmd.contains("/opt/mypkg/1.0/bin/setup.sh"),
        "Root should be expanded in command: {}",
        cmd
    );
}

/// command() with {version} variable expansion
#[test]
fn test_command_with_version_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"command("echo Installing version {version}")"#,
            "mypkg",
            None,
            Some("3.2.1"),
        )
        .unwrap();
    assert_eq!(env.startup_commands.len(), 1);
    assert!(
        env.startup_commands[0].contains("3.2.1"),
        "Version should be expanded: {}",
        env.startup_commands[0]
    );
}

/// Multiple command() calls produce multiple startup_commands in order
#[test]
fn test_multiple_commands_order_preserved() {
    let mut exec = RexExecutor::new();
    let commands = r#"
command("first_cmd")
command("second_cmd")
command("third_cmd")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(env.startup_commands.len(), 3, "Should have 3 commands");
    assert_eq!(env.startup_commands[0], "first_cmd");
    assert_eq!(env.startup_commands[1], "second_cmd");
    assert_eq!(env.startup_commands[2], "third_cmd");
}

/// command() mixed with setenv preserves both
#[test]
fn test_command_mixed_with_setenv() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv("MY_PKG_HOME", "{root}")
command("{root}/bin/init.sh")
env.prepend_path("PATH", "{root}/bin")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", Some("/opt/mypkg/2.0"), Some("2.0"))
        .unwrap();

    assert_eq!(
        env.vars.get("MY_PKG_HOME"),
        Some(&"/opt/mypkg/2.0".to_string())
    );
    assert!(!env.startup_commands.is_empty());
    assert!(env.startup_commands[0].contains("/opt/mypkg/2.0/bin/init.sh"));
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/opt/mypkg/2.0/bin"))
        .unwrap_or(false));
}

/// command() with custom context variable
#[test]
fn test_command_with_custom_context_var() {
    let mut exec = RexExecutor::new();
    exec.set_context_var("install_prefix", "/usr/local/mypkg");
    let env = exec
        .execute_commands(
            r#"command("{install_prefix}/bin/start")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert_eq!(env.startup_commands.len(), 1);
    assert_eq!(env.startup_commands[0], "/usr/local/mypkg/bin/start");
}

/// No command() calls → startup_commands is empty
#[test]
fn test_no_command_leaves_startup_commands_empty() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"env.setenv("FOO", "bar")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        env.startup_commands.is_empty(),
        "No command() calls should leave startup_commands empty"
    );
}
