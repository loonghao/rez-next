//! Version system benchmarks

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_core::version::Version;
use std::hint::black_box;

fn version_parsing_benchmark(c: &mut Criterion) {
    c.bench_function("version_parsing", |b| {
        b.iter(|| {
            let v = Version::parse(black_box("1.2.3-alpha.1")).unwrap();
            black_box(v);
        })
    });
}

fn version_comparison_benchmark(c: &mut Criterion) {
    let v1 = Version::parse("1.2.3").unwrap();
    let v2 = Version::parse("1.2.4").unwrap();

    c.bench_function("version_comparison", |b| {
        b.iter(|| {
            black_box(v1.cmp(black_box(&v2)));
        })
    });
}

fn version_sorting_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_sorting");

    for size in [10, 100, 1000].iter() {
        let versions: Vec<Version> = (0..*size)
            .map(|i| Version::parse(&format!("1.{}.0", i)).unwrap())
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let mut sorted_versions = versions.clone();
                sorted_versions.sort();
                black_box(sorted_versions);
            });
        });
    }
    group.finish();
}

fn version_creation_scale_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_creation_scale");

    for size in [10, 100, 1000].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let version_strings: Vec<String> = (0..size)
                .map(|i| format!("1.{}.{}", i % 100, i % 10))
                .collect();

            b.iter(|| {
                for version_str in &version_strings {
                    black_box(Version::parse(version_str).unwrap());
                }
            });
        });
    }
    group.finish();
}

fn optimized_vs_legacy_parsing_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_vs_legacy_parsing");

    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
        "10.20.30",
        "1.2.3-alpha1.beta2.gamma3",
    ];

    group.bench_function("standard_parsing", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                black_box(Version::parse(black_box(version_str)).unwrap());
            }
        });
    });

    group.finish();
}

fn state_machine_parser_benchmark(c: &mut Criterion) {
    use rez_core::version::parser::StateMachineParser;

    let parser = StateMachineParser::new();
    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
    ];

    c.bench_function("state_machine_token_parsing", |b| {
        b.iter(|| {
            for version_str in &test_versions {
                black_box(parser.parse_tokens(black_box(version_str)).unwrap());
            }
        });
    });
}

fn configure_criterion() -> Criterion {
    // In CI (CRITERION_QUICK=1) use shorter warm-up and measurement times
    // so the full bench suite finishes within the 15-minute job timeout.
    let ci_quick = std::env::var("CRITERION_QUICK").is_ok();
    let warm_up = if ci_quick {
        std::time::Duration::from_millis(500)
    } else {
        std::time::Duration::from_secs(3)
    };
    let measurement = if ci_quick {
        std::time::Duration::from_secs(2)
    } else {
        std::time::Duration::from_secs(5)
    };
    let sample_size: usize = if ci_quick { 30 } else { 100 };

    Criterion::default()
        .warm_up_time(warm_up)
        .measurement_time(measurement)
        .sample_size(sample_size)
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = version_parsing_benchmark,
              version_comparison_benchmark,
              version_sorting_benchmark,
              version_creation_scale_benchmark,
              optimized_vs_legacy_parsing_benchmark,
              state_machine_parser_benchmark
}
criterion_main!(benches);
