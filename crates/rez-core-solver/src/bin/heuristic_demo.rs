//! Heuristic Functions Demonstration
//!
//! This binary demonstrates the usage and performance of different heuristic functions
//! for dependency resolution using the A* search algorithm.

use rez_core_solver::astar::{
    AStarSearch, SearchState, PackageRequirement, Package, DependencyConflict, ConflictType,
    HeuristicFactory, CompositeHeuristic, AdaptiveHeuristic,
    DependencyHeuristic, HeuristicConfig, RepositoryManager, SolverConfig,
    PackageSearchCriteria, HeuristicBenchmark, BenchmarkConfig
};
use std::sync::Arc;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Rez-Core Heuristic Functions Demonstration ===\n");
    
    // Demo 1: Basic heuristic usage
    demo_basic_heuristics().await?;
    
    // Demo 2: A* search with different heuristics
    demo_astar_with_heuristics().await?;
    
    // Demo 3: Adaptive heuristic behavior
    demo_adaptive_heuristic().await?;
    
    // Demo 4: Performance benchmarking
    demo_performance_benchmark();
    
    // Demo 5: Heuristic factory usage
    demo_heuristic_factory().await?;
    
    println!("\n=== Demonstration Complete ===");
    Ok(())
}

/// Demonstrate basic heuristic function usage
async fn demo_basic_heuristics() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 1: Basic Heuristic Usage ---");
    
    // Create a test state with some requirements and conflicts
    let requirements = vec![
        PackageRequirement {
            name: "numpy".to_string(),
            requirement_string: "numpy>=1.20".to_string(),
        },
        PackageRequirement {
            name: "scipy".to_string(),
            requirement_string: "scipy>=1.7".to_string(),
        },
        PackageRequirement {
            name: "matplotlib".to_string(),
            requirement_string: "matplotlib>=3.0".to_string(),
        },
    ];
    
    let mut state = SearchState::new_initial(requirements);
    
    // Add a resolved package
    let resolved_package = Package {
        name: "numpy".to_string(),
        requires: vec!["blas".to_string()],
    };
    state.resolved_packages.insert("numpy".to_string(), resolved_package);
    
    // Add a conflict
    let conflict = DependencyConflict {
        package_name: "scipy".to_string(),
        conflicting_requirements: vec![],
        severity: 0.8,
        conflict_type: ConflictType::VersionConflict,
    };
    state.add_conflict(conflict);
    
    // Test different heuristics
    let config = HeuristicConfig::default();
    
    let fast_heuristic = CompositeHeuristic::new_fast();
    let thorough_heuristic = CompositeHeuristic::new_thorough();
    let adaptive_heuristic = AdaptiveHeuristic::new(config);
    
    println!("State complexity: {}", state.calculate_complexity());
    println!("Fast heuristic cost: {:.2}", fast_heuristic.calculate(&state));
    println!("Thorough heuristic cost: {:.2}", thorough_heuristic.calculate(&state));
    println!("Adaptive heuristic cost: {:.2}", adaptive_heuristic.calculate(&state));
    println!();
    
    Ok(())
}

