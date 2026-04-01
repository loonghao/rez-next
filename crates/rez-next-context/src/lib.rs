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
mod execution;
mod resolved_context;
mod serialization;
mod shell;

pub use context::*;
pub use environment::*;
pub use execution::*;
pub use resolved_context::*;
pub use serialization::*;
pub use shell::*;

#[cfg(test)]
mod tests;

