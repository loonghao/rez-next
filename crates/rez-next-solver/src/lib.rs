//! # Rez Core Solver
//!
//! Dependency resolution algorithms for Rez Core.
//!
//! This crate provides:
//! - Dependency resolution algorithms
//! - Conflict detection and resolution
//! - Package selection strategies
//! - Solver optimization techniques

// Temporarily simplified for compilation
pub mod dependency_resolver;
mod graph;
mod solver;
// mod resolution;
// mod conflict;
// mod cache;
// mod optimized_solver;
// mod astar;

pub use dependency_resolver::*;
pub use graph::*;
pub use solver::*;
// pub use resolution::*;
// pub use conflict::*;
// pub use cache::*;
// pub use optimized_solver::*;
// pub use astar::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the solver module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_next_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DependencySolver>()?;
    Ok(())
}
