//! CLI compatibility stub functions exposed to Python.
//!
//! This module currently validates command names against a fixed table and returns
//! compatibility-style exit codes. It does not dispatch to the real rez CLI yet.

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

/// Validate a known rez command name and return a compatibility success code.
///
/// This is currently a stub: `args` are ignored and no real CLI dispatch happens.
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

/// Compatibility-style main entry point for the Python stubbed CLI surface.
/// Returns a synthetic exit code based on the first argument, if present.
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
    use std::collections::HashSet;

    use super::{cli_main, cli_run, KNOWN_COMMANDS};

    #[test]
    fn test_all_known_commands_return_zero() {
        for &cmd in KNOWN_COMMANDS {
            assert_eq!(cli_run(cmd, None).unwrap(), 0, "known command '{cmd}' must return 0");
        }
    }

    #[test]
    fn test_known_commands_are_unique_non_empty_and_lowercase() {
        let mut seen = HashSet::new();

        for &cmd in KNOWN_COMMANDS {
            assert!(!cmd.is_empty(), "KNOWN_COMMANDS must not contain an empty string entry");
            assert_eq!(cmd, cmd.to_lowercase(), "command '{cmd}' must be lowercase");
            assert!(seen.insert(cmd), "duplicate command found: '{cmd}'");
        }
    }

    #[test]
    fn test_known_commands_include_python_stub_surface() {
        for cmd in ["benchmark", "bind", "complete", "forward", "gui", "suite"] {
            assert!(KNOWN_COMMANDS.contains(&cmd), "{cmd} must remain in the compatibility table");
        }
    }

    #[test]
    fn test_cli_run_known_command_ignores_args() {
        let args = Some(vec!["python-3.9".to_string(), "maya-2024".to_string()]);
        assert_eq!(cli_run("solve", args).unwrap(), 0);
    }

    #[test]
    fn test_cli_run_unknown_or_malformed_command_returns_error() {
        assert!(cli_run("not_a_real_command_xyz", None).is_err());
        assert!(cli_run("", None).is_err());
        assert!(cli_run("  env  ", None).is_err());
    }

    #[test]
    fn test_cli_main_without_command_returns_zero() {
        assert_eq!(cli_main(None).unwrap(), 0);
        assert_eq!(cli_main(Some(vec![])).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_dispatches_first_arg_to_cli_run() {
        assert_eq!(
            cli_main(Some(vec!["env".to_string(), "python-3.9".to_string()])).unwrap(),
            0
        );
        assert_eq!(cli_main(Some(vec!["release".to_string()])).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_unknown_command_returns_err() {
        assert!(cli_main(Some(vec!["not_a_cmd_xyz".to_string()])).is_err());
    }
}
