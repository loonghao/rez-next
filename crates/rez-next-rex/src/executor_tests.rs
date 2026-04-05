//! Tests for RexExecutor — split from executor.rs to keep file size under 1000 lines.

use super::RexExecutor;
use crate::{generate_shell_script, ShellType};

#[test]
fn test_execute_setenv() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"env.setenv("MY_VAR", "hello")"#, "mypkg", None, None)
        .unwrap();
    assert_eq!(env.vars.get("MY_VAR"), Some(&"hello".to_string()));
}

#[test]
fn test_execute_prepend_path() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.prepend_path("PATH", "/usr/bin")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    let path = env.vars.get("PATH").cloned().unwrap_or_default();
    assert!(path.contains("/usr/bin"));
}

#[test]
fn test_execute_append_path() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.append_path("PYTHONPATH", "/opt/lib")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert!(env
        .vars
        .get("PYTHONPATH")
        .map(|v| v.contains("/opt/lib"))
        .unwrap_or(false));
}

#[test]
fn test_execute_unsetenv() {
    let mut exec = RexExecutor::new();
    exec.execute_commands(r#"env.setenv("TO_REMOVE", "value")"#, "pkg", None, None)
        .unwrap();
    let env = exec
        .execute_commands(r#"env.unsetenv("TO_REMOVE")"#, "pkg", None, None)
        .unwrap();
    assert!(!env.vars.contains_key("TO_REMOVE"));
}

#[test]
fn test_execute_alias() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.alias("mymaya", "/opt/maya/bin/maya")"#,
            "maya",
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        env.aliases.get("mymaya"),
        Some(&"/opt/maya/bin/maya".to_string())
    );
}

#[test]
fn test_context_variable_expansion_root() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv("MY_ROOT", "{root}")"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            None,
        )
        .unwrap();
    assert_eq!(env.vars.get("MY_ROOT"), Some(&"/opt/mypkg/1.0".to_string()));
}

#[test]
fn test_context_variable_expansion_version() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv("PKG_VERSION", "{version}")"#,
            "mypkg",
            None,
            Some("2.1.0"),
        )
        .unwrap();
    assert_eq!(env.vars.get("PKG_VERSION"), Some(&"2.1.0".to_string()));
}

#[test]
fn test_multiple_commands_applied_in_order() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv("FIRST", "1")
env.setenv("SECOND", "2")
env.setenv("THIRD", "3")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(env.vars.get("FIRST"), Some(&"1".to_string()));
    assert_eq!(env.vars.get("SECOND"), Some(&"2".to_string()));
    assert_eq!(env.vars.get("THIRD"), Some(&"3".to_string()));
}

#[test]
fn test_set_context_var() {
    let mut exec = RexExecutor::new();
    exec.set_context_var("custom_var", "custom_value");
    let env = exec
        .execute_commands(
            r#"env.setenv("RESULT", "{custom_var}")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert_eq!(env.vars.get("RESULT"), Some(&"custom_value".to_string()));
}

#[test]
fn test_empty_commands_returns_empty_env() {
    let mut exec = RexExecutor::new();
    let env = exec.execute_commands("", "mypkg", None, None).unwrap();
    assert!(env.vars.is_empty());
    assert!(env.aliases.is_empty());
}

#[test]
fn test_clear_resets_actions() {
    let mut exec = RexExecutor::new();
    exec.execute_commands(r#"env.setenv("A", "1")"#, "p", None, None)
        .unwrap();
    exec.clear();
    assert_eq!(exec.get_actions().len(), 0);
}

// ── Phase 74: package.py commands field end-to-end simulation ─────────────

/// Simulate a typical maya package.py commands block
#[test]
fn test_package_commands_maya_simulation() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv('MAYA_VERSION', '{version}')
env.setenv('MAYA_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
alias('maya', '{root}/bin/maya')
"#;
    let env = exec
        .execute_commands(commands, "maya", Some("/opt/maya/2024.1"), Some("2024.1"))
        .unwrap();

    assert_eq!(env.vars.get("MAYA_VERSION"), Some(&"2024.1".to_string()));
    assert_eq!(
        env.vars.get("MAYA_ROOT"),
        Some(&"/opt/maya/2024.1".to_string())
    );
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/opt/maya/2024.1/bin"))
        .unwrap_or(false));
    assert!(env
        .vars
        .get("LD_LIBRARY_PATH")
        .map(|v| v.contains("/opt/maya/2024.1/lib"))
        .unwrap_or(false));
    assert_eq!(
        env.aliases.get("maya"),
        Some(&"/opt/maya/2024.1/bin/maya".to_string())
    );
}

/// Simulate a python package.py commands block
#[test]
fn test_package_commands_python_simulation() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv('PYTHONHOME', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
env.setenv_if_empty('PYTHON_VERSION', '{version}')
"#;
    let env = exec
        .execute_commands(commands, "python", Some("/usr/local"), Some("3.11.0"))
        .unwrap();

    assert_eq!(env.vars.get("PYTHONHOME"), Some(&"/usr/local".to_string()));
    assert!(env
        .vars
        .get("PYTHONPATH")
        .map(|v| v.contains("site-packages"))
        .unwrap_or(false));
    assert_eq!(env.vars.get("PYTHON_VERSION"), Some(&"3.11.0".to_string()));
}

/// Simulate two packages being applied sequentially (PATH accumulation)
#[test]
fn test_sequential_package_commands_path_accumulation() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(
        r#"env.prepend_path('PATH', '/opt/python/bin')"#,
        "python",
        None,
        None,
    )
    .unwrap();

    let env = exec
        .execute_commands(
            r#"env.prepend_path('PATH', '/opt/maya/bin')"#,
            "maya",
            None,
            None,
        )
        .unwrap();

    let path = env.vars.get("PATH").cloned().unwrap_or_default();
    assert!(
        path.contains("/opt/maya/bin"),
        "maya bin should be in PATH: {}",
        path
    );
    assert!(
        path.contains("/opt/python/bin"),
        "python bin should be in PATH: {}",
        path
    );
    let maya_pos = path.find("/opt/maya/bin").unwrap();
    let python_pos = path.find("/opt/python/bin").unwrap();
    assert!(maya_pos < python_pos, "maya should precede python in PATH");
}

