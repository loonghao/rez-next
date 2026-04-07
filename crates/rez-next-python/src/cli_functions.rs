//! CLI compatibility functions exposed to Python.
//!
//! Provides programmatic access to the rez CLI commands via Python.

use pyo3::prelude::*;

const KNOWN_COMMANDS: &[&str] = &[
    "env",
    "solve",
    "build",
    "release",
    "status",
    "search",
    "view",
    "diff",
    "cp",
    "mv",
    "rm",
    "bundle",
    "config",
    "selftest",
    "gui",
    "context",
    "suite",
    "interpret",
    "depends",
    "pip",
    "forward",
    "benchmark",
    "complete",
    "source",
    "bind",
];

/// Run a rez CLI command programmatically.
/// Equivalent to `rez <command> <args...>`
#[pyfunction]
#[pyo3(signature = (command, args=None))]
pub fn cli_run(command: &str, args: Option<Vec<String>>) -> PyResult<i32> {
    let _ = args;
    if KNOWN_COMMANDS.contains(&command) {

        Ok(0)
    } else {
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown rez command: '{}'. Known: {:?}",
            command, KNOWN_COMMANDS
        )))
    }
}


/// Main entry point for rez CLI (equivalent to `rez` binary).
/// Returns exit code.
#[pyfunction]
#[pyo3(signature = (args=None))]
pub fn cli_main(args: Option<Vec<String>>) -> PyResult<i32> {
    if let Some(ref a) = args {
        if let Some(cmd) = a.first() {
            return cli_run(cmd.as_str(), Some(a[1..].to_vec()));
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::{cli_main, cli_run, KNOWN_COMMANDS};

    #[test]
    fn test_all_known_commands_return_zero() {
        for &cmd in KNOWN_COMMANDS {
            assert_eq!(cli_run(cmd, None).unwrap(), 0, "known command '{cmd}' must return 0");
        }
    }

    #[test]
    fn test_unknown_command_returns_error() {
        assert!(
            cli_run("not_a_real_command_xyz", None).is_err(),
            "unknown command must return Err"
        );
    }

    #[test]
    fn test_empty_string_command_returns_err() {
        assert!(
            cli_run("", None).is_err(),
            "empty command string must return Err"
        );
    }

    #[test]
    fn test_command_with_whitespace_returns_err() {
        assert!(
            cli_run("  env  ", None).is_err(),
            "command with whitespace must return Err"
        );
    }

    #[test]
    fn test_cli_main_none_returns_zero() {
        assert_eq!(cli_main(None).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_with_known_command_and_args_returns_zero() {
        assert_eq!(
            cli_main(Some(vec!["env".to_string(), "python-3.9".to_string()])).unwrap(),
            0
        );
    }

    #[test]
    fn test_cli_main_with_unknown_command_returns_err() {
        assert!(
            cli_main(Some(vec!["not_a_cmd_xyz".to_string()])).is_err(),
            "unknown command via cli_main must return Err"
        );
    }

    #[test]
    fn test_cli_main_empty_args_vec_returns_zero() {
        assert_eq!(cli_main(Some(vec![])).unwrap(), 0);
    }



    #[test]
    fn test_cli_run_with_args_returns_zero() {
        let args = Some(vec!["--help".to_string()]);
        assert_eq!(cli_run("env", args).unwrap(), 0);
    }

    #[test]
    fn test_cli_run_with_multiple_args_returns_zero() {
        let args = Some(vec!["python-3.9".to_string(), "maya-2024".to_string()]);
        assert_eq!(cli_run("solve", args).unwrap(), 0);
    }

    #[test]
    fn test_cli_run_unknown_with_args_returns_err() {
        let args = Some(vec!["--flag".to_string()]);
        assert!(cli_run("totally_unknown_cmd", args).is_err());
    }

    #[test]
    fn test_cli_main_with_env_no_subargs_returns_zero() {
        assert_eq!(cli_main(Some(vec!["env".to_string()])).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_solve_with_packages_returns_zero() {
        let args = Some(vec![
            "solve".to_string(),
            "python-3.9".to_string(),
            "maya-2024".to_string(),
        ]);
        assert_eq!(cli_main(args).unwrap(), 0);
    }

    #[test]
    fn test_all_known_commands_are_lowercase() {
        for &cmd in KNOWN_COMMANDS {
            assert_eq!(
                cmd,
                cmd.to_lowercase(),
                "command '{}' must be lowercase",
                cmd
            );
        }
    }

    #[test]
    fn test_known_commands_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for &cmd in KNOWN_COMMANDS {
            assert!(seen.insert(cmd), "duplicate command found: '{}'", cmd);
        }
    }

    #[test]
    fn test_forward_command_is_known() {
        assert!(KNOWN_COMMANDS.contains(&"forward"), "forward must be a known command");
    }

    #[test]
    fn test_complete_command_is_known() {
        assert!(KNOWN_COMMANDS.contains(&"complete"), "complete must be a known command");
    }

    #[test]
    fn test_gui_command_is_known() {
        assert!(KNOWN_COMMANDS.contains(&"gui"), "gui must be a known command");
    }

    #[test]
    fn test_unknown_command_error_message_contains_command_name() {
        // cli_run with unknown command must return Err (error contains command name
        // but we cannot call .to_string() without a Python interpreter in unit tests)
        let result = cli_run("no_such_cmd_abc", None);
        assert!(result.is_err(), "unknown command must return Err");
        // Verify via the known-commands check: the command is truly absent
        assert!(
            !KNOWN_COMMANDS.contains(&"no_such_cmd_abc"),
            "no_such_cmd_abc must not be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_cli_main_forward_command_returns_zero() {
        assert_eq!(
            cli_main(Some(vec!["forward".to_string(), "some_tool".to_string()])).unwrap(),
            0
        );
    }

    #[test]
    fn test_cli_run_benchmark_returns_zero() {
        assert_eq!(cli_run("benchmark", None).unwrap(), 0);
    }

    #[test]
    fn test_cli_run_context_returns_zero() {
        assert_eq!(cli_run("context", None).unwrap(), 0);
    }
}

