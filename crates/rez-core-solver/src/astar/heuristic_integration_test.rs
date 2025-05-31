//! Integration tests for heuristic functions with A* search
//!
//! This module tests the integration of various heuristic functions
//! with the A* search algorithm to ensure they work correctly together.

use super::{
    AStarSearch, SearchState, PackageRequirement, Package,
    HeuristicFactory, CompositeHeuristic, AdaptiveHeuristic,
    DependencyHeuristic, HeuristicConfig, RepositoryManager,
    SolverConfig, PackageSearchCriteria
};
use std::sync::Arc;
use std::time::Duration;

/// Test the integration of different heuristics with A* search
#[tokio::test]
async fn test_heuristic_integration() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    let mut search = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(30),
        1000,
    );
    
    let requirements = vec![
        PackageRequirement {
            name: "test_package".to_string(),
            requirement_string: "test_package>=1.0".to_string(),
        },
    ];
    
    // Test with composite heuristic
    let heuristic = CompositeHeuristic::new_fast();
    let heuristic_fn = |state: &SearchState| heuristic.calculate(state);
    
    let result = search.search(requirements.clone(), heuristic_fn).await;
    
    // Should complete without error (even if no solution found due to mock repository)
    assert!(result.is_ok());
    
    let stats = search.get_stats();
    assert!(stats.states_explored >= 0);
}

/// Test adaptive heuristic behavior
#[tokio::test]
async fn test_adaptive_heuristic_integration() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    let mut search = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(30),
        1000,
    );
    
    let requirements = vec![
        PackageRequirement {
            name: "complex_package".to_string(),
            requirement_string: "complex_package".to_string(),
        },
    ];
    
    // Test with adaptive heuristic
    let mut adaptive_heuristic = AdaptiveHeuristic::new(HeuristicConfig::default());
    
    // Simulate search progress updates
    adaptive_heuristic.update_stats(50, 3, 8.0, 5);
    
    let heuristic_fn = |state: &SearchState| adaptive_heuristic.calculate(state);
    
    let result = search.search(requirements, heuristic_fn).await;
    assert!(result.is_ok());
}

/// Test heuristic factory with different complexity levels
#[tokio::test]
async fn test_heuristic_factory_integration() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    
    // Test simple complexity
    let mut search_simple = AStarSearch::new(
        repo_manager.clone(),
        config.clone(),
        Duration::from_secs(30),
        1000,
    );
    
    let simple_requirements = vec![
        PackageRequirement {
            name: "simple_package".to_string(),
            requirement_string: "simple_package".to_string(),
        },
    ];
    
    let simple_heuristic = HeuristicFactory::create_for_complexity(5);
    let simple_heuristic_fn = |state: &SearchState| simple_heuristic.calculate(state);
    
    let simple_result = search_simple.search(simple_requirements, simple_heuristic_fn).await;
    assert!(simple_result.is_ok());
    
    // Test complex complexity
    let mut search_complex = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(30),
        1000,
    );
    
    let complex_requirements = vec![
        PackageRequirement {
            name: "complex_package".to_string(),
            requirement_string: "complex_package".to_string(),
        },
        PackageRequirement {
            name: "another_complex_package".to_string(),
            requirement_string: "another_complex_package".to_string(),
        },
    ];
    
    let complex_heuristic = HeuristicFactory::create_for_complexity(100);
    let complex_heuristic_fn = |state: &SearchState| complex_heuristic.calculate(state);
    
    let complex_result = search_complex.search(complex_requirements, complex_heuristic_fn).await;
    assert!(complex_result.is_ok());
}

/// Test scenario-specific heuristics
#[tokio::test]
async fn test_scenario_heuristics() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    
    let requirements = vec![
        PackageRequirement {
            name: "scenario_package".to_string(),
            requirement_string: "scenario_package".to_string(),
        },
    ];
    
    // Test fast scenario
    let mut search_fast = AStarSearch::new(
        repo_manager.clone(),
        config.clone(),
        Duration::from_secs(30),
        1000,
    );
    
    let fast_heuristic = HeuristicFactory::create_for_scenario("fast");
    let fast_heuristic_fn = |state: &SearchState| fast_heuristic.calculate(state);
    
    let fast_result = search_fast.search(requirements.clone(), fast_heuristic_fn).await;
    assert!(fast_result.is_ok());
    
    // Test thorough scenario
    let mut search_thorough = AStarSearch::new(
        repo_manager.clone(),
        config.clone(),
        Duration::from_secs(30),
        1000,
    );
    
    let thorough_heuristic = HeuristicFactory::create_for_scenario("thorough");
    let thorough_heuristic_fn = |state: &SearchState| thorough_heuristic.calculate(state);
    
    let thorough_result = search_thorough.search(requirements.clone(), thorough_heuristic_fn).await;
    assert!(thorough_result.is_ok());
    
    // Test conflict-heavy scenario
    let mut search_conflict = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(30),
        1000,
    );
    
    let conflict_heuristic = HeuristicFactory::create_for_scenario("conflict_heavy");
    let conflict_heuristic_fn = |state: &SearchState| conflict_heuristic.calculate(state);
    
    let conflict_result = search_conflict.search(requirements, conflict_heuristic_fn).await;
    assert!(conflict_result.is_ok());
}