/// Simulate setenv_if_empty: second pkg should not overwrite first pkg's value
#[test]
fn test_setenv_if_empty_does_not_overwrite() {
    let mut exec = RexExecutor::new();

    exec.execute_commands(r#"env.setenv('RENDERER', 'arnold')"#, "arnold", None, None)
        .unwrap();

    let env = exec
        .execute_commands(
            r#"env.setenv_if_empty('RENDERER', 'prman')"#,
            "prman",
            None,
            None,
        )
        .unwrap();

    assert_eq!(env.vars.get("RENDERER"), Some(&"arnold".to_string()));
}

/// Package with comment lines and blank lines mixed
#[test]
fn test_package_commands_with_comments_and_blanks() {
    let mut exec = RexExecutor::new();
    let commands = r#"
# Setup the root path
env.setenv('HOUDINI_PATH', '{root}')

# Add to PATH
env.prepend_path('PATH', '{root}/bin')

# Aliases
alias('houdini', '{root}/bin/houdini')
alias('hython', '{root}/bin/hython')
"#;
    let env = exec
        .execute_commands(commands, "houdini", Some("/opt/houdini/20.0"), Some("20.0"))
        .unwrap();

    assert_eq!(
        env.vars.get("HOUDINI_PATH"),
        Some(&"/opt/houdini/20.0".to_string())
    );
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/opt/houdini/20.0/bin"))
        .unwrap_or(false));
    assert_eq!(
        env.aliases.get("houdini"),
        Some(&"/opt/houdini/20.0/bin/houdini".to_string())
    );
    assert_eq!(
        env.aliases.get("hython"),
        Some(&"/opt/houdini/20.0/bin/hython".to_string())
    );
}

/// Verify action source_package is recorded correctly
#[test]
fn test_actions_have_correct_source_package() {
    let mut exec = RexExecutor::new();
    exec.execute_commands(r#"env.setenv('TEST_VAR', 'hello')"#, "testpkg", None, None)
        .unwrap();

    let actions = exec.get_actions();
    assert!(!actions.is_empty());
    assert_eq!(actions[0].source_package, Some("testpkg".to_string()));
}

/// Shell script integration: execute commands then generate bash activation script
#[test]
fn test_execute_then_generate_bash_script() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"
env.setenv('PKG_ROOT', '/opt/pkg/1.0')
env.prepend_path('PATH', '/opt/pkg/1.0/bin')
alias('pkg', '/opt/pkg/1.0/bin/pkg')
"#,
            "pkg",
            Some("/opt/pkg/1.0"),
            Some("1.0"),
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(script.contains("export PKG_ROOT="));
    assert!(script.contains("export PATH="));
    assert!(script.contains("alias pkg="));
}

/// Verify unsetenv inside package commands removes a previously set var
#[test]
fn test_package_commands_unsetenv() {
    let mut exec = RexExecutor::new();
    exec.execute_commands(r#"env.setenv('LEGACY_VAR', 'old')"#, "pkgA", None, None)
        .unwrap();
    let env = exec
        .execute_commands(r#"env.unsetenv('LEGACY_VAR')"#, "pkgB", None, None)
        .unwrap();
    assert!(!env.vars.contains_key("LEGACY_VAR"));
}

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

// ── Phase 120 / Cycle 62: stop() / error() boundary tests ────────────────────

/// stop() with no message sets stopped=true and stop_message=None
#[test]
fn test_stop_no_message_sets_stopped_flag() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop()"#, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped, "stop() should set stopped=true");
    assert!(
        env.stop_message.is_none(),
        "stop() with no message should leave stop_message=None"
    );
}

/// stop("msg") sets both stopped=true and stop_message
#[test]
fn test_stop_with_message_sets_both_fields() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop("hard stop triggered")"#, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped, "stop() should set stopped=true");
    assert_eq!(
        env.stop_message.as_deref(),
        Some("hard stop triggered"),
        "stop message should match"
    );
}

