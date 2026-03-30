//! # Rez Core Solver
//!
//! Dependency resolution algorithms for Rez Core.
//!
//! This crate provides:
//! - Dependency resolution algorithms
//! - Conflict detection and resolution
//! - Package selection strategies
//! - Solver optimization techniques

pub mod dependency_resolver;
mod graph;
mod solver;
pub mod resolution;
pub mod conflict;
pub mod astar;
// mod cache;
// mod optimized_solver;

pub use dependency_resolver::*;
pub use graph::*;
pub use solver::*;
pub use resolution::*;
pub use conflict::*;
pub use astar::astar_search::{AStarSearch, SearchStats};
pub use astar::heuristics::{
    AdaptiveHeuristic, CompositeHeuristic, ConflictPenaltyHeuristic, DependencyDepthHeuristic,
    DependencyHeuristic, HeuristicConfig, HeuristicFactory, RemainingRequirementsHeuristic,
    VersionPreferenceHeuristic,
};
pub use astar::search_state::{ConflictType as AStarConflictType, DependencyConflict as AStarDependencyConflict, SearchState};
// pub use cache::*;
// pub use optimized_solver::*;

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

/// Initialize the solver module for Python
#[cfg(feature = "python-bindings")]
#[pymodule]
fn rez_next_solver(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<DependencySolver>()?;
    Ok(())
}
