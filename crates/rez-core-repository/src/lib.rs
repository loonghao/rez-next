//! # Rez Core Repository
//!
//! Repository scanning, caching, and management for Rez Core.
//!
//! This crate provides:
//! - Repository scanning and indexing
//! - Package discovery and caching
//! - Repository metadata management
//! - Async repository operations

pub mod repository;
pub mod filesystem;
pub mod cache;
pub mod scanner;
pub mod high_performance_scanner;

pub use repository::*;
pub use filesystem::*;
pub use cache::*;
pub use scanner::*;
pub use high_performance_scanner::*;

use pyo3::prelude::*;

/// Initialize the repository module for Python
#[pymodule]
fn rez_core_repository(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<FileSystemRepository>()?;
    m.add_class::<RepositoryManager>()?;
    Ok(())
}
