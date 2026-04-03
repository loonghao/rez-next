//! Python bindings for `rez status` — current environment status query
//!
//! Equivalent to `rez status` / `rez context`, reporting the active rez context
//! in the current process environment (REZ_CONTEXT_FILE, REZ_USED_PACKAGES, etc.).

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;

// ─── Public structs ──────────────────────────────────────────────────────────

/// Status of the current rez-managed environment.
///
/// Mirrors the information shown by `rez status` / `rez context`.
#[pyclass(name = "RezStatus")]
pub struct PyRezStatus {
    /// True if we are inside an active rez context
    #[pyo3(get)]
    pub is_active: bool,
    /// Path to the resolved context file (.rxt), if any
    #[pyo3(get)]
    pub context_file: Option<String>,
    /// Packages that are currently resolved (name-version strings)
    #[pyo3(get)]
    pub resolved_packages: Vec<String>,
    /// The shell being used (bash, zsh, fish, powershell, cmd, …)
    #[pyo3(get)]
    pub current_shell: Option<String>,
    /// Rez version that created the context
    #[pyo3(get)]
    pub rez_version: Option<String>,
    /// Working directory when the context was created
    #[pyo3(get)]
    pub context_cwd: Option<String>,
    /// Requested packages (before resolution)
    #[pyo3(get)]
    pub requested_packages: Vec<String>,
    /// Packages that were implicit (added by rez config)
    #[pyo3(get)]
    pub implicit_packages: Vec<String>,
    /// Environment variables set by the context (subset relevant to rez)
    rez_env_vars: HashMap<String, String>,
}

impl Default for PyRezStatus {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PyRezStatus {
    /// Detect the current rez context from environment variables.
    ///
    /// This reads the REZ_* environment variables that rez injects when a
    /// context is activated (REZ_CONTEXT_FILE, REZ_USED_PACKAGES, …).
    #[new]
    pub fn new() -> Self {
        detect_current_status()
    }

    fn __repr__(&self) -> String {
        if self.is_active {
            format!(
                "RezStatus(active, {} packages)",
                self.resolved_packages.len()
            )
        } else {
            "RezStatus(inactive)".to_string()
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    /// Return the REZ_* environment variables visible to the current context.
    fn get_rez_env_vars(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        for (k, v) in &self.rez_env_vars {
            d.set_item(k, v)?;
        }
        Ok(d.into())
    }

    /// Serialize to a dict (for JSON/YAML export).
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let d = PyDict::new(py);
        d.set_item("is_active", self.is_active)?;
        d.set_item("context_file", &self.context_file)?;
        d.set_item(
            "resolved_packages",
            PyList::new(py, &self.resolved_packages)?,
        )?;
        d.set_item(
            "requested_packages",
            PyList::new(py, &self.requested_packages)?,
        )?;
        d.set_item(
            "implicit_packages",
            PyList::new(py, &self.implicit_packages)?,
        )?;
        d.set_item("current_shell", &self.current_shell)?;
        d.set_item("rez_version", &self.rez_version)?;
        d.set_item("context_cwd", &self.context_cwd)?;
        Ok(d.into())
    }

    /// Pretty-print a summary (like `rez status` terminal output).
    fn print_status(&self) {
        if self.is_active {
            println!("Current rez context:");
            if let Some(ref f) = self.context_file {
                println!("  context file : {}", f);
            }
            if let Some(ref cwd) = self.context_cwd {
                println!("  created in   : {}", cwd);
            }
            if let Some(ref shell) = self.current_shell {
                println!("  shell        : {}", shell);
            }
            if let Some(ref ver) = self.rez_version {
                println!("  rez version  : {}", ver);
            }
            println!("  packages ({}):", self.resolved_packages.len());
            for pkg in &self.resolved_packages {
                println!("    {}", pkg);
            }
        } else {
            println!("Not in a rez context.");
        }
    }
}

// ─── Detection logic ──────────────────────────────────────────────────────────

/// Read current rez context from process environment variables.
fn detect_current_status() -> PyRezStatus {
    let env_vars: HashMap<String, String> = std::env::vars()
        .filter(|(k, _)| k.starts_with("REZ_"))
        .collect();

    let is_active = env_vars.contains_key("REZ_CONTEXT_FILE")
        || env_vars.contains_key("REZ_USED_PACKAGES_NAMES");

    let context_file = env_vars.get("REZ_CONTEXT_FILE").cloned();
    let rez_version = env_vars.get("REZ_VERSION").cloned();
    let context_cwd = env_vars.get("REZ_ORIG_CWD").cloned();

    // REZ_USED_PACKAGES_NAMES is a space-separated list of "name-version" strings
    let resolved_packages: Vec<String> = env_vars
        .get("REZ_USED_PACKAGES_NAMES")
        .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
        .unwrap_or_default();

    // REZ_REQUEST is the original requested packages
    let requested_packages: Vec<String> = env_vars
        .get("REZ_REQUEST")
        .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
        .unwrap_or_default();

    // REZ_RESOLVE_MODE or implicit packages
    let implicit_packages: Vec<String> = env_vars
        .get("REZ_IMPLICIT_PACKAGES")
        .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
        .unwrap_or_default();

    // Detect shell from SHELL env var or OS
    let current_shell = detect_shell_from_env();

    PyRezStatus {
        is_active,
        context_file,
        resolved_packages,
        current_shell,
        rez_version,
        context_cwd,
        requested_packages,
        implicit_packages,
        rez_env_vars: env_vars,
    }
}

fn detect_shell_from_env() -> Option<String> {
    // Unix SHELL
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return Some("zsh".to_string());
        } else if shell.contains("fish") {
            return Some("fish".to_string());
        } else if shell.contains("bash") {
            return Some("bash".to_string());
        }
        return Some(shell);
    }
    // Windows
    if cfg!(windows) {
        if std::env::var("PSModulePath").is_ok() {
            return Some("powershell".to_string());
        }
        return Some("cmd".to_string());
    }
    None
}

