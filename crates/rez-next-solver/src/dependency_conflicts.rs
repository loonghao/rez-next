//! DependencyConflicts collection type
//!
//! Defines `DependencyConflicts`, a collection of `DependencyConflict`.
//! Mirrors `rez.solver.DependencyConflicts`.

use crate::DependencyConflict;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A collection of dependency conflicts.
///
/// This mirrors `rez.solver.DependencyConflicts` and provides
/// methods to add, remove, and query conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyConflicts {
    /// Internal storage: package_name -> Vec<DependencyConflict>
    conflicts: HashMap<String, Vec<DependencyConflict>>,
}

impl DependencyConflicts {
    /// Create a new empty `DependencyConflicts`.
    pub fn new() -> Self {
        Self {
            conflicts: HashMap::new(),
        }
    }

    /// Add a conflict to the collection.
    pub fn add_conflict(&mut self, conflict: DependencyConflict) {
        self.conflicts
            .entry(conflict.package_name.clone())
            .or_default()
            .push(conflict);
    }

    /// Remove all conflicts for a package.
    pub fn remove_conflicts(&mut self, package_name: &str) {
        self.conflicts.remove(package_name);
    }

    /// Get all conflicts for a package.
    pub fn get_conflicts(&self, package_name: &str) -> Vec<&DependencyConflict> {
        self.conflicts
            .get(package_name)
            .map(|list| list.iter().collect())
            .unwrap_or_default()
    }

    /// Get all conflicts as a flat vector.
    pub fn all_conflicts(&self) -> Vec<&DependencyConflict> {
        self.conflicts.values().flatten().collect()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.conflicts.is_empty()
    }

    /// Number of packages with conflicts.
    pub fn len(&self) -> usize {
        self.conflicts.len()
    }
}

impl Default for DependencyConflicts {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ConflictSeverity, DependencyConflict};

    fn make_conflict(name: &str) -> DependencyConflict {
        DependencyConflict {
            package_name: name.to_string(),
            conflicting_requirements: vec![],
            source_packages: vec![],
            severity: ConflictSeverity::Major,
        }
    }

    #[test]
    fn test_dependency_conflicts_new() {
        let dc = DependencyConflicts::new();
        assert!(dc.is_empty());
        assert_eq!(dc.len(), 0);
    }

    #[test]
    fn test_dependency_conflicts_add() {
        let mut dc = DependencyConflicts::new();
        let conflict = make_conflict("python");
        dc.add_conflict(conflict);
        assert!(!dc.is_empty());
        assert_eq!(dc.len(), 1);
    }

    #[test]
    fn test_dependency_conflicts_get() {
        let mut dc = DependencyConflicts::new();
        let conflict = make_conflict("python");
        dc.add_conflict(conflict);
        let found = dc.get_conflicts("python");
        assert_eq!(found.len(), 1);
    }
}
