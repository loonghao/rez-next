//! # Rez Core
//!
//! High-performance core components for the Rez package manager, written in Rust.
//!
//! This crate provides optimized implementations of critical Rez components:
//! - Version parsing and comparison
//! - Dependency resolution algorithms
//! - Repository scanning and caching
//!
//! ## ⚠️ Work In Progress
//!
//! This is an experimental project. Do not use in production environments.

use pyo3::prelude::*;

// Core modules
pub mod common;
pub mod version;
pub mod solver;
pub mod repository;

// Python bindings
mod python;

/// Python module initialization
#[pymodule]
fn _rez_core(_py: Python, m: &PyModule) -> PyResult<()> {
    // Version system
    m.add_class::<version::Version>()?;
    m.add_class::<version::VersionRange>()?;
    m.add_class::<version::PyVersionToken>()?;

    // Version parsing functions
    m.add_function(wrap_pyfunction!(version::parse_version, m)?)?;
    m.add_function(wrap_pyfunction!(version::parse_version_range, m)?)?;

    // Error types
    m.add("RezCoreError", _py.get_type::<common::error::RezCoreError>())?;
    m.add("VersionParseError", _py.get_type::<version::VersionParseError>())?;

    // Configuration
    m.add_class::<common::config::Config>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Basic test to ensure modules compile
        assert!(true);
    }
}
