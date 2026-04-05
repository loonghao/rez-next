//! Python bindings for Version and VersionRange
//!
//! Provides rez-compatible Version and VersionRange classes.

use pyo3::prelude::*;
use rez_next_version::{Version, VersionRange};

/// Python-accessible Version class, compatible with rez.vendor.version.Version
#[pyclass(name = "Version")]
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
#[pyclass(name = "VersionRange")]
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
mod tests {
    use super::*;
    use rez_next_version::{Version, VersionRange};

    fn pv(s: &str) -> PyVersion {
        PyVersion(Version::parse(s).unwrap())
    }

    fn pvr(s: &str) -> PyVersionRange {
        PyVersionRange(VersionRange::parse(s).unwrap())
    }

    #[test]
    fn test_py_version_str() {
        let v = pv("1.2.3");
        assert_eq!(v.__str__(), "1.2.3");
        assert_eq!(v.__repr__(), "Version('1.2.3')");
    }

    #[test]
    fn test_py_version_cmp() {
        let v1 = pv("1.0.0");
        let v2 = pv("2.0.0");
        assert!(v1.__lt__(&v2));
        assert!(v2.__gt__(&v1));
        assert!(v1.__le__(&v1));
        assert!(v1.__ge__(&v1));
        assert!(v1.__eq__(&v1));
        assert!(v1.__ne__(&v2));
    }

    #[test]
    fn test_py_version_major_minor_patch() {
        let v = pv("1.2.3");
        assert_eq!(v.major(), Some("1".to_string()));
        assert_eq!(v.minor(), Some("2".to_string()));
        assert_eq!(v.patch(), Some("3".to_string()));
    }

    #[test]
    fn test_py_version_next() {
        let v = pv("1.2.3");
        let next = v.next().unwrap();
        assert_eq!(next.__str__(), "1.2.4");
    }

    #[test]
    fn test_py_version_trim() {
        let v = pv("1.2.3");
        let trimmed = v.trim(2).unwrap();
        assert_eq!(trimmed.__str__(), "1.2");
    }

    #[test]
    fn test_py_version_range_str() {
        let r = pvr(">=1.0,<2.0");
        assert_eq!(r.__str__(), ">=1.0,<2.0");
        assert!(r.__repr__().contains(">=1.0,<2.0"));
    }

    #[test]
    fn test_py_version_range_contains() {
        let r = pvr(">=1.0,<2.0");
        assert!(r.contains(&pv("1.5")));
        assert!(!r.contains(&pv("2.0")));
        assert!(!r.contains(&pv("0.9")));
    }

    #[test]
    fn test_py_version_range_is_any() {
        let r = pvr("*");
        assert!(r.is_any());
        let r2 = pvr(">=1.0");
        assert!(!r2.is_any());
    }

    #[test]
    fn test_py_version_range_is_empty() {
        let r = pvr("empty");
        assert!(r.is_empty());
    }

    #[test]
    fn test_py_version_range_intersect() {
        let r1 = pvr(">=1.0");
        let r2 = pvr("<=2.0");
        let i = r1.intersect(&r2).unwrap();
        assert!(i.contains(&pv("1.5")));
    }

    #[test]
    fn test_py_version_range_intersects() {
        let r1 = pvr(">=1.0,<2.0");
        let r2 = pvr(">=1.5,<3.0");
        assert!(r1.intersects(&r2));

        let r3 = pvr("<1.0");
        assert!(!r1.intersects(&r3));
    }

    #[test]
    fn test_py_version_range_is_subset_of() {
        let r1 = pvr(">=1.0,<2.0");
        let r2 = pvr(">=1.0");
        assert!(r1.is_subset_of(&r2));
        assert!(!r2.is_subset_of(&r1));
    }

    #[test]
    fn test_py_version_range_is_superset_of() {
        let r1 = pvr(">=1.0");
        let r2 = pvr(">=1.0,<2.0");
        assert!(r1.is_superset_of(&r2));
        assert!(!r2.is_superset_of(&r1));
    }

    #[test]
    fn test_py_version_range_subtract() {
        let r1 = pvr(">=1.0");
        let r2 = pvr(">=2.0");
        let diff = r1.subtract(&r2).unwrap();
        assert!(diff.contains(&pv("1.5")));
        assert!(!diff.contains(&pv("2.5")));
    }

    #[test]
    fn test_py_version_range_union() {
        let r1 = pvr(">=1.0,<1.5");
        let r2 = pvr(">=2.0");
        // union() requires PyResult - test via internal method
        let u = r1.0.union(&r2.0);
        assert!(u.contains(&rez_next_version::Version::parse("1.2").unwrap()));
        assert!(u.contains(&rez_next_version::Version::parse("2.5").unwrap()));
        assert!(!u.contains(&rez_next_version::Version::parse("1.7").unwrap()));
    }

    #[test]
    fn test_py_version_hash_stability() {
        let v1 = pv("1.2.3");
        let v2 = pv("1.2.3");
        assert_eq!(v1.__hash__(), v2.__hash__());
        let v3 = pv("2.0.0");
        assert_ne!(v1.__hash__(), v3.__hash__());
    }

    #[test]
    fn test_py_version_range_hash_stability() {
        let r1 = pvr(">=1.0");
        let r2 = pvr(">=1.0");
        assert_eq!(r1.__hash__(), r2.__hash__());
    }

    #[test]
    fn test_py_version_range_any_classmethod() {
        // any() should be equivalent to VersionRange::any() — matches every version
        let r = PyVersionRange(VersionRange::any());
        assert!(r.is_any());
        assert!(r.contains(&pv("0.0.1")));
        assert!(r.contains(&pv("999.999.999")));
    }

    #[test]
    fn test_py_version_range_none_classmethod() {
        // none() should match no version
        let r = PyVersionRange(VersionRange::none());
        assert!(r.is_empty());
        assert!(!r.contains(&pv("1.0.0")));
    }

    #[test]
    fn test_py_version_range_from_str_static() {
        let r = PyVersionRange::from_str(">=1.0,<2.0").unwrap();
        assert!(r.contains(&pv("1.5")));
        assert!(!r.contains(&pv("2.0")));
    }

    #[test]
    fn test_py_version_range_from_str_invalid() {
        // invalid range string must return Err, not panic
        let result = PyVersionRange::from_str("!!!invalid!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_py_version_range_as_str() {
        let r = pvr(">=1.0,<2.0");
        assert_eq!(r.as_str(), ">=1.0,<2.0");
    }

    #[test]
    fn test_py_version_range_any_union_identity() {
        // any() union with anything = any()
        let any = PyVersionRange(VersionRange::any());
        let r = pvr(">=3.0");
        // any().union(r) should give back any
        let union_result = any.union(&r).unwrap();
        assert!(union_result.is_any());
    }
}
