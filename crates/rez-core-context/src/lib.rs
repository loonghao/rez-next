//! # Rez Core Context
//!
//! Context management and environment generation for Rez Core.
//!
//! This crate provides:
//! - Resolved context representation
//! - Environment variable generation
//! - Context serialization/deserialization
//! - Shell integration and command execution

mod context;
mod environment;
mod shell;
mod serialization;
mod execution;

pub use context::*;
pub use environment::*;
pub use shell::*;
pub use serialization::*;
pub use execution::*;

use pyo3::prelude::*;

/// Initialize the context module for Python
#[pymodule]
fn rez_core_context(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ResolvedContext>()?;
    m.add_class::<EnvironmentManager>()?;
    m.add_class::<ShellExecutor>()?;
    Ok(())
}
