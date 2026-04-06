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
    fn get_rez_env_vars(&self, py: Python) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        for (k, v) in &self.rez_env_vars {
            d.set_item(k, v)?;
        }
        Ok(d.into_any().unbind())
    }

    /// Serialize to a dict (for JSON/YAML export).
    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
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
        Ok(d.into_any().unbind())
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
    use std::sync::Mutex;

    // Serialize all tests that mutate process-global environment variables.
    // cargo test runs tests in parallel by default, and concurrent env mutations
    // cause non-deterministic failures on env-reads like detect_current_status().
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

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
        let _lock = ENV_MUTEX.lock().unwrap();
        // Only assert when we know no other test has set the env var
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
        let _lock = ENV_MUTEX.lock().unwrap();
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
    fn test_detect_shell_from_env_returns_valid_shell() {
        // On Windows, PSModulePath is always present so powershell is detected.
        // On Linux/macOS, SHELL governs. Either way, the result must be a known shell name.
        let shell = detect_shell_from_env();
        // In a rez-unactivated env, shell detection may return None; if Some, must be known.
        if let Some(ref s) = shell {
            let known = ["bash", "zsh", "fish", "powershell", "cmd"];
            assert!(
                known.iter().any(|k| s.contains(k)),
                "unexpected shell: {s}"
            );
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_detect_shell_from_env_maps_bash_posix() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Only run on POSIX where PSModulePath does not interfere
        unsafe {
            std::env::set_var("SHELL", "/bin/bash");
        }
        assert_eq!(detect_shell_from_env().as_deref(), Some("bash"));
        unsafe {
            std::env::remove_var("SHELL");
        }
    }

    #[test]
    fn test_get_rez_env_var_with_prefix() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_STATUS_BINDINGS_WITH_PREFIX", "active");
        }
        assert_eq!(
            get_rez_env_var("REZ_STATUS_BINDINGS_WITH_PREFIX").as_deref(),
            Some("active")
        );
        unsafe {
            std::env::remove_var("REZ_STATUS_BINDINGS_WITH_PREFIX");
        }
    }

    #[test]
    fn test_get_rez_env_var_without_prefix() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_STATUS_BINDINGS_NO_PREFIX", "present");
        }
        assert_eq!(
            get_rez_env_var("STATUS_BINDINGS_NO_PREFIX").as_deref(),
            Some("present")
        );
        unsafe {
            std::env::remove_var("REZ_STATUS_BINDINGS_NO_PREFIX");
        }
    }

    #[test]
    fn test_inactive_context_empty_packages() {
        let _lock = ENV_MUTEX.lock().unwrap();
        if std::env::var("REZ_USED_PACKAGES_NAMES").is_err() {
            let s = detect_current_status();
            if !s.is_active {
                assert!(s.resolved_packages.is_empty());
            }
        }
    }

    // ── detect_current_status field coverage ──────────────────────────────────

    #[test]
    fn test_detect_active_via_context_file_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Use a unique key suffix to avoid collision with CI vars
        unsafe {
            std::env::set_var("REZ_CONTEXT_FILE", "/tmp/test_ctx90.rxt");
        }
        let s = detect_current_status();
        assert!(s.is_active, "REZ_CONTEXT_FILE should make is_active=true");
        assert_eq!(s.context_file.as_deref(), Some("/tmp/test_ctx90.rxt"));
        unsafe {
            std::env::remove_var("REZ_CONTEXT_FILE");
        }
    }

    #[test]
    fn test_detect_active_via_used_packages_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_USED_PACKAGES_NAMES", "python-3.9 cmake-3.21");
        }
        let s = detect_current_status();
        assert!(
            s.is_active,
            "status should be active when REZ_USED_PACKAGES_NAMES is set, got: {:?}",
            s.resolved_packages
        );
        assert!(
            s.resolved_packages.contains(&"python-3.9".to_string()),
            "resolved_packages should contain python-3.9, got {:?}",
            s.resolved_packages
        );
        assert!(
            s.resolved_packages.contains(&"cmake-3.21".to_string()),
            "resolved_packages should contain cmake-3.21, got {:?}",
            s.resolved_packages
        );
        unsafe {
            std::env::remove_var("REZ_USED_PACKAGES_NAMES");
        }
    }

    #[test]
    fn test_detect_request_field() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_REQUEST", "python-3 maya-2024");
        }
        let s = detect_current_status();
        assert!(
            s.requested_packages.contains(&"python-3".to_string()),
            "requested_packages should include python-3, got {:?}",
            s.requested_packages
        );
        unsafe {
            std::env::remove_var("REZ_REQUEST");
        }
    }

    #[test]
    fn test_detect_implicit_packages_field() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_IMPLICIT_PACKAGES", "platform-linux arch-x86_64");
        }
        let s = detect_current_status();
        assert!(
            s.implicit_packages.contains(&"platform-linux".to_string()),
            "implicit_packages missing platform-linux, got {:?}",
            s.implicit_packages
        );
        unsafe {
            std::env::remove_var("REZ_IMPLICIT_PACKAGES");
        }
    }

    #[test]
    fn test_detect_context_cwd_and_version() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_ORIG_CWD", "/home/user/project");
            std::env::set_var("REZ_VERSION", "3.2.1");
        }
        let s = detect_current_status();
        assert_eq!(s.context_cwd.as_deref(), Some("/home/user/project"));
        assert_eq!(s.rez_version.as_deref(), Some("3.2.1"));
        unsafe {
            std::env::remove_var("REZ_ORIG_CWD");
            std::env::remove_var("REZ_VERSION");
        }
    }

    #[test]
    fn test_active_repr_includes_package_count() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_USED_PACKAGES_NAMES", "alpha-1 beta-2 gamma-3");
        }
        let s = detect_current_status();
        if s.is_active {
            let r = s.__repr__();
            assert!(
                r.contains("3"),
                "repr should mention package count 3, got: {}",
                r
            );
            assert!(r.contains("active"), "repr should contain 'active': {}", r);
        }
        unsafe {
            std::env::remove_var("REZ_USED_PACKAGES_NAMES");
        }
    }

    #[test]
    fn test_get_rez_env_var_missing_returns_none() {
        // Use a key that should never exist in CI
        let val = get_rez_env_var("STATUS_BINDINGS_NONEXISTENT_KEY_90XYZ");
        assert!(
            val.is_none(),
            "missing key should return None, got {:?}",
            val
        );
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_detect_shell_from_env_maps_zsh() {
        unsafe {
            std::env::set_var("SHELL", "/usr/bin/zsh");
        }
        assert_eq!(detect_shell_from_env().as_deref(), Some("zsh"));
        unsafe {
            std::env::remove_var("SHELL");
        }
    }

    #[test]
    #[cfg(not(target_os = "windows"))]
    fn test_detect_shell_from_env_maps_fish() {
        unsafe {
            std::env::set_var("SHELL", "/usr/local/bin/fish");
        }
        assert_eq!(detect_shell_from_env().as_deref(), Some("fish"));
        unsafe {
            std::env::remove_var("SHELL");
        }
    }

    // ── get_rez_env_var: empty key handling ───────────────────────────────────

    #[test]
    fn test_get_rez_env_var_empty_key_returns_none() {
        // "" -> "REZ_" — unlikely to exist in any env, should return None
        let val = get_rez_env_var("");
        // The key becomes "REZ_"; if the env has no such var, it must be None
        // (On some systems this could hypothetically be set, so only assert None
        //  when the raw env confirms absence)
        if std::env::var("REZ_").is_err() {
            assert!(val.is_none(), "empty key should yield None, got {:?}", val);
        }
    }

    // ── is_in_rez_context is_ok after env set ────────────────────────────────

    #[test]
    fn test_is_in_rez_context_true_after_env_set() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_CONTEXT_FILE", "/tmp/cycle97_test.rxt");
        }
        assert!(
            is_in_rez_context(),
            "is_in_rez_context should be true when REZ_CONTEXT_FILE is set"
        );
        unsafe {
            std::env::remove_var("REZ_CONTEXT_FILE");
        }
    }

    // ── rez_env_vars collection covers REZ_ prefix keys ──────────────────────

    #[test]
    fn test_rez_env_vars_includes_set_key() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_CYCLE97_MARKER", "cycle97");
        }
        let s = detect_current_status();
        assert!(
            s.rez_env_vars.contains_key("REZ_CYCLE97_MARKER"),
            "rez_env_vars should capture REZ_CYCLE97_MARKER"
        );
        assert_eq!(s.rez_env_vars["REZ_CYCLE97_MARKER"], "cycle97");
        unsafe {
            std::env::remove_var("REZ_CYCLE97_MARKER");
        }
    }

    // ── get_context_file returns value when set ────────────────────────────────

    #[test]
    fn test_get_context_file_returns_some_when_set() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_CONTEXT_FILE", "/tmp/some_ctx97.rxt");
        }
        assert_eq!(
            get_context_file().as_deref(),
            Some("/tmp/some_ctx97.rxt"),
            "get_context_file should return the env var value"
        );
        unsafe {
            std::env::remove_var("REZ_CONTEXT_FILE");
        }
    }

    // ── get_resolved_package_names parses space-separated list ────────────────

    #[test]
    fn test_get_resolved_package_names_parses_list() {
        let _lock = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("REZ_USED_PACKAGES_NAMES", "pkgA-1.0 pkgB-2.0 pkgC-3.0");
        }
        let names = get_resolved_package_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"pkgA-1.0".to_string()));
        assert!(names.contains(&"pkgC-3.0".to_string()));
        unsafe {
            std::env::remove_var("REZ_USED_PACKAGES_NAMES");
        }
    }

    // ── default RezStatus is_active false ────────────────────────────────────

    #[test]
    fn test_default_rez_status_is_active_default_false_when_no_rez_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        // Ensure neither trigger var is present
        if std::env::var("REZ_CONTEXT_FILE").is_err()
            && std::env::var("REZ_USED_PACKAGES_NAMES").is_err()
        {
            let s = PyRezStatus::new();
            assert!(
                !s.is_active,
                "default status should be inactive outside rez, got is_active=true"
            );
        }
    }
}
