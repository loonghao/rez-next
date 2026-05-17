//! Python bindings for package resources.
//!
//! This module provides Python bindings for PackageFamilyResource, PackageResource,
//! and VariantResource, aligning with Rez's `package_resources.py` interface.

use pyo3::prelude::*;
use rez_next_repository::resources::{PackageFamilyResource, PackageResource, VariantResource};

// ── PyPackageFamilyResource ─────────────────────────────────────

/// Package family resource.
///
/// This corresponds to the PackageFamilyResource class in Rez's package_resources.py.
/// It represents a named group of package versions (e.g., "python" is a family
/// that contains versions like "3.7", "3.8", "3.9", etc.).
#[pyclass(name = "PackageFamilyResource")]
#[derive(Clone)]
pub struct PyPackageFamilyResource {
    inner: PackageFamilyResource,
}

#[pymethods]
impl PyPackageFamilyResource {
    /// Create a new package family resource.
    ///
    /// Args:
    ///     name: Package family name (e.g., "python")
    ///     repository_type: Repository type (e.g., "filesystem")
    ///     repository_location: Repository location (e.g., "/packages")
    #[new]
    pub fn new(name: String, repository_type: String, repository_location: String) -> Self {
        let family = PackageFamilyResource::new(name, repository_type, repository_location);
        Self { inner: family }
    }

    /// Get the package family name.
    #[getter]
    pub fn get_name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the repository type.
    #[getter]
    pub fn get_repository_type(&self) -> String {
        self.inner.repository_type.clone()
    }

    /// Get the repository location.
    #[getter]
    pub fn get_repository_location(&self) -> String {
        self.inner.repository_location.clone()
    }

    /// String representation.
    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Representation for debugging.
    pub fn __repr__(&self) -> String {
        format!(
            "<PackageFamilyResource name={} repository_type={} location={}>",
            self.inner.name, self.inner.repository_type, self.inner.repository_location
        )
    }
}

// ── PyPackageResource ─────────────────────────────────────────

/// Package resource.
///
/// This corresponds to the PackageResource class in Rez's package_resources.py.
/// It represents a specific version of a package (e.g., "python-3.9.0").
#[pyclass(name = "PackageResource")]
#[derive(Clone)]
pub struct PyPackageResource {
    inner: PackageResource,
}

#[pymethods]
impl PyPackageResource {
    /// Create a new package resource from a package name.
    ///
    /// Args:
    ///     name: Package name (e.g., "python")
    ///     repository_type: Repository type (e.g., "filesystem")
    ///     repository_location: Repository location (e.g., "/packages")
    #[new]
    pub fn new(
        name: String,
        repository_type: String,
        repository_location: String,
    ) -> PyResult<Self> {
        // Create a minimal package
        let pkg = rez_next_package::Package::new(name);
        
        let resource = PackageResource::new(pkg, repository_type, repository_location);
        Ok(Self { inner: resource })
    }

    /// Get the package name.
    #[getter]
    pub fn get_name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Get the package version.
    #[getter]
    pub fn get_version(&self) -> Option<String> {
        self.inner.version().map(|v| v.as_str().to_string())
    }

    /// Get the repository type.
    #[getter]
    pub fn get_repository_type(&self) -> String {
        self.inner.repository_type.clone()
    }

    /// Get the repository location.
    #[getter]
    pub fn get_repository_location(&self) -> String {
        self.inner.repository_location.clone()
    }

    /// String representation.
    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Representation for debugging.
    pub fn __repr__(&self) -> String {
        format!(
            "<PackageResource name={} version={:?} repository_type={} location={}>",
            self.inner.name(),
            self.inner.version(),
            self.inner.repository_type,
            self.inner.repository_location
        )
    }
}

// ── PyVariantResource ─────────────────────────────────────────

/// Variant resource.
///
/// This corresponds to the VariantResource class in Rez's package_resources.py.
/// It represents a specific variant (build) of a package version.
#[pyclass(name = "VariantResource")]
#[derive(Clone)]
pub struct PyVariantResource {
    inner: VariantResource,
}

#[pymethods]
impl PyVariantResource {
    /// Create a new variant resource.
    ///
    /// Args:
    ///     name: Package name
    ///     version: Package version (optional)
    ///     index: Variant index (0-based)
    ///     repository_type: Repository type (e.g., "filesystem")
    ///     repository_location: Repository location (e.g., "/packages")
    #[new]
    pub fn new(
        name: String,
        version: Option<String>,
        index: usize,
        repository_type: String,
        repository_location: String,
    ) -> Self {
        let variant = VariantResource::new(
            name,
            version,
            index,
            repository_type,
            repository_location,
        );
        Self { inner: variant }
    }

    /// Get the package name.
    #[getter]
    pub fn get_name(&self) -> String {
        self.inner.name.clone()
    }

    /// Get the package version.
    #[getter]
    pub fn get_version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    /// Get the variant index.
    #[getter]
    pub fn get_index(&self) -> usize {
        self.inner.index()
    }

    /// Get the repository type.
    #[getter]
    pub fn get_repository_type(&self) -> String {
        self.inner.repository_type.clone()
    }

    /// Get the repository location.
    #[getter]
    pub fn get_repository_location(&self) -> String {
        self.inner.repository_location.clone()
    }

    /// Get the variant root path.
    #[getter]
    pub fn get_root(&self) -> Option<String> {
        self.inner.root.clone()
    }

    /// Set the variant root path.
    #[setter]
    pub fn set_root(&mut self, root: Option<String>) {
        self.inner.root = root;
    }

    /// String representation.
    pub fn __str__(&self) -> String {
        format!("{}", self.inner)
    }

    /// Representation for debugging.
    pub fn __repr__(&self) -> String {
        format!(
            "<VariantResource name={} version={:?} index={} repository_type={} location={}>",
            self.inner.name,
            self.inner.version,
            self.inner.index(),
            self.inner.repository_type,
            self.inner.repository_location
        )
    }
}

/// Register the `package_resources` submodule.
pub fn register_package_resources_submodule(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "package_resources")?;

    m.add_class::<PyPackageFamilyResource>()?;
    m.add_class::<PyPackageResource>()?;
    m.add_class::<PyVariantResource>()?;

    // Register submodule
    super::register_submodule(parent, "package_resources", &m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_package_family_resource_create() {
        let family = PyPackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            "/packages".to_string(),
        );
        
        assert_eq!(family.get_name(), "python");
        assert_eq!(family.get_repository_type(), "filesystem");
        assert_eq!(family.get_repository_location(), "/packages");
    }

    #[test]
    fn test_py_variant_resource_create() {
        let mut variant = PyVariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        );
        
        assert_eq!(variant.get_name(), "python");
        assert_eq!(variant.get_version(), Some("3.9.0".to_string()));
        assert_eq!(variant.get_index(), 0);
        assert_eq!(variant.get_repository_type(), "filesystem");
        assert_eq!(variant.get_repository_location(), "/packages");
        assert!(variant.get_root().is_none());
        
        // Test set_root
        variant.set_root(Some("/packages/python/3.9.0/platform-windows".to_string()));
        assert_eq!(
            variant.get_root(),
            Some("/packages/python/3.9.0/platform-windows".to_string())
        );
    }
}
