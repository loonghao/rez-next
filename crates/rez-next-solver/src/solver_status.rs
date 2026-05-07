//! Solver status enumeration
//!
//! Defines the possible states of a solver instance, matching the original
//! `rez.solver.SolverStatus` enum.

use serde::{Deserialize, Serialize};

/// Represents the current state of a solver instance.
///
/// This enum mirrors `rez.solver.SolverStatus` and includes a human-readable
/// description for each variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverStatus {
    /// The solve has not yet started.
    Pending,
    /// The solve has completed successfully.
    Solved,
    /// The current solve is exhausted and must be split to continue further.
    Exhausted,
    /// The solve is not possible (dependency conflict).
    Failed,
    /// The solve contains a cyclic dependency.
    Cyclic,
    /// The solve has started, but is not yet solved.
    Unsolved,
}

impl SolverStatus {
    /// Returns the human-readable description for this status.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rez_next_solver::SolverStatus;
    /// let status = SolverStatus::Solved;
    /// assert_eq!(status.description(), "The solve has completed successfully.");
    /// ```
    pub fn description(&self) -> &'static str {
        match self {
            SolverStatus::Pending => "The solve has not yet started.",
            SolverStatus::Solved => "The solve has completed successfully.",
            SolverStatus::Exhausted => {
                "The current solve is exhausted and must be split to continue further."
            }
            SolverStatus::Failed => "The solve is not possible.",
            SolverStatus::Cyclic => "The solve contains a cycle.",
            SolverStatus::Unsolved => "The solve has started, but is not yet solved.",
        }
    }

    /// Returns the variant name as a string (matching Python enum member name).
    pub fn name(&self) -> &'static str {
        match self {
            SolverStatus::Pending => "pending",
            SolverStatus::Solved => "solved",
            SolverStatus::Exhausted => "exhausted",
            SolverStatus::Failed => "failed",
            SolverStatus::Cyclic => "cyclic",
            SolverStatus::Unsolved => "unsolved",
        }
    }
}

impl std::fmt::Display for SolverStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name(), self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solver_status_pending() {
        let status = SolverStatus::Pending;
        assert_eq!(status.name(), "pending");
        assert_eq!(status.description(), "The solve has not yet started.");
    }

    #[test]
    fn test_solver_status_solved() {
        let status = SolverStatus::Solved;
        assert_eq!(status.name(), "solved");
        assert_eq!(
            status.description(),
            "The solve has completed successfully."
        );
    }

    #[test]
    fn test_solver_status_exhausted() {
        let status = SolverStatus::Exhausted;
        assert_eq!(status.name(), "exhausted");
        assert_eq!(
            status.description(),
            "The current solve is exhausted and must be split to continue further."
        );
    }

    #[test]
    fn test_solver_status_failed() {
        let status = SolverStatus::Failed;
        assert_eq!(status.name(), "failed");
        assert_eq!(status.description(), "The solve is not possible.");
    }

    #[test]
    fn test_solver_status_cyclic() {
        let status = SolverStatus::Cyclic;
        assert_eq!(status.name(), "cyclic");
        assert_eq!(
            status.description(),
            "The solve contains a cycle."
        );
    }

    #[test]
    fn test_solver_status_unsolved() {
        let status = SolverStatus::Unsolved;
        assert_eq!(status.name(), "unsolved");
        assert_eq!(
            status.description(),
            "The solve has started, but is not yet solved."
        );
    }

    #[test]
    fn test_solver_status_display() {
        let status = SolverStatus::Solved;
        let display_str = format!("{}", status);
        assert!(display_str.contains("solved"));
        assert!(display_str.contains("completed successfully"));
    }

    #[test]
    fn test_solver_status_equality() {
        assert_eq!(SolverStatus::Solved, SolverStatus::Solved);
        assert_ne!(SolverStatus::Solved, SolverStatus::Failed);
    }
}
