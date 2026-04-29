//! Python bindings for Version and VersionRange
//!
//! Provides rez-compatible Version and VersionRange classes.

use pyo3::prelude::*;
use rez_next_version::{Version, VersionRange};

/// Python-accessible Version class, compatible with rez.vendor.version.Version
#[pyclass(name = "Version", from_py_object)]
#[derive(Clone)]
pub struct PyVersion(pub Version);

#[pymethods]
impl PyVersion {
    /// Create a new Version from a string.
    /// Compatible with `rez.vendor.version.Version("1.2.3")`
    #[new]
    #[pyo3(signature = (s=None))]
    pub fn new(s: Option<&str>) -> PyResult<Self> {
        let version_str = s.unwrap_or("");
        Version::parse(version_str)
            .map(PyVersion)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// String representation
    fn __str__(&self) -> String {
        self.0.as_str().to_string()
    }

    fn __repr__(&self) -> String {
        format!("Version('{}')", self.0.as_str())
    }

    /// Rich comparison support
    fn __eq__(&self, other: &PyVersion) -> bool {
        self.0 == other.0
    }

    fn __ne__(&self, other: &PyVersion) -> bool {
        self.0 != other.0
    }

    fn __lt__(&self, other: &PyVersion) -> bool {
        self.0 < other.0
    }

    fn __le__(&self, other: &PyVersion) -> bool {
        self.0 <= other.0
    }

    fn __gt__(&self, other: &PyVersion) -> bool {
        self.0 > other.0
    }

    fn __ge__(&self, other: &PyVersion) -> bool {
        self.0 >= other.0
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.as_str().hash(&mut h);
        h.finish()
    }

    /// Get version as string (rez compat)
    #[getter]
    fn string(&self) -> String {
        self.0.as_str().to_string()
    }

    /// Major version token
    #[getter]
    fn major(&self) -> Option<String> {
        let s = self.0.as_str();
        s.split('.').next().map(|t| t.to_string())
    }

    /// Minor version token
    #[getter]
    fn minor(&self) -> Option<String> {
        let s = self.0.as_str();
        let mut parts = s.split('.');
        parts.next();
        parts.next().map(|t| t.to_string())
    }

    /// Patch version token
    #[getter]
    fn patch(&self) -> Option<String> {
        let s = self.0.as_str();
        let mut parts = s.split('.');
        parts.next();
        parts.next();
        parts.next().map(|t| t.to_string())
    }

    /// Trim to N tokens (rez compat)
    fn trim(&self, len: usize) -> PyResult<Self> {
        let parts: Vec<&str> = self.0.as_str().split('.').take(len).collect();
        Version::parse(&parts.join("."))
            .map(PyVersion)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Return True if version is empty
    fn is_empty(&self) -> bool {
        self.0.as_str().is_empty()
    }

    /// Next version (increment last numeric component)
    fn next(&self) -> PyResult<Self> {
        let s = self.0.as_str();
        let parts: Vec<&str> = s.split('.').collect();
        if parts.is_empty() {
            return Ok(self.clone());
        }

        let mut new_parts: Vec<String> = parts.iter().map(|p| p.to_string()).collect();
        // Try to increment last numeric part
        if let Some(last) = new_parts.last_mut() {
            if let Ok(n) = last.parse::<u64>() {
                *last = (n + 1).to_string();
            } else {
                // Append .1 if last part is not numeric
                new_parts.push("1".to_string());
            }
        }

        Version::parse(&new_parts.join("."))
            .map(PyVersion)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// Python-accessible VersionRange class, compatible with rez.vendor.version.VersionRange
#[pyclass(name = "VersionRange", from_py_object)]
#[derive(Clone)]
pub struct PyVersionRange(pub VersionRange);

#[pymethods]
impl PyVersionRange {
    /// Create a new VersionRange from a string.
    /// Compatible with `rez.vendor.version.VersionRange("1.2+")`
    #[new]
    #[pyo3(signature = (s=None))]
    pub fn new(s: Option<&str>) -> PyResult<Self> {
        let range_str = s.unwrap_or("");
        VersionRange::parse(range_str)
            .map(PyVersionRange)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __str__(&self) -> String {
        self.0.as_str().to_string()
    }

    fn __repr__(&self) -> String {
        format!("VersionRange('{}')", self.0.as_str())
    }

    fn __eq__(&self, other: &PyVersionRange) -> bool {
        self.0.as_str() == other.0.as_str()
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.as_str().hash(&mut h);
        h.finish()
    }

    /// Check if a version is contained in this range
    fn contains(&self, version: &PyVersion) -> bool {
        self.0.contains(&version.0)
    }

    /// Check if this range is "any" (no restrictions)
    fn is_any(&self) -> bool {
        self.0.is_any()
    }

    /// Check if range is empty (impossible to satisfy)
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Intersect two ranges
    fn intersect(&self, other: &PyVersionRange) -> Option<PyVersionRange> {
        self.0.intersect(&other.0).map(PyVersionRange)
    }

    /// Union of two ranges (combines both ranges)
    fn union(&self, other: &PyVersionRange) -> PyResult<PyVersionRange> {
        let s1 = self.0.as_str();
        let s2 = other.0.as_str();

        // If either is "any" (empty string), union is "any"
        if s1.is_empty() || s2.is_empty() {
            return VersionRange::parse("")
                .map(PyVersionRange)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()));
        }

        // If ranges are identical, return as-is
        if s1 == s2 {
            return Ok(self.clone());
        }

        // Combine with pipe separator (rez union notation)
        let union_str = format!("{}|{}", s1, s2);
        VersionRange::parse(&union_str)
            .map(PyVersionRange)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Check if this range intersects with another (has any overlap)
    fn intersects(&self, other: &PyVersionRange) -> bool {
        self.0.intersects(&other.0)
    }

    /// Check if this range is a subset of another
    fn is_subset_of(&self, other: &PyVersionRange) -> bool {
        self.0.is_subset_of(&other.0)
    }

    /// Check if this range is a superset of another
    fn is_superset_of(&self, other: &PyVersionRange) -> bool {
        self.0.is_superset_of(&other.0)
    }

    /// Subtract another range from this range (set difference)
    fn subtract(&self, other: &PyVersionRange) -> Option<PyVersionRange> {
        self.0.subtract(&other.0).map(PyVersionRange)
    }

    /// Class method: create a VersionRange that matches any version.
    /// Equivalent to `VersionRange("")` or `VersionRange("*")`.
    #[classmethod]
    fn any(_cls: &pyo3::Bound<'_, pyo3::types::PyType>) -> Self {
        PyVersionRange(VersionRange::any())
    }

    /// Class method: create a VersionRange that matches no version (empty set).
    /// Equivalent to `VersionRange("!*")`.
    #[classmethod]
    fn none(_cls: &pyo3::Bound<'_, pyo3::types::PyType>) -> Self {
        PyVersionRange(VersionRange::none())
    }

    /// Static method: parse from string (alias for `VersionRange(s)`).
    /// Provided for rez API compatibility where `.from_str()` is used.
    #[staticmethod]
    fn from_str(s: &str) -> PyResult<Self> {
        VersionRange::parse(s)
            .map(PyVersionRange)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Return the string representation of the range (rez compat: `.as_str()`).
    fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
#[path = "version_bindings_tests.rs"]
mod tests;
