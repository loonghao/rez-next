//! Unified Benchmark Test
//!
//! This is a simplified test to verify the comprehensive benchmark framework works.
//! This benchmark is completely independent and doesn't rely on any rez-core modules.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;
use std::collections::HashMap;

/// Simple benchmark to test the framework
fn simple_benchmark(c: &mut Criterion) {
    c.bench_function("simple_test", |b| {
        b.iter(|| {
            // Simple computation to benchmark
            let result = (0..1000).sum::<i32>();
            black_box(result);
        });
    });
}

/// Version parsing simulation benchmark
fn version_parsing_simulation(c: &mut Criterion) {
    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
    ];

    c.bench_function("version_parsing_simulation", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                // Simulate version parsing work
                std::thread::sleep(Duration::from_nanos(100));
                let parts: Vec<&str> = version_str.split('.').collect();
                black_box(parts);
            }
        });
    });
}

/// Package processing simulation benchmark
fn package_processing_simulation(c: &mut Criterion) {
    let test_packages = vec![
        "package_a",
        "package_b", 
        "package_c",
        "package_d",
        "package_e",
    ];

    c.bench_function("package_processing_simulation", |b| {
        b.iter(|| {
            for package_name in &test_packages {
                // Simulate package processing work
                std::thread::sleep(Duration::from_nanos(50));
                let processed = format!("processed_{}", package_name);
                black_box(processed);
            }
        });
    });
}

criterion_group!(
    unified_benches,
    simple_benchmark,
    version_parsing_simulation,
    package_processing_simulation
);

criterion_main!(unified_benches);
