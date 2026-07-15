//! CLI compatibility functions exposed to Python.
//!
//! Python-side dispatch is deliberately fail-closed until it can invoke the same
//! implementation as the native executable. Returning a synthetic zero exit code
//! would make automation report commands as successful without executing them.

use pyo3::prelude::*;

const KNOWN_COMMANDS: &[&str] = &[
    "env",
    "solve",
    "build",
    "release",
    "test",
    "status",
    "search",
    "view",
    "diff",
    "cp",
    "mv",
    "rm",
    "bundle",
    "config",
    "pkg-help",
    "plugins",
    "pkg-cache",
    "suites",
    "gui",
    "context",
    "depends",
    "pip",
    "forward",
    "complete",
    "bind",
    "parse-version",
    "self-test",
    "self-update",
];

fn dispatch_unavailable() -> PyErr {
    pyo3::exceptions::PyNotImplementedError::new_err(
        "Python CLI dispatch is not available; run the `rez` or `rez-next` executable instead",
    )
}

/// Validate a rez command name and fail clearly while Python dispatch is unavailable.
#[pyfunction]
#[pyo3(signature = (command, args=None))]
pub fn cli_run(command: &str, args: Option<Vec<String>>) -> PyResult<i32> {
    let _ = args;
    if !KNOWN_COMMANDS.contains(&command) {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown rez command: '{}'. Known: {:?}",
            command, KNOWN_COMMANDS
        )));
    }
    Err(dispatch_unavailable())
}

/// Python CLI entry point. Fails clearly instead of reporting synthetic success.
#[pyfunction]
#[pyo3(signature = (args=None))]
pub fn cli_main(args: Option<Vec<String>>) -> PyResult<i32> {
    if let Some(ref a) = args
        && let Some(cmd) = a.first()
    {
        return cli_run(cmd.as_str(), Some(a[1..].to_vec()));
    }
    Err(dispatch_unavailable())
}

#[cfg(test)]
mod tests {
    use super::{KNOWN_COMMANDS, cli_main, cli_run};

    #[test]
    fn test_known_commands_fail_closed_until_dispatch_is_implemented() {
        for &cmd in KNOWN_COMMANDS {
            assert!(
                cli_run(cmd, None).is_err(),
                "known command '{cmd}' must not report synthetic success"
            );
        }
    }

    #[test]
    fn test_cli_run_does_not_ignore_arguments_and_report_success() {
        let args = Some(vec!["python-3.9".to_string(), "maya-2024".to_string()]);
        assert!(cli_run("solve", args).is_err());
    }

    #[test]
    fn test_cli_run_unknown_or_malformed_command_returns_error() {
        assert!(cli_run("not_a_real_command_xyz", None).is_err());
        assert!(cli_run("", None).is_err());
        assert!(cli_run("  env  ", None).is_err());
    }

    #[test]
    fn test_cli_main_without_command_fails_closed() {
        assert!(cli_main(None).is_err());
        assert!(cli_main(Some(vec![])).is_err());
    }

    #[test]
    fn test_cli_main_propagates_unavailable_dispatch() {
        assert!(cli_main(Some(vec!["env".to_string(), "--help".to_string()])).is_err());
    }

    #[test]
    fn test_cli_main_unknown_command_returns_err() {
        assert!(cli_main(Some(vec!["not_a_cmd_xyz".to_string()])).is_err());
    }
}
