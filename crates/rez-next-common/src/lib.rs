//! # Rez Core Common
//!
//! Common utilities and types shared across Rez Core components.
//!
//! This crate provides:
//! - Error handling types
//! - Configuration management
//! - Utility functions
//! - Shared data structures

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

pub mod config;
pub mod error;
pub mod utils;

// Re-export commonly used types
pub use config::RezCoreConfig;
#[cfg(not(feature = "python-bindings"))]
pub use error::RezCoreError;
#[cfg(feature = "python-bindings")]
pub use error::{PyRezCoreError, RezCoreError};

/// Python module for rez_core.common
#[cfg(feature = "python-bindings")]
#[pymodule]
pub fn common_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Configuration
    m.add_class::<RezCoreConfig>()?;

    // Error types
    m.add("RezCoreError", m.py().get_type::<PyRezCoreError>())?;

    Ok(())
}
