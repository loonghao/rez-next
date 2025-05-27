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

// Re-export from workspace crates
pub use rez_core_common as common;
pub use rez_core_version as version;
pub use rez_core_solver as solver;
pub use rez_core_repository as repository;

/// Main Python module that includes all sub-modules
#[pymodule]
fn _rez_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Create and add common submodule
    let common_module = PyModule::new(m.py(), "common")?;
    rez_core_common::common_module(&common_module)?;
    m.add_submodule(&common_module)?;

    // Create and add version submodule
    let version_module = PyModule::new(m.py(), "version")?;
    rez_core_version::version_module(&version_module)?;
    m.add_submodule(&version_module)?;

    // Also expose main classes at the top level for convenience
    // Version system
    m.add_class::<version::Version>()?;
    m.add_class::<version::VersionRange>()?;

    // Version tokens (rez-compatible)
    m.add_class::<version::VersionToken>()?;
    m.add_class::<version::NumericToken>()?;
    m.add_class::<version::AlphanumericVersionToken>()?;

    // Internal version token (for compatibility)
    m.add_class::<version::PyVersionToken>()?;

    // Version parsing functions
    m.add_function(wrap_pyfunction!(version::parse_version, m)?)?;
    m.add_function(wrap_pyfunction!(version::parse_version_range, m)?)?;

    // Error types
    m.add(
        "RezCoreError",
        m.py().get_type::<common::PyRezCoreError>(),
    )?;
    m.add(
        "PyVersionParseError",
        m.py().get_type::<version::PyVersionParseError>(),
    )?;

    // Configuration
    m.add_class::<common::RezCoreConfig>()?;

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
