//! # Rez Core Repository
//!
//! Repository scanning, caching, and management for Rez Core.
//!
//! This crate provides:
//! - Repository scanning and indexing
//! - Package discovery and caching
//! - Repository metadata management
//! - Async repository operations

pub mod cache;
pub mod filesystem;
pub mod high_performance_scanner;
pub mod repository;
pub mod scanner;
pub mod simple_repository;

pub use cache::*;
pub use filesystem::*;
pub use high_performance_scanner::*;
pub use repository::*;
pub use scanner::*;
pub use simple_repository::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the repository module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_next_repository(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FileSystemRepository>()?;
    m.add_class::<RepositoryManager>()?;
    Ok(())
}
