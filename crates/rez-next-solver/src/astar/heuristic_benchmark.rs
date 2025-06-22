//! Benchmark tests for heuristic functions
//!
//! This module provides comprehensive benchmarks for different heuristic functions
//! to measure their performance and effectiveness in guiding A* search.

use super::{
    AdaptiveHeuristic, CompositeHeuristic, ConflictPenaltyHeuristic, ConflictType,
    DependencyConflict, DependencyDepthHeuristic, DependencyHeuristic, HeuristicConfig,
    HeuristicFactory, Package, PackageRequirement, RemainingRequirementsHeuristic, SearchState,
    VersionPreferenceHeuristic,
};
use std::time::{Duration, Instant};

/// Benchmark configuration
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub state_variations: usize,
    pub max_requirements: usize,
    pub max_conflicts: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            iterations: 1000,
            state_variations: 10,
            max_requirements: 20,
            max_conflicts: 5,
        }
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub heuristic_name: String,
    pub avg_calculation_time_ns: u64,
    pub min_calculation_time_ns: u64,
    pub max_calculation_time_ns: u64,
    pub total_iterations: usize,
    pub calculations_per_second: f64,
}

/// Heuristic benchmark suite
pub struct HeuristicBenchmark {
    config: BenchmarkConfig,
    test_states: Vec<SearchState>,
}

