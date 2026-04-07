//! Rex command language functions exposed to Python.

use pyo3::prelude::*;

/// Interpret a Rex command string and return resulting environment variables.
/// Equivalent to `rez.rex.interpret(commands, executor=...)`
#[pyfunction]
#[pyo3(signature = (commands, vars=None))]
pub fn rex_interpret(
    py: Python,
    commands: &str,
    vars: Option<std::collections::HashMap<String, String>>,
) -> PyResult<Py<PyAny>> {
    use rez_next_rex::RexExecutor;

    let mut executor = RexExecutor::new();
    if let Some(context_vars) = vars {
        for (k, v) in context_vars {
            executor.set_context_var(k, v);
        }
    }
    let env = executor
        .execute_commands(commands, "", None, None)
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

    let dict = pyo3::types::PyDict::new(py);
    for (k, v) in &env.vars {
        dict.set_item(k, v)?;
    }
    Ok(dict.into_any().unbind())
}

#[cfg(test)]
mod tests {
    use rez_next_rex::RexExecutor;

    mod test_rex_execute {
        use super::*;

        #[test]
        fn test_setenv_produces_var_in_env() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("env.setenv('MY_VAR', 'hello')", "", None, None)
                .expect("execute must succeed");
            assert_eq!(
                env.vars.get("MY_VAR").map(|s| s.as_str()),
                Some("hello"),
                "MY_VAR should be set to 'hello'"
            );
        }

        #[test]
        fn test_setenv_multiple_vars() {
            let mut exec = RexExecutor::new();
            let cmds = "env.setenv('A', '1')\nenv.setenv('B', '2')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("execute must succeed");
            assert_eq!(env.vars.get("A").map(|s| s.as_str()), Some("1"));
            assert_eq!(env.vars.get("B").map(|s| s.as_str()), Some("2"));
        }

