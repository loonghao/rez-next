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

// use pyo3::prelude::*;  // Temporarily disabled due to DLL issues

// Re-export from workspace crates
pub use rez_next_common as common;
pub use rez_next_package as package;
pub use rez_next_solver as solver;
pub use rez_next_version as version;
// pub use rez_next_repository as repository;  // Temporarily disabled
pub use rez_next_context as context;

// CLI module
pub mod cli;

// /// Main Python module that includes all sub-modules - temporarily disabled
// #[pymodule]
// fn _rez_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     // Add version classes
//     m.add_class::<version::Version>()?;
//     m.add_class::<version::VersionRange>()?;
//
//     // Add package classes
//     m.add_class::<package::Package>()?;
//     m.add_class::<package::PackageVariant>()?;
//     m.add_class::<package::PackageRequirement>()?;
//
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_structure() {
        // Basic test to ensure modules compile
        assert!(true);
    }
}
