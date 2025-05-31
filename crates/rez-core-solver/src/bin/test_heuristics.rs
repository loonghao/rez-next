//! Standalone test for heuristic functions
//!
//! This binary tests the heuristic functions independently without dependencies

use rez_core_solver::astar::{
    SearchState, PackageRequirement, Package, DependencyConflict, ConflictType,
    HeuristicFactory, CompositeHeuristic, AdaptiveHeuristic,
    DependencyHeuristic, HeuristicConfig,
    RemainingRequirementsHeuristic, ConflictPenaltyHeuristic,
    DependencyDepthHeuristic, VersionPreferenceHeuristic,
    HeuristicBenchmark, BenchmarkConfig
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Heuristic Functions Standalone Test ===\n");
    
    // Test 1: Basic heuristic creation and calculation
    test_basic_heuristics()?;
    
    // Test 2: Heuristic factory
    test_heuristic_factory()?;
    
    // Test 3: Adaptive heuristic
    test_adaptive_heuristic()?;
    
    // Test 4: Performance benchmark
    test_performance_benchmark()?;
    
    println!("\n=== All Tests Passed! ===");
    Ok(())
}

fn test_basic_heuristics() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 1: Basic Heuristic Functions ---");
    
    // Create test state
    let requirements = vec![
        PackageRequirement {
            name: "numpy".to_string(),
            requirement_string: "numpy>=1.20".to_string(),
        },
        PackageRequirement {
            name: "scipy".to_string(),
            requirement_string: "scipy>=1.7".to_string(),
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
    
    // Test individual heuristics
    let config = HeuristicConfig::default();
    
    let remaining_req_heuristic = RemainingRequirementsHeuristic::new(config.clone());
    let remaining_cost = remaining_req_heuristic.calculate(&state);
    println!("Remaining requirements heuristic: {:.2}", remaining_cost);
    assert!(remaining_cost > 0.0);
    
    let conflict_penalty_heuristic = ConflictPenaltyHeuristic::new(config.clone());
    let conflict_cost = conflict_penalty_heuristic.calculate(&state);
    println!("Conflict penalty heuristic: {:.2}", conflict_cost);
    assert!(conflict_cost > 0.0);
    
    let dependency_depth_heuristic = DependencyDepthHeuristic::new(config.clone());
    let depth_cost = dependency_depth_heuristic.calculate(&state);
    println!("Dependency depth heuristic: {:.2}", depth_cost);
    assert!(depth_cost > 0.0);
    
    let version_preference_heuristic = VersionPreferenceHeuristic::new(config);
    let version_cost = version_preference_heuristic.calculate(&state);
    println!("Version preference heuristic: {:.2}", version_cost);
    assert!(version_cost >= 0.0);
    
    // Test composite heuristics
    let fast_composite = CompositeHeuristic::new_fast();
    let fast_cost = fast_composite.calculate(&state);
    println!("Fast composite heuristic: {:.2}", fast_cost);
    assert!(fast_cost > 0.0);
    
    let thorough_composite = CompositeHeuristic::new_thorough();
    let thorough_cost = thorough_composite.calculate(&state);
    println!("Thorough composite heuristic: {:.2}", thorough_cost);
    assert!(thorough_cost > 0.0);
    
    println!("✓ Basic heuristics test passed\n");
    Ok(())
}

fn test_heuristic_factory() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 2: Heuristic Factory ---");
    
    let test_state = SearchState::new_initial(vec![
        PackageRequirement {
            name: "test_package".to_string(),
            requirement_string: "test_package".to_string(),
        },
    ]);
    
    // Test complexity-based factory
    let complexities = vec![5, 25, 75];
    for complexity in complexities {
        let heuristic = HeuristicFactory::create_for_complexity(complexity);
        let cost = heuristic.calculate(&test_state);
        println!("Complexity {}: cost = {:.2}", complexity, cost);
        assert!(cost > 0.0);
    }
    
    // Test scenario-based factory
    let scenarios = vec!["fast", "thorough", "conflict_heavy"];
    for scenario in scenarios {
        let heuristic = HeuristicFactory::create_for_scenario(scenario);
        let cost = heuristic.calculate(&test_state);
        println!("Scenario '{}': cost = {:.2}", scenario, cost);
        assert!(cost > 0.0);
    }
    
    println!("✓ Heuristic factory test passed\n");
    Ok(())
}

