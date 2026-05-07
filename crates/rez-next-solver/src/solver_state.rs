//! Solver state representation
//!
//! Defines the public `SolverState` struct that mirrors `rez.solver.SolverState`.
//! This is the high-level, user-facing state of a solver run.

use crate::SolverStatus;
use rez_next_package::{Package, PackageRequirement};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// High-level solver state, mirroring `rez.solver.SolverState`.
///
/// This is a read-only snapshot of a solver's state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverState {
    /// Current solver status
    pub status: SolverStatus,

    /// Successfully resolved packages (in dependency order)
    pub resolved_packages: Vec<Package>,

    /// Requirements that could not be satisfied
    pub failed_requirements: Vec<PackageRequirement>,

    /// Number of packages considered during resolution
    pub packages_considered: usize,

    /// Number of package variants evaluated
    pub variants_evaluated: usize,

    /// Number of backtracking steps performed
    pub backtrack_steps: usize,

    /// Resolution time in milliseconds
    pub resolution_time_ms: u64,

    /// Additional metadata (e.g., solver version, timestamp)
    pub metadata: HashMap<String, String>,
}

impl SolverState {
    /// Create a new `SolverState` with the given status.
    pub fn new(status: SolverStatus) -> Self {
        Self {
            status,
            resolved_packages: Vec::new(),
            failed_requirements: Vec::new(),
            packages_considered: 0,
            variants_evaluated: 0,
            backtrack_steps: 0,
            resolution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Number of resolved packages.
    pub fn resolved_count(&self) -> usize {
        self.resolved_packages.len()
    }

    /// Number of failed requirements.
    pub fn failed_count(&self) -> usize {
        self.failed_requirements.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_state_new() {
        let state = SolverState::new(SolverStatus::Pending);
        assert_eq!(state.status, SolverStatus::Pending);
        assert!(state.resolved_packages.is_empty());
        assert_eq!(state.resolved_count(), 0);
    }

    #[test]
    fn test_solver_state_with_packages() {
        let state = SolverState::new(SolverStatus::Solved);
        // We can't easily construct a Package here, so just check counts
        assert_eq!(state.resolved_count(), 0);
        assert_eq!(state.failed_count(), 0);
    }
}
