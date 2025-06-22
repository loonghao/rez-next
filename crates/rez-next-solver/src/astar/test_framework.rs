//! Test framework for A* search implementation
//!
//! This module provides basic testing functionality for the A* search
//! without depending on other potentially broken modules.

use super::astar_search::{AStarSearch, SearchStats};
use super::search_state::{ConflictType, DependencyConflict, SearchState, StatePool};
use std::collections::HashMap;
use std::time::Duration;

/// Mock package requirement for testing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockPackageRequirement {
    pub name: String,
    pub requirement_string: String,
}

impl MockPackageRequirement {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            requirement_string: name.to_string(),
        }
    }
}

/// Mock package for testing
#[derive(Debug, Clone)]
pub struct MockPackage {
    pub name: String,
    pub requires: Vec<String>,
}

impl MockPackage {
    pub fn new(name: &str, requires: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            requires: requires.into_iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Test the basic functionality of SearchState
pub fn test_search_state_basic() -> Result<(), String> {
    println!("Testing SearchState basic functionality...");

    // Create a mock requirement
    let req = MockPackageRequirement::new("test_package");

    // Convert to the format expected by SearchState
    // Note: This is a simplified version for testing
    let state = SearchState::new_initial(vec![]);

    // Test basic properties
    if state.depth != 0 {
        return Err("Initial state should have depth 0".to_string());
    }

    if !state.resolved_packages.is_empty() {
        return Err("Initial state should have no resolved packages".to_string());
    }

    if !state.conflicts.is_empty() {
        return Err("Initial state should have no conflicts".to_string());
    }

    println!("✅ SearchState basic functionality test passed");
    Ok(())
}

/// Test the state pool functionality
pub fn test_state_pool() -> Result<(), String> {
    println!("Testing StatePool functionality...");

    let mut pool = StatePool::new(5);

    // Test initial state
    if pool.size() != 0 {
        return Err("New pool should be empty".to_string());
    }

    // Get a state from empty pool
    let state = pool.get_state();

    // Return state to pool
    pool.return_state(state);
    if pool.size() != 1 {
        return Err("Pool should have 1 state after return".to_string());
    }

    // Get state from pool
    let _state = pool.get_state();
    if pool.size() != 0 {
        return Err("Pool should be empty after get".to_string());
    }

    println!("✅ StatePool functionality test passed");
    Ok(())
}

/// Test conflict detection
pub fn test_conflict_detection() -> Result<(), String> {
    println!("Testing conflict detection...");

    let mut state = SearchState::new_initial(vec![]);

    // Add a conflict
    let conflict = DependencyConflict {
        package_name: "test_package".to_string(),
        conflicting_requirements: vec![],
        severity: 0.8,
        conflict_type: ConflictType::VersionConflict,
    };

    state.add_conflict(conflict);

    if state.conflicts.is_empty() {
        return Err("State should have conflicts after adding one".to_string());
    }

    if state.conflicts.len() != 1 {
        return Err("State should have exactly 1 conflict".to_string());
    }

    // Test that state is still valid (version conflicts are not fatal)
    if !state.is_valid() {
        return Err("State with version conflict should still be valid".to_string());
    }

    // Add a fatal conflict
    let fatal_conflict = DependencyConflict {
        package_name: "missing_package".to_string(),
        conflicting_requirements: vec![],
        severity: 1.0,
        conflict_type: ConflictType::MissingPackage,
    };

    state.add_conflict(fatal_conflict);

    // Now state should be invalid
    if state.is_valid() {
        return Err("State with missing package should be invalid".to_string());
    }

    println!("✅ Conflict detection test passed");
    Ok(())
}

/// Test state hashing and equality
pub fn test_state_hashing() -> Result<(), String> {
    println!("Testing state hashing and equality...");

    let state1 = SearchState::new_initial(vec![]);
    let state2 = SearchState::new_initial(vec![]);

    // States with same content should be equal
    if state1 != state2 {
        return Err("States with same content should be equal".to_string());
    }

    if state1.get_hash() != state2.get_hash() {
        return Err("States with same content should have same hash".to_string());
    }

    println!("✅ State hashing test passed");
    Ok(())
}

/// Run all tests
pub fn run_all_tests() -> Result<(), String> {
    println!("🧪 Running A* Search Framework Tests");
    println!("=====================================");

    test_search_state_basic()?;
    test_state_pool()?;
    test_conflict_detection()?;
    test_state_hashing()?;

    println!("=====================================");
    println!("🎉 All tests passed!");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_functionality() {
        run_all_tests().expect("All tests should pass");
    }
}