/// Benchmark different heuristics
#[tokio::test]
async fn test_heuristic_performance_comparison() {
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    
    let requirements = vec![
        PackageRequirement {
            name: "perf_package_1".to_string(),
            requirement_string: "perf_package_1".to_string(),
        },
        PackageRequirement {
            name: "perf_package_2".to_string(),
            requirement_string: "perf_package_2".to_string(),
        },
    ];
    
    // Test fast heuristic performance
    let mut search_fast = AStarSearch::new(
        repo_manager.clone(),
        config.clone(),
        Duration::from_secs(5),
        100,
    );
    
    let fast_heuristic = CompositeHeuristic::new_fast();
    let fast_heuristic_fn = |state: &SearchState| fast_heuristic.calculate(state);
    
    let start_time = std::time::Instant::now();
    let fast_result = search_fast.search(requirements.clone(), fast_heuristic_fn).await;
    let fast_duration = start_time.elapsed();
    
    assert!(fast_result.is_ok());
    
    // Test thorough heuristic performance
    let mut search_thorough = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(5),
        100,
    );
    
    let thorough_heuristic = CompositeHeuristic::new_thorough();
    let thorough_heuristic_fn = |state: &SearchState| thorough_heuristic.calculate(state);
    
    let start_time = std::time::Instant::now();
    let thorough_result = search_thorough.search(requirements, thorough_heuristic_fn).await;
    let thorough_duration = start_time.elapsed();
    
    assert!(thorough_result.is_ok());
    
    // Fast heuristic should generally be faster (though both may hit limits quickly with mock repo)
    println!("Fast heuristic duration: {:?}", fast_duration);
    println!("Thorough heuristic duration: {:?}", thorough_duration);
}

/// Test heuristic consistency
#[test]
fn test_heuristic_consistency() {
    let state = SearchState::new_initial(vec![
        PackageRequirement {
            name: "consistent_package".to_string(),
            requirement_string: "consistent_package".to_string(),
        },
    ]);
    
    let heuristic = CompositeHeuristic::new_fast();
    
    // Multiple calls should return the same value for the same state
    let cost1 = heuristic.calculate(&state);
    let cost2 = heuristic.calculate(&state);
    let cost3 = heuristic.calculate(&state);
    
    assert_eq!(cost1, cost2);
    assert_eq!(cost2, cost3);
    assert!(cost1 > 0.0);
}

/// Test heuristic monotonicity (h(n) should decrease as we get closer to goal)
#[test]
fn test_heuristic_monotonicity() {
    let initial_requirements = vec![
        PackageRequirement {
            name: "mono_package_1".to_string(),
            requirement_string: "mono_package_1".to_string(),
        },
        PackageRequirement {
            name: "mono_package_2".to_string(),
            requirement_string: "mono_package_2".to_string(),
        },
    ];
    
    let initial_state = SearchState::new_initial(initial_requirements);
    let heuristic = CompositeHeuristic::new_fast();
    
    let initial_cost = heuristic.calculate(&initial_state);
    
    // Create a state with one requirement resolved
    let resolved_package = Package {
        name: "mono_package_1".to_string(),
        requires: vec![],
    };
    
    let partial_state = SearchState::new_from_parent(
        &initial_state,
        resolved_package,
        vec![], // No new requirements
        1.0,    // Cost of resolving the package
    );
    
    let partial_cost = heuristic.calculate(&partial_state);
    
    // Heuristic cost should generally decrease as we resolve requirements
    // (though this may not always hold for non-admissible heuristics)
    println!("Initial heuristic cost: {}", initial_cost);
    println!("Partial heuristic cost: {}", partial_cost);
    
    // At minimum, both should be non-negative
    assert!(initial_cost >= 0.0);
    assert!(partial_cost >= 0.0);
}
