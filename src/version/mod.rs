//! Version system implementation
//!
//! This module provides high-performance version parsing, comparison, and range operations.

use crate::common::RezCoreError;
use pyo3::prelude::*;

pub mod parser;
pub mod range;
pub mod token;
pub mod version;

pub use range::VersionRange;
pub use token::{PyVersionToken, VersionToken};
pub use version::Version;

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
pyo3::create_exception!(rez_core, PyVersionParseError, pyo3::exceptions::PyException);

/// Parse a version string into a Version object
#[pyfunction]
pub fn parse_version(version_str: &str) -> PyResult<Version> {
    Version::parse(version_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Parse a version range string into a VersionRange object
#[pyfunction]
pub fn parse_version_range(range_str: &str) -> PyResult<VersionRange> {
    VersionRange::parse(range_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}
