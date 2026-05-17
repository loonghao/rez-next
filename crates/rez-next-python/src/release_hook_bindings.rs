//! Python bindings for release hook system.
//!
//! Exposes the release hook registry and base hook class to Python.

use pyo3::prelude::*;
use rez_next_release_hook::{create_release_hook, get_release_hook_types, ReleaseHook};
use std::path::PathBuf;

/// Get available release hook types.
#[pyfunction]
pub fn py_get_release_hook_types() -> PyResult<Vec<String>> {
    Ok(get_release_hook_types())
}

/// Create a release hook by name.
#[pyfunction]
pub fn py_create_release_hook(name: &str, source_path: &str) -> PyResult<PyReleaseHook> {
    let hook = create_release_hook(name, PathBuf::from(source_path))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Wrap in PyReleaseHook
    Ok(PyReleaseHook::new(hook))
}

/// Register the release_hook module with Python.
pub fn register_release_hook_module(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Initialize built-in hooks
    rez_next_release_hook::init();

    // Add classes
    m.add_class::<PyReleaseHook>()?;

    // Add module-level functions
    m.add_function(wrap_pyfunction!(py_get_release_hook_types, m)?)?;
    m.add_function(wrap_pyfunction!(py_create_release_hook, m)?)?;

    Ok(())
}

/// Python wrapper for ReleaseHook trait objects.
///
/// This allows Python code to work with ReleaseHook instances.
#[pyclass(name = "ReleaseHook")]
pub struct PyReleaseHook {
    inner: Box<dyn ReleaseHook>,
}

impl PyReleaseHook {
    /// Create a new PyReleaseHook wrapper.
    pub fn new(hook: Box<dyn ReleaseHook>) -> Self {
        Self { inner: hook }
    }
}

#[pymethods]
impl PyReleaseHook {
    /// Call pre-build hook.
    fn pre_build(
        &self,
        user: &str,
        install_path: &str,
        variants: Option<Vec<usize>>,
        release_message: Option<&str>,
        changelog: Option<Vec<String>>,
        previous_version: Option<&str>,
        previous_revision: Option<&str>,
    ) -> PyResult<()> {
        let variants_slice = variants.as_deref();
        let changelog_slice = changelog.as_deref();

        self.inner
            .pre_build(
                user,
                std::path::Path::new(install_path),
                variants_slice,
                release_message,
                changelog_slice,
                previous_version,
                previous_revision,
            )
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Call pre-release hook.
    fn pre_release(
        &self,
        user: &str,
        install_path: &str,
        variants: Option<Vec<usize>>,
        release_message: Option<&str>,
        changelog: Option<Vec<String>>,
        previous_version: Option<&str>,
        previous_revision: Option<&str>,
    ) -> PyResult<()> {
        let variants_slice = variants.as_deref();
        let changelog_slice = changelog.as_deref();

        self.inner
            .pre_release(
                user,
                std::path::Path::new(install_path),
                variants_slice,
                release_message,
                changelog_slice,
                previous_version,
                previous_revision,
            )
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Call post-release hook.
    fn post_release(
        &self,
        user: &str,
        install_path: &str,
        variants: Vec<String>,
        release_message: Option<&str>,
        changelog: Option<Vec<String>>,
        previous_version: Option<&str>,
        previous_revision: Option<&str>,
    ) -> PyResult<()> {
        let changelog_slice = changelog.as_deref();

        self.inner
            .post_release(
                user,
                std::path::Path::new(install_path),
                &variants,
                release_message,
                changelog_slice,
                previous_version,
                previous_revision,
            )
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}
