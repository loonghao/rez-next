//! # Rez Core Build
//!
//! Build system for Rez Core.
//!
//! This crate provides:
//! - Build system abstraction and implementation
//! - Build process management and execution
//! - Build environment setup and configuration
//! - Build artifact management

mod artifacts;
mod builder;
mod environment;
mod process;
mod sources;
mod systems;

pub use artifacts::*;
pub use builder::*;
pub use environment::*;
pub use process::*;
pub use sources::*;
pub use systems::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the build module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_core_build(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<BuildManager>()?;
    m.add_class::<BuildProcess>()?;
    Ok(())
}
