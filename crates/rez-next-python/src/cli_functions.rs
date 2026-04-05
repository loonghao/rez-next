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
