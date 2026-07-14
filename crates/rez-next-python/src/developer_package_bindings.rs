//! Python bindings for the developer_package module.
//!
//! Exposes `DeveloperPackage` and `PreprocessMode` to Python, aligned with
//! rez's `developer_package.py` API.

use pyo3::prelude::*;
use pyo3::types::PySet;

use rez_next_package::developer_package::{DeveloperPackage, PreprocessMode};

/// Python-exposed PreprocessMode enum.
#[pyclass(name = "PreprocessMode", skip_from_py_object)]
#[derive(Clone)]
pub struct PyPreprocessMode {
    inner: PreprocessMode,
}

#[pymethods]
#[allow(non_upper_case_globals)]
impl PyPreprocessMode {
    /// Local preprocess runs BEFORE global preprocess (value = 0).
    #[classattr]
    const Before: i32 = 0;

    /// Local preprocess runs AFTER global preprocess (value = 1).
    #[classattr]
    const After: i32 = 1;

    /// Local preprocess OVERRIDES global preprocess (value = 2).
    #[classattr]
    const Override: i32 = 2;

    fn __repr__(&self) -> String {
        match self.inner {
            PreprocessMode::Before => "PreprocessMode.Before".to_string(),
            PreprocessMode::After => "PreprocessMode.After".to_string(),
            PreprocessMode::Override => "PreprocessMode.Override".to_string(),
        }
    }

    fn __eq__(&self, other: &PyPreprocessMode) -> bool {
        self.inner == other.inner
    }
}

impl Default for PyPreprocessMode {
    fn default() -> Self {
        Self {
            inner: PreprocessMode::Before,
        }
    }
}

/// Python-exposed DeveloperPackage class.
#[pyclass(name = "DeveloperPackage", skip_from_py_object)]
pub struct PyDeveloperPackage {
    inner: DeveloperPackage,
}

#[pymethods]
impl PyDeveloperPackage {
    /// Load a DeveloperPackage from a directory path.
    ///
    /// Args:
    ///     path: Directory containing package.py or package.yaml
    ///
    /// Returns:
    ///     DeveloperPackage instance
    #[staticmethod]
    fn from_path(path: &str) -> PyResult<Self> {
        let dev_pkg = DeveloperPackage::from_path(std::path::Path::new(path))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self { inner: dev_pkg })
    }

    /// The package name.
    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    /// The package version string (if available).
    #[getter]
    fn version_string(&self) -> Option<&str> {
        self.inner.version_string()
    }

    /// Path to the package definition file.
    #[getter]
    fn filepath(&self) -> String {
        self.inner.filepath.to_string_lossy().to_string()
    }

    /// Root directory of the package.
    #[getter]
    fn root(&self) -> String {
        self.inner.root.to_string_lossy().to_string()
    }

    /// Included module names from @include decorators.
    fn get_includes(&self, py: Python<'_>) -> PyResult<Py<PySet>> {
        let includes = PySet::new(py, &[] as &[&str])?;
        for inc in &self.inner.includes {
            includes.add(inc.as_str())?;
        }
        Ok(includes.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "DeveloperPackage('{}' at {})",
            self.inner.name(),
            self.inner.root.display()
        )
    }
}

/// Register developer_package types in the given module.
pub fn register_developer_package_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDeveloperPackage>()?;
    m.add_class::<PyPreprocessMode>()?;
    Ok(())
}
