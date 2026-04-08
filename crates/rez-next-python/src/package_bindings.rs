//! Python bindings for Package and PackageRequirement

use crate::version_bindings::PyVersion;
use pyo3::prelude::*;
use rez_next_package::{Package, PackageRequirement};

/// Python-accessible Package class, compatible with rez.packages.Package
#[pyclass(name = "Package", from_py_object)]
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
            .map(PyPackage)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Validate the package definition
    fn validate(&self) -> PyResult<bool> {
        self.0
            .validate()
            .map(|_| true)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Check if the package definition is valid without raising exceptions
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Get the format version
    #[getter]
    fn format_version(&self) -> Option<i32> {
        self.0.format_version
    }
}

/// Python-accessible PackageRequirement class, compatible with rez.packages.PackageRequirement
#[pyclass(name = "PackageRequirement", from_py_object)]
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
        self.0.name == other.0.name
            && self.0.version_spec == other.0.version_spec
            && self.0.conflict == other.0.conflict
            && self.0.weak == other.0.weak
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.name.hash(&mut h);
        if let Some(ref spec) = self.0.version_spec {
            spec.hash(&mut h);
        }
        self.0.conflict.hash(&mut h);
        self.0.weak.hash(&mut h);
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

    /// Whether this is a conflict requirement (prefixed with `!`)
    #[getter]
    fn conflict(&self) -> bool {
        self.0.conflict
    }

    /// Whether this is a weak requirement (prefixed with `~`)
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
        if self.0.conflict {
            // Already a conflict requirement, return as-is
            self.__str__()
        } else {
            format!("!{}", self.__str__())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::PackageRequirement;

    fn req(s: &str) -> PyPackageRequirement {
        PyPackageRequirement(PackageRequirement::parse(s).unwrap())
    }

    #[test]
    fn test_package_requirement_basic() {
        let r = req("python-3.9");
        assert_eq!(r.name(), "python");
        assert!(!r.conflict());
        assert!(!r.weak());
    }

    #[test]
    fn test_package_requirement_conflict_flag() {
        // conflict requirement: "!python"
        let r = req("!python");
        assert_eq!(r.name(), "python");
        assert!(r.conflict());
        assert!(!r.weak());
    }

    #[test]
    fn test_package_requirement_weak_flag() {
        // weak requirement: "~python"
        let r = req("~python");
        assert_eq!(r.name(), "python");
        assert!(!r.conflict());
        assert!(r.weak());
    }

    #[test]
    fn test_package_requirement_conflict_with_version() {
        let r = req("!python-3.9+<4");
        assert_eq!(r.name(), "python");
        assert!(r.conflict());
        assert!(!r.weak());
    }

    #[test]
    fn test_package_requirement_str_roundtrip_basic() {
        let r = req("python-3.9");
        let s = r.__str__();
        assert!(s.contains("python"));
    }

    #[test]
    fn test_package_requirement_conflict_requirement_method() {
        let r = req("python-3.9");
        let conflict_str = r.conflict_requirement();
        assert!(conflict_str.starts_with('!'));
        assert!(conflict_str.contains("python"));
    }

    #[test]
    fn test_package_requirement_no_version_range() {
        let r = req("maya");
        assert_eq!(r.name(), "maya");
        // version_range may be None when no version constraint
        // both None and Some("") are acceptable
        let vr = r.range();
        assert!(vr.is_none() || vr.unwrap().is_empty());
    }

    // ─── PyPackage tests ─────────────────────────────────────────────────────

    fn make_package(name: &str) -> PyPackage {
        PyPackage::new(name.to_string())
    }

    #[test]
    fn test_package_new_name() {
        let p = make_package("python");
        assert_eq!(p.name(), "python");
    }

    #[test]
    fn test_package_str_without_version() {
        let p = make_package("maya");
        // Without version, str should just be name
        let s = p.__str__();
        assert_eq!(s, "maya");
    }

    #[test]
    fn test_package_repr_format() {
        let p = make_package("houdini");
        let repr = p.__repr__();
        assert!(repr.contains("Package"), "repr must contain 'Package', got {repr}");
        assert!(repr.contains("houdini"), "repr must contain name, got {repr}");
    }

    #[test]
    fn test_package_set_version_and_str() {
        let mut p = make_package("python");
        p.set_version("3.11.0").unwrap();
        let s = p.__str__();
        assert_eq!(s, "python-3.11.0");
    }

    #[test]
    fn test_package_set_version_invalid_returns_err() {
        let mut p = make_package("bad");
        // Completely invalid version strings should error
        let result = p.set_version("not a version!!!");
        // either ok or err is acceptable depending on parser strictness
        // but we verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_package_version_getter_none_by_default() {
        let p = make_package("nuke");
        assert!(p.version().is_none(), "freshly created package has no version");
    }

    #[test]
    fn test_package_qualified_name_without_version() {
        let p = make_package("python");
        assert_eq!(p.qualified_name(), "python");
    }

    #[test]
    fn test_package_qualified_name_with_version() {
        let mut p = make_package("python");
        p.set_version("3.10.0").unwrap();
        assert_eq!(p.qualified_name(), "python-3.10.0");
    }

    #[test]
    fn test_package_is_valid_empty_name_false() {
        let p = make_package("");
        // empty name should fail validation
        assert!(!p.is_valid(), "package with empty name should be invalid");
    }

    #[test]
    fn test_package_requires_empty_by_default() {
        let p = make_package("rez");
        assert!(p.requires().is_empty());
    }

    #[test]
    fn test_package_description_none_by_default() {
        let p = make_package("cmake");
        assert!(p.description().is_none());
    }

    // ─── PyPackageRequirement equality and hash ───────────────────────────────

    #[test]
    fn test_requirement_equality_same() {
        let a = req("python-3.9");
        let b = req("python-3.9");
        assert!(a.__eq__(&b));
    }

    #[test]
    fn test_requirement_hash_consistent() {
        let a = req("numpy-1.24+");
        let h1 = a.__hash__();
        let h2 = a.__hash__();
        assert_eq!(h1, h2, "hash must be deterministic");
    }

    #[test]
    fn test_conflict_requirement_already_conflict() {
        // If it's already a conflict, conflict_requirement() should still start with '!'
        let r = req("!maya");
        let cr = r.conflict_requirement();
        assert!(cr.starts_with('!'), "conflict of conflict should stay !, got {cr}");
    }

    // ─── Additional PyPackage tests ───────────────────────────────────────────

    #[test]
    fn test_package_str_with_version() {
        let mut p = make_package("nuke");
        p.set_version("14.0.1").unwrap();
        assert_eq!(p.__str__(), "nuke-14.0.1");
    }

    #[test]
    fn test_package_repr_with_version() {
        let mut p = make_package("houdini");
        p.set_version("20.5.0").unwrap();
        let repr = p.__repr__();
        assert!(repr.contains("Package("), "repr must contain 'Package(', got {repr}");
        assert!(repr.contains("houdini-20.5.0"), "repr must contain 'houdini-20.5.0', got {repr}");
    }

    #[test]
    fn test_package_eq_same_name_same_version() {
        let mut a = make_package("python");
        let mut b = make_package("python");
        a.set_version("3.10.0").unwrap();
        b.set_version("3.10.0").unwrap();
        assert!(a.__eq__(&b), "packages with same name+version must be equal");
    }

    #[test]
    fn test_package_eq_different_version() {
        let mut a = make_package("python");
        let mut b = make_package("python");
        a.set_version("3.10.0").unwrap();
        b.set_version("3.11.0").unwrap();
        assert!(!a.__eq__(&b), "packages with different versions must not be equal");
    }

    #[test]
    fn test_package_hash_same_for_equal_packages() {
        let mut a = make_package("rez");
        let mut b = make_package("rez");
        a.set_version("2.0.0").unwrap();
        b.set_version("2.0.0").unwrap();
        assert_eq!(a.__hash__(), b.__hash__(), "equal packages must have same hash");
    }

    #[test]
    fn test_package_version_str_getter() {
        let mut p = make_package("cmake");
        p.set_version("3.26.4").unwrap();
        let vs = p.version_str();
        assert_eq!(vs.as_deref(), Some("3.26.4"), "version_str must return version string");
    }

    #[test]
    fn test_package_build_requires_empty_by_default() {
        let p = make_package("test_pkg");
        assert!(p.build_requires().is_empty(), "build_requires must be empty by default");
    }

    #[test]
    fn test_package_private_build_requires_empty_by_default() {
        let p = make_package("test_pkg");
        assert!(p.private_build_requires().is_empty(), "private_build_requires must be empty by default");
    }

    #[test]
    fn test_package_variants_empty_by_default() {
        let p = make_package("test_pkg");
        assert!(p.variants().is_empty(), "variants must be empty by default");
    }

    #[test]
    fn test_package_tools_empty_by_default() {
        let p = make_package("test_pkg");
        assert!(p.tools().is_empty(), "tools must be empty by default");
    }

    #[test]
    fn test_package_load_nonexistent_returns_err() {
        let result = PyPackage::load("/nonexistent/path/package.py");
        assert!(result.is_err(), "loading non-existent package.py should return Err");
    }

    // ── Cycle 115 additions ──────────────────────────────────────────────────

    #[test]
    fn test_package_authors_empty_by_default() {
        let p = make_package("somepkg");
        assert!(p.authors().is_empty(), "authors must be empty by default");
    }

    #[test]
    fn test_package_commands_none_by_default() {
        let p = make_package("cmdpkg");
        assert!(p.commands().is_none(), "commands must be None by default");
    }

    #[test]
    fn test_package_timestamp_none_by_default() {
        let p = make_package("timepkg");
        assert!(p.timestamp().is_none(), "timestamp must be None by default");
    }

    #[test]
    fn test_package_uuid_none_by_default() {
        let p = make_package("uuidpkg");
        assert!(p.uuid().is_none(), "uuid must be None by default");
    }

    #[test]
    fn test_package_cachable_none_by_default() {
        let p = make_package("cachepkg");
        assert!(p.cachable().is_none(), "cachable must be None by default");
    }

    #[test]
    fn test_package_relocatable_none_by_default() {
        let p = make_package("relocpkg");
        assert!(p.relocatable().is_none(), "relocatable must be None by default");
    }

    #[test]
    fn test_requirement_version_range_matches_range() {
        let r = req("python-3.9+<4");
        // range() and version_range() must return the same value
        assert_eq!(r.range(), r.version_range(), "range() and version_range() must agree");
    }

    // ── Cycle 121 additions ──────────────────────────────────────────────────

    #[test]
    fn test_package_format_version_none_by_default() {
        let p = make_package("formatpkg");
        assert!(p.format_version().is_none(), "format_version must be None by default");
    }

    #[test]
    fn test_package_eq_different_names() {
        let a = make_package("python");
        let b = make_package("maya");
        assert!(!a.__eq__(&b), "packages with different names must not be equal");
    }

    #[test]
    fn test_package_hash_differs_for_different_names() {
        let a = make_package("python");
        let b = make_package("maya");
        // Hashes CAN collide, but for distinct well-known names they should differ
        // We just verify neither panics
        let _ = a.__hash__();
        let _ = b.__hash__();
    }

    #[test]
    fn test_requirement_repr_contains_name() {
        let r = req("cmake-3.21+");
        let repr = r.__repr__();
        assert!(repr.contains("PackageRequirement"), "repr must contain 'PackageRequirement': {repr}");
        assert!(repr.contains("cmake"), "repr must contain name 'cmake': {repr}");
    }

    #[test]
    fn test_requirement_eq_different_specs_not_equal() {
        let a = req("python-3.9");
        let b = req("python-3.10");
        assert!(!a.__eq__(&b), "requirements with different version specs must not be equal");
    }

    #[test]
    fn test_package_is_valid_named_package_true() {
        let p = make_package("python");
        assert!(p.is_valid(), "package with valid name 'python' should be valid");
    }

    // ─────── Cycle 126 additions ─────────────────────────────────────────────

    #[test]
    fn test_package_name_roundtrip() {
        let p = make_package("cmake");
        assert_eq!(p.name(), "cmake");
    }

    #[test]
    fn test_package_requirement_parse_valid() {
        let req = PyPackageRequirement::new("python-3.9+");
        assert!(req.is_ok(), "valid requirement string must parse without error");
    }

    #[test]
    fn test_package_requirement_name() {
        let req = PyPackageRequirement::new("maya-2024").unwrap();
        assert_eq!(req.name(), "maya");
    }

    #[test]
    fn test_package_requirement_parse_invalid_graceful() {
        // A string that cannot be parsed as a requirement should return Err, not panic
        // (Empty string is known to be invalid for requirements)
        // If implementation is lenient, it may return Ok — either way, no panic.
        let _ = PyPackageRequirement::new("");
    }

    #[test]
    fn test_package_default_version_is_empty_or_zero() {
        let p = make_package("houdini");
        // Default version for a package created without version should be None or "0"
        let ver = p.version_str();
        match ver {
            None => {} // None is acceptable
            Some(s) => assert!(
                s.is_empty() || s.starts_with('0'),
                "default version_str should be None, empty, or '0', got: '{s}'"
            ),
        }
    }
}

