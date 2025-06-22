//! Simple Build and Cache Benchmark
//!
//! A minimal benchmark for testing the Build and Cache system functionality
//! without complex dependencies. This focuses on core Build and Cache operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_core_build::{
    BuildConfig, BuildEnvironment, BuildManager, BuildOptions, BuildProcess, BuildRequest,
    BuildStats, BuildSystem, BuildVerbosity,
};
use rez_core_cache::{
    AdaptiveTuner, IntelligentCacheManager, PredictivePreheater, UnifiedCache, UnifiedCacheConfig,
    UnifiedPerformanceMonitor,
};
use rez_core_context::{ContextBuilder, ContextConfig, ResolvedContext};
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::{Version, VersionRange};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Test basic Build functionality
fn bench_build_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_basic");

    // Test Build manager creation
    group.bench_function("create_build_manager", |b| {
        b.iter(|| black_box(BuildManager::new()));
    });

    // Test Build environment creation
    group.bench_function("create_build_environment", |b| {
        let package = Package::new(
            "test".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );
        let base_build_dir = PathBuf::from("build");

        b.iter(|| black_box(BuildEnvironment::new(&package, &base_build_dir, None)));
    });

    // Test Build system detection
    group.bench_function("detect_build_system", |b| {
        let source_dir = PathBuf::from("test");

        b.iter(|| black_box(BuildSystem::detect(&source_dir)));
    });

    group.finish();
}

/// Test Build configuration performance
fn bench_build_configuration(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_configuration");

    // Test different build configurations
    let configs = vec![
        ("default", BuildConfig::default()),
        (
            "single_threaded",
            BuildConfig {
                max_concurrent_builds: 1,
                verbosity: BuildVerbosity::Silent,
                ..Default::default()
            },
        ),
        (
            "parallel",
            BuildConfig {
                max_concurrent_builds: 4,
                verbosity: BuildVerbosity::Normal,
                ..Default::default()
            },
        ),
        (
            "high_performance",
            BuildConfig {
                max_concurrent_builds: 8,
                verbosity: BuildVerbosity::Silent,
                clean_before_build: false,
                keep_artifacts: false,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("build_config", name),
            &config,
            |b, config| {
                b.iter(|| black_box(BuildManager::with_config(config.clone())));
            },
        );
    }

    group.finish();
}

/// Test Build request processing
fn bench_build_requests(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_requests");

    // Simple build request
    group.bench_function("simple_build_request", |b| {
        let package = Package::new(
            "simple-lib".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );
        let request = BuildRequest {
            package,
            context: None,
            source_dir: PathBuf::from("test/simple-lib"),
            variant: None,
            options: BuildOptions::default(),
        };

        b.iter(|| black_box(&request));
    });

    // Complex build request
    group.bench_function("complex_build_request", |b| {
        let package = Package::new(
            "complex-lib".to_string(),
            Version::new("2.1.0".to_string()).unwrap(),
        );
        let mut env_vars = HashMap::new();
        env_vars.insert("BUILD_TYPE".to_string(), "Release".to_string());
        env_vars.insert("PARALLEL_JOBS".to_string(), "4".to_string());

        let request = BuildRequest {
            package,
            context: Some(create_test_context()),
            source_dir: PathBuf::from("test/complex-lib"),
            variant: Some("optimized".to_string()),
            options: BuildOptions {
                force_rebuild: true,
                skip_tests: false,
                release_mode: true,
                build_args: vec!["--parallel".to_string(), "--optimize".to_string()],
                env_vars,
            },
        };

        b.iter(|| black_box(&request));
    });

    group.finish();
}

/// Test Build statistics
fn bench_build_statistics(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_statistics");

    // Test statistics creation
    group.bench_function("create_build_stats", |b| {
        b.iter(|| black_box(BuildStats::default()));
    });

    // Test statistics updates
    group.bench_function("update_build_stats", |b| {
        let mut stats = BuildStats::default();

        b.iter(|| {
            stats.builds_started += 1;
            stats.total_build_time_ms += 1000;
            stats.avg_build_time_ms =
                stats.total_build_time_ms as f64 / stats.builds_started as f64;
            black_box(&stats);
        });
    });

    group.finish();
}

/// Test basic Cache functionality
fn bench_cache_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_basic");

    // Test cache creation
    group.bench_function("create_intelligent_cache", |b| {
        b.iter(|| black_box(IntelligentCacheManager::new(UnifiedCacheConfig::default())));
    });

    // Test predictive preheater creation
    group.bench_function("create_predictive_preheater", |b| {
        b.iter(|| black_box(PredictivePreheater::new()));
    });

    // Test adaptive tuner creation
    group.bench_function("create_adaptive_tuner", |b| {
        b.iter(|| black_box(AdaptiveTuner::new()));
    });

    // Test performance monitor creation
    group.bench_function("create_performance_monitor", |b| {
        b.iter(|| black_box(UnifiedPerformanceMonitor::new()));
    });

    group.finish();
}

