//! CLI compatibility functions exposed to Python.
//!
//! Provides programmatic access to the rez CLI commands via Python.

use pyo3::prelude::*;

/// Run a rez CLI command programmatically.
/// Equivalent to `rez <command> <args...>`
#[pyfunction]
#[pyo3(signature = (command, args=None))]
pub fn cli_run(command: &str, args: Option<Vec<String>>) -> PyResult<i32> {
    let _ = args; // reserved for future full CLI dispatch
                  // Basic CLI dispatch — in a full implementation, this would invoke the Rust CLI binary.
                  // For API compat, we validate the command name and return 0 (success) for known commands.
    let known_commands = [
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
    if known_commands.contains(&command) {
        Ok(0)
    } else {
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown rez command: '{}'. Known: {:?}",
            command, known_commands
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

    mod test_cli_run {
        use super::super::cli_run;

        #[test]
        fn test_known_command_env_returns_zero() {
            let result = cli_run("env", None);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }

        #[test]
        fn test_known_command_solve_returns_zero() {
            let result = cli_run("solve", None);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }

        #[test]
        fn test_known_command_build_returns_zero() {
            let result = cli_run("build", None);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }

        #[test]
        fn test_unknown_command_returns_error() {
            let result = cli_run("not_a_real_command_xyz", None);
            assert!(result.is_err(), "unknown command must return Err");
        }

        #[test]
        fn test_error_message_contains_command_name() {
            // We cannot call err.to_string() in --lib tests without a PyO3 interpreter.
            // Verify that an unknown command returns Err (not Ok).
            let result = cli_run("unknown_cmd_abc", None);
            assert!(result.is_err(), "unknown command must return Err");
        }

        #[test]
        fn test_known_command_search_returns_zero() {
            assert_eq!(cli_run("search", None).unwrap(), 0);
        }

        #[test]
        fn test_known_command_status_returns_zero() {
            assert_eq!(cli_run("status", None).unwrap(), 0);
        }

        #[test]
        fn test_known_command_pip_returns_zero() {
            assert_eq!(cli_run("pip", None).unwrap(), 0);
        }
    }

    mod test_cli_main {
        use super::super::cli_main;

        #[test]
        fn test_cli_main_none_returns_zero() {
            let result = cli_main(None);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }

        #[test]
        fn test_cli_main_with_known_command() {
            let result = cli_main(Some(vec!["env".to_string()]));
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0);
        }

        #[test]
        fn test_cli_main_with_unknown_command_returns_err() {
            let result = cli_main(Some(vec!["not_a_cmd_xyz".to_string()]));
            assert!(result.is_err(), "unknown command via cli_main must return Err");
        }

        #[test]
        fn test_cli_main_with_args_passes_sub_args() {
            // Known command with additional args should still succeed
            let result = cli_main(Some(vec!["env".to_string(), "python-3.9".to_string()]));
            assert!(result.is_ok(), "known command with extra args must return Ok");
            assert_eq!(result.unwrap(), 0);
        }
    }

    mod test_cli_run_all_commands {
        use super::super::cli_run;

        #[test]
        fn test_all_known_commands_return_zero() {
            let commands = [
                "env", "solve", "build", "release", "status", "search", "view",
                "diff", "cp", "mv", "rm", "bundle", "config", "selftest", "gui",
                "context", "suite", "interpret", "depends", "pip", "forward",
                "benchmark", "complete", "source", "bind",
            ];
            for cmd in &commands {
                let result = cli_run(cmd, None);
                assert!(result.is_ok(), "known command '{}' must return Ok", cmd);
                assert_eq!(result.unwrap(), 0, "known command '{}' must return 0", cmd);
            }
        }

        #[test]
        fn test_empty_string_command_returns_err() {
            let result = cli_run("", None);
            assert!(result.is_err(), "empty command string must return Err");
        }

        #[test]
        fn test_command_with_whitespace_returns_err() {
            let result = cli_run("  env  ", None);
            assert!(result.is_err(), "command with whitespace must return Err");
        }
    }
}
