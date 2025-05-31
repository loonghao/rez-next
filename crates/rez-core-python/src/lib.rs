//! # Rez Core Python
//!
//! Python bindings for Rez Core components.
//!
//! This crate provides:
//! - Python module initialization
//! - Python class and function exports
//! - Error handling for Python integration
//! - Type conversions between Rust and Python

use pyo3::prelude::*;

// Import all the components we need to expose
use rez_core_common::{RezCoreConfig, PyRezCoreError};
use rez_core_version::{
    Version, VersionRange, PyVersionToken, VersionToken,
    NumericToken, AlphanumericVersionToken, parse_version,
    parse_version_range, PyVersionParseError
};

/// Parse a version string with GIL release optimization
#[pyfunction]
pub fn parse_version_optimized(version_str: &str) -> PyResult<Version> {
    Version::parse_with_gil_release(version_str)
        .map_err(|e| PyErr::new::<PyVersionParseError, _>(e.to_string()))
}

/// Compare two versions with GIL release optimization
#[pyfunction]
pub fn compare_versions_optimized(version1: &Version, version2: &Version) -> i8 {
    match version1.cmp_with_gil_release(version2) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Python module initialization
#[pymodule]
pub fn _rez_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
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

    // GIL-optimized functions
    m.add_function(wrap_pyfunction!(parse_version_optimized, m)?)?;
    m.add_function(wrap_pyfunction!(compare_versions_optimized, m)?)?;

    // Error types
    m.add(
        "RezCoreError",
        m.py().get_type::<PyRezCoreError>(),
    )?;
    m.add(
        "PyVersionParseError",
        m.py().get_type::<PyVersionParseError>(),
    )?;

    // Configuration
    m.add_class::<RezCoreConfig>()?;

    Ok(())
}

#[cfg(test)]
mod gil_optimization_test;
