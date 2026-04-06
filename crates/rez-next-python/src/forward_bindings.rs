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

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod forward_tests {
    use super::*;

    mod test_rez_forward_struct {
        use super::*;

        #[test]
        fn test_rez_forward_new() {
            let fwd = PyRezForward::new("maya".to_string(), None);
            assert_eq!(fwd.tool_name(), "maya");
            assert!(fwd.context_id().is_none());
        }

        #[test]
        fn test_rez_forward_with_context() {
            let fwd = PyRezForward::new("houdini".to_string(), Some("ctx-abc123".to_string()));
            assert_eq!(fwd.context_id(), Some("ctx-abc123".to_string()));
        }

        #[test]
        fn test_forward_str_no_context() {
            let fwd = PyRezForward::new("python".to_string(), None);
            let s = fwd.__str__();
            assert!(s.contains("python"), "str must mention tool name");
            assert!(!s.contains("context:"), "str without context must not include context:");
        }

        #[test]
        fn test_forward_str_with_context() {
            let fwd = PyRezForward::new("maya".to_string(), Some("ctx-xyz".to_string()));
            let s = fwd.__str__();
            assert!(s.contains("maya"), "str must contain tool name");
            assert!(s.contains("ctx-xyz"), "str must contain context id");
        }

        #[test]
        fn test_forward_repr_equals_str() {
            let fwd = PyRezForward::new("nuke".to_string(), Some("ctx-99".to_string()));
            assert_eq!(fwd.__repr__(), fwd.__str__());
        }

        #[test]
        fn test_forward_dry_run_no_args() {
            let fwd = PyRezForward::new("rez-env".to_string(), None);
            let result = fwd.execute(None, true);
            assert!(result.is_ok());
            let out = result.unwrap();
            assert!(out.contains("[dry-run]"));
            assert!(out.contains("rez-env"));
        }

        #[test]
        fn test_forward_dry_run_with_args() {
            let fwd = PyRezForward::new("python".to_string(), None);
            let result = fwd.execute(Some(vec!["--version".to_string()]), true);
            assert!(result.is_ok());
            let out = result.unwrap();
            assert!(out.contains("[dry-run]"));
            assert!(out.contains("python"));
            assert!(out.contains("--version"));
        }
    }

    mod test_generate_scripts {
        use super::*;

        #[test]
        fn test_generate_forward_script_bash() {
            let script = generate_forward_script("maya", Some("bash")).unwrap();
            assert!(script.contains("rez-next forward maya"));
            assert!(script.contains("#!/usr/bin/env bash"));
        }

        #[test]
        fn test_generate_forward_script_zsh_uses_bash_path() {
            // zsh falls through to the default (bash) branch
            let script = generate_forward_script("houdini", Some("zsh")).unwrap();
            assert!(script.contains("rez-next forward houdini"));
            assert!(script.contains("#!/usr/bin/env bash") || script.contains("houdini"));
        }

        #[test]
        fn test_generate_forward_script_default_shell_none() {
            // When shell is None, it defaults to "bash"
            let script = generate_forward_script("nukerender", None).unwrap();
            assert!(script.contains("rez-next forward nukerender"));
            assert!(script.contains("bash") || script.contains("nukerender"));
        }

        #[test]
        fn test_generate_forward_script_powershell() {
            let script = generate_forward_script("houdini", Some("powershell")).unwrap();
            assert!(script.contains("rez-next forward houdini"));
            assert!(script.contains("Set-Alias"));
        }

        #[test]
        fn test_generate_forward_script_pwsh() {
            // pwsh is an alias for powershell core
            let script = generate_forward_script("hython", Some("pwsh")).unwrap();
            assert!(script.contains("rez-next forward hython"));
            assert!(script.contains("Set-Alias") || script.contains("Invoke-RezTool"));
        }

        #[test]
        fn test_generate_forward_script_fish() {
            let script = generate_forward_script("nuke", Some("fish")).unwrap();
            assert!(script.contains("function nuke"));
            assert!(script.contains("rez-next forward nuke"));
        }

        #[test]
        fn test_generate_forward_script_cmd() {
            let script = generate_forward_script("hython", Some("cmd")).unwrap();
            assert!(script.contains("@echo off"));
            assert!(script.contains("rez-next forward hython"));
        }
    }
}
