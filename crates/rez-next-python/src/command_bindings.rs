//! Python bindings for command execution utilities.
//!
//! This module provides Python bindings for the `rez_next_util::command` module.

use pyo3::prelude::*;
use rez_next_util::{
    command_exists as rs_command_exists, execute_command as rs_execute_command,
    execute_command_with_timeout as rs_execute_command_with_timeout,
    get_command_output as rs_get_command_output, get_command_path as rs_get_command_path,
    CommandResult,
};

/// Python wrapper for `CommandResult`.
#[pyclass(name = "CommandResult")]
#[derive(Debug, Clone)]
pub struct PyCommandResult {
    #[pyo3(get)]
    pub stdout: String,
    #[pyo3(get)]
    pub stderr: String,
    #[pyo3(get)]
    pub exit_code: i32,
    #[pyo3(get)]
    pub success: bool,
}

impl From<CommandResult> for PyCommandResult {
    fn from(result: CommandResult) -> Self {
        PyCommandResult {
            stdout: result.stdout,
            stderr: result.stderr,
            exit_code: result.exit_code,
            success: result.success,
        }
    }
}

#[pymethods]
impl PyCommandResult {
    /// Create a new CommandResult.
    #[new]
    fn new(stdout: String, stderr: String, exit_code: i32, success: bool) -> Self {
        PyCommandResult {
            stdout,
            stderr,
            exit_code,
            success,
        }
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "CommandResult(success={}, exit_code={}, stdout={:?}, stderr={:?})",
            self.success, self.exit_code, self.stdout, self.stderr
        )
    }
}

/// Execute a command and capture its output.
#[pyfunction]
fn execute_command(command: String, args: Vec<String>) -> PyResult<PyCommandResult> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match rs_execute_command(&command, &args_str) {
        Ok(result) => Ok(result.into()),
        Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
    }
}

/// Execute a command with a timeout.
#[pyfunction]
fn execute_command_with_timeout(
    command: String,
    args: Vec<String>,
    timeout_secs: u64,
) -> PyResult<PyCommandResult> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match rs_execute_command_with_timeout(&command, &args_str, timeout_secs) {
        Ok(result) => Ok(result.into()),
        Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
    }
}

/// Check if a command exists in PATH.
#[pyfunction]
fn command_exists(command: String) -> bool {
    rs_command_exists(&command)
}

/// Get the full path to a command.
#[pyfunction]
fn get_command_path(command: String) -> Option<String> {
    rs_get_command_path(&command).map(|p| p.to_string_lossy().to_string())
}

/// Execute a command and return stdout as a string.
#[pyfunction]
fn get_command_output(command: String, args: Vec<String>) -> PyResult<String> {
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match rs_get_command_output(&command, &args_str) {
        Ok(output) => Ok(output),
        Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
    }
}

/// Register the command module.
pub fn register_command_module(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let command_mod = PyModule::new(py, "command")?;

    // Add classes
    command_mod.add_class::<PyCommandResult>()?;

    // Add functions
    command_mod.add_function(wrap_pyfunction!(execute_command, &command_mod)?)?;
    command_mod.add_function(wrap_pyfunction!(
        execute_command_with_timeout,
        &command_mod
    )?)?;
    command_mod.add_function(wrap_pyfunction!(command_exists, &command_mod)?)?;
    command_mod.add_function(wrap_pyfunction!(get_command_path, &command_mod)?)?;
    command_mod.add_function(wrap_pyfunction!(get_command_output, &command_mod)?)?;

    // Register as submodule
    parent.add_submodule(&command_mod)?;

    // Register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    let parent_name = parent.name()?;
    let full_name = format!("{}.command", parent_name);
    modules.set_item(full_name.as_str(), &command_mod)?;

    Ok(())
}
