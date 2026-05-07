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
pub mod dependency_conflicts;
pub mod dependency_resolver;
pub mod failure_reason;
mod graph;
pub mod package_variant;
pub mod reduction;
pub mod requirement_list;
pub mod resolution;
pub(crate) mod resolution_state;
pub mod solver_state;
mod solver;
pub mod solver_status;

#[cfg(test)]
mod dependency_resolver_tests;

#[cfg(test)]
mod resolver_version_strategy_tests;

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
pub use failure_reason::*;
pub use graph::*;
pub use resolution::*;
pub use dependency_conflicts::DependencyConflicts;
pub use reduction::{Reduction, TotalReduction};
pub use requirement_list::RequirementList;
pub use solver::*;
pub use solver_state::SolverState;
pub use solver_status::SolverStatus;
