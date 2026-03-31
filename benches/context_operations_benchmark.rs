//! Context Operations Benchmarks
//!
//! Measures performance of key ResolvedContext operations:
//! - context creation from requirements
//! - environment variable injection (bulk)
//! - JSON serialization / deserialization round-trip
//! - summary generation
//!
//! These benchmarks establish a baseline for future rez vs rez-next
//! performance comparisons once the Python layer is wired up.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_next_context::{ContextFormat, ContextSerializer, ContextStatus, ResolvedContext};
use rez_next_package::{Package, PackageRequirement};
use rez_next_version::Version;
use std::time::Duration;

// ── Helper builders ────────────────────────────────────────────────────────────

fn make_context_n_pkgs(n: usize) -> ResolvedContext {
    let reqs: Vec<PackageRequirement> = (0..n)
        .map(|i| PackageRequirement::parse(&format!("pkg{}-1.0", i)).unwrap())
        .collect();
    let mut ctx = ResolvedContext::from_requirements(reqs);

    for i in 0..n {
        let mut pkg = Package::new(format!("pkg{}", i));
        pkg.version = Some(Version::parse("1.0").unwrap());
        ctx.resolved_packages.push(pkg);
    }

    ctx.status = ContextStatus::Resolved;
    ctx
}

fn inject_env_vars(ctx: &mut ResolvedContext, n: usize) {
    for i in 0..n {
        ctx.environment_vars
            .insert(format!("PKG{}_ROOT", i), format!("/opt/pkg{}/1.0", i));
        ctx.environment_vars
            .insert(format!("PKG{}_VERSION", i), "1.0".to_string());
    }
}

// ── Benchmarks ─────────────────────────────────────────────────────────────────

/// Benchmark: context creation with varying numbers of requirements
fn bench_context_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_creation");

    for n in [1usize, 5, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::new("n_pkgs", n), &n, |b, &n| {
            b.iter(|| black_box(make_context_n_pkgs(n)))
        });
    }

    group.finish();
}

/// Benchmark: environment variable bulk injection
fn bench_env_var_injection(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_env_injection");

    for n in [5usize, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::new("n_vars", n * 2), &n, |b, &n| {
            b.iter(|| {
                let mut ctx = make_context_n_pkgs(n);
                inject_env_vars(&mut ctx, n);
                black_box(ctx)
            })
        });
    }

    group.finish();
}

/// Benchmark: JSON serialization (serialize to bytes)
fn bench_json_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_json_serialize");

    for n in [5usize, 10, 20] {
        let mut ctx = make_context_n_pkgs(n);
        inject_env_vars(&mut ctx, n);

        group.bench_with_input(BenchmarkId::new("n_pkgs", n), &ctx, |b, ctx| {
            b.iter(|| {
                black_box(
                    ContextSerializer::serialize(black_box(ctx), ContextFormat::Json).unwrap(),
                )
            })
        });
    }

    group.finish();
}

/// Benchmark: JSON deserialization (deserialize from bytes)
fn bench_json_deserialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_json_deserialize");

    for n in [5usize, 10, 20] {
        let mut ctx = make_context_n_pkgs(n);
        inject_env_vars(&mut ctx, n);
        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();

        group.bench_with_input(BenchmarkId::new("n_pkgs", n), &bytes, |b, bytes| {
            b.iter(|| {
                black_box(
                    ContextSerializer::deserialize(black_box(bytes), ContextFormat::Json).unwrap(),
                )
            })
        });
    }

    group.finish();
}

/// Benchmark: full JSON round-trip (serialize then deserialize)
fn bench_json_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_json_roundtrip");

    for n in [5usize, 10, 20] {
        let mut ctx = make_context_n_pkgs(n);
        inject_env_vars(&mut ctx, n);

        group.bench_with_input(BenchmarkId::new("n_pkgs", n), &ctx, |b, ctx| {
            b.iter(|| {
                let bytes =
                    ContextSerializer::serialize(black_box(ctx), ContextFormat::Json).unwrap();
                black_box(ContextSerializer::deserialize(&bytes, ContextFormat::Json).unwrap())
            })
        });
    }

    group.finish();
}

/// Benchmark: get_summary on a context
fn bench_context_summary(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_get_summary");

    for n in [5usize, 10, 20] {
        let mut ctx = make_context_n_pkgs(n);
        inject_env_vars(&mut ctx, n);

        group.bench_with_input(BenchmarkId::new("n_pkgs", n), &ctx, |b, ctx| {
            b.iter(|| black_box(ctx.get_summary()))
        });
    }

    group.finish();
}

fn ci_criterion(sample: usize, measure_s: u64, warmup_ms: u64) -> Criterion {
    let ci = std::env::var("CRITERION_QUICK").is_ok();
    Criterion::default()
        .sample_size(if ci { 20 } else { sample })
        .measurement_time(Duration::from_secs(if ci { 2 } else { measure_s }))
        .warm_up_time(Duration::from_millis(if ci { 300 } else { warmup_ms }))
}

// ── Groups ─────────────────────────────────────────────────────────────────────

criterion_group!(
    name = context_creation_benches;
    config = ci_criterion(200, 5, 1000);
    targets = bench_context_creation, bench_env_var_injection
);

criterion_group!(
    name = context_serialization_benches;
    config = ci_criterion(100, 8, 2000);
    targets = bench_json_serialize, bench_json_deserialize, bench_json_roundtrip
);

criterion_group!(
    name = context_ops_benches;
    config = ci_criterion(200, 5, 1000);
    targets = bench_context_summary
);

criterion_main!(
    context_creation_benches,
    context_serialization_benches,
    context_ops_benches
);
