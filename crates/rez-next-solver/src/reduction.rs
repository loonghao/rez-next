//! Reduction and TotalReduction types
//!
//! Defines `Reduction` and `TotalReduction`, mirroring `rez.solver`.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single reduction step during dependency resolution.
///
/// Mirrors `rez.solver.Reduction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reduction {
    /// The package that was reduced (removed from consideration).
    pub package_name: String,

    /// The version that was reduced (if applicable).
    pub version: Option<String>,

    /// Reason for the reduction.
    pub reason: String,

    /// Timestamp (seconds since Unix epoch).
    pub timestamp: u64,
}

impl Reduction {
    /// Create a new `Reduction`.
    pub fn new(package_name: String, version: Option<String>, reason: String) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            package_name,
            version,
            reason,
            timestamp,
        }
    }
}

/// Aggregated reductions for a solver run.
///
/// Mirrors `rez.solver.TotalReduction`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotalReduction {
    /// Individual reduction steps.
    pub reductions: Vec<Reduction>,

    /// Total number of reductions.
    pub total_count: usize,
}

impl TotalReduction {
    /// Create a new empty `TotalReduction`.
    pub fn new() -> Self {
        Self {
            reductions: Vec::new(),
            total_count: 0,
        }
    }

    /// Add a reduction.
    pub fn add_reduction(&mut self, reduction: Reduction) {
        self.reductions.push(reduction);
        self.total_count += 1;
    }

    /// Check if there are no reductions.
    pub fn is_empty(&self) -> bool {
        self.reductions.is_empty()
    }

    /// Number of reductions.
    pub fn len(&self) -> usize {
        self.reductions.len()
    }
}

impl Default for TotalReduction {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reduction_new() {
        let r = Reduction::new(
            "python".to_string(),
            Some("3.9".to_string()),
            "conflict".to_string(),
        );
        assert_eq!(r.package_name, "python");
        assert_eq!(r.version, Some("3.9".to_string()));
        assert_eq!(r.reason, "conflict");
        assert!(r.timestamp > 0);
    }

    #[test]
    fn test_total_reduction_new() {
        let tr = TotalReduction::new();
        assert!(tr.is_empty());
        assert_eq!(tr.len(), 0);
    }

    #[test]
    fn test_total_reduction_add() {
        let mut tr = TotalReduction::new();
        let r = Reduction::new(
            "python".to_string(),
            Some("3.9".to_string()),
            "conflict".to_string(),
        );
        tr.add_reduction(r);
        assert!(!tr.is_empty());
        assert_eq!(tr.len(), 1);
        assert_eq!(tr.total_count, 1);
    }
}
