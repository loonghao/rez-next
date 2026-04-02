//! Standalone tests for A* search framework

use super::search_state::{ConflictType, DependencyConflict, SearchState, StatePool};
use rez_next_package::PackageRequirement;

/// Run standalone tests for A* search framework
pub fn run_standalone_tests() -> Result<(), String> {
    test_search_state_creation()?;
    test_state_pool_functionality()?;
    test_conflict_management()?;
    test_state_hashing()?;
    test_state_transitions()?;
    Ok(())
}

fn test_search_state_creation() -> Result<(), String> {
    let req = PackageRequirement::new("test_package".to_string());
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
    if state.is_goal() {
        return Err("State with pending requirements should not be a goal".to_string());
    }
    Ok(())
}

fn test_state_pool_functionality() -> Result<(), String> {
    let mut pool = StatePool::new(10);
    if pool.size() != 0 {
        return Err("Pool should start empty".to_string());
    }

    let state = pool.get_state();
    pool.return_state(state);
    if pool.size() != 1 {
        return Err("Pool should have 1 state after return".to_string());
    }

    let _state = pool.get_state();
    if pool.size() != 0 {
        return Err("Pool should be empty after get".to_string());
    }

    Ok(())
}

fn test_conflict_management() -> Result<(), String> {
    let mut state = SearchState::new_initial(vec![]);

    let conflict = DependencyConflict::new(
        "test_package".to_string(),
        vec!["req1".to_string()],
        1.0,
        ConflictType::VersionConflict,
    );
    state.add_conflict(conflict);

    if state.conflicts.is_empty() {
        return Err("State should have a conflict".to_string());
    }
    if state.is_goal() {
        return Err("State with conflicts should not be a goal".to_string());
    }

    Ok(())
}

fn test_state_hashing() -> Result<(), String> {
    let req = PackageRequirement::new("test_package".to_string());
    let state1 = SearchState::new_initial(vec![req.clone()]);
    let state2 = SearchState::new_initial(vec![req]);

    if state1.get_hash() != state2.get_hash() {
        return Err("Identical states should have equal hashes".to_string());
    }
    if state1 != state2 {
        return Err("Identical states should be equal".to_string());
    }

    Ok(())
}

fn test_state_transitions() -> Result<(), String> {
    use rez_next_package::Package;

    let req = PackageRequirement::new("test_package".to_string());
    let parent = SearchState::new_initial(vec![req]);
    assert_eq!(parent.depth, 0);

    let pkg = Package::new("test_package".to_string());
    let child = SearchState::new_from_parent(&parent, pkg, vec![], 1.0);

    if child.depth != 1 {
        return Err(format!("Child state should have depth 1, got {}", child.depth));
    }
    if child.parent_id != Some(parent.state_id) {
        return Err("Child state should have parent's id".to_string());
    }
    if !child.resolved_packages.contains_key("test_package") {
        return Err("Child state should have test_package resolved".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_standalone_tests() {
        run_standalone_tests().expect("All standalone tests should pass");
    }
}
