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
mod resolved_context;

pub use context::*;
pub use environment::*;
pub use shell::*;
pub use serialization::*;
pub use execution::*;
pub use resolved_context::*;

// use pyo3::prelude::*;  // Temporarily disabled due to DLL issues

// Python module temporarily disabled due to DLL issues
/*
/// Initialize the context module for Python
#[pymodule]
fn rez_core_context(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ResolvedContext>()?;
    m.add_class::<EnvironmentManager>()?;
    m.add_class::<ShellExecutor>()?;
    Ok(())
}
*/
