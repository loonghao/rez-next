//! Python bindings for package_repository module
//!
//! This module provides Python bindings for the PackageRepository trait
//! and FilesystemPackageRepository implementation.

use pyo3::prelude::*;
use pyo3::types::PyType;

use rez_next_repository::package_repository::{
    FilesystemPackageRepository as RustFilesystemPackageRepository,
    PackageRepository,
};

// ── PyFilesystemPackageRepository ─────────────────────────────────────

/// Filesystem-based package repository
///
/// This corresponds to the FilesystemPackageRepository class in rez's
/// package_repository.py. It reads packages from a filesystem path.
#[pyclass(name = "FilesystemPackageRepository", from_py_object)]
#[derive(Clone)]
pub struct PyFilesystemPackageRepository {
    inner: RustFilesystemPackageRepository,
}

#[pymethods]
impl PyFilesystemPackageRepository {
    /// Create a new filesystem package repository
    ///
    /// Args:
    ///     path: Filesystem path to the repository
    #[new]
    pub fn new(path: &str) -> PyResult<Self> {
        let repo = RustFilesystemPackageRepository::new(path);
        Ok(Self { inner: repo })
    }

    /// Create with explicit name
    ///
    /// Args:
    ///     path: Filesystem path to the repository
    ///     name: Explicit name for the repository
    #[classmethod]
    pub fn with_name(_cls: &Bound<'_, PyType>, path: &str, name: String) -> PyResult<Self> {
        let repo = RustFilesystemPackageRepository::with_name(path, name);
        Ok(Self { inner: repo })
    }

    /// Get the repository name
    #[getter]
    pub fn get_name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get the repository location (path)
    #[getter]
    pub fn get_location(&self) -> String {
        self.inner.location().display().to_string()
    }

    /// Get the repository type
    #[getter]
    pub fn get_type(&self) -> String {
        self.inner.repository_type().to_string()
    }

    /// String representation
    pub fn __str__(&self) -> String {
        format!(
            "FilesystemPackageRepository({}, {})",
            self.inner.repository_type(),
            self.inner.location().display()
        )
    }

    /// Representation for debugging
    pub fn __repr__(&self) -> String {
        format!(
            "<FilesystemPackageRepository type={} location={}>",
            self.inner.repository_type(),
            self.inner.location().display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_filesystem_package_repository_create() {
        let repo = PyFilesystemPackageRepository::new("/tmp/packages");
        assert!(repo.is_ok());
    }

    #[test]
    fn test_py_filesystem_package_repository_name() {
        let repo = PyFilesystemPackageRepository::new("/tmp/test_repo");
        let repo = repo.unwrap();
        // Name should be directory name
        assert_eq!(repo.get_name(), "test_repo");
    }

    #[test]
    fn test_py_filesystem_package_repository_location() {
        let repo = PyFilesystemPackageRepository::new("/tmp/packages");
        let repo = repo.unwrap();
        assert_eq!(repo.get_location(), "/tmp/packages");
    }

    #[test]
    fn test_py_filesystem_package_repository_type() {
        let repo = PyFilesystemPackageRepository::new("/tmp/packages");
        let repo = repo.unwrap();
        assert_eq!(repo.get_type(), "filesystem");
    }
}
