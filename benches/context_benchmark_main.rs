//! Context Benchmark Main Entry Point
//!
//! This file provides the main entry point for running context benchmarks
//! using the Criterion framework. It integrates with the comprehensive
//! benchmark suite and provides standalone context testing capabilities.

use criterion::{criterion_group, criterion_main, Criterion};

// Import benchmark modules
mod comprehensive_benchmark_suite;
mod context_benchmark;
mod solver_benchmark;
mod version_benchmark;

use comprehensive_benchmark_suite::{create_comprehensive_suite, BenchmarkSuite, ModuleBenchmark};
use context_benchmark::ContextBenchmark;

/// Run all context benchmarks
fn run_context_benchmarks(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    // Validate the benchmark module before running
    if let Err(e) = context_benchmark.validate() {
        eprintln!("Context benchmark validation failed: {}", e);
        return;
    }

    println!("Running Context Benchmark Suite...");
    println!("{}", context_benchmark.generate_performance_report());

    // Run all context benchmarks
    context_benchmark.run_benchmarks(c);
}

/// Run comprehensive benchmark suite including context
fn run_comprehensive_benchmarks(c: &mut Criterion) {
    let suite = create_comprehensive_suite();

    println!("Running Comprehensive Benchmark Suite...");
    println!("Registered modules: {:?}", suite.list_modules());

    // Run all registered benchmarks
    if let Err(e) = suite.run_all() {
        eprintln!("Comprehensive benchmark suite failed: {}", e);
    }
}

/// Run context-specific performance tests
fn run_context_performance_tests(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    // Run specific performance-focused benchmarks
    context_benchmark.benchmark_context_creation(c);
    context_benchmark.benchmark_environment_generation(c);
    context_benchmark.benchmark_shell_execution(c);
    context_benchmark.benchmark_execution_performance(c);
    context_benchmark.benchmark_scalability(c);
}

/// Run context memory and resource usage tests
fn run_context_resource_tests(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    // Run resource-focused benchmarks
    context_benchmark.benchmark_memory_usage(c);
    context_benchmark.benchmark_validation(c);
    context_benchmark.benchmark_context_caching(c);
}

/// Run context serialization and I/O tests
fn run_context_serialization_tests(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    // Run serialization-focused benchmarks
    context_benchmark.benchmark_serialization(c);
}

/// Quick context benchmark for development
fn quick_context_benchmark(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    // Run a subset of benchmarks for quick feedback
    let mut group = c.benchmark_group("quick_context_test");

    // Test one simple context creation
    if let Some(scenario) = context_benchmark.test_data.simple_contexts.first() {
        group.bench_function("quick_context_creation", |b| {
            b.iter(|| {
                let builder = rez_core_context::ContextBuilder::new()
                    .requirements(scenario.requirements.clone())
                    .config(scenario.config.clone());
                criterion::black_box(builder.build())
            });
        });
    }

    // Test one environment generation
    if let Some(scenario) = context_benchmark.test_data.env_scenarios.first() {
        group.bench_function("quick_env_generation", |b| {
            let env_manager = rez_core_context::EnvironmentManager::new(scenario.config.clone());
            b.iter(|| {
                criterion::black_box(env_manager.generate_environment_sync(&scenario.packages))
            });
        });
    }

    group.finish();
}

/// Baseline establishment benchmark
fn establish_context_baseline(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Establishing Context Performance Baseline...");

    // Run baseline measurements
    let baseline_metrics = context_benchmark.get_baseline_metrics();
    println!(
        "Baseline metrics collected for module: {}",
        baseline_metrics.module_name
    );

    // Run a representative set of benchmarks for baseline
    context_benchmark.benchmark_context_creation(c);
    context_benchmark.benchmark_environment_generation(c);
    context_benchmark.benchmark_scalability(c);
}

/// Regression testing benchmark
fn context_regression_test(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Running Context Regression Tests...");

    // This would compare against saved baselines in a real implementation
    // For now, just run the benchmarks
    context_benchmark.run_benchmarks(c);
}

/// Performance validation benchmark (verify performance improvements)
fn validate_context_performance(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Validating Context Performance Improvements...");

    // Run benchmarks that validate performance targets
    context_benchmark.benchmark_context_creation(c);
    context_benchmark.benchmark_environment_generation(c);
    context_benchmark.benchmark_shell_execution(c);
    context_benchmark.benchmark_execution_performance(c);

    // TODO: Add actual performance validation logic
    // This would compare results against baseline and verify improvement targets
}

/// Environment-specific benchmarks
fn context_environment_benchmarks(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Running Context Environment-Specific Benchmarks...");

    // Focus on environment generation and management
    context_benchmark.benchmark_environment_generation(c);
    context_benchmark.benchmark_shell_execution(c);
}

/// Shell execution benchmarks
fn context_shell_benchmarks(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Running Context Shell Execution Benchmarks...");

    // Focus on shell execution performance
    context_benchmark.benchmark_shell_execution(c);
    context_benchmark.benchmark_execution_performance(c);
}

/// Context caching and fingerprinting benchmarks
fn context_caching_benchmarks(c: &mut Criterion) {
    let context_benchmark = ContextBenchmark::new();

    println!("Running Context Caching Benchmarks...");

    // Focus on caching mechanisms
    context_benchmark.benchmark_context_caching(c);
}

// Define criterion groups for different types of benchmarks
criterion_group!(
    name = context_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = run_context_benchmarks
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
    name = context_performance;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(std::time::Duration::from_secs(20))
        .warm_up_time(std::time::Duration::from_secs(5));
    targets = run_context_performance_tests
);

criterion_group!(
    name = context_resources;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = run_context_resource_tests
);

criterion_group!(
    name = context_serialization;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(8))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = run_context_serialization_tests
);

criterion_group!(
    name = quick_context;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(3))
        .warm_up_time(std::time::Duration::from_secs(1));
    targets = quick_context_benchmark
);

criterion_group!(
    name = context_baseline;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = establish_context_baseline
);

criterion_group!(
    name = context_regression;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = context_regression_test
);

criterion_group!(
    name = context_validation;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(std::time::Duration::from_secs(20))
        .warm_up_time(std::time::Duration::from_secs(5));
    targets = validate_context_performance
);

criterion_group!(
    name = context_environment;
    config = Criterion::default()
        .sample_size(150)
        .measurement_time(std::time::Duration::from_secs(12))
        .warm_up_time(std::time::Duration::from_secs(3));
    targets = context_environment_benchmarks
);

criterion_group!(
    name = context_shell;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(15))
        .warm_up_time(std::time::Duration::from_secs(4));
    targets = context_shell_benchmarks
);

criterion_group!(
    name = context_caching;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(std::time::Duration::from_secs(8))
        .warm_up_time(std::time::Duration::from_secs(2));
    targets = context_caching_benchmarks
);

// Main entry point - runs all context benchmarks by default
criterion_main!(
    context_benchmarks,
    context_performance,
    context_environment,
    context_shell,
    context_resources
);

// Alternative entry points for specific benchmark types:
// To run only quick tests: cargo bench --bench context_benchmark_main quick_context
// To run comprehensive suite: cargo bench --bench context_benchmark_main comprehensive_benchmarks
// To run performance validation: cargo bench --bench context_benchmark_main context_validation
// To establish baseline: cargo bench --bench context_benchmark_main context_baseline
// To run regression tests: cargo bench --bench context_benchmark_main context_regression
// To run serialization tests: cargo bench --bench context_benchmark_main context_serialization
// To run caching tests: cargo bench --bench context_benchmark_main context_caching
