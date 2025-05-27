//! # Rez Core Solver
//!
//! Dependency resolution algorithms for Rez Core.
//!
//! This crate provides:
//! - Dependency resolution algorithms
//! - Conflict detection and resolution
//! - Package selection strategies
//! - Solver optimization techniques

// Re-export from mod.rs for now
pub use mod_solver::*;

// Rename the module to avoid conflicts
#[path = "mod.rs"]
mod mod_solver;
