//! Solver Benchmark Main Entry Point
//!
//! This file provides the main entry point for running solver benchmarks
//! using the Criterion framework. It integrates with the comprehensive
//! benchmark suite and provides standalone solver testing capabilities.

use criterion::{criterion_group, criterion_main, Criterion};

// Import benchmark modules
mod comprehensive_benchmark_suite;
mod solver_benchmark;
mod version_benchmark;

use comprehensive_benchmark_suite::{create_comprehensive_suite, BenchmarkSuite, ModuleBenchmark};
use solver_benchmark::SolverBenchmark;

/// Run all solver benchmarks
fn run_solver_benchmarks(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    // Validate the benchmark module before running
    if let Err(e) = solver_benchmark.validate() {
        eprintln!("Solver benchmark validation failed: {}", e);
        return;
    }

    println!("Running Solver Benchmark Suite...");
    println!("{}", solver_benchmark.generate_performance_report());

    // Run all solver benchmarks
    solver_benchmark.run_benchmarks(c);
}

/// Run comprehensive benchmark suite including solver
fn run_comprehensive_benchmarks(c: &mut Criterion) {
    let suite = create_comprehensive_suite();

    println!("Running Comprehensive Benchmark Suite...");
    println!("Registered modules: {:?}", suite.list_modules());

    // Run all registered benchmarks
    if let Err(e) = suite.run_all() {
        eprintln!("Comprehensive benchmark suite failed: {}", e);
    }
}

/// Run solver-specific performance tests
fn run_solver_performance_tests(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    // Run specific performance-focused benchmarks
    solver_benchmark.benchmark_basic_resolution(c);
    solver_benchmark.benchmark_conflict_resolution(c);
    solver_benchmark.benchmark_cache_performance(c);
    solver_benchmark.benchmark_parallel_solving(c);
    solver_benchmark.benchmark_scalability(c);
}

/// Run solver memory and resource usage tests
fn run_solver_resource_tests(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    // Run resource-focused benchmarks
    solver_benchmark.benchmark_memory_usage(c);
    solver_benchmark.benchmark_statistics_collection(c);
}

/// Run solver algorithm comparison tests
fn run_solver_algorithm_tests(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    // Run algorithm comparison benchmarks
    solver_benchmark.benchmark_astar_algorithm(c);
    solver_benchmark.benchmark_optimized_solver(c);
}

/// Quick solver benchmark for development
fn quick_solver_benchmark(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    // Run a subset of benchmarks for quick feedback
    let mut group = c.benchmark_group("quick_solver_test");

    // Test one simple scenario
    if let Some(scenario) = solver_benchmark.test_data.simple_scenarios.first() {
        group.bench_function("quick_simple_resolution", |b| {
            let solver = rez_core_solver::DependencySolver::new();
            b.iter(|| {
                let request = rez_core_solver::SolverRequest {
                    requirements: scenario.requirements.clone(),
                    config: rez_core_solver::SolverConfig::default(),
                };
                criterion::black_box(solver.resolve(criterion::black_box(request)))
            });
        });
    }

    // Test one conflict scenario
    if let Some(scenario) = solver_benchmark.test_data.conflict_scenarios.first() {
        group.bench_function("quick_conflict_resolution", |b| {
            let solver = rez_core_solver::DependencySolver::new();
            b.iter(|| {
                let request = rez_core_solver::SolverRequest {
                    requirements: scenario.conflicting_requirements.clone(),
                    config: rez_core_solver::SolverConfig::default(),
                };
                criterion::black_box(solver.resolve(criterion::black_box(request)))
            });
        });
    }

    group.finish();
}

/// Baseline establishment benchmark
fn establish_solver_baseline(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    println!("Establishing Solver Performance Baseline...");

    // Run baseline measurements
    let baseline_metrics = solver_benchmark.get_baseline_metrics();
    println!(
        "Baseline metrics collected for module: {}",
        baseline_metrics.module_name
    );

    // Run a representative set of benchmarks for baseline
    solver_benchmark.benchmark_basic_resolution(c);
    solver_benchmark.benchmark_scalability(c);
}

/// Regression testing benchmark
fn solver_regression_test(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    println!("Running Solver Regression Tests...");

    // This would compare against saved baselines in a real implementation
    // For now, just run the benchmarks
    solver_benchmark.run_benchmarks(c);
}

/// Performance validation benchmark (verify 3-5x improvements)
fn validate_solver_performance(c: &mut Criterion) {
    let solver_benchmark = SolverBenchmark::new();

    println!("Validating Solver Performance Improvements...");

    // Run benchmarks that validate performance targets
    solver_benchmark.benchmark_optimized_solver(c);
    solver_benchmark.benchmark_parallel_solving(c);
    solver_benchmark.benchmark_cache_performance(c);

    // TODO: Add actual performance validation logic
    // This would compare results against baseline and verify improvement targets
}

// Define criterion groups for different types of benchmarks
criterion_group!(
    name = solver_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = run_solver_benchmarks
);

criterion_group!(
    name = comprehensive_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(15))
        .warm_up_time(std::time::Duration::from_secs(5));
    targets = run_comprehensive_benchmarks
);

criterion_group!(
    name = solver_performance;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(std::time::Duration::from_secs(20))
        .warm_up_time(std::time::Duration::from_secs(5));
    targets = run_solver_performance_tests
);

criterion_group!(
    name = solver_resources;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = run_solver_resource_tests
);

criterion_group!(
    name = solver_algorithms;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(15))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = run_solver_algorithm_tests
);

criterion_group!(
    name = quick_solver;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(3))
        .warm_up_time(std::time::Duration::from_secs(1));
    targets = quick_solver_benchmark
);

criterion_group!(
    name = solver_baseline;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = establish_solver_baseline
);

criterion_group!(
    name = solver_regression;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = solver_regression_test
);

criterion_group!(
    name = solver_validation;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(std::time::Duration::from_secs(20))
        .warm_up_time(std::time::Duration::from_secs(5));
    targets = validate_solver_performance
);

// Main entry point - runs all solver benchmarks by default
criterion_main!(
    solver_benchmarks,
    solver_performance,
    solver_algorithms,
    solver_resources
);

// Alternative entry points for specific benchmark types:
// To run only quick tests: cargo bench --bench solver_benchmark_main quick_solver
// To run comprehensive suite: cargo bench --bench solver_benchmark_main comprehensive_benchmarks
// To run performance validation: cargo bench --bench solver_benchmark_main solver_validation
// To establish baseline: cargo bench --bench solver_benchmark_main solver_baseline
// To run regression tests: cargo bench --bench solver_benchmark_main solver_regression
