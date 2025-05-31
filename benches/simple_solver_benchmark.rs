//! Simple Solver Benchmark
//!
//! A minimal benchmark for testing the solver system functionality
//! without complex dependencies. This focuses on core solver operations.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use rez_core_solver::{DependencySolver, SolverConfig, SolverRequest, ConflictStrategy};
use rez_core_package::PackageRequirement;
use rez_core_version::VersionRange;
use std::time::Duration;

/// Test basic solver functionality
fn bench_solver_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_basic");
    
    // Test solver creation
    group.bench_function("create_solver", |b| {
        b.iter(|| {
            black_box(DependencySolver::new())
        });
    });
    
    // Test solver with custom config
    group.bench_function("create_solver_with_config", |b| {
        b.iter(|| {
            let config = SolverConfig {
                enable_parallel: true,
                enable_caching: true,
                max_workers: 4,
                ..Default::default()
            };
            black_box(DependencySolver::with_config(config))
        });
    });
    
    group.finish();
}

/// Test solver resolution with empty requests
fn bench_solver_empty_resolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_empty_resolution");
    
    let solver = DependencySolver::new();
    
    group.bench_function("resolve_empty", |b| {
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

/// Test package requirement creation
fn bench_package_requirements(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_requirements");
    
    // Simple requirement
    group.bench_function("simple_requirement", |b| {
        b.iter(|| {
            black_box(PackageRequirement::new(
                "python".to_string(),
                Some(VersionRange::new("3.9+".to_string()).unwrap())
            ))
        });
    });
    
    // Requirement without version
    group.bench_function("requirement_no_version", |b| {
        b.iter(|| {
            black_box(PackageRequirement::new(
                "python".to_string(),
                None
            ))
        });
    });
    
    group.finish();
}

/// Test solver with single requirement
fn bench_solver_single_requirement(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_single_requirement");
    
    let solver = DependencySolver::new();
    
    group.bench_function("resolve_single_python", |b| {
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
    
    group.finish();
}

/// Test different conflict strategies
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

/// Test solver configuration variations
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
        ("optimized", SolverConfig {
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

/// Test solver statistics
fn bench_solver_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("solver_stats");
    
    let solver = DependencySolver::new();
    
    group.bench_function("get_stats", |b| {
        b.iter(|| {
            black_box(solver.stats())
        });
    });
    
    group.finish();
}

/// Test multiple requirements
fn bench_multiple_requirements(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_requirements");
    
    let solver = DependencySolver::new();
    
    // Test with 2 requirements
    group.bench_function("two_requirements", |b| {
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
            ];
            let request = SolverRequest {
                requirements,
                config: SolverConfig::default(),
            };
            black_box(solver.resolve(black_box(request)))
        });
    });
    
    // Test with 5 requirements
    group.bench_function("five_requirements", |b| {
        b.iter(|| {
            let requirements = vec![
                PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap())),
                PackageRequirement::new("numpy".to_string(), Some(VersionRange::new("1.20+".to_string()).unwrap())),
                PackageRequirement::new("pandas".to_string(), None),
                PackageRequirement::new("scipy".to_string(), Some(VersionRange::new("1.7+".to_string()).unwrap())),
                PackageRequirement::new("matplotlib".to_string(), None),
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

/// Performance comparison benchmark
fn bench_performance_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_comparison");
    
    // Basic solver
    group.bench_function("basic_solver", |b| {
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
    
    // Optimized solver
    group.bench_function("optimized_solver", |b| {
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
    name = solver_basic_tests;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_solver_basic, bench_solver_empty_resolution, bench_package_requirements
);

criterion_group!(
    name = solver_functionality_tests;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_solver_single_requirement, bench_conflict_strategies, bench_solver_configs
);

criterion_group!(
    name = solver_advanced_tests;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_multiple_requirements, bench_performance_comparison, bench_solver_stats
);

// Main entry point
criterion_main!(solver_basic_tests, solver_functionality_tests, solver_advanced_tests);
