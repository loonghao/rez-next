//! # A* Search Algorithm for Dependency Resolution
//!
//! This module implements A* search algorithm optimized for dependency resolution.
//! It provides heuristic-guided search to find optimal dependency solutions efficiently.
//!
//! ## Key Components
//!
//! - `SearchState`: Represents a state in the dependency resolution search space
//! - `AStarSearch`: Core A* search algorithm implementation
//! - State management and memory optimization

pub mod search_state;
pub mod astar_search;
pub mod heuristics;
pub mod test_framework;
pub mod standalone_test;

#[cfg(test)]
pub mod heuristic_integration_test;

#[cfg(test)]
pub mod heuristic_benchmark;

pub use search_state::*;
pub use astar_search::*;
pub use heuristics::*;
pub use test_framework::*;
pub use standalone_test::*;
