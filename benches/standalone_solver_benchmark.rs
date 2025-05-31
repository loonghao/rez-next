//! Standalone Solver Benchmark
//!
//! A simplified standalone benchmark for testing the solver system
//! without complex dependencies. This is useful for quick testing
//! and development validation.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use rez_core_solver::{DependencySolver, SolverConfig, SolverRequest, ConflictStrategy};
use rez_core_package::PackageRequirement;
use rez_core_version::VersionRange;
use std::time::Duration;

/// Simple solver benchmark for basic functionality
fn bench_simple_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_solver");
    
    // Test basic solver creation and configuration
    group.bench_function("solver_creation", |b| {
        b.iter(|| {
            black_box(DependencySolver::new())
        });
    });
    
    // Test solver with default config
    group.bench_function("solver_with_config", |b| {
        b.iter(|| {
            let config = SolverConfig::default();
            black_box(DependencySolver::with_config(config))
        });
    });
    
    // Test basic resolution with empty requirements
    group.bench_function("empty_resolution", |b| {
        let solver = DependencySolver::new();
        b.iter(|| {
            let request = SolverRequest {
                requirements: vec![],
                config: SolverConfig::default(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    group.finish();
}

/// Benchmark solver configuration variations
fn bench_solver_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_configs");
    
    let configs = vec![
        ("default", SolverConfig::default()),
        ("parallel_enabled", SolverConfig {
            enable_parallel: true,
            max_workers: 4,
            ..Default::default()
        }),
        ("cache_enabled", SolverConfig {
            enable_caching: true,
            cache_ttl_seconds: 3600,
            ..Default::default()
        }),
        ("high_performance", SolverConfig {
            enable_parallel: true,
            enable_caching: true,
            max_workers: 8,
            cache_ttl_seconds: 7200,
            ..Default::default()
        }),
    ];
    
    for (name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("config", name),
            &config,
            |b, config| {
                b.iter(|| {
                    let solver = DependencySolver::with_config(config.clone());
                    let request = SolverRequest {
                        requirements: vec![],
                        config: config.clone(),
                    };
                    black_box(solver.resolve(black_box(request)))
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark conflict resolution strategies
fn bench_conflict_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("conflict_strategies");
    
    let strategies = vec![
        ConflictStrategy::LatestWins,
        ConflictStrategy::EarliestWins,
        ConflictStrategy::FailOnConflict,
        ConflictStrategy::FindCompatible,
    ];
    
    for strategy in strategies {
        group.bench_with_input(
            BenchmarkId::new("strategy", format!("{:?}", strategy)),
            &strategy,
            |b, strategy| {
                let config = SolverConfig {
                    conflict_strategy: strategy.clone(),
                    ..Default::default()
                };
                let solver = DependencySolver::with_config(config.clone());
                
                b.iter(|| {
                    let request = SolverRequest {
                        requirements: vec![],
                        config: config.clone(),
                    };
                    black_box(solver.resolve(black_box(request)))
                });
            },
        );
    }
    
    group.finish();
}

/// Benchmark solver statistics collection
fn bench_solver_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_stats");
    
    group.bench_function("stats_collection", |b| {
        let solver = DependencySolver::new();
        b.iter(|| {
            black_box(solver.stats())
        });
    });
    
    group.finish();
}

/// Benchmark package requirement creation
fn bench_package_requirements(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_requirements");
    
    // Test simple requirement creation
    group.bench_function("simple_requirement", |b| {
        b.iter(|| {
            black_box(PackageRequirement::new(
                "python".to_string(),
                Some(VersionRange::new("3.9+".to_string()).unwrap())
            ))
        });
    });
    
    // Test requirement without version
    group.bench_function("requirement_no_version", |b| {
        b.iter(|| {
            black_box(PackageRequirement::new(
                "python".to_string(),
                None
            ))
        });
    });
    
    // Test complex version range
    group.bench_function("complex_version_range", |b| {
        b.iter(|| {
            let version_range = VersionRange::new(">=3.8,<4.0".to_string()).unwrap();
            black_box(PackageRequirement::new(
                "python".to_string(),
                Some(version_range)
            ))
        });
    });
    
    group.finish();
}

/// Benchmark solver request creation and processing
fn bench_solver_requests(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_requests");
    
    // Test request with single requirement
    group.bench_function("single_requirement_request", |b| {
        let solver = DependencySolver::new();
        b.iter(|| {
            let requirement = PackageRequirement::new(
                "python".to_string(),
                Some(VersionRange::new("3.9+".to_string()).unwrap())
            );
            let request = SolverRequest {
                requirements: vec![requirement],
                config: SolverConfig::default(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    // Test request with multiple requirements
    group.bench_function("multiple_requirements_request", |b| {
        let solver = DependencySolver::new();
        b.iter(|| {
            let requirements = vec![
                PackageRequirement::new(
                    "python".to_string(),
                    Some(VersionRange::new("3.9+".to_string()).unwrap())
                ),
                PackageRequirement::new(
                    "numpy".to_string(),
                    Some(VersionRange::new("1.20+".to_string()).unwrap())
                ),
                PackageRequirement::new(
                    "pandas".to_string(),
                    None
                ),
            ];
            let request = SolverRequest {
                requirements,
                config: SolverConfig::default(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    group.finish();
}

/// Performance validation benchmark
fn bench_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_validation");
    
    // This benchmark helps validate that solver performance meets targets
    group.bench_function("baseline_performance", |b| {
        let solver = DependencySolver::new();
        let requirement = PackageRequirement::new(
            "test_package".to_string(),
            Some(VersionRange::new("1.0+".to_string()).unwrap())
        );
        
        b.iter(|| {
            let request = SolverRequest {
                requirements: vec![requirement.clone()],
                config: SolverConfig::default(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    // Test with optimized configuration
    group.bench_function("optimized_performance", |b| {
        let config = SolverConfig {
            enable_parallel: true,
            enable_caching: true,
            max_workers: 4,
            cache_ttl_seconds: 3600,
            ..Default::default()
        };
        let solver = DependencySolver::with_config(config.clone());
        let requirement = PackageRequirement::new(
            "test_package".to_string(),
            Some(VersionRange::new("1.0+".to_string()).unwrap())
        );
        
        b.iter(|| {
            let request = SolverRequest {
                requirements: vec![requirement.clone()],
                config: config.clone(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    group.finish();
}

// Configure criterion groups
criterion_group!(
    name = solver_basic;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_simple_solver, bench_solver_configs, bench_solver_stats
);

criterion_group!(
    name = solver_advanced;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_conflict_strategies, bench_package_requirements, bench_solver_requests
);

criterion_group!(
    name = solver_performance;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5));
    targets = bench_performance_validation
);

// Main entry point
criterion_main!(solver_basic, solver_advanced, solver_performance);
