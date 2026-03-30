//! Python bindings for rez.bind (system tool binding)
//!
//! Equivalent to `rez bind <tool>` / Python `from rez.bind import bind`

use pyo3::prelude::*;
use rez_next_bind::{
    BindOptions, PackageBinder,
    list_builtin_binders, get_builtin_binder,
    detect_tool_version, find_tool_executable,
    extract_version_from_output,
};
use std::path::PathBuf;

/// Python-accessible bound package result.
#[pyclass(name = "BindResult")]
#[derive(Clone)]
pub struct PyBindResult {
    /// Package name
    pub name: String,
    /// Version string
    pub version: String,
    /// Installation path
    pub install_path: String,
    /// Executable path if found
    pub executable_path: Option<String>,
}

#[pymethods]
impl PyBindResult {
    #[getter]
    fn name(&self) -> &str { &self.name }

    #[getter]
    fn version(&self) -> &str { &self.version }

    #[getter]
    fn install_path(&self) -> &str { &self.install_path }

    #[getter]
    fn executable_path(&self) -> Option<&str> { self.executable_path.as_deref() }

    fn __repr__(&self) -> String {
        format!("BindResult(name='{}', version='{}', path='{}')",
            self.name, self.version, self.install_path)
    }
}

/// Python-accessible bind manager.
#[pyclass(name = "BindManager")]
pub struct PyBindManager {}

#[pymethods]
impl PyBindManager {
    #[new]
    pub fn new() -> Self { Self {} }

    /// Bind a system tool as a rez package.
    /// Equivalent to `rez bind <tool_name> [--version <ver>] [--install-path <path>]`
    #[pyo3(signature = (tool_name, version=None, install_path=None, force=false))]
    fn bind(
        &self,
        tool_name: &str,
        version: Option<&str>,
        install_path: Option<&str>,
        force: bool,
    ) -> PyResult<PyBindResult> {
        let binder = PackageBinder::new();
        let opts = BindOptions {
            version_override: version.map(|s| s.to_string()),
            install_path: install_path.map(PathBuf::from),
            force,
            search_path: true,
            extra_metadata: Vec::new(),
        };

        let result = binder.bind(tool_name, &opts)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyBindResult {
            name: result.name,
            version: result.version,
            install_path: result.install_path.to_string_lossy().to_string(),
            executable_path: result.executable_path
                .map(|p| p.to_string_lossy().to_string()),
        })
    }

    /// List all built-in bindable tool names.
    fn list_binders(&self) -> Vec<String> {
        list_builtin_binders().into_iter().map(|s| s.to_string()).collect()
    }

    /// Check whether a given tool name is a known built-in binder.
    fn is_builtin(&self, name: &str) -> bool {
        get_builtin_binder(name).is_some()
    }

    fn __repr__(&self) -> String {
        "BindManager()".to_string()
    }
}

/// Bind a system tool and return a BindResult.
/// Equivalent to `rez.bind.bind(name, version=None, path=None)`
#[pyfunction]
#[pyo3(signature = (tool_name, version=None, install_path=None, force=false))]
pub fn bind_tool(
    tool_name: &str,
    version: Option<&str>,
    install_path: Option<&str>,
    force: bool,
) -> PyResult<PyBindResult> {
    let mgr = PyBindManager::new();
    mgr.bind(tool_name, version, install_path, force)
}

/// List all known built-in binder names.
#[pyfunction]
pub fn list_binders() -> Vec<String> {
    list_builtin_binders().into_iter().map(|s| s.to_string()).collect()
}

/// Detect the version of a system tool.
/// Equivalent to rez's internal `_detect_version()`.
#[pyfunction]
pub fn detect_version(tool_name: &str) -> String {
    detect_tool_version(tool_name)
}

/// Find the path of a tool executable in PATH.
#[pyfunction]
pub fn find_tool(tool_name: &str) -> Option<String> {
    find_tool_executable(tool_name).map(|p| p.to_string_lossy().to_string())
}

/// Extract a version token from a raw version output string.
#[pyfunction]
pub fn extract_version(raw_output: &str) -> Option<String> {
    extract_version_from_output(raw_output)
}
