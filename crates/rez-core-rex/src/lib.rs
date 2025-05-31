//! # Rez Core Rex
//!
//! Rex command system for Rez Core.
//!
//! This crate provides:
//! - Rex command parsing and execution
//! - Environment variable manipulation
//! - Shell integration and binding
//! - Command script interpretation

mod parser;
mod interpreter;
mod commands;
mod bindings;
mod executor;
mod cache;
mod optimized_parser;

pub use parser::*;
pub use interpreter::*;
pub use commands::*;
pub use bindings::*;
pub use executor::*;
pub use cache::*;
pub use optimized_parser::*;

use pyo3::prelude::*;

/// Initialize the rex module for Python
#[pymodule]
fn rez_core_rex(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<RexInterpreter>()?;
    m.add_class::<RexExecutor>()?;
    Ok(())
}
