//! Search state representation for A* dependency resolution
//!
//! This module defines the SearchState structure that represents a state
//! in the dependency resolution search space, along with utilities for
//! state management and comparison.

// Temporarily comment out problematic imports for testing
// use crate::{ConflictStrategy};
// use rez_core_common::RezCoreError;
// use rez_core_package::{Package, PackageRequirement};
// use rez_core_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

// Temporary type definitions for testing
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub requires: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageRequirement {
    pub name: String,
    pub requirement_string: String,
}

/// Represents a state in the dependency resolution search space
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchState {
    /// Packages that have been resolved in this state
    pub resolved_packages: HashMap<String, Package>,

    /// Requirements that still need to be resolved
    pub pending_requirements: Vec<PackageRequirement>,

    /// Current conflicts in this state
    pub conflicts: Vec<DependencyConflict>,

    /// Actual cost from start state to this state (g(n))
    pub cost_so_far: f64,

    /// Estimated total cost (f(n) = g(n) + h(n))
    pub estimated_total_cost: f64,

    /// Depth in the search tree
    pub depth: usize,

    /// Parent state ID for path reconstruction
    pub parent_id: Option<u64>,

    /// Unique identifier for this state
    pub state_id: u64,

    /// Hash of the state for quick comparison
    state_hash: u64,
}

/// Represents a dependency conflict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyConflict {
    /// Name of the conflicting package
    pub package_name: String,

    /// Conflicting requirements
    pub conflicting_requirements: Vec<PackageRequirement>,

    /// Severity of the conflict (0.0 = minor, 1.0 = major)
    pub severity: f64,

    /// Type of conflict
    pub conflict_type: ConflictType,
}

/// Types of dependency conflicts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictType {
    /// Version range conflicts
    VersionConflict,

    /// Circular dependency
    CircularDependency,

    /// Missing package
    MissingPackage,

    /// Platform incompatibility
    PlatformConflict,
}

impl SearchState {
    /// Create a new initial search state
    pub fn new_initial(requirements: Vec<PackageRequirement>) -> Self {
        let mut state = Self {
            resolved_packages: HashMap::new(),
            pending_requirements: requirements,
            conflicts: Vec::new(),
            cost_so_far: 0.0,
            estimated_total_cost: 0.0,
            depth: 0,
            parent_id: None,
            state_id: 0,
            state_hash: 0,
        };

        state.update_hash();
        state.state_id = state.state_hash;
        state
    }

    /// Create a new state from a parent state
    pub fn new_from_parent(
        parent: &SearchState,
        resolved_package: Package,
        new_requirements: Vec<PackageRequirement>,
        additional_cost: f64,
    ) -> Self {
        let mut resolved_packages = parent.resolved_packages.clone();
        resolved_packages.insert(resolved_package.name.clone(), resolved_package);

        // Filter out requirements that are now satisfied
        let mut pending_requirements = parent.pending_requirements.clone();
        pending_requirements.extend(new_requirements);

        let mut state = Self {
            resolved_packages,
            pending_requirements,
            conflicts: parent.conflicts.clone(),
            cost_so_far: parent.cost_so_far + additional_cost,
            estimated_total_cost: 0.0, // Will be set by heuristic function
            depth: parent.depth + 1,
            parent_id: Some(parent.state_id),
            state_id: 0,
            state_hash: 0,
        };

        state.update_hash();
        state.state_id = state.state_hash;
        state
    }

    /// Check if this state represents a goal (all requirements resolved)
    pub fn is_goal(&self) -> bool {
        self.pending_requirements.is_empty() && self.conflicts.is_empty()
    }

    /// Check if this state is valid (no unresolvable conflicts)
    pub fn is_valid(&self) -> bool {
        // Check for fatal conflicts
        for conflict in &self.conflicts {
            match conflict.conflict_type {
                ConflictType::MissingPackage => return false,
                ConflictType::CircularDependency => return false,
                _ => {}
            }
        }
        true
    }

    /// Get the next requirement to resolve
    pub fn get_next_requirement(&self) -> Option<&PackageRequirement> {
        self.pending_requirements.first()
    }