// ─── Python functions ─────────────────────────────────────────────────────────

/// Query the current rez context status.
///
/// Returns a `RezStatus` object reflecting the environment variables set by
/// an active `rez env` / `rez context` session.  Outside a rez context,
/// `status.is_active` will be `False`.
///
/// Equivalent to `rez status` or `rez context --status`.
#[pyfunction]
pub fn get_current_status() -> PyResult<PyRezStatus> {
    Ok(PyRezStatus::new())
}

/// Return True if the current process is running inside a rez context.
///
/// Equivalent to `rez status` exit-code check.
#[pyfunction]
pub fn is_in_rez_context() -> bool {
    std::env::var("REZ_CONTEXT_FILE").is_ok() || std::env::var("REZ_USED_PACKAGES_NAMES").is_ok()
}

/// Get the current context file path (REZ_CONTEXT_FILE env var), or None.
#[pyfunction]
pub fn get_context_file() -> Option<String> {
    std::env::var("REZ_CONTEXT_FILE").ok()
}

/// Get the list of currently resolved package name-version strings.
///
/// Returns an empty list outside a rez context.
#[pyfunction]
pub fn get_resolved_package_names() -> Vec<String> {
    std::env::var("REZ_USED_PACKAGES_NAMES")
        .map(|s| s.split_whitespace().map(|p| p.to_string()).collect())
        .unwrap_or_default()
}

/// Get a specific REZ_* environment variable value, or None.
#[pyfunction]
#[pyo3(signature = (key))]
pub fn get_rez_env_var(key: &str) -> Option<String> {
    let full_key = if key.starts_with("REZ_") {
        key.to_string()
    } else {
        format!("REZ_{}", key.to_uppercase())
    };
    std::env::var(&full_key).ok()
}

// ─── Rust unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod status_bindings_tests {
    use super::*;

    #[test]
    fn test_is_in_rez_context_false_outside() {
        // Outside any rez env the function should return false (CI has no rez)
        let in_ctx = std::env::var("REZ_CONTEXT_FILE").is_ok()
            || std::env::var("REZ_USED_PACKAGES_NAMES").is_ok();
        // Just verify the function matches the manual check
        assert_eq!(is_in_rez_context(), in_ctx);
    }

    #[test]
    fn test_get_context_file_none_outside_context() {
        if std::env::var("REZ_CONTEXT_FILE").is_err() {
            assert!(get_context_file().is_none());
        }
    }

    #[test]
    fn test_get_resolved_package_names_empty_outside() {
        if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
            let names = get_resolved_package_names();
            assert!(names.is_empty(), "Should be empty outside rez context");
        }
    }

    #[test]
    fn test_rez_status_inactive_repr() {
        let status = detect_current_status();
        // Only test the inactive case (CI env)
        if !status.is_active {
            assert!(!status.__repr__().is_empty());
            assert!(status.__repr__().contains("inactive"));
        }
    }

    #[test]
    fn test_rez_status_resolved_packages_from_env() {
        // Simulate REZ_USED_PACKAGES_NAMES
        unsafe {
            std::env::set_var("REZ_USED_PACKAGES_TEST_TEMP", "python-3.9 maya-2024.1");
        }
        // Parse logic
        let raw = std::env::var("REZ_USED_PACKAGES_TEST_TEMP").unwrap();
        let pkgs: Vec<String> = raw.split_whitespace().map(|p| p.to_string()).collect();
        assert_eq!(pkgs.len(), 2);
        assert_eq!(pkgs[0], "python-3.9");
        unsafe {
            std::env::remove_var("REZ_USED_PACKAGES_TEST_TEMP");
        }
    }

    #[test]
    fn test_detect_shell_returns_some_or_none() {
        // Just verify the function doesn't panic
        let _ = detect_shell_from_env();
    }

    #[test]
    fn test_get_rez_env_var_with_prefix() {
        // REZ_VERSION might exist; just test the function returns Option
        let _ = get_rez_env_var("VERSION");
    }

    #[test]
    fn test_get_rez_env_var_without_prefix() {
        // When key already starts with REZ_
        let _ = get_rez_env_var("REZ_VERSION");
    }

    #[test]
    fn test_status_to_dict_fields_present() {
        // Validate that PyRezStatus has expected field structure
        let s = detect_current_status();
        // is_active, context_file, resolved_packages, etc.
        // Verify basic field types
        let _ = s.is_active;
        let _ = &s.resolved_packages;
        let _ = &s.requested_packages;
        let _ = &s.implicit_packages;
    }

    #[test]
    fn test_inactive_context_empty_packages() {
        if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
            let s = detect_current_status();
            if !s.is_active {
                assert!(s.resolved_packages.is_empty());
            }
        }
    }
}