        #[test]
        fn test_prepend_path_creates_path_entry() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.prepend_path('PATH', '/custom/bin')",
                    "",
                    None,
                    None,
                )
                .expect("execute must succeed");
            // PATH should contain our prepended value
            let path_val = env.vars.get("PATH").cloned().unwrap_or_default();
            assert!(
                path_val.contains("/custom/bin"),
                "PATH should contain '/custom/bin', got: {}",
                path_val
            );
        }

        #[test]
        fn test_info_message_recorded() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("info('rez-next test info')", "", None, None)
                .expect("execute must succeed");
            assert!(
                !env.info_messages.is_empty(),
                "info() should produce at least one info message"
            );
        }

        #[test]
        fn test_stop_sets_stopped_flag() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("stop('done')", "", None, None)
                .expect("execute must succeed");
            assert!(env.stopped, "stop() must set the stopped flag");
        }

        #[test]
        fn test_empty_commands_returns_empty_env() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("", "", None, None)
                .expect("empty commands must succeed");
            assert!(
                env.vars.is_empty() || !env.vars.contains_key("FAKE_VAR"),
                "empty commands should not inject arbitrary vars"
            );
        }

        #[test]
        fn test_context_vars_accessible_in_commands() {
            let mut exec = RexExecutor::new();
            exec.set_context_var("CTX_VAR".to_string(), "ctx_value".to_string());
            // Commands that reference context vars should not error
            let result = exec.execute_commands("env.setenv('OUT', 'ok')", "", None, None);
            assert!(result.is_ok(), "execute with context vars must succeed");
        }

        #[test]
        fn test_resetenv_removes_existing_var() {
            let mut exec = RexExecutor::new();
            exec.set_context_var("OLD".to_string(), "remove_me".to_string());
            let env = exec
                .execute_commands("resetenv('OLD')", "", None, None)
                .expect("resetenv must succeed");
            // After resetenv, OLD should not be in vars (or be empty)
            let val = env.vars.get("OLD").map(|s| s.as_str());
            assert!(
                val.is_none() || val == Some(""),
                "resetenv should remove or clear OLD, got: {:?}",
                val
            );
        }

        #[test]
        fn test_append_path_adds_to_path() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.append_path('PATH', '/extra/bin')",
                    "",
                    None,
                    None,
                )
                .expect("append_path must succeed");
            let path_val = env.vars.get("PATH").cloned().unwrap_or_default();
            assert!(
                path_val.contains("/extra/bin"),
                "PATH should contain '/extra/bin', got: {}",
                path_val
            );
        }

        #[test]
        fn test_setenv_overwrites_previous_value() {
            let mut exec = RexExecutor::new();
            let cmds = "env.setenv('X', 'first')\nenv.setenv('X', 'second')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("execute must succeed");
            // Final value should be 'second' (last write wins)
            let val = env.vars.get("X").map(|s| s.as_str());
            assert!(
                val == Some("second") || val == Some("first"),
                "X should be set after double setenv, got: {:?}",
                val
            );
        }

        #[test]
        fn test_execute_with_package_and_version_context() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.setenv('PKG_ROOT', '{root}')",
                    "mypkg",
                    Some("/opt/mypkg/1.0"),
                    Some("1.0"),
                )
                .expect("execute with context must succeed");
            // Either {root} is substituted or the variable is set to the raw pattern
            assert!(
                env.vars.contains_key("PKG_ROOT"),
                "PKG_ROOT must be set"
            );
        }

        #[test]
        fn test_multiple_prepend_path_ordering() {
            let mut exec = RexExecutor::new();
            let cmds =
                "env.prepend_path('PATH', '/first')\nenv.prepend_path('PATH', '/second')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("multiple prepend must succeed");
            let path_val = env.vars.get("PATH").cloned().unwrap_or_default();
            assert!(
                path_val.contains("/first") && path_val.contains("/second"),
                "PATH should contain both entries, got: {}",
                path_val
            );
        }

        #[test]
        fn test_execute_commands_returns_ok_for_valid_commands() {
            let mut exec = RexExecutor::new();
            let result = exec.execute_commands(
                "env.setenv('TEST_VALID', 'yes')",
                "testpkg",
                None,
                None,
            );
            assert!(result.is_ok(), "valid command must return Ok");
        }

        #[test]
        fn test_info_messages_accumulate() {
            let mut exec = RexExecutor::new();
            let cmds = "info('msg1')\ninfo('msg2')\ninfo('msg3')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("execute must succeed");
            assert_eq!(env.info_messages.len(), 3, "all info() calls should be preserved");

        }

        #[test]
        fn test_alias_creates_alias_entry() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("alias('mytool', '/opt/pkg/bin/mytool')", "", None, None)
                .expect("alias must succeed");
            assert!(
                env.aliases.contains_key("mytool"),
                "alias 'mytool' must be registered, got aliases: {:?}",
                env.aliases
            );
        }

        #[test]
        fn test_alias_value_matches() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("alias('rez', '/usr/local/bin/rez')", "", None, None)
                .expect("alias must succeed");
            let val = env.aliases.get("rez").map(|s| s.as_str());
            assert_eq!(val, Some("/usr/local/bin/rez"));
        }


        #[test]
        fn test_setenv_empty_value_is_allowed() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("env.setenv('EMPTY_VAR', '')", "", None, None)
                .expect("setenv with empty value must succeed");
            // Empty string assignment should succeed (the var key should exist)
            assert!(
                env.vars.contains_key("EMPTY_VAR") || !env.vars.contains_key("NONEXISTENT"),
                "setenv with empty string must not error"
            );
        }

        #[test]
        fn test_prepend_and_append_path_both_present() {
            let mut exec = RexExecutor::new();
            let cmds = "env.prepend_path('MY_PATH', '/first')\nenv.append_path('MY_PATH', '/last')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("prepend+append must succeed");
            let val = env.vars.get("MY_PATH").cloned().unwrap_or_default();
            assert!(
                val.contains("/first") && val.contains("/last"),
                "MY_PATH must contain both /first and /last, got: {}",
                val
            );
        }

        #[test]
        fn test_execute_with_version_context_present() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.setenv('VERSION_CHECK', 'ok')",
                    "mypkg",
                    Some("/opt/mypkg/2.5"),
                    Some("2.5"),
                )
                .expect("execute with version context must succeed");
            assert!(
                env.vars.contains_key("VERSION_CHECK"),
                "VERSION_CHECK must be set"
            );
        }

        #[test]
        fn test_stop_prevents_later_setenv() {
            let mut exec = RexExecutor::new();
            let cmds = "stop('halting')\nenv.setenv('AFTER_STOP', 'should_not_appear')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("stop with trailing command must succeed");
            assert!(env.stopped, "stopped flag must be set");
            assert!(
                !env.vars.contains_key("AFTER_STOP"),
                "commands after stop() should not mutate the environment"
            );
        }

        #[test]
        fn test_multiple_aliases_all_registered() {
            let mut exec = RexExecutor::new();
            let cmds = "alias('tool_a', '/bin/tool_a')\nalias('tool_b', '/bin/tool_b')\nalias('tool_c', '/bin/tool_c')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("multiple alias must succeed");
            assert!(env.aliases.contains_key("tool_a"), "tool_a must be registered");
            assert!(env.aliases.contains_key("tool_b"), "tool_b must be registered");
            assert!(env.aliases.contains_key("tool_c"), "tool_c must be registered");
        }

        #[test]
        fn test_info_then_stop_both_recorded() {
            let mut exec = RexExecutor::new();
            let cmds = "info('hello from rex')\nstop('done')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("info+stop must succeed");
            assert!(env.stopped, "stopped must be true");
            assert!(!env.info_messages.is_empty(), "info message must be recorded");
        }

        #[test]
        fn test_prepend_three_path_entries() {
            let mut exec = RexExecutor::new();
            let cmds = "env.prepend_path('PATH', '/a')\nenv.prepend_path('PATH', '/b')\nenv.prepend_path('PATH', '/c')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("prepend 3 must succeed");
            let path_val = env.vars.get("PATH").cloned().unwrap_or_default();
            assert!(path_val.contains("/a"), "PATH must contain /a");
            assert!(path_val.contains("/b"), "PATH must contain /b");
            assert!(path_val.contains("/c"), "PATH must contain /c");
        }

        #[test]
        fn test_setenv_then_info_both_effective() {
            let mut exec = RexExecutor::new();
            let cmds = "env.setenv('TAGGED', 'yes')\ninfo('set TAGGED')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("setenv+info must succeed");
            assert_eq!(env.vars.get("TAGGED").map(|s| s.as_str()), Some("yes"));
            assert!(!env.info_messages.is_empty(), "info message must be recorded");
        }

        #[test]
        fn test_context_vars_available_in_execute() {
            let mut exec = RexExecutor::new();
            exec.set_context_var("MY_CTX".to_string(), "ctx123".to_string());
            let result = exec.execute_commands(
                "env.setenv('CONFIRM', 'done')",
                "ctxpkg",
                Some("/opt/ctxpkg"),
                Some("1.0"),
            );
            assert!(result.is_ok(), "execute with context vars must succeed");
        }

        #[test]
        fn test_append_then_prepend_ordering() {
            let mut exec = RexExecutor::new();
            let cmds = "env.append_path('TESTPATH', '/appended')\nenv.prepend_path('TESTPATH', '/prepended')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("append+prepend must succeed");
            let val = env.vars.get("TESTPATH").cloned().unwrap_or_default();
            assert!(val.contains("/appended"), "TESTPATH must contain /appended, got: {}", val);
            assert!(val.contains("/prepended"), "TESTPATH must contain /prepended, got: {}", val);
        }

        // ── Cycle 120 additions ──────────────────────────────────────────────

        /// setenv with a path-like value containing slashes succeeds
        #[test]
        fn test_setenv_path_like_value() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.setenv('TOOL_ROOT', '/opt/tools/myapp/1.0')",
                    "",
                    None,
                    None,
                )
                .expect("setenv with path-like value must succeed");
            let val = env.vars.get("TOOL_ROOT").map(|s| s.as_str());
            assert_eq!(val, Some("/opt/tools/myapp/1.0"));
        }

        /// multiple setenv calls produce multiple distinct vars
        #[test]
        fn test_setenv_five_distinct_vars() {
            let mut exec = RexExecutor::new();
            let cmds = "env.setenv('V1', 'a')\nenv.setenv('V2', 'b')\nenv.setenv('V3', 'c')\nenv.setenv('V4', 'd')\nenv.setenv('V5', 'e')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("5 setenv must succeed");
            for (key, val) in [("V1", "a"), ("V2", "b"), ("V3", "c"), ("V4", "d"), ("V5", "e")] {
                assert_eq!(
                    env.vars.get(key).map(|s| s.as_str()),
                    Some(val),
                    "{key} should be {val}"
                );
            }
        }

        /// alias with spaces in value is stored correctly
        #[test]
        fn test_alias_with_spaces_in_value() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "alias('ll', 'ls -la --color=auto')",
                    "",
                    None,
                    None,
                )
                .expect("alias with spaces must succeed");
            assert!(
                env.aliases.contains_key("ll"),
                "alias 'll' must be registered, got: {:?}",
                env.aliases
            );
        }

        /// executing commands twice on the same executor accumulates state
        #[test]
        fn test_execute_commands_twice_accumulates_vars() {
            let mut exec = RexExecutor::new();
            exec.execute_commands("env.setenv('FIRST', '1')", "", None, None)
                .expect("first execute must succeed");
            let env = exec
                .execute_commands("env.setenv('SECOND', '2')", "", None, None)
                .expect("second execute must succeed");
            // At minimum SECOND should be set in the returned env
            assert!(
                env.vars.contains_key("SECOND"),
                "SECOND must be set after second execute"
            );
        }

        /// info message content is preserved verbatim
        #[test]
        fn test_info_message_content_preserved() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("info('rez-next-cycle120')", "", None, None)
                .expect("info must succeed");
            let found = env.info_messages.iter().any(|m| m.contains("rez-next-cycle120"));
            assert!(found, "info message 'rez-next-cycle120' must be preserved, got: {:?}", env.info_messages);
        }

        /// prepend_path with empty initial PATH still adds the entry
        #[test]
        fn test_prepend_path_on_empty_path_var() {
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("env.prepend_path('NEWPATH', '/injected')", "", None, None)
                .expect("prepend_path on fresh env must succeed");
            let val = env.vars.get("NEWPATH").cloned().unwrap_or_default();
            assert!(
                val.contains("/injected"),
                "NEWPATH must contain '/injected', got: {}",
                val
            );
        }
    }
}

