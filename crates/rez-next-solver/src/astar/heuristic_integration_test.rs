//! Integration tests for heuristic functions with A* search

use super::astar_search::AStarSearch;
use super::heuristics::{
    AdaptiveHeuristic, CompositeHeuristic, DependencyHeuristic, HeuristicConfig, HeuristicFactory,
};
use super::search_state::SearchState;
use crate::SolverConfig;
use rez_next_package::PackageRequirement;
use rez_next_repository::simple_repository::RepositoryManager;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_heuristic_integration_empty_requirements() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    let mut search = AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

    let heuristic = CompositeHeuristic::new_fast();
    let heuristic_fn = |state: &SearchState| heuristic.calculate(state);

    let result = search.search(vec![], heuristic_fn).await;
    assert!(
        result.is_ok(),
        "Search with empty requirements should succeed"
    );
    assert!(result.unwrap().is_some(), "Should find a goal immediately");
}

#[tokio::test]
async fn test_heuristic_integration_unsatisfiable() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    let mut search = AStarSearch::new(repo_manager, config, Duration::from_secs(5), 100);

    // No packages in repo, so this requirement can't be satisfied
    let requirements = vec![PackageRequirement::new("missing_package".to_string())];

    let heuristic = CompositeHeuristic::new_fast();
    let heuristic_fn = |state: &SearchState| heuristic.calculate(state);

    let result = search.search(requirements, heuristic_fn).await;
    // Either returns Ok(None) or an error — both are valid when no solution exists
    match result {
        Ok(None) => {} // No solution found — expected
        Ok(Some(_)) => panic!("Should not find a solution for a missing package"),
        Err(_) => {} // Timeout/limit — acceptable
    }
}

#[tokio::test]
async fn test_adaptive_heuristic_integration() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    let mut search = AStarSearch::new(repo_manager, config, Duration::from_secs(30), 1000);

    let mut adaptive_heuristic = AdaptiveHeuristic::new(HeuristicConfig::default());
    adaptive_heuristic.update_stats(50, 3, 8.0, 5);

    let heuristic_fn = |state: &SearchState| adaptive_heuristic.calculate(state);
    let result = search.search(vec![], heuristic_fn).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_heuristic_factory_complexity_levels() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();

    for &complexity in &[5usize, 30, 100] {
        let mut search = AStarSearch::new(
            repo_manager.clone(),
            config.clone(),
            Duration::from_secs(5),
            200,
        );
        let heuristic = HeuristicFactory::create_for_complexity(complexity);
        let heuristic_fn = |state: &SearchState| heuristic.calculate(state);
        let result = search.search(vec![], heuristic_fn).await;
        assert!(
            result.is_ok(),
            "complexity={} should not error on empty reqs",
            complexity
        );
    }
}

#[tokio::test]
async fn test_scenario_heuristics_fast_thorough_conflict_heavy() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();

    for scenario in &["fast", "thorough", "conflict_heavy", "unknown"] {
        let mut search = AStarSearch::new(
            repo_manager.clone(),
            config.clone(),
            Duration::from_secs(5),
            200,
        );
        let heuristic = HeuristicFactory::create_for_scenario(scenario);
        let heuristic_fn = |state: &SearchState| heuristic.calculate(state);
        let result = search.search(vec![], heuristic_fn).await;
        assert!(
            result.is_ok(),
            "scenario={} should not error on empty reqs",
            scenario
        );
    }
}
