//! Standalone test for A* search framework
//!
//! This module provides a completely independent test that doesn't rely on
//! other potentially broken modules in the project.

use super::search_state::{
    ConflictType, DependencyConflict, Package, PackageRequirement, SearchState, StatePool,
};
use std::collections::HashMap;

/// Run standalone tests for A* search framework
pub fn run_standalone_tests() -> Result<(), String> {
    println!("🧪 Running Standalone A* Search Framework Tests");
    println!("===============================================");

    test_search_state_creation()?;
    test_state_pool_functionality()?;
    test_conflict_management()?;
    test_state_hashing()?;
    test_state_transitions()?;

    println!("===============================================");
    println!("🎉 All standalone tests passed!");

    Ok(())
}

/// Test basic SearchState creation and properties
fn test_search_state_creation() -> Result<(), String> {
    println!("Testing SearchState creation...");

    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };

    let state = SearchState::new_initial(vec![req]);

    if state.depth != 0 {
        return Err("Initial state should have depth 0".to_string());
    }

    if !state.resolved_packages.is_empty() {
        return Err("Initial state should have no resolved packages".to_string());
    }

    if !state.conflicts.is_empty() {
        return Err("Initial state should have no conflicts".to_string());
    }

    if state.pending_requirements.len() != 1 {
        return Err("Initial state should have 1 pending requirement".to_string());
    }

    if state.is_goal() {
        return Err("Initial state with pending requirements should not be goal".to_string());
    }

    println!("✅ SearchState creation test passed");
    Ok(())
}

/// Test StatePool functionality
fn test_state_pool_functionality() -> Result<(), String> {
    println!("Testing StatePool functionality...");

    let mut pool = StatePool::new(5);

    if pool.size() != 0 {
        return Err("New pool should be empty".to_string());
    }

    // Get a state from empty pool (should create new one)
    let state = pool.get_state();

    // Return state to pool
    pool.return_state(state);
    if pool.size() != 1 {
        return Err("Pool should have 1 state after return".to_string());
    }

    // Get state from pool (should reuse)
    let _state = pool.get_state();
    if pool.size() != 0 {
        return Err("Pool should be empty after get".to_string());
    }

    // Test pool capacity limit
    for i in 0..10 {
        let state = SearchState::new_initial(vec![]);
        pool.return_state(state);
    }

    if pool.size() > 5 {
        return Err("Pool should not exceed max capacity".to_string());
    }

    println!("✅ StatePool functionality test passed");
    Ok(())
}

/// Test conflict management
fn test_conflict_management() -> Result<(), String> {
    println!("Testing conflict management...");

    let mut state = SearchState::new_initial(vec![]);

    // Test adding version conflict (non-fatal)
    let version_conflict = DependencyConflict {
        package_name: "test_package".to_string(),
        conflicting_requirements: vec![],
        severity: 0.8,
        conflict_type: ConflictType::VersionConflict,
    };

    state.add_conflict(version_conflict);

    if state.conflicts.is_empty() {
        return Err("State should have conflicts after adding one".to_string());
    }

    if !state.is_valid() {
        return Err("State with version conflict should still be valid".to_string());
    }

    // Test adding fatal conflict
    let fatal_conflict = DependencyConflict {
        package_name: "missing_package".to_string(),
        conflicting_requirements: vec![],
        severity: 1.0,
        conflict_type: ConflictType::MissingPackage,
    };

    state.add_conflict(fatal_conflict);

    if state.is_valid() {
        return Err("State with missing package should be invalid".to_string());
    }

    println!("✅ Conflict management test passed");
    Ok(())
}

/// Test state hashing and equality
fn test_state_hashing() -> Result<(), String> {
    println!("Testing state hashing and equality...");

    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };

    let state1 = SearchState::new_initial(vec![req.clone()]);
    let state2 = SearchState::new_initial(vec![req]);

    // States with same content should be equal
    if state1 != state2 {
        return Err("States with same content should be equal".to_string());
    }

    if state1.get_hash() != state2.get_hash() {
        return Err("States with same content should have same hash".to_string());
    }

    // Test different states
    let different_req = PackageRequirement {
        name: "different_package".to_string(),
        requirement_string: "different_package".to_string(),
    };

    let state3 = SearchState::new_initial(vec![different_req]);

    if state1 == state3 {
        return Err("States with different content should not be equal".to_string());
    }

    if state1.get_hash() == state3.get_hash() {
        return Err("States with different content should have different hashes".to_string());
    }

    println!("✅ State hashing test passed");
    Ok(())
}

/// Test state transitions
fn test_state_transitions() -> Result<(), String> {
    println!("Testing state transitions...");

    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };

    let parent_state = SearchState::new_initial(vec![req.clone()]);

    // Create a package to resolve the requirement
    let package = Package {
        name: "test_package".to_string(),
        requires: vec!["dependency1".to_string(), "dependency2".to_string()],
    };

    // Create new requirements for dependencies
    let new_requirements = vec![
        PackageRequirement {
            name: "dependency1".to_string(),
            requirement_string: "dependency1".to_string(),
        },
        PackageRequirement {
            name: "dependency2".to_string(),
            requirement_string: "dependency2".to_string(),
        },
    ];

    let child_state = SearchState::new_from_parent(
        &parent_state,
        package,
        new_requirements,
        1.0, // cost
    );

    // Verify state transition
    if child_state.depth != parent_state.depth + 1 {
        return Err("Child state should have incremented depth".to_string());
    }

    if child_state.cost_so_far != parent_state.cost_so_far + 1.0 {
        return Err("Child state should have accumulated cost".to_string());
    }

    if child_state.resolved_packages.len() != 1 {
        return Err("Child state should have 1 resolved package".to_string());
    }

    if !child_state.resolved_packages.contains_key("test_package") {
        return Err("Child state should contain resolved package".to_string());
    }

    if child_state.parent_id != Some(parent_state.state_id) {
        return Err("Child state should reference parent ID".to_string());
    }

    // Test complexity calculation
    let complexity = child_state.calculate_complexity();
    let expected_complexity = 1 + 2 + 0; // 1 resolved + 2 pending + 0 conflicts
    if complexity != expected_complexity {
        return Err(format!(
            "Expected complexity {}, got {}",
            expected_complexity, complexity
        ));
    }

    println!("✅ State transitions test passed");
    Ok(())
}

/// Test goal state detection
fn test_goal_state_detection() -> Result<(), String> {
    println!("Testing goal state detection...");

    // Create state with no pending requirements and no conflicts
    let goal_state = SearchState::new_initial(vec![]);

    if !goal_state.is_goal() {
        return Err("State with no pending requirements should be goal".to_string());
    }

    // Create state with pending requirements
    let req = PackageRequirement {
        name: "test_package".to_string(),
        requirement_string: "test_package".to_string(),
    };

    let non_goal_state = SearchState::new_initial(vec![req]);

    if non_goal_state.is_goal() {
        return Err("State with pending requirements should not be goal".to_string());
    }

    println!("✅ Goal state detection test passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standalone_functionality() {
        run_standalone_tests().expect("All standalone tests should pass");
    }
}