fn test_adaptive_heuristic() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 3: Adaptive Heuristic ---");
    
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
    println!("Initial adaptive cost: {:.2}", initial_cost);
    assert!(initial_cost > 0.0);
    
    // Test adaptation
    let scenarios = vec![
        (10, 0, 5.0, 3, "Low conflict scenario"),
        (50, 8, 12.0, 8, "High conflict scenario"),
        (100, 15, 20.0, 15, "Very complex scenario"),
    ];
    
    for (states, conflicts, branching, depth, description) in scenarios {
        adaptive_heuristic.update_stats(states, conflicts, branching, depth);
        let adapted_cost = adaptive_heuristic.calculate(&test_state);
        println!("{}: {:.2}", description, adapted_cost);
        assert!(adapted_cost > 0.0);
    }
    
    println!("✓ Adaptive heuristic test passed\n");
    Ok(())
}

fn test_performance_benchmark() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Test 4: Performance Benchmark ---");
    
    let config = BenchmarkConfig {
        iterations: 10,
        state_variations: 3,
        max_requirements: 5,
        max_conflicts: 2,
    };
    
    let benchmark = HeuristicBenchmark::new(config);
    println!("Running mini benchmark...");
    
    let results = benchmark.run_comprehensive_benchmark();
    
    // Verify results
    assert!(!results.is_empty());
    for result in &results {
        assert!(!result.heuristic_name.is_empty());
        assert!(result.avg_calculation_time_ns > 0);
        assert!(result.calculations_per_second > 0.0);
        println!("{}: {:.0} calc/sec", result.heuristic_name, result.calculations_per_second);
    }
    
    // Find fastest and slowest
    if let (Some(fastest), Some(slowest)) = (
        results.iter().min_by_key(|r| r.avg_calculation_time_ns),
        results.iter().max_by_key(|r| r.avg_calculation_time_ns)
    ) {
        println!("Fastest: {} ({:.0} calc/sec)", fastest.heuristic_name, fastest.calculations_per_second);
        println!("Slowest: {} ({:.0} calc/sec)", slowest.heuristic_name, slowest.calculations_per_second);
        
        if slowest.avg_calculation_time_ns > 0 {
            let speedup = slowest.avg_calculation_time_ns as f64 / fastest.avg_calculation_time_ns as f64;
            println!("Speedup: {:.2}x", speedup);
        }
    }
    
    println!("✓ Performance benchmark test passed\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
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
    
    #[test]
    fn test_heuristic_admissibility() {
        let config = HeuristicConfig::default();
        
        let remaining_req_heuristic = RemainingRequirementsHeuristic::new(config.clone());
        assert!(remaining_req_heuristic.is_admissible());
        
        let conflict_penalty_heuristic = ConflictPenaltyHeuristic::new(config.clone());
        assert!(!conflict_penalty_heuristic.is_admissible()); // Conflict penalty is not admissible
        
        let dependency_depth_heuristic = DependencyDepthHeuristic::new(config.clone());
        assert!(dependency_depth_heuristic.is_admissible());
        
        let version_preference_heuristic = VersionPreferenceHeuristic::new(config);
        assert!(version_preference_heuristic.is_admissible());
    }
    
    #[test]
    fn test_state_complexity_calculation() {
        let requirements = vec![
            PackageRequirement {
                name: "pkg1".to_string(),
                requirement_string: "pkg1".to_string(),
            },
            PackageRequirement {
                name: "pkg2".to_string(),
                requirement_string: "pkg2".to_string(),
            },
        ];
        
        let mut state = SearchState::new_initial(requirements);
        
        // Initial complexity: 2 requirements
        assert_eq!(state.calculate_complexity(), 2);
        
        // Add a resolved package
        let package = Package {
            name: "pkg1".to_string(),
            requires: vec![],
        };
        state.resolved_packages.insert("pkg1".to_string(), package);
        
        // Complexity: 1 resolved + 2 requirements = 3
        assert_eq!(state.calculate_complexity(), 3);
        
        // Add a conflict
        let conflict = DependencyConflict {
            package_name: "pkg2".to_string(),
            conflicting_requirements: vec![],
            severity: 1.0,
            conflict_type: ConflictType::VersionConflict,
        };
        state.add_conflict(conflict);
        
        // Complexity: 1 resolved + 2 requirements + 1 conflict * 2 = 5
        assert_eq!(state.calculate_complexity(), 5);
    }
}
