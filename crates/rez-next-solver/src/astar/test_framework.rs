//! Test framework for A* search implementation

use super::search_state::{ConflictType, DependencyConflict, SearchState};
use rez_next_package::{Package, PackageRequirement};

/// Test the basic functionality of SearchState
pub fn test_search_state_basic() -> Result<(), String> {
    let state = SearchState::new_initial(vec![]);
    if state.depth != 0 {
        return Err("Initial state should have depth 0".to_string());
    }
    if !state.resolved_packages.is_empty() {
        return Err("Initial state should have no resolved packages".to_string());
    }
    if !state.conflicts.is_empty() {
        return Err("Initial state should have no conflicts".to_string());
    }
    if !state.is_goal() {
        return Err("Empty state should be a goal".to_string());
    }
    Ok(())
}

/// Test SearchState with requirements
pub fn test_search_state_with_requirements() -> Result<(), String> {
    let req = PackageRequirement::new("python".to_string());
    let state = SearchState::new_initial(vec![req]);

    if state.is_goal() {
        return Err("State with pending requirements should not be a goal".to_string());
    }
    if state.pending_requirements.len() != 1 {
        return Err("State should have exactly 1 pending requirement".to_string());
    }
    Ok(())
}

/// Test state transitions
pub fn test_state_transition() -> Result<(), String> {
    let req = PackageRequirement::new("python".to_string());
    let parent = SearchState::new_initial(vec![req.clone()]);

    let pkg = Package::new("python".to_string());
    let mut child = SearchState::new_from_parent(&parent, pkg, vec![], 1.0);
    child.remove_requirement(&req);

    if !child.resolved_packages.contains_key("python") {
        return Err("Python should be resolved in child state".to_string());
    }
    if child.depth != 1 {
        return Err(format!("Child depth should be 1, got {}", child.depth));
    }

    Ok(())
}

/// Test conflict detection
pub fn test_conflict_detection() -> Result<(), String> {
    let mut state = SearchState::new_initial(vec![]);
    state.add_conflict(DependencyConflict::new(
        "conflicting_pkg".to_string(),
        vec!["req_a".to_string(), "req_b".to_string()],
        1.0,
        ConflictType::VersionConflict,
    ));

    if state.conflicts.is_empty() {
        return Err("State should have a version conflict".to_string());
    }
    // VersionConflict alone doesn't make state invalid
    if !state.is_valid() {
        return Err("State with only VersionConflict should still be valid".to_string());
    }

    let mut state2 = SearchState::new_initial(vec![]);
    state2.add_conflict(DependencyConflict::new(
        "missing_pkg".to_string(),
        vec![],
        1.0,
        ConflictType::MissingPackage,
    ));
    if state2.is_valid() {
        return Err("State with MissingPackage conflict should be invalid".to_string());
    }

    Ok(())
}

/// Run all framework tests
pub fn run_framework_tests() -> Result<(), String> {
    test_search_state_basic()?;
    test_search_state_with_requirements()?;
    test_state_transition()?;
    test_conflict_detection()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::astar::StatePool;

    #[test]
    fn test_run_framework_tests() {
        run_framework_tests().expect("All framework tests should pass");
    }

    #[test]
    fn test_state_pool_basic() {
        let mut pool = StatePool::new(5);
        assert_eq!(pool.size(), 0);
        let s = pool.get_state();
        pool.return_state(s);
        assert_eq!(pool.size(), 1);
    }
}
