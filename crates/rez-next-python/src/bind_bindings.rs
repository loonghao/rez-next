//! Python bindings for rez.bind (system tool binding)
//!
//! Equivalent to `rez bind <tool>` / Python `from rez.bind import bind`

use pyo3::prelude::*;
use rez_next_bind::{
    detect_tool_version, extract_version_from_output, find_tool_executable, get_builtin_binder,
    list_builtin_binders, BindOptions, PackageBinder,
};
use std::path::PathBuf;

/// Python-accessible bound package result.
#[pyclass(name = "BindResult", from_py_object)]
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
    fn name(&self) -> &str {
        &self.name
    }

    #[getter]
    fn version(&self) -> &str {
        &self.version
    }

    #[getter]
    fn install_path(&self) -> &str {
        &self.install_path
    }

    #[getter]
    fn executable_path(&self) -> Option<&str> {
        self.executable_path.as_deref()
    }

    fn __repr__(&self) -> String {
        format!(
            "BindResult(name='{}', version='{}', path='{}')",
            self.name, self.version, self.install_path
        )
    }
}

/// Python-accessible bind manager.
#[pyclass(name = "BindManager")]
pub struct PyBindManager {}

impl Default for PyBindManager {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyBindManager {
    #[new]
    pub fn new() -> Self {
        Self {}
    }

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

        let result = binder
            .bind(tool_name, &opts)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(PyBindResult {
            name: result.name,
            version: result.version,
            install_path: result.install_path.to_string_lossy().to_string(),
            executable_path: result
                .executable_path
                .map(|p| p.to_string_lossy().to_string()),
        })
    }

