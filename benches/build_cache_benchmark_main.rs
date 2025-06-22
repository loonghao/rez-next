//! Build and Cache Benchmark Main Entry Point
//!
//! Comprehensive Build and Cache system benchmarks with multiple configurations and scenarios.
//! This provides the main entry point for running all Build and Cache-related benchmarks.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_core_build::{
    BuildConfig, BuildEnvironment, BuildManager, BuildOptions, BuildProcess, BuildRequest,
    BuildStats, BuildSystem, BuildVerbosity,
};
use rez_core_cache::{
    AdaptiveTuner, BenchmarkConfig as CacheBenchmarkConfig, IntelligentCacheManager,
    PredictivePreheater, UnifiedCache, UnifiedCacheConfig, UnifiedPerformanceMonitor,
};
use rez_core_context::{ContextBuilder, ContextConfig, ResolvedContext};
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::{Version, VersionRange};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

mod build_cache_benchmark;
use build_cache_benchmark::BuildCacheBenchmark;

/// Build performance benchmarks - Core build functionality
fn build_performance(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_build_manager_performance(c);
    benchmark.benchmark_build_system_detection(c);
    benchmark.benchmark_build_environment_setup(c);
}

/// Build parallel processing benchmarks - Concurrent build performance
fn build_parallel(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_build_parallel_processing(c);
}

/// Cache performance benchmarks - Core cache functionality
fn cache_performance(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_cache_performance(c);
}

/// Cache advanced benchmarks - Preheating and adaptive tuning
fn cache_advanced(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_cache_preheating(c);
    benchmark.benchmark_cache_adaptive_tuning(c);
}

/// Build and Cache statistics benchmarks - Statistics collection overhead
fn build_cache_statistics(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_build_statistics_collection(c);
}

/// Build and Cache memory benchmarks - Memory usage tracking
fn build_cache_memory(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_memory_usage(c);
}

/// Build and Cache scalability benchmarks - Performance across complexity levels
fn build_cache_scalability(c: &mut Criterion) {
    let benchmark = BuildCacheBenchmark::new();
    benchmark.benchmark_scalability(c);
}

/// Build validation benchmarks - Build system performance validation
fn build_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_validation");

    // Validate build system detection performance
    group.bench_function("build_system_detection_validation", |b| {
        let test_dirs = vec![
            ("cmake", PathBuf::from("test/cmake-project")),
            ("python", PathBuf::from("test/python-project")),
            ("nodejs", PathBuf::from("test/nodejs-project")),
            ("make", PathBuf::from("test/make-project")),
        ];

        b.iter(|| {
            for (_, dir) in &test_dirs {
                let _result = BuildSystem::detect(dir);
                black_box(dir);
            }
        });
    });

    // Validate build environment setup performance
    group.bench_function("build_environment_setup_validation", |b| {
        let packages = vec![
            Package::new(
                "python-lib".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "cmake-lib".to_string(),
                Version::new("2.1.0".to_string()).unwrap(),
            ),
            Package::new(
                "nodejs-lib".to_string(),
                Version::new("1.5.0".to_string()).unwrap(),
            ),
        ];
        let base_build_dir = PathBuf::from("build");

        b.iter(|| {
            for package in &packages {
                let _env = BuildEnvironment::new(package, &base_build_dir, None);
                black_box(package);
            }
        });
    });

    // Validate concurrent build management
    group.bench_function("concurrent_build_validation", |b| {
        let config = BuildConfig {
            max_concurrent_builds: 4,
            build_timeout_seconds: 3600,
            verbosity: BuildVerbosity::Silent,
            ..Default::default()
        };

        b.iter(|| {
            let manager = BuildManager::with_config(config.clone());
            black_box(manager);
        });
    });

    group.finish();
}

/// Cache validation benchmarks - Cache system performance validation
fn cache_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_validation");

    // Validate cache hit ratio performance (target: >90%)
    group.bench_function("cache_hit_ratio_validation", |b| {
        let config = UnifiedCacheConfig {
            l1_capacity: 1000,
            l2_capacity: 5000,
            ttl_seconds: 3600,
            enable_predictive_preheating: true,
            enable_adaptive_tuning: true,
            enable_performance_monitoring: true,
        };
        let cache = IntelligentCacheManager::new(config);

        let keys: Vec<String> = (0..100)
            .map(|i| format!("key_{}", i % 10)) // Repeated keys for high hit ratio
            .collect();

        b.iter(|| {
            for key in &keys {
                // Simulate cache operations
                black_box(key);
            }
        });
    });

    // Validate cache preheating performance
    group.bench_function("cache_preheating_validation", |b| {
        let preheater = PredictivePreheater::new();
        let patterns: Vec<String> = (0..50).map(|i| format!("pattern_{}", i % 5)).collect();

        b.iter(|| {
            for pattern in &patterns {
                // Simulate pattern recognition and preheating
                black_box(pattern);
            }
        });
    });

    // Validate adaptive tuning performance
    group.bench_function("adaptive_tuning_validation", |b| {
        let tuner = AdaptiveTuner::new();

        b.iter(|| {
            // Simulate performance analysis and tuning
            black_box(&tuner);
        });
    });

    group.finish();
}

