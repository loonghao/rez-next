//! # Rez Core Solver
//!
//! Dependency resolution algorithms for Rez Core.
//!
//! This crate provides:
//! - Dependency resolution algorithms
//! - Conflict detection and resolution
//! - Package selection strategies
//! - Solver optimization techniques

pub mod astar;
pub mod conflict;
pub mod dependency_resolver;
mod graph;
pub mod resolution;
pub(crate) mod resolution_state;
mod solver;

#[cfg(test)]
mod dependency_resolver_tests;

pub use astar::astar_search::{AStarSearch, SearchStats};
pub use astar::heuristics::{
    AdaptiveHeuristic, CompositeHeuristic, ConflictPenaltyHeuristic, DependencyDepthHeuristic,
    DependencyHeuristic, HeuristicConfig, HeuristicFactory, RemainingRequirementsHeuristic,
    VersionPreferenceHeuristic,
};
pub use astar::search_state::{
    ConflictType as AStarConflictType, DependencyConflict as AStarDependencyConflict, SearchState,
};
pub use conflict::*;
pub use dependency_resolver::*;
pub use graph::*;
pub use resolution::*;
pub use solver::*;
