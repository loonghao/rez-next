//! # Rez Core Build
//!
//! Build system for Rez Core.
//!
//! This crate provides:
//! - Build system abstraction and implementation
//! - Build process management and execution
//! - Build environment setup and configuration
//! - Build artifact management
//! - Release workflow orchestration

mod artifacts;
mod builder;
mod environment;
mod process;
pub mod release;
mod sources;
mod systems;
mod tests;
pub mod vcs;

pub use artifacts::*;
pub use builder::*;
pub use environment::*;
pub use process::*;
pub use release::*;
pub use sources::*;
pub use systems::*;
pub use vcs::*;

/// Get all available build system types
pub fn get_buildsys_types() -> Vec<&'static str> {
    systems::get_buildsys_types()
}