    /// Add a conflict to this state
    pub fn add_conflict(&mut self, conflict: DependencyConflict) {
        self.conflicts.push(conflict);
        self.update_hash();
    }

    /// Remove a requirement from pending list
    pub fn remove_requirement(&mut self, requirement: &PackageRequirement) {
        self.pending_requirements
            .retain(|req| req.name != requirement.name);
        self.update_hash();
    }

    /// Update the state hash for comparison
    fn update_hash(&mut self) {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();

        // Hash resolved packages
        let mut package_names: Vec<_> = self.resolved_packages.keys().collect();
        package_names.sort();
        for name in package_names {
            name.hash(&mut hasher);
            // TODO: Add version hashing when version system is available
        }

        // Hash pending requirements
        let mut req_strings: Vec<_> = self
            .pending_requirements
            .iter()
            .map(|req| &req.requirement_string)
            .collect();
        req_strings.sort();
        for req_str in req_strings {
            req_str.hash(&mut hasher);
        }

        // Hash conflicts
        for conflict in &self.conflicts {
            conflict.package_name.hash(&mut hasher);
            conflict.conflict_type.hash(&mut hasher);
        }

        self.state_hash = hasher.finish();
    }

    /// Get state hash for quick comparison
    pub fn get_hash(&self) -> u64 {
        self.state_hash
    }

    /// Calculate the complexity of this state (for algorithm selection)
    pub fn calculate_complexity(&self) -> usize {
        self.resolved_packages.len() + self.pending_requirements.len() + self.conflicts.len() * 2
        // Conflicts add more complexity
    }
}

impl PartialEq for SearchState {
    fn eq(&self, other: &Self) -> bool {
        self.state_hash == other.state_hash
    }
}

impl Eq for SearchState {}

impl Hash for SearchState {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.state_hash.hash(state);
    }
}

impl PartialOrd for SearchState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // For priority queue (min-heap), we want states with lower f(n) to have higher priority
        other
            .estimated_total_cost
            .partial_cmp(&self.estimated_total_cost)
    }
}

impl Ord for SearchState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// State pool for memory management
pub struct StatePool {
    /// Pool of reusable states
    pool: Vec<SearchState>,

    /// Maximum pool size
    max_size: usize,
}

impl StatePool {
    /// Create a new state pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pool: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Get a state from the pool or create a new one
    pub fn get_state(&mut self) -> SearchState {
        self.pool
            .pop()
            .unwrap_or_else(|| SearchState::new_initial(Vec::new()))
    }

    /// Return a state to the pool
    pub fn return_state(&mut self, mut state: SearchState) {
        if self.pool.len() < self.max_size {
            // Reset state for reuse
            state.resolved_packages.clear();
            state.pending_requirements.clear();
            state.conflicts.clear();
            state.cost_so_far = 0.0;
            state.estimated_total_cost = 0.0;
            state.depth = 0;
            state.parent_id = None;
            state.state_id = 0;
            state.state_hash = 0;

            self.pool.push(state);
        }
    }

    /// Get current pool size
    pub fn size(&self) -> usize {
        self.pool.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use rez_core_version::VersionRange;

    #[test]
    fn test_search_state_creation() {
        let req = PackageRequirement {
            name: "test_package".to_string(),
            requirement_string: "test_package".to_string(),
        };

        let state = SearchState::new_initial(vec![req]);
        assert_eq!(state.pending_requirements.len(), 1);
        assert_eq!(state.resolved_packages.len(), 0);
        assert_eq!(state.depth, 0);
        assert!(!state.is_goal());
    }

    #[test]
    fn test_state_hash_consistency() {
        let req = PackageRequirement {
            name: "test_package".to_string(),
            requirement_string: "test_package".to_string(),
        };

        let state1 = SearchState::new_initial(vec![req.clone()]);
        let state2 = SearchState::new_initial(vec![req]);

        assert_eq!(state1.get_hash(), state2.get_hash());
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_state_pool() {
        let mut pool = StatePool::new(5);
        assert_eq!(pool.size(), 0);

        let state = pool.get_state();
        pool.return_state(state);
        assert_eq!(pool.size(), 1);

        let _state = pool.get_state();
        assert_eq!(pool.size(), 0);
    }
}
