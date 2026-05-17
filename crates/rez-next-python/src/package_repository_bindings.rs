//! Python bindings for package_repository module
//!
//! This module provides Python bindings for the PackageRepository trait
//! and FilesystemPackageRepository implementation.
//!
//! Corresponds to rez's package_repository.py.

use pyo3::prelude::*;
use pyo3::types::PyType;

use rez_next_repository::package_repository::{
    FilesystemPackageRepository as RustFilesystemPackageRepository, PackageRepository,
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

    /// Check if the repository is empty
    pub fn is_empty(&self) -> PyResult<bool> {
        self.inner
            .is_empty()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
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

    /// Remove a package (can remove ignored packages)
    ///
    /// Args:
    ///     name: Package name
    ///     version: Optional version string (None to remove all versions)
    ///
    /// Returns:
    ///     True if package was removed, False if not found
    #[pyo3(signature = (name, version = None))]
    pub fn remove_package(&mut self, name: &str, version: Option<&str>) -> PyResult<bool> {
        self.inner
            .remove_package(name, version)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Remove a package family
    ///
    /// Args:
    ///     name: Package family name
    ///     force: If True, remove even if family has packages
    ///
    /// Returns:
    ///     True if family was removed, False if not found
    #[pyo3(signature = (name, force = false))]
    pub fn remove_package_family(&mut self, name: &str, force: bool) -> PyResult<bool> {
        self.inner
            .remove_package_family(name, force)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Remove ignored packages older than specified days
    ///
    /// Args:
    ///     days: Remove packages ignored for more than this many days
    ///     dry_run: If True, only count without removing
    ///     verbose: If True, print verbose output
    ///
    /// Returns:
    ///     Number of packages removed (or would be removed if dry_run)
    #[pyo3(signature = (days, dry_run = false, verbose = false))]
    pub fn remove_ignored_since(
        &mut self,
        days: i32,
        dry_run: bool,
        verbose: bool,
    ) -> PyResult<i32> {
        self.inner
            .remove_ignored_since(days, dry_run, verbose)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }
}

/// Register the package_repository submodule with all its classes and functions.
///
/// This function creates the `rez_next._native.package_repository` submodule
/// and registers it in `sys.modules` for proper dotted-path imports.
pub fn register_package_repository_submodule(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent_module.py(), "package_repository")?;

    // Add FilesystemPackageRepository class
    m.add_class::<PyFilesystemPackageRepository>()?;

    // Add module-level functions
    // Get the list of available package repository types
    m.add_function(wrap_pyfunction!(get_package_repository_types, &m)?)?;

    // Register in sys.modules for `from rez_next.package_repository import ...`
    let sys = parent_module.py().import("sys")?;
    let modules = sys.getattr("modules")?;
    let parent_name = parent_module.name()?;
    let full_name = format!("{}.{}", parent_name, "package_repository");
    modules.set_item(full_name.as_str(), &m)?;

    // Also register as submodule
    parent_module.add_submodule(&m)?;

    Ok(())
}

/// Get the list of available package repository types.
///
/// Returns:
///     List of registered repository type names (e.g., ["filesystem"]).
#[pyfunction]
pub fn get_package_repository_types() -> Vec<&'static str> {
    vec!["filesystem"]
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

    #[test]
    fn test_get_package_repository_types() {
        let types = get_package_repository_types();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], "filesystem");
    }
}