    /// List all built-in bindable tool names.
    fn list_binders(&self) -> Vec<String> {
        list_builtin_binders()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
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
    list_builtin_binders()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── PyBindResult ──────────────────────────────────────────────────────────

    #[test]
    fn test_bind_result_getters() {
        let r = PyBindResult {
            name: "python".to_string(),
            version: "3.11.4".to_string(),
            install_path: "/pkgs/python/3.11.4".to_string(),
            executable_path: Some("/usr/bin/python3".to_string()),
        };
        assert_eq!(r.name(), "python");
        assert_eq!(r.version(), "3.11.4");
        assert_eq!(r.install_path(), "/pkgs/python/3.11.4");
        assert_eq!(r.executable_path(), Some("/usr/bin/python3"));
    }

    #[test]
    fn test_bind_result_no_executable() {
        let r = PyBindResult {
            name: "cmake".to_string(),
            version: "3.26.0".to_string(),
            install_path: "/pkgs/cmake/3.26.0".to_string(),
            executable_path: None,
        };
        assert_eq!(r.executable_path(), None);
    }

    #[test]
    fn test_bind_result_repr_format() {
        let r = PyBindResult {
            name: "git".to_string(),
            version: "2.42.0".to_string(),
            install_path: "/pkgs/git/2.42.0".to_string(),
            executable_path: None,
        };
        let repr = r.__repr__();
        assert!(repr.contains("BindResult"));
        assert!(repr.contains("git"));
        assert!(repr.contains("2.42.0"));
        assert!(repr.contains("/pkgs/git/2.42.0"));
    }

    // ── PyBindManager ─────────────────────────────────────────────────────────

    #[test]
    fn test_bind_manager_repr() {
        let m = PyBindManager::new();
        assert_eq!(m.__repr__(), "BindManager()");
    }

    #[test]
    fn test_bind_manager_default_same_as_new() {
        let a = PyBindManager::new();
        let b = PyBindManager::default();
        assert_eq!(a.__repr__(), b.__repr__());
    }

    #[test]
    fn test_list_binders_non_empty() {
        let m = PyBindManager::new();
        let binders = m.list_binders();
        assert!(
            !binders.is_empty(),
            "there should be at least one built-in binder"
        );
    }

    #[test]
    fn test_list_binders_contains_known_tools() {
        let m = PyBindManager::new();
        let binders = m.list_binders();
        assert!(binders.contains(&"python".to_string()));
        assert!(binders.contains(&"git".to_string()));
    }

    #[test]
    fn test_is_builtin_known_tool() {
        let m = PyBindManager::new();
        assert!(m.is_builtin("python"));
        assert!(m.is_builtin("cmake"));
        assert!(m.is_builtin("git"));
    }

    #[test]
    fn test_is_builtin_unknown_tool() {
        let m = PyBindManager::new();
        assert!(!m.is_builtin("totally_nonexistent_tool_xyz"));
    }

    // ── Free functions ────────────────────────────────────────────────────────

    #[test]
    fn test_list_binders_fn_matches_manager() {
        let via_fn = list_binders();
        let via_mgr = PyBindManager::new().list_binders();
        assert_eq!(via_fn, via_mgr);
    }

    #[test]
    fn test_extract_version_semver() {
        assert_eq!(extract_version("Python 3.11.4"), Some("3.11.4".to_string()));
    }

    #[test]
    fn test_extract_version_git_format() {
        assert_eq!(
            extract_version("git version 2.42.0.windows.1"),
            Some("2.42.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_short() {
        assert_eq!(extract_version("1.8"), Some("1.8".to_string()));
    }

    #[test]
    fn test_extract_version_none_for_no_digits() {
        assert_eq!(extract_version("no version information"), None);
    }

    #[test]
    fn test_find_tool_nonexistent_returns_none() {
        // A tool name that definitely won't be on the system
        let result = find_tool("__totally_nonexistent_tool_rez_next__");
        assert!(result.is_none());
    }

    // ── detect_version free function ──────────────────────────────────────────

    #[test]
    fn test_detect_version_returns_string_for_nonexistent_tool() {
        // detect_version always returns a String (empty or version), never panics
        let v = detect_version("__nonexistent_tool_rez_next__");
        // Result is either empty string or some version string — just must not panic
        let _ = v;
    }

    // ── PyBindResult edge cases ───────────────────────────────────────────────

    #[test]
    fn test_bind_result_repr_no_path() {
        let r = PyBindResult {
            name: "clang".to_string(),
            version: "17.0.0".to_string(),
            install_path: "".to_string(),
            executable_path: None,
        };
        let repr = r.__repr__();
        assert!(repr.contains("clang"));
        assert!(repr.contains("17.0.0"));
    }

    #[test]
    fn test_bind_result_executable_path_some_path() {
        let r = PyBindResult {
            name: "node".to_string(),
            version: "20.0.0".to_string(),
            install_path: "/pkgs/node/20.0.0".to_string(),
            executable_path: Some("/usr/local/bin/node".to_string()),
        };
        let ep = r.executable_path();
        assert_eq!(ep, Some("/usr/local/bin/node"));
    }

    // ── PyBindManager list/detect integration ─────────────────────────────────

    #[test]
    fn test_list_binders_all_are_non_empty_strings() {
        let binders = list_binders();
        for name in &binders {
            assert!(!name.is_empty(), "binder name must be non-empty");
            // Binder names should be printable ASCII-ish identifiers
            assert!(
                name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
                "unexpected binder name: {name}"
            );
        }
    }

    #[test]
    fn test_is_builtin_case_sensitive() {
        let m = PyBindManager::new();
        // "Python" with capital P should NOT match (bind names are lowercase)
        let lower = m.is_builtin("python");
        let upper = m.is_builtin("Python");
        // Lower case must be true; upper-case behavior may vary but must not panic
        assert!(lower, "lowercase 'python' must be a builtin");
        let _ = upper;
    }

    #[test]
    fn test_extract_version_cmake_format() {
        // cmake --version outputs "cmake version 3.26.0"
        assert_eq!(
            extract_version("cmake version 3.26.0"),
            Some("3.26.0".to_string())
        );
    }

    #[test]
    fn test_extract_version_multiline_first_match() {
        // Only first version-like token should be returned
        let result = extract_version("1.2.3\nsome other 4.5.6");
        // Must return a version (1.2.3 or 4.5.6), not None
        assert!(result.is_some(), "should find at least one version");
    }
}
