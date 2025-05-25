//! Version implementation

use pyo3::prelude::*;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use crate::common::RezCoreError;
use super::token::VersionToken;

/// High-performance version representation
#[pyclass]
#[derive(Clone, Debug, Hash)]
pub struct Version {
    tokens: Vec<VersionToken>,
    separators: Vec<char>,
    #[pyo3(get)]
    string_repr: String,
}

#[pymethods]
impl Version {
    #[new]
    pub fn new(version_str: &str) -> PyResult<Self> {
        Self::parse(version_str).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.string_repr
    }

    fn __str__(&self) -> String {
        self.string_repr.clone()
    }

    fn __repr__(&self) -> String {
        format!("Version('{}')", self.string_repr)
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Less
    }

    fn __le__(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Less | Ordering::Equal)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.cmp(other) != Ordering::Equal
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Greater
    }

    fn __ge__(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Greater | Ordering::Equal)
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.string_repr.hash(&mut hasher);
        hasher.finish()
    }
}

impl Version {
    /// Parse a version string into a Version object
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        // TODO: Implement high-performance version parsing
        // For now, create a placeholder implementation
        Ok(Self {
            tokens: vec![],
            separators: vec![],
            string_repr: s.to_string(),
        })
    }

    /// Compare two versions
    pub fn cmp(&self, other: &Self) -> Ordering {
        // TODO: Implement optimized version comparison
        // For now, use string comparison as placeholder
        self.string_repr.cmp(&other.string_repr)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use pretty_assertions::assert_eq;

    #[rstest]
    #[case("1.0.0")]
    #[case("2.1.3")]
    #[case("0.9.12")]
    #[case("10.0.0")]
    fn test_version_creation(#[case] version_str: &str) {
        let version = Version::parse(version_str).unwrap();
        assert_eq!(version.as_str(), version_str);
    }

    #[rstest]
    #[case("")]
    #[case("not.a.version")]
    #[case("1.2.3.4.5")]
    fn test_version_creation_invalid(#[case] invalid_str: &str) {
        assert!(Version::parse(invalid_str).is_err());
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();
        let v3 = Version::parse("1.0.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 > v1);
        assert_eq!(v1, v3);
    }

    #[test]
    fn test_version_ordering() {
        let mut versions = vec![
            Version::parse("2.0.0").unwrap(),
            Version::parse("1.0.0").unwrap(),
            Version::parse("1.2.0").unwrap(),
            Version::parse("1.1.0").unwrap(),
        ];

        versions.sort();

        let expected = vec!["1.0.0", "1.1.0", "1.2.0", "2.0.0"];
        let actual: Vec<&str> = versions.iter().map(|v| v.as_str()).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_version_hash() {
        use std::collections::HashMap;

        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("1.0.0").unwrap();
        let v3 = Version::parse("2.0.0").unwrap();

        let mut map = HashMap::new();
        map.insert(v1, "first");
        map.insert(v2, "second");
        map.insert(v3, "third");

        // v1 and v2 should be the same key
        assert_eq!(map.len(), 2);
    }
}
