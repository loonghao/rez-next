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

// Re-export from workspace crates
pub use rez_next_common as common;
pub use rez_next_context as context;
pub use rez_next_package as package;
pub use rez_next_solver as solver;
pub use rez_next_suites as suites;
pub use rez_next_version as version;

// CLI module
pub mod cli;

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_module_structure() {
        // Basic test to ensure modules compile without panicking
    }
}
