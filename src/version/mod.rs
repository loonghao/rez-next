//! Version system implementation
//!
//! This module provides high-performance version parsing, comparison, and range operations.

use pyo3::prelude::*;
use crate::common::RezCoreError;

pub mod version;
pub mod range;
pub mod token;
pub mod parser;

pub use version::Version;
pub use range::VersionRange;
pub use token::{VersionToken, PyVersionToken};

// Define a custom error type for version parsing
#[derive(Debug)]
pub struct VersionParseError(pub String);

impl std::fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version parse error: {}", self.0)
    }
}

impl std::error::Error for VersionParseError {}

impl From<RezCoreError> for VersionParseError {
    fn from(err: RezCoreError) -> Self {
        VersionParseError(err.to_string())
    }
}

// Make it a Python exception
pyo3::create_exception!(rez_core, VersionParseError, pyo3::exceptions::PyException);

/// Parse a version string into a Version object
#[pyfunction]
pub fn parse_version(version_str: &str) -> PyResult<Version> {
    Version::parse(version_str).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Parse a version range string into a VersionRange object
#[pyfunction]
pub fn parse_version_range(range_str: &str) -> PyResult<VersionRange> {
    VersionRange::parse(range_str).map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}
