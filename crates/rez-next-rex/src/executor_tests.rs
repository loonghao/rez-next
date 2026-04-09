//! Tests for RexExecutor — basic env operations and package.py simulation.
//! Phase/command ordering tests → executor_phase_cmd_tests.rs
//! stop()/error() behaviour tests → executor_stop_error_tests.rs

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
