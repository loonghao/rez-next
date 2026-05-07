//! RequirementList type
//!
//! Defines `RequirementList`, mirroring `rez.solver.RequirementList`.

use rez_next_package::PackageRequirement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A list of package requirements.
///
/// This mirrors `rez.solver.RequirementList` and provides
/// methods to add, remove, and query requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementList {
    /// Internal storage: package_name -> Vec<PackageRequirement>
    requirements: HashMap<String, Vec<PackageRequirement>>,
}

impl RequirementList {
    /// Create a new empty `RequirementList`.
    pub fn new() -> Self {
        Self {
            requirements: HashMap::new(),
        }
    }

    /// Add a requirement to the list.
    pub fn add_requirement(&mut self, requirement: PackageRequirement) {
        self.requirements
            .entry(requirement.name.clone())
            .or_default()
            .push(requirement);
    }

    /// Remove all requirements for a package.
    pub fn remove_requirements(&mut self, package_name: &str) {
        self.requirements.remove(package_name);
    }

    /// Get all requirements for a package.
    pub fn get_requirements(&self, package_name: &str) -> Vec<&PackageRequirement> {
        self.requirements
            .get(package_name)
            .map(|list| list.iter().collect())
            .unwrap_or_default()
    }

    /// Get all requirements as a flat vector.
    pub fn all_requirements(&self) -> Vec<&PackageRequirement> {
        self.requirements.values().flatten().collect()
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }

    /// Number of packages with requirements.
    pub fn len(&self) -> usize {
        self.requirements.len()
    }
}

impl Default for RequirementList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::PackageRequirement;

    fn make_requirement(name: &str) -> PackageRequirement {
        PackageRequirement {
            name: name.to_string(),
            version_spec: None,
            weak: false,
            conflict: false,
        }
    }

    #[test]
    fn test_requirement_list_new() {
        let rl = RequirementList::new();
        assert!(rl.is_empty());
        assert_eq!(rl.len(), 0);
    }

    #[test]
    fn test_requirement_list_add() {
        let mut rl = RequirementList::new();
        let req = make_requirement("python");
        rl.add_requirement(req);
        assert!(!rl.is_empty());
        assert_eq!(rl.len(), 1);
    }

    #[test]
    fn test_requirement_list_get() {
        let mut rl = RequirementList::new();
        let req = make_requirement("python");
        rl.add_requirement(req);
        let found = rl.get_requirements("python");
        assert_eq!(found.len(), 1);
    }
}