/// stop() message with {root} variable expansion
#[test]
fn test_stop_message_variable_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("conflict at {root}")"#,
            "mypkg",
            Some("/opt/mypkg/1.0"),
            None,
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("conflict at /opt/mypkg/1.0"),
        "stop message should have {{root}} expanded"
    );
}

/// stop() message with {version} expansion
#[test]
fn test_stop_message_version_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("version {version} not supported")"#,
            "mypkg",
            None,
            Some("2.0.0"),
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("version 2.0.0 not supported")
    );
}

/// stop() aborts processing — actions after stop() are NOT applied
#[test]
fn test_actions_after_stop_are_not_applied() {
    let mut exec = RexExecutor::new();
    let commands = r#"
env.setenv("BEFORE_STOP", "yes")
stop()
env.setenv("AFTER_STOP", "yes")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.vars.get("BEFORE_STOP"),
        Some(&"yes".to_string()),
        "BEFORE_STOP should be set"
    );
    assert!(
        !env.vars.contains_key("AFTER_STOP"),
        "AFTER_STOP must not be set because stop() aborts processing"
    );
}

/// error() records message with [error] prefix in info_messages
#[test]
fn test_error_action_recorded_in_info_messages() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"error("something went wrong")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        !env.info_messages.is_empty(),
        "error() should produce an info message"
    );
    let msg = &env.info_messages[0];
    assert!(
        msg.contains("something went wrong"),
        "info_messages should contain the error text: {}",
        msg
    );
    assert!(
        msg.starts_with("[error]"),
        "error message should be prefixed with [error]: {}",
        msg
    );
}

/// error() message with {root} variable expansion
#[test]
fn test_error_action_variable_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"error("failed to find {root}/lib")"#,
            "mypkg",
            Some("/opt/mypkg/3.0"),
            None,
        )
        .unwrap();
    assert!(!env.info_messages.is_empty());
    let msg = &env.info_messages[0];
    assert!(
        msg.contains("/opt/mypkg/3.0/lib"),
        "error message should have {{root}} expanded: {}",
        msg
    );
}

/// Multiple error() calls all recorded in order
#[test]
fn test_multiple_error_actions_all_recorded() {
    let mut exec = RexExecutor::new();
    let commands = r#"
error("first error")
error("second error")
error("third error")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(
        env.info_messages.len(),
        3,
        "All three error() calls should be recorded"
    );
    assert!(env.info_messages[0].contains("first error"));
    assert!(env.info_messages[1].contains("second error"));
    assert!(env.info_messages[2].contains("third error"));
}

/// error() does NOT set stopped=true
#[test]
fn test_error_action_does_not_set_stopped() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"error("non-fatal issue")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        !env.stopped,
        "error() should not set stopped flag (only stop() does)"
    );
    assert!(
        env.stop_message.is_none(),
        "error() should not set stop_message"
    );
}

/// stop() does NOT add to info_messages
#[test]
fn test_stop_action_does_not_populate_info_messages() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(r#"stop("quit now")"#, "mypkg", None, None)
        .unwrap();
    assert!(
        env.info_messages.is_empty(),
        "stop() should not produce info messages, only set stopped flag"
    );
}

/// Combined: error() followed by stop() — both effects recorded independently
#[test]
fn test_error_then_stop_combined() {
    let mut exec = RexExecutor::new();
    let commands = r#"
error("pre-stop warning")
stop("final halt")
"#;
    let env = exec
        .execute_commands(commands, "mypkg", None, None)
        .unwrap();
    assert_eq!(env.info_messages.len(), 1);
    assert!(env.info_messages[0].contains("pre-stop warning"));
    assert!(env.stopped);
    assert_eq!(env.stop_message.as_deref(), Some("final halt"));
}

/// stop() with {name} (package name) expansion
#[test]
fn test_stop_message_name_expansion() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"stop("{name} requires Python 3")"#,
            "mypackage",
            None,
            None,
        )
        .unwrap();
    assert!(env.stopped);
    assert_eq!(
        env.stop_message.as_deref(),
        Some("mypackage requires Python 3")
    );
}

/// error() with custom context variable expansion
#[test]
fn test_error_action_custom_context_var_expansion() {
    let mut exec = RexExecutor::new();
    exec.set_context_var("expected_arch", "x86_64");
    let env = exec
        .execute_commands(
            r#"error("expected arch: {expected_arch}")"#,
            "mypkg",
            None,
            None,
        )
        .unwrap();
    assert!(!env.info_messages.is_empty());
    assert!(
        env.info_messages[0].contains("x86_64"),
        "Custom context var should be expanded in error message: {}",
        env.info_messages[0]
    );
}