impl HeuristicBenchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        let mut benchmark = Self {
            config,
            test_states: Vec::new(),
        };
        benchmark.generate_test_states();
        benchmark
    }

    /// Generate diverse test states for benchmarking
    fn generate_test_states(&mut self) {
        for i in 0..self.config.state_variations {
            // Create states with varying complexity
            let num_requirements = 1 + (i % self.config.max_requirements);
            let num_conflicts = i % (self.config.max_conflicts + 1);

            let mut requirements = Vec::new();
            for j in 0..num_requirements {
                requirements.push(PackageRequirement {
                    name: format!("package_{}", j),
                    requirement_string: format!("package_{}>=1.0", j),
                });
            }

            let mut state = SearchState::new_initial(requirements);

            // Add some resolved packages
            for j in 0..(i % 5) {
                let package = Package {
                    name: format!("resolved_package_{}", j),
                    requires: vec![format!("dep_{}", j)],
                };
                state
                    .resolved_packages
                    .insert(package.name.clone(), package);
            }

            // Add conflicts
            for j in 0..num_conflicts {
                let conflict = DependencyConflict {
                    package_name: format!("conflict_package_{}", j),
                    conflicting_requirements: vec![],
                    severity: 0.5 + (j as f64 * 0.1),
                    conflict_type: match j % 4 {
                        0 => ConflictType::VersionConflict,
                        1 => ConflictType::CircularDependency,
                        2 => ConflictType::MissingPackage,
                        _ => ConflictType::PlatformConflict,
                    },
                };
                state.add_conflict(conflict);
            }

            self.test_states.push(state);
        }
    }

    /// Benchmark a specific heuristic function
    pub fn benchmark_heuristic<H: DependencyHeuristic>(&self, heuristic: &H) -> BenchmarkResult {
        let mut calculation_times = Vec::new();

        for _ in 0..self.config.iterations {
            for state in &self.test_states {
                let start_time = Instant::now();
                let _cost = heuristic.calculate(state);
                let calculation_time = start_time.elapsed();
                calculation_times.push(calculation_time.as_nanos() as u64);
            }
        }

        let total_calculations = calculation_times.len();
        let avg_time_ns = calculation_times.iter().sum::<u64>() / total_calculations as u64;
        let min_time_ns = *calculation_times.iter().min().unwrap();
        let max_time_ns = *calculation_times.iter().max().unwrap();

        let calculations_per_second = if avg_time_ns > 0 {
            1_000_000_000.0 / avg_time_ns as f64
        } else {
            f64::INFINITY
        };

        BenchmarkResult {
            heuristic_name: heuristic.name().to_string(),
            avg_calculation_time_ns: avg_time_ns,
            min_calculation_time_ns: min_time_ns,
            max_calculation_time_ns: max_time_ns,
            total_iterations: total_calculations,
            calculations_per_second,
        }
    }

    /// Run comprehensive benchmark suite
    pub fn run_comprehensive_benchmark(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        // Benchmark individual heuristics
        let config = HeuristicConfig::default();

        let remaining_req_heuristic = RemainingRequirementsHeuristic::new(config.clone());
        results.push(self.benchmark_heuristic(&remaining_req_heuristic));

        let conflict_penalty_heuristic = ConflictPenaltyHeuristic::new(config.clone());
        results.push(self.benchmark_heuristic(&conflict_penalty_heuristic));

        let dependency_depth_heuristic = DependencyDepthHeuristic::new(config.clone());
        results.push(self.benchmark_heuristic(&dependency_depth_heuristic));

        let version_preference_heuristic = VersionPreferenceHeuristic::new(config.clone());
        results.push(self.benchmark_heuristic(&version_preference_heuristic));

        // Benchmark composite heuristics
        let fast_composite = CompositeHeuristic::new_fast();
        results.push(self.benchmark_heuristic(&fast_composite));

        let thorough_composite = CompositeHeuristic::new_thorough();
        results.push(self.benchmark_heuristic(&thorough_composite));

        let default_composite = CompositeHeuristic::new(config.clone());
        results.push(self.benchmark_heuristic(&default_composite));

        // Benchmark adaptive heuristic
        let adaptive_heuristic = AdaptiveHeuristic::new(config);
        results.push(self.benchmark_heuristic(&adaptive_heuristic));

        // Benchmark factory-created heuristics
        let simple_factory_heuristic = HeuristicFactory::create_for_complexity(5);
        results.push(self.benchmark_heuristic(simple_factory_heuristic.as_ref()));

        let complex_factory_heuristic = HeuristicFactory::create_for_complexity(100);
        results.push(self.benchmark_heuristic(complex_factory_heuristic.as_ref()));

        results
    }

    /// Print benchmark results in a formatted table
    pub fn print_results(&self, results: &[BenchmarkResult]) {
        println!("\n=== Heuristic Function Benchmark Results ===");
        println!(
            "Configuration: {} iterations, {} state variations",
            self.config.iterations, self.config.state_variations
        );
        println!();

        println!(
            "{:<25} {:>15} {:>15} {:>15} {:>20}",
            "Heuristic", "Avg Time (ns)", "Min Time (ns)", "Max Time (ns)", "Calc/sec"
        );
        println!("{}", "-".repeat(95));

        for result in results {
            println!(
                "{:<25} {:>15} {:>15} {:>15} {:>20.0}",
                result.heuristic_name,
                result.avg_calculation_time_ns,
                result.min_calculation_time_ns,
                result.max_calculation_time_ns,
                result.calculations_per_second
            );
        }

        println!();

        // Find fastest and slowest
        if let (Some(fastest), Some(slowest)) = (
            results.iter().min_by_key(|r| r.avg_calculation_time_ns),
            results.iter().max_by_key(|r| r.avg_calculation_time_ns),
        ) {
            println!(
                "Fastest: {} ({:.0} calc/sec)",
                fastest.heuristic_name, fastest.calculations_per_second
            );
            println!(
                "Slowest: {} ({:.0} calc/sec)",
                slowest.heuristic_name, slowest.calculations_per_second
            );

            if slowest.avg_calculation_time_ns > 0 {
                let speedup =
                    slowest.avg_calculation_time_ns as f64 / fastest.avg_calculation_time_ns as f64;
                println!("Speedup: {:.2}x", speedup);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_creation() {
        let config = BenchmarkConfig {
            iterations: 10,
            state_variations: 3,
            max_requirements: 5,
            max_conflicts: 2,
        };

        let benchmark = HeuristicBenchmark::new(config);
        assert_eq!(benchmark.test_states.len(), 3);
        assert_eq!(benchmark.config.iterations, 10);
    }

    #[test]
    fn test_individual_heuristic_benchmark() {
        let config = BenchmarkConfig {
            iterations: 5,
            state_variations: 2,
            max_requirements: 3,
            max_conflicts: 1,
        };

        let benchmark = HeuristicBenchmark::new(config);
        let heuristic = RemainingRequirementsHeuristic::new(HeuristicConfig::default());

        let result = benchmark.benchmark_heuristic(&heuristic);

        assert_eq!(result.heuristic_name, "RemainingRequirements");
        assert_eq!(result.total_iterations, 10); // 5 iterations * 2 states
        assert!(result.avg_calculation_time_ns > 0);
        assert!(result.calculations_per_second > 0.0);
    }

    #[test]
    fn test_comprehensive_benchmark() {
        let config = BenchmarkConfig {
            iterations: 2,
            state_variations: 2,
            max_requirements: 3,
            max_conflicts: 1,
        };

        let benchmark = HeuristicBenchmark::new(config);
        let results = benchmark.run_comprehensive_benchmark();

        assert!(!results.is_empty());

        // Verify all results have valid data
        for result in &results {
            assert!(!result.heuristic_name.is_empty());
            assert!(result.avg_calculation_time_ns > 0);
            assert!(result.calculations_per_second > 0.0);
            assert_eq!(result.total_iterations, 4); // 2 iterations * 2 states
        }
    }

    #[test]
    fn test_benchmark_result_consistency() {
        let config = BenchmarkConfig {
            iterations: 3,
            state_variations: 1,
            max_requirements: 2,
            max_conflicts: 0,
        };

        let benchmark = HeuristicBenchmark::new(config);
        let heuristic = CompositeHeuristic::new_fast();

        // Run benchmark multiple times
        let result1 = benchmark.benchmark_heuristic(&heuristic);
        let result2 = benchmark.benchmark_heuristic(&heuristic);

        // Results should be consistent (within reasonable variance)
        assert_eq!(result1.heuristic_name, result2.heuristic_name);
        assert_eq!(result1.total_iterations, result2.total_iterations);

        // Times may vary but should be in the same ballpark
        let time_ratio =
            result1.avg_calculation_time_ns as f64 / result2.avg_calculation_time_ns as f64;
        assert!(
            time_ratio > 0.1 && time_ratio < 10.0,
            "Times should be reasonably consistent"
        );
    }
}
