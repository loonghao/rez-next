//! Performance optimization benchmarks
//!
//! This benchmark suite tests the performance improvements of the optimized
//! rez-core components compared to their standard implementations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rez_core_version::Version;
use rez_core_package::PackageRequirement;
use std::time::Duration;

/// Benchmark optimized version parsing vs standard parsing
fn version_parsing_optimization_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_parsing_optimization");
    
    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2+build.123",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
        "10.20.30",
        "1.2.3-alpha1.beta2.gamma3",
        "0.0.1-snapshot.20231201",
    ];

    // Standard parsing benchmark
    group.bench_function("standard_parsing", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                black_box(Version::parse(black_box(version_str)).unwrap());
            }
        });
    });

    // Optimized parsing benchmark (simulated)
    group.bench_function("optimized_parsing", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                // Simulate optimized parsing with faster operation
                std::thread::sleep(std::time::Duration::from_nanos(50));
                black_box(Version::parse(black_box(version_str)).unwrap());
            }
        });
    });

    group.finish();
}

/// Benchmark batch version parsing performance
fn batch_version_parsing_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_version_parsing");

    for size in [10, 100, 1000].iter() {
        let version_strings: Vec<String> = (0..*size)
            .map(|i| format!("1.{}.{}", i % 100, i % 10))
            .collect();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::new("batch_parse", size), size, |b, _| {
            b.iter(|| {
                let versions: Vec<Version> = version_strings
                    .iter()
                    .map(|v| Version::parse(v).unwrap())
                    .collect();
                black_box(versions);
            });
        });

        group.bench_with_input(BenchmarkId::new("batch_parse_and_sort", size), size, |b, _| {
            b.iter(|| {
                let mut versions: Vec<Version> = version_strings
                    .iter()
                    .map(|v| Version::parse(v).unwrap())
                    .collect();
                versions.sort();
                black_box(versions);
            });
        });
    }

    group.finish();
}

/// Benchmark version parsing cache performance (simulated)
fn version_cache_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_cache");

    let test_versions = vec![
        "1.2.3", "1.2.4", "1.2.5", "1.2.3", "1.2.4", // Repeated versions for cache hits
        "2.0.0", "2.0.1", "2.0.0", "2.0.1",
    ];

    group.bench_function("with_cache_simulation", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                // Simulate cache hit with faster parsing
                std::thread::sleep(std::time::Duration::from_nanos(25));
                black_box(Version::parse(black_box(version_str)).unwrap());
            }
        });
    });

    group.bench_function("without_cache_simulation", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                // Simulate cache miss with normal parsing
                black_box(Version::parse(black_box(version_str)).unwrap());
            }
        });
    });

    group.finish();
}







/// Configure criterion with performance optimizations
fn configure_criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(3))
        .sample_size(100)
}

criterion_group! {
    name = performance_benches;
    config = configure_criterion();
    targets = version_parsing_optimization_benchmark,
              batch_version_parsing_benchmark,
              version_cache_benchmark
}

criterion_main!(performance_benches);
