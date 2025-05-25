//! Version system benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rez_core::version::Version;

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

criterion_group!(benches, version_parsing_benchmark, version_comparison_benchmark);
criterion_main!(benches);