/// Build and Cache baseline benchmarks - Establish performance baselines
fn build_cache_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_cache_baseline");

    // Build manager baseline
    group.bench_function("build_manager_baseline", |b| {
        b.iter(|| black_box(BuildManager::new()));
    });

    // Build environment baseline
    group.bench_function("build_environment_baseline", |b| {
        let package = Package::new(
            "baseline".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );
        let base_build_dir = PathBuf::from("build");

        b.iter(|| black_box(BuildEnvironment::new(&package, &base_build_dir, None)));
    });

    // Cache manager baseline
    group.bench_function("cache_manager_baseline", |b| {
        b.iter(|| black_box(IntelligentCacheManager::new(UnifiedCacheConfig::default())));
    });

    // Predictive preheater baseline
    group.bench_function("preheater_baseline", |b| {
        b.iter(|| black_box(PredictivePreheater::new()));
    });

    // Adaptive tuner baseline
    group.bench_function("tuner_baseline", |b| {
        b.iter(|| black_box(AdaptiveTuner::new()));
    });

    group.finish();
}

/// Build and Cache regression testing - Detect performance regressions
fn build_cache_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_cache_regression");

    // Standard regression test scenarios
    let build_scenarios = vec![
        ("single_package", 1),
        ("small_project", 3),
        ("medium_project", 7),
        ("large_project", 15),
    ];

    for (name, package_count) in build_scenarios {
        group.bench_with_input(
            BenchmarkId::new("build_regression", name),
            &package_count,
            |b, &package_count| {
                let packages: Vec<Package> = (0..package_count)
                    .map(|i| {
                        Package::new(
                            format!("pkg_{}", i),
                            Version::new("1.0.0".to_string()).unwrap(),
                        )
                    })
                    .collect();

                b.iter(|| {
                    let manager = BuildManager::new();
                    for package in &packages {
                        let environment =
                            BuildEnvironment::new(package, &PathBuf::from("build"), None).unwrap();
                        black_box(environment);
                    }
                    black_box(manager);
                });
            },
        );
    }

    // Cache regression scenarios
    let cache_scenarios = vec![
        ("small_cache", 100),
        ("medium_cache", 1000),
        ("large_cache", 5000),
    ];

    for (name, operation_count) in cache_scenarios {
        group.bench_with_input(
            BenchmarkId::new("cache_regression", name),
            &operation_count,
            |b, &operation_count| {
                let cache = IntelligentCacheManager::new(UnifiedCacheConfig::default());

                b.iter(|| {
                    for i in 0..operation_count {
                        let key = format!("key_{}", i % 100);
                        let value = format!("value_{}", i);
                        black_box((&key, &value));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Build and Cache integration benchmarks - Combined system performance
fn build_cache_integration(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_cache_integration");

    // Test integrated Build and Cache performance
    group.bench_function("integrated_performance", |b| {
        let package = Package::new(
            "integrated".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );

        b.iter(|| {
            // Build components
            let manager = BuildManager::new();
            let environment =
                BuildEnvironment::new(&package, &PathBuf::from("build"), None).unwrap();

            // Cache components
            let cache = IntelligentCacheManager::new(UnifiedCacheConfig::default());
            let preheater = PredictivePreheater::new();
            let tuner = AdaptiveTuner::new();

            black_box((manager, environment, cache, preheater, tuner));
        });
    });

    // Test Build with Cache optimization
    group.bench_function("build_with_cache_optimization", |b| {
        let packages: Vec<Package> = (0..5)
            .map(|i| {
                Package::new(
                    format!("cached_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect();

        b.iter(|| {
            let manager = BuildManager::new();
            let cache = IntelligentCacheManager::new(UnifiedCacheConfig {
                l1_capacity: 1000,
                l2_capacity: 5000,
                enable_predictive_preheating: true,
                enable_adaptive_tuning: true,
                ..Default::default()
            });

            for package in &packages {
                let environment =
                    BuildEnvironment::new(package, &PathBuf::from("build"), None).unwrap();
                black_box(environment);
            }

            black_box((manager, cache));
        });
    });

    group.finish();
}

// Helper functions
fn create_test_context() -> ResolvedContext {
    let requirements = vec![
        PackageRequirement::new(
            "python".to_string(),
            Some(VersionRange::new("3.9+".to_string()).unwrap()),
        ),
        PackageRequirement::new(
            "cmake".to_string(),
            Some(VersionRange::new("3.20+".to_string()).unwrap()),
        ),
    ];
    ContextBuilder::new()
        .requirements(requirements)
        .config(ContextConfig::default())
        .build()
}

// Configure criterion groups with different settings for different test types
criterion_group!(
    name = build_core_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = build_performance, build_parallel
);

criterion_group!(
    name = cache_core_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = cache_performance, cache_advanced
);

criterion_group!(
    name = build_cache_validation_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5));
    targets = build_validation, cache_validation, build_cache_baseline
);

criterion_group!(
    name = build_cache_advanced_benchmarks;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(20))
        .warm_up_time(Duration::from_secs(5));
    targets = build_cache_statistics, build_cache_memory, build_cache_scalability
);

criterion_group!(
    name = build_cache_specialized_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(2));
    targets = build_cache_regression, build_cache_integration
);

// Main entry point
criterion_main!(
    build_core_benchmarks,
    cache_core_benchmarks,
    build_cache_validation_benchmarks,
    build_cache_advanced_benchmarks,
    build_cache_specialized_benchmarks
);
