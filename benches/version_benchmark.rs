//! Version system benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_core::version::Version;

#[cfg(feature = "flamegraph")]
use pprof::criterion::{Output, PProfProfiler};

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

fn configure_criterion() -> Criterion {
    #[cfg(feature = "flamegraph")]
    {
        Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
    }
    #[cfg(not(feature = "flamegraph"))]
    {
        Criterion::default()
    }
}

criterion_group! {
    name = benches;
    config = configure_criterion();
    targets = version_parsing_benchmark,
              version_comparison_benchmark,
              version_sorting_benchmark,
              version_creation_scale_benchmark
}
criterion_main!(benches);
