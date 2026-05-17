//! Python bindings for explicit packages.

use pyo3::prelude::*;
use pyo3::types::PyType;
use rez_next_explicit::{ExplicitPackage, ExplicitPackages};
use std::path::PathBuf;

/// Python bindings for ExplicitPackage.
#[pyclass(name = "ExplicitPackage", from_py_object)]
#[derive(Clone)]
pub struct PyExplicitPackage {
    inner: ExplicitPackage,
}

#[pymethods]
impl PyExplicitPackage {
    /// Create a new explicit package.
    #[new]
    fn new(name: &str) -> Self {
        Self {
            inner: ExplicitPackage::new(name),
        }
    }

    /// Get the package name.
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Set the package name.
    #[setter]
    fn set_name(&mut self, name: &str) {
        self.inner.name = name.to_string();
    }

    /// Get the version.
    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    /// Set the version.
    #[setter]
    fn set_version(&mut self, version: Option<String>) {
        self.inner.version = version;
    }

    /// Get the path.
    #[getter]
    fn path(&self) -> Option<String> {
        self.inner
            .path
            .clone()
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Set the path.
    #[setter]
    fn set_path(&mut self, path: Option<String>) {
        self.inner.path = path.map(PathBuf::from);
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "ExplicitPackage(name={:?}, version={:?})",
            self.inner.name, self.inner.version
        )
    }
}

/// Python bindings for ExplicitPackages.
#[pyclass(name = "ExplicitPackages", from_py_object)]
#[derive(Clone)]
pub struct PyExplicitPackages {
    inner: ExplicitPackages,
}

#[pymethods]
impl PyExplicitPackages {
    /// Create a new empty collection.
    #[new]
    fn new() -> Self {
        Self {
            inner: ExplicitPackages::new(),
        }
    }

    /// Get the name.
    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    /// Set the name.
    #[setter]
    fn set_name(&mut self, name: Option<String>) {
        self.inner.name = name;
    }

    /// Get the number of packages.
    fn __len__(&self) -> usize {
        self.inner.packages.len()
    }

    /// Add a package.
    fn add_package(&mut self, package: PyExplicitPackage) {
        self.inner.packages.push(package.inner);
    }

    /// Get all packages.
    fn packages(&self) -> Vec<PyExplicitPackage> {
        self.inner
            .packages
            .iter()
            .map(|p| PyExplicitPackage { inner: p.clone() })
            .collect()
    }

    /// Load from a JSON file.
    #[classmethod]
    fn from_path(_cls: &Bound<'_, PyType>, path: &str) -> PyResult<Self> {
        let packages = ExplicitPackages::from_path(path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to load: {}", e))
        })?;
        Ok(Self { inner: packages })
    }

    /// Save to a JSON file.
    fn to_path(&self, path: &str) -> PyResult<()> {
        self.inner.to_path(path).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyIOError, _>(format!("Failed to save: {}", e))
        })?;
        Ok(())
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "ExplicitPackages(name={:?}, count={})",
            self.inner.name,
            self.inner.packages.len()
        )
    }
}

/// Register the explicit module.
pub fn register_explicit_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyExplicitPackage>()?;
    m.add_class::<PyExplicitPackages>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_package_creation() {
        let pkg = PyExplicitPackage::new("python");
        assert_eq!(pkg.name(), "python");
    }

    #[test]
    fn test_explicit_packages_collection() {
        let mut collection = PyExplicitPackages::new();
        collection.set_name(Some("my-suite".to_string()));

        let pkg = PyExplicitPackage::new("python");
        collection.add_package(pkg);

        assert_eq!(collection.__len__(), 1);
        assert_eq!(collection.name(), Some("my-suite".to_string()));
    }
}
