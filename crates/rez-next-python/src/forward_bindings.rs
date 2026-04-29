//! Python bindings for rez.forward — shell forward function compatibility
//!
//! Implements `rez forward` semantics: allows a shell to call a rez tool by
//! forwarding execution to the correct package context.

use pyo3::prelude::*;

use crate::runtime::get_runtime;

/// Represents a forwarded rez tool call.
///
/// Equivalent to rez's `forward` command which routes CLI calls to the
/// correct context. Usage:
///   `rez forward <tool_name> [args...]`
#[pyclass(name = "RezForward")]
pub struct PyRezForward {
    tool: String,
    context_id: Option<String>,
}

#[pymethods]
impl PyRezForward {
    #[new]
    #[pyo3(signature = (tool, context_id=None))]
    pub fn new(tool: String, context_id: Option<String>) -> Self {
        PyRezForward { tool, context_id }
    }

    fn __str__(&self) -> String {
        match &self.context_id {
            Some(ctx) => format!("RezForward({} -> context:{})", self.tool, ctx),
            None => format!("RezForward({})", self.tool),
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }

    /// Tool name being forwarded
    #[getter]
    fn tool_name(&self) -> String {
        self.tool.clone()
    }

    /// Context ID used for forwarding
    #[getter]
    fn context_id(&self) -> Option<String> {
        self.context_id.clone()
    }

    /// Execute the forward (dry run: returns the command string)
    #[pyo3(signature = (args=None, dry_run=false))]
    fn execute(&self, args: Option<Vec<String>>, dry_run: bool) -> PyResult<String> {
        let args = args.unwrap_or_default();
        let cmd = if args.is_empty() {
            self.tool.clone()
        } else {
            format!("{} {}", self.tool, args.join(" "))
        };

        if dry_run {
            return Ok(format!("[dry-run] rez-forward: {}", cmd));
        }

        // In real usage this would spawn the binary in the correct context.
        // Here we validate the tool exists in PATH or return a descriptive error.
        let status = std::process::Command::new(&self.tool).args(&args).status();

        match status {
            Ok(s) => Ok(format!("exit:{}", s.code().unwrap_or(-1))),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to forward to '{}': {}",
                self.tool, e
            ))),
        }
    }
}

/// Resolve which context should handle a given tool call.
///
/// Compatible with rez's `rez forward <tool>` resolution logic.
/// Returns the context ID (if found) and the package providing the tool.
#[pyfunction]
#[pyo3(signature = (tool_name, paths=None))]
pub fn resolve_forward_tool(
    tool_name: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Option<(String, String)>> {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use std::path::PathBuf;

    let _ = paths;
    let config = RezCoreConfig::load();
    let rt = get_runtime();

    let mut repo_manager = RepositoryManager::new();
    for (i, p) in config.packages_path.iter().enumerate() {
        let path = PathBuf::from(crate::package_functions::expand_home(p));
        if path.exists() {
            repo_manager
                .add_repository(Box::new(SimpleRepository::new(path, format!("repo_{}", i))));
        }
    }

    let packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    for pkg in &packages {
        if pkg.tools.iter().any(|t| t == tool_name) {
            let ver = pkg
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");
            return Ok(Some((format!("{}-{}", pkg.name, ver), pkg.name.clone())));
        }
    }

    Ok(None)
}

/// Generate a shell wrapper script for forwarding a tool call.
///
/// This is equivalent to the shell stubs rez installs in `~/.rez/bin/rez-<tool>`.
/// Shell: "bash" | "zsh" | "fish" | "cmd" | "powershell"
#[pyfunction]
#[pyo3(signature = (tool_name, shell=None))]
pub fn generate_forward_script(tool_name: &str, shell: Option<&str>) -> PyResult<String> {
    let shell = shell.unwrap_or("bash");
    let script = match shell {
        "powershell" | "pwsh" => format!(
            r#"# rez-next forward wrapper for {}
function Invoke-RezTool {{
    & rez-next forward {} @args
}}
Set-Alias -Name {} -Value Invoke-RezTool
"#,
            tool_name, tool_name, tool_name
        ),
        "cmd" => format!(
            r#"@echo off
rem rez-next forward wrapper for {}
rez-next forward {} %*
"#,
            tool_name, tool_name
        ),
        "fish" => format!(
            r#"# rez-next forward wrapper for {}
function {}
    rez-next forward {} $argv
end
"#,
            tool_name, tool_name, tool_name
        ),
        _ => format!(
            r#"#!/usr/bin/env bash
# rez-next forward wrapper for {}
exec rez-next forward {} "$@"
"#,
            tool_name, tool_name
        ),
    };
    Ok(script)
}

#[cfg(test)]
#[path = "forward_bindings_tests.rs"]
mod forward_tests;
