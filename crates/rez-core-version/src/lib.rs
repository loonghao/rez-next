//! # Rez Core Version
//!
//! Version parsing, comparison, and range handling for Rez Core.
//!
//! This crate provides:
//! - Version parsing and validation
//! - Version comparison and ordering
//! - Version range operations
//! - Token-based version representation
//! - Python bindings for version operations

use rez_core_common::RezCoreError;
use pyo3::prelude::*;

pub mod parser;
pub mod range;
pub mod token;
pub mod version;
pub mod version_token;

// Re-export main types
pub use range::VersionRange;
pub use token::PyVersionToken;
pub use version::Version;
pub use version_token::{VersionToken, NumericToken, AlphanumericVersionToken};

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
        .map_err(|e| PyErr::new::<PyVersionParseError, _>(e.to_string()))
}

/// Parse a version range string into a VersionRange object
#[pyfunction]
pub fn parse_version_range(range_str: &str) -> PyResult<VersionRange> {
    VersionRange::parse(range_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{:?}", e)))
}

/// Python module for rez_core.version
#[pymodule]
pub fn version_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Version system
    m.add_class::<Version>()?;
    m.add_class::<VersionRange>()?;
    m.add_class::<PyVersionToken>()?;

    // Version tokens (rez-compatible)
    m.add_class::<VersionToken>()?;
    m.add_class::<NumericToken>()?;
    m.add_class::<AlphanumericVersionToken>()?;

    // Version parsing functions
    m.add_function(wrap_pyfunction!(parse_version, m)?)?;
    m.add_function(wrap_pyfunction!(parse_version_range, m)?)?;

    // Error types
    m.add(
        "PyVersionParseError",
        m.py().get_type::<PyVersionParseError>(),
    )?;

    Ok(())
}

#[cfg(test)]
mod tests;
