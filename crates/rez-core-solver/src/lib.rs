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
mod solver;
// mod graph;
// mod resolution;
// mod conflict;
// mod cache;
// mod optimized_solver;
// mod astar;

pub use solver::*;
// pub use graph::*;
// pub use resolution::*;
// pub use conflict::*;
// pub use cache::*;
// pub use optimized_solver::*;
// pub use astar::*;

use pyo3::prelude::*;

/// Initialize the solver module for Python
#[pymodule]
fn rez_core_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DependencySolver>()?;
    Ok(())
}