/// Demonstrate A* search with different heuristics
async fn demo_astar_with_heuristics() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 2: A* Search with Different Heuristics ---");
    
    let repo_manager = Arc::new(RepositoryManager::new());
    let config = SolverConfig::default();
    
    let requirements = vec![
        PackageRequirement {
            name: "web_framework".to_string(),
            requirement_string: "web_framework>=2.0".to_string(),
        },
        PackageRequirement {
            name: "database_driver".to_string(),
            requirement_string: "database_driver>=1.5".to_string(),
        },
    ];
    
    // Test with fast heuristic
    let mut search_fast = AStarSearch::new(
        repo_manager.clone(),
        config.clone(),
        Duration::from_secs(5),
        50,
    );
    
    let fast_heuristic = CompositeHeuristic::new_fast();
    let fast_heuristic_fn = |state: &SearchState| fast_heuristic.calculate(state);
    
    println!("Running A* search with fast heuristic...");
    let start_time = std::time::Instant::now();
    let fast_result = search_fast.search(requirements.clone(), fast_heuristic_fn).await;
    let fast_duration = start_time.elapsed();
    
    match fast_result {
        Ok(Some(_solution)) => println!("Fast heuristic: Solution found in {:?}", fast_duration),
        Ok(None) => println!("Fast heuristic: No solution found in {:?}", fast_duration),
        Err(e) => println!("Fast heuristic: Search error - {} (in {:?})", e, fast_duration),
    }
    
    let fast_stats = search_fast.get_stats();
    println!("  States explored: {}", fast_stats.states_explored);
    println!("  Search time: {} ms", fast_stats.search_time_ms);
    
    // Test with thorough heuristic
    let mut search_thorough = AStarSearch::new(
        repo_manager,
        config,
        Duration::from_secs(5),
        50,
    );
    
    let thorough_heuristic = CompositeHeuristic::new_thorough();
    let thorough_heuristic_fn = |state: &SearchState| thorough_heuristic.calculate(state);
    
    println!("\nRunning A* search with thorough heuristic...");
    let start_time = std::time::Instant::now();
    let thorough_result = search_thorough.search(requirements, thorough_heuristic_fn).await;
    let thorough_duration = start_time.elapsed();
    
    match thorough_result {
        Ok(Some(_solution)) => println!("Thorough heuristic: Solution found in {:?}", thorough_duration),
        Ok(None) => println!("Thorough heuristic: No solution found in {:?}", thorough_duration),
        Err(e) => println!("Thorough heuristic: Search error - {} (in {:?})", e, thorough_duration),
    }
    
    let thorough_stats = search_thorough.get_stats();
    println!("  States explored: {}", thorough_stats.states_explored);
    println!("  Search time: {} ms", thorough_stats.search_time_ms);
    println!();
    
    Ok(())
}

/// Demonstrate adaptive heuristic behavior
async fn demo_adaptive_heuristic() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 3: Adaptive Heuristic Behavior ---");
    
    let config = HeuristicConfig::default();
    let mut adaptive_heuristic = AdaptiveHeuristic::new(config);
    
    let test_state = SearchState::new_initial(vec![
        PackageRequirement {
            name: "adaptive_test".to_string(),
            requirement_string: "adaptive_test".to_string(),
        },
    ]);
    
    // Test initial behavior
    let initial_cost = adaptive_heuristic.calculate(&test_state);
    println!("Initial adaptive heuristic cost: {:.2}", initial_cost);
    
    // Simulate different search scenarios and show adaptation
    let scenarios = vec![
        (10, 0, 5.0, 3, "Low conflict scenario"),
        (50, 8, 12.0, 8, "High conflict scenario"),
        (100, 15, 20.0, 15, "Very complex scenario"),
    ];
    
    for (states, conflicts, branching, depth, description) in scenarios {
        adaptive_heuristic.update_stats(states, conflicts, branching, depth);
        let adapted_cost = adaptive_heuristic.calculate(&test_state);
        println!("{}: {:.2} (states: {}, conflicts: {}, branching: {:.1}, depth: {})", 
                 description, adapted_cost, states, conflicts, branching, depth);
    }
    println!();
    
    Ok(())
}

/// Demonstrate performance benchmarking
fn demo_performance_benchmark() {
    println!("--- Demo 4: Performance Benchmarking ---");
    
    let config = BenchmarkConfig {
        iterations: 100,
        state_variations: 5,
        max_requirements: 10,
        max_conflicts: 3,
    };
    
    let benchmark = HeuristicBenchmark::new(config);
    println!("Running comprehensive heuristic benchmark...");
    
    let results = benchmark.run_comprehensive_benchmark();
    benchmark.print_results(&results);
}

/// Demonstrate heuristic factory usage
async fn demo_heuristic_factory() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Demo 5: Heuristic Factory Usage ---");
    
    let test_state = SearchState::new_initial(vec![
        PackageRequirement {
            name: "factory_test".to_string(),
            requirement_string: "factory_test".to_string(),
        },
    ]);
    
    // Test complexity-based factory
    let complexities = vec![5, 25, 75];
    for complexity in complexities {
        let heuristic = HeuristicFactory::create_for_complexity(complexity);
        let cost = heuristic.calculate(&test_state);
        println!("Complexity {}: heuristic cost = {:.2}", complexity, cost);
    }
    
    println!();
    
    // Test scenario-based factory
    let scenarios = vec!["fast", "thorough", "conflict_heavy"];
    for scenario in scenarios {
        let heuristic = HeuristicFactory::create_for_scenario(scenario);
        let cost = heuristic.calculate(&test_state);
        println!("Scenario '{}': heuristic cost = {:.2}", scenario, cost);
    }
    
    println!();
    Ok(())
}