/// Test Cache configurations
fn bench_cache_configurations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_configurations");

    let configs = vec![
        (
            "small",
            UnifiedCacheConfig {
                l1_capacity: 100,
                l2_capacity: 500,
                ttl_seconds: 3600,
                enable_predictive_preheating: false,
                enable_adaptive_tuning: false,
                enable_performance_monitoring: false,
            },
        ),
        (
            "medium",
            UnifiedCacheConfig {
                l1_capacity: 1000,
                l2_capacity: 5000,
                ttl_seconds: 7200,
                enable_predictive_preheating: true,
                enable_adaptive_tuning: false,
                enable_performance_monitoring: true,
            },
        ),
        (
            "large",
            UnifiedCacheConfig {
                l1_capacity: 5000,
                l2_capacity: 20000,
                ttl_seconds: 1800,
                enable_predictive_preheating: true,
                enable_adaptive_tuning: true,
                enable_performance_monitoring: true,
            },
        ),
        (
            "optimized",
            UnifiedCacheConfig {
                l1_capacity: 10000,
                l2_capacity: 50000,
                ttl_seconds: 900,
                enable_predictive_preheating: true,
                enable_adaptive_tuning: true,
                enable_performance_monitoring: false, // Disable for pure performance
            },
        ),
    ];

    for (name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("cache_config", name),
            &config,
            |b, config| {
                b.iter(|| black_box(IntelligentCacheManager::new(config.clone())));
            },
        );
    }

    group.finish();
}

/// Test Cache operations
fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Simple cache operations
    group.bench_function("simple_cache_operations", |b| {
        let cache = IntelligentCacheManager::new(UnifiedCacheConfig::default());
        let keys: Vec<String> = (0..10).map(|i| format!("key_{}", i)).collect();
        let values: Vec<String> = (0..10).map(|i| format!("value_{}", i)).collect();

        b.iter(|| {
            for (key, value) in keys.iter().zip(values.iter()) {
                // Simulate cache operations
                black_box((key, value));
            }
        });
    });

    // Batch cache operations
    group.bench_function("batch_cache_operations", |b| {
        let cache = IntelligentCacheManager::new(UnifiedCacheConfig {
            l1_capacity: 1000,
            l2_capacity: 5000,
            ..Default::default()
        });
        let keys: Vec<String> = (0..100).map(|i| format!("key_{}", i)).collect();
        let values: Vec<String> = (0..100).map(|i| format!("value_{}", i)).collect();

        b.iter(|| {
            for (key, value) in keys.iter().zip(values.iter()) {
                black_box((key, value));
            }
        });
    });

    group.finish();
}

/// Test Build and Cache scalability
fn bench_build_cache_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_cache_scalability");

    // Test with different numbers of packages
    for package_count in &[1, 5, 10, 20] {
        let packages: Vec<Package> = (0..*package_count)
            .map(|i| {
                Package::new(
                    format!("package_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("build_scalability", package_count),
            &packages,
            |b, packages| {
                b.iter(|| {
                    let manager = BuildManager::new();
                    for package in packages {
                        let environment =
                            BuildEnvironment::new(package, &PathBuf::from("build"), None).unwrap();
                        black_box(environment);
                    }
                    black_box(manager);
                });
            },
        );
    }

    // Test cache scalability
    for operation_count in &[100, 500, 1000, 5000] {
        group.bench_with_input(
            BenchmarkId::new("cache_scalability", operation_count),
            operation_count,
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

/// Performance validation benchmark
fn bench_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_validation");

    // Build performance baseline
    group.bench_function("build_baseline_performance", |b| {
        let package = Package::new(
            "baseline".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );

        b.iter(|| {
            let manager = BuildManager::new();
            let environment =
                BuildEnvironment::new(&package, &PathBuf::from("build"), None).unwrap();
            black_box((manager, environment));
        });
    });

    // Cache performance baseline
    group.bench_function("cache_baseline_performance", |b| {
        b.iter(|| {
            let cache = IntelligentCacheManager::new(UnifiedCacheConfig::default());
            let preheater = PredictivePreheater::new();
            let tuner = AdaptiveTuner::new();
            black_box((cache, preheater, tuner));
        });
    });

    // Combined Build and Cache performance
    group.bench_function("combined_performance", |b| {
        let package = Package::new(
            "combined".to_string(),
            Version::new("1.0.0".to_string()).unwrap(),
        );

        b.iter(|| {
            // Build components
            let manager = BuildManager::new();
            let environment =
                BuildEnvironment::new(&package, &PathBuf::from("build"), None).unwrap();

            // Cache components
            let cache = IntelligentCacheManager::new(UnifiedCacheConfig::default());

            black_box((manager, environment, cache));
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

// Configure criterion groups
criterion_group!(
    name = build_basic_tests;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_build_basic, bench_build_configuration, bench_build_requests
);

criterion_group!(
    name = cache_basic_tests;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_cache_basic, bench_cache_configurations, bench_cache_operations
);

criterion_group!(
    name = build_cache_advanced_tests;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_build_statistics, bench_build_cache_scalability, bench_performance_validation
);

// Main entry point
criterion_main!(
    build_basic_tests,
    cache_basic_tests,
    build_cache_advanced_tests
);
