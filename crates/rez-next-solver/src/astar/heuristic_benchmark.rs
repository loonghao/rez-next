//! Benchmark tests for heuristic functions

use super::heuristics::{
    AdaptiveHeuristic, CompositeHeuristic, ConflictPenaltyHeuristic, DependencyDepthHeuristic,
    DependencyHeuristic, HeuristicConfig, HeuristicFactory, RemainingRequirementsHeuristic,
    VersionPreferenceHeuristic,
};
use super::search_state::{ConflictType, DependencyConflict, SearchState};
use rez_next_package::{Package, PackageRequirement};
use std::time::Instant;

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
            let num_requirements = 1 + (i % self.config.max_requirements);
            let num_conflicts = i % (self.config.max_conflicts + 1);

            let requirements: Vec<PackageRequirement> = (0..num_requirements)
                .map(|j| PackageRequirement::with_version(
                    format!("package_{}", j),
                    format!(">=1.0"),
                ))
                .collect();

            let mut state = SearchState::new_initial(requirements);

            // Add some resolved packages
            for j in 0..(i % 5) {
                let mut pkg = Package::new(format!("resolved_package_{}", j));
                pkg.requires.push(format!("dep_{}", j));
                state.resolved_packages.insert(pkg.name.clone(), pkg);
            }

            // Add conflicts
            for j in 0..num_conflicts {
                let conflict_type = match j % 4 {
                    0 => ConflictType::VersionConflict,
                    1 => ConflictType::CircularDependency,
                    2 => ConflictType::MissingPackage,
                    _ => ConflictType::PlatformConflict,
                };
                state.add_conflict(DependencyConflict::new(
                    format!("conflict_package_{}", j),
                    vec![],
                    0.5 + (j as f64 * 0.1),
                    conflict_type,
                ));
            }

            self.test_states.push(state);
        }
    }

    /// Benchmark a specific heuristic function (dynamic dispatch version)
    pub fn benchmark_heuristic_dyn(&self, heuristic: &dyn DependencyHeuristic) -> BenchmarkResult {
        let mut calculation_times = Vec::new();

        for _ in 0..self.config.iterations {
            for state in &self.test_states {
                let start_time = Instant::now();
                let _cost = heuristic.calculate(state);
                let elapsed = start_time.elapsed();
                calculation_times.push(elapsed.as_nanos() as u64);
            }
        }

        let total_calculations = calculation_times.len();
        let avg_time_ns = calculation_times.iter().sum::<u64>() / total_calculations.max(1) as u64;
        let min_time_ns = *calculation_times.iter().min().unwrap_or(&0);
        let max_time_ns = *calculation_times.iter().max().unwrap_or(&0);

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

    /// Benchmark a specific heuristic function (static dispatch version)
    pub fn benchmark_heuristic<H: DependencyHeuristic>(&self, heuristic: &H) -> BenchmarkResult {
        self.benchmark_heuristic_dyn(heuristic)
    }

    /// Run comprehensive benchmark suite
    pub fn run_comprehensive_benchmark(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        let config = HeuristicConfig::default();

        results.push(self.benchmark_heuristic(&RemainingRequirementsHeuristic::new(config.clone())));
        results.push(self.benchmark_heuristic(&ConflictPenaltyHeuristic::new(config.clone())));
        results.push(self.benchmark_heuristic(&DependencyDepthHeuristic::new(config.clone())));
        results.push(self.benchmark_heuristic(&VersionPreferenceHeuristic::new(config.clone())));
        results.push(self.benchmark_heuristic(&CompositeHeuristic::new_fast()));
        results.push(self.benchmark_heuristic(&CompositeHeuristic::new_thorough()));
        results.push(self.benchmark_heuristic(&CompositeHeuristic::new(config.clone())));
        results.push(self.benchmark_heuristic(&AdaptiveHeuristic::new(config)));

        let simple_heuristic = HeuristicFactory::create_for_complexity(5);
        results.push(self.benchmark_heuristic_dyn(simple_heuristic.as_ref()));

        let complex_heuristic = HeuristicFactory::create_for_complexity(100);
        results.push(self.benchmark_heuristic_dyn(complex_heuristic.as_ref()));

        results
    }

    /// Print benchmark results in a formatted table
    pub fn print_results(&self, results: &[BenchmarkResult]) {
        println!("\n=== Heuristic Function Benchmark Results ===");
        println!(
            "Config: {} iterations, {} state variations",
            self.config.iterations, self.config.state_variations
        );
        println!(
            "{:<28} {:>14} {:>14} {:>14} {:>20}",
            "Heuristic", "Avg (ns)", "Min (ns)", "Max (ns)", "Calc/sec"
        );
        println!("{}", "-".repeat(95));

        for result in results {
            println!(
                "{:<28} {:>14} {:>14} {:>14} {:>20.0}",
                result.heuristic_name,
                result.avg_calculation_time_ns,
                result.min_calculation_time_ns,
                result.max_calculation_time_ns,
                result.calculations_per_second,
            );
        }
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_generation() {
        let config = BenchmarkConfig {
            iterations: 10,
            state_variations: 5,
            max_requirements: 5,
            max_conflicts: 2,
        };
        let benchmark = HeuristicBenchmark::new(config);
        assert_eq!(benchmark.test_states.len(), 5);
    }

    #[test]
    fn test_benchmark_runs() {
        let config = BenchmarkConfig {
            iterations: 5,
            state_variations: 3,
            max_requirements: 3,
            max_conflicts: 1,
        };
        let benchmark = HeuristicBenchmark::new(config);
        let results = benchmark.run_comprehensive_benchmark();
        assert!(!results.is_empty());
        for result in &results {
            assert!(result.avg_calculation_time_ns < 1_000_000_000,
                    "Heuristic '{}' should complete in < 1s per calculation", result.heuristic_name);
        }
    }
}
