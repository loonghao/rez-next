//! Failure reason types for dependency solving
//!
//! This module provides the `FailureReason` struct and related types
//! that describe why a dependency solve failed.

use std::fmt;

/// Represents a reason why the solver failed to resolve dependencies.
///
/// Compatible with `rez.solver.FailureReason`.
#[derive(Debug, Clone, PartialEq)]
pub struct FailureReason {
    /// A human-readable description of the failure
    pub description: String,
    /// The requirements involved in this failure
    pub involved_requirements: Vec<String>,
}

impl FailureReason {
    /// Create a new FailureReason with a description.
    pub fn new(description: &str) -> Self {
        Self {
            description: description.to_string(),
            involved_requirements: Vec::new(),
        }
    }

    /// Create a new FailureReason with description and involved requirements.
    pub fn with_requirements(description: &str, requirements: Vec<String>) -> Self {
        Self {
            description: description.to_string(),
            involved_requirements: requirements,
        }
    }

    /// Get the description of this failure reason.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get the requirements involved in this failure.
    pub fn involved_requirements(&self) -> &[String] {
        &self.involved_requirements
    }
}

impl fmt::Display for FailureReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_reason_new() {
        let reason = FailureReason::new("Package not found");
        assert_eq!(reason.description(), "Package not found");
        assert!(reason.involved_requirements().is_empty());
    }

    #[test]
    fn test_failure_reason_with_requirements() {
        let reqs = vec!["python-3.9".to_string(), "maya-2024".to_string()];
        let reason = FailureReason::with_requirements("Conflict detected", reqs.clone());
        assert_eq!(reason.description(), "Conflict detected");
        assert_eq!(reason.involved_requirements(), reqs.as_slice());
    }

    #[test]
    fn test_failure_reason_display() {
        let reason = FailureReason::new("Test failure");
        assert_eq!(format!("{}", reason), "Test failure");
    }

    #[test]
    fn test_failure_reason_equality() {
        let r1 = FailureReason::new("same");
        let r2 = FailureReason::new("same");
        let r3 = FailureReason::new("different");
        assert_eq!(r1, r2);
        assert_ne!(r1, r3);
    }

    #[test]
    fn test_failure_reason_clone() {
        let reason = FailureReason::with_requirements(
            "test",
            vec!["req1".to_string()],
        );
        let cloned = reason.clone();
        assert_eq!(reason.description(), cloned.description());
        assert_eq!(reason.involved_requirements(), cloned.involved_requirements());
    }
}
