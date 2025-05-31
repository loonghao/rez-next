//! # Rez Core Build
//!
//! Build system for Rez Core.
//!
//! This crate provides:
//! - Build system abstraction and implementation
//! - Build process management and execution
//! - Build environment setup and configuration
//! - Build artifact management

mod builder;
mod process;
mod environment;
mod artifacts;
mod systems;

pub use builder::*;
pub use process::*;
pub use environment::*;
pub use artifacts::*;
pub use systems::*;

use pyo3::prelude::*;

/// Initialize the build module for Python
#[pymodule]
fn rez_core_build(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<BuildManager>()?;
    m.add_class::<BuildProcess>()?;
    Ok(())
}
