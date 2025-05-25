//! Version implementation

use pyo3::prelude::*;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use crate::common::RezCoreError;
use super::token::VersionToken;

/// High-performance version representation
#[pyclass]
#[derive(Clone, Debug)]
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
