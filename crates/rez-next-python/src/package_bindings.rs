//! Python bindings for Package and PackageRequirement

use crate::version_bindings::PyVersion;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_package::{Package, PackageRequirement};
use std::collections::HashMap;

/// Python-accessible Package class, compatible with rez.packages.Package
#[pyclass(name = "Package")]
#[derive(Clone)]
pub struct PyPackage(pub Package);

#[pymethods]
impl PyPackage {
    /// Create a new Package with the given name
    #[new]
    pub fn new(name: String) -> Self {
        PyPackage(Package::new(name))
    }

    fn __str__(&self) -> String {
        match &self.0.version {
            Some(v) => format!("{}-{}", self.0.name, v.as_str()),
            None => self.0.name.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Package('{}')", self.__str__())
    }

    fn __eq__(&self, other: &PyPackage) -> bool {
        self.0.name == other.0.name
            && self.0.version.as_ref().map(|v| v.as_str())
                == other.0.version.as_ref().map(|v| v.as_str())
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.name.hash(&mut h);
        if let Some(ref v) = self.0.version {
            v.as_str().hash(&mut h);
        }
        h.finish()
    }

    /// Package name
    #[getter]
    fn name(&self) -> String {
        self.0.name.clone()
    }

    /// Package version as PyVersion (or None)
    #[getter]
    fn version(&self) -> Option<PyVersion> {
        self.0.version.as_ref().map(|v| PyVersion(v.clone()))
    }

    /// Package version as string
    #[getter]
    fn version_str(&self) -> Option<String> {
        self.0.version.as_ref().map(|v| v.as_str().to_string())
    }

    /// Qualified name (name-version)
    #[getter]
    fn qualified_name(&self) -> String {
        self.__str__()
    }

    /// Description
    #[getter]
    fn description(&self) -> Option<String> {
        self.0.description.clone()
    }

    #[setter]
    fn set_description(&mut self, desc: Option<String>) {
        self.0.description = desc;
    }

    /// Authors
    #[getter]
    fn authors(&self) -> Vec<String> {
        self.0.authors.clone()
    }

    /// Runtime requires
    #[getter]
    fn requires(&self) -> Vec<String> {
        self.0.requires.clone()
    }

    /// Build requires
    #[getter]
    fn build_requires(&self) -> Vec<String> {
        self.0.build_requires.clone()
    }

    /// Private build requires
    #[getter]
    fn private_build_requires(&self) -> Vec<String> {
        self.0.private_build_requires.clone()
    }

    /// Variants
    #[getter]
    fn variants(&self) -> Vec<Vec<String>> {
        self.0.variants.clone()
    }

    /// Tools
    #[getter]
    fn tools(&self) -> Vec<String> {
        self.0.tools.clone()
    }

    /// Commands string
    #[getter]
    fn commands(&self) -> Option<String> {
        self.0.commands.clone()
    }

    /// Timestamp (Unix)
    #[getter]
    fn timestamp(&self) -> Option<i64> {
        self.0.timestamp
    }

    /// UUID
    #[getter]
    fn uuid(&self) -> Option<String> {
        self.0.uuid.clone()
    }

    /// Whether package is cachable
    #[getter]
    fn cachable(&self) -> Option<bool> {
        self.0.cachable
    }

    /// Whether package is relocatable
    #[getter]
    fn relocatable(&self) -> Option<bool> {
        self.0.relocatable
    }

    /// Set the version string (rez compat helper)
    fn set_version(&mut self, version_str: &str) -> PyResult<()> {
        use rez_next_version::Version;
        let v = Version::parse(version_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        self.0.version = Some(v);
        Ok(())
    }

    /// Load a package from file (package.py or package.yaml)
    #[staticmethod]
    fn load(path: &str) -> PyResult<PyPackage> {
        use rez_next_package::serialization::PackageSerializer;
        use std::path::PathBuf;

        PackageSerializer::load_from_file(&PathBuf::from(path))
            .map(|p| PyPackage(p))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Validate the package definition
    fn validate(&self) -> PyResult<bool> {
        self.0
            .validate()
            .map(|_| true)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Get the format version
    #[getter]
    fn format_version(&self) -> Option<i32> {
        self.0.format_version
    }
}

/// Python-accessible PackageRequirement class, compatible with rez.packages.PackageRequirement
#[pyclass(name = "PackageRequirement")]
#[derive(Clone)]
pub struct PyPackageRequirement(pub PackageRequirement);

#[pymethods]
impl PyPackageRequirement {
    /// Create a new PackageRequirement from a string like "python-3.9" or "maya>=2024"
    #[new]
    pub fn new(requirement_str: &str) -> PyResult<Self> {
        PackageRequirement::parse(requirement_str)
            .map(PyPackageRequirement)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("PackageRequirement('{}')", self.__str__())
    }

    fn __eq__(&self, other: &PyPackageRequirement) -> bool {
        self.0.name == other.0.name && self.0.version_spec == other.0.version_spec
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.name.hash(&mut h);
        if let Some(ref spec) = self.0.version_spec {
            spec.hash(&mut h);
        }
        h.finish()
    }

    /// Package name
    #[getter]
    fn name(&self) -> String {
        self.0.name.clone()
    }

    /// Version specification string (rez compat: .range)
    #[getter]
    fn range(&self) -> Option<String> {
        self.0.version_spec.clone()
    }

    /// Version specification string (rez compat alias: .version_range)
    #[getter]
    fn version_range(&self) -> Option<String> {
        self.0.version_spec.clone()
    }

    /// Whether this is a weak requirement
    #[getter]
    fn weak(&self) -> bool {
        self.0.weak
    }

    /// Check if a version satisfies this requirement
    fn satisfied_by(&self, version: &PyVersion) -> bool {
        self.0.satisfied_by(&version.0)
    }

    /// Convert to conflict requirement (negate range)
    fn conflict_requirement(&self) -> String {
        format!("!{}", self.__str__())
    }
}
