//! Solver Benchmark v2
//!
//! Benchmarks using the current DependencyResolver API.
//! Tests empty-repo resolution performance and config variations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_next_package::Requirement;
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;
use std::time::Duration;

fn make_resolver(config: SolverConfig) -> DependencyResolver {
    let repo = Arc::new(RepositoryManager::new());
    DependencyResolver::new(repo, config)
}

fn make_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

/// Benchmark: resolver construction overhead
fn bench_resolver_creation(c: &mut Criterion) {
    c.bench_function("resolver_create_default", |b| {
        b.iter(|| black_box(make_resolver(SolverConfig::default())))
    });
}

/// Benchmark: resolve empty requirements (no-op resolution path)
fn bench_resolve_empty(c: &mut Criterion) {
    let rt = make_rt();

    c.bench_function("resolve_empty_requirements", |b| {
        b.iter(|| {
            let mut resolver = make_resolver(SolverConfig::default());
            rt.block_on(resolver.resolve(black_box(vec![]))).unwrap()
        })
    });
}

/// Benchmark: parse + resolve a single requirement string
fn bench_resolve_single_requirement(c: &mut Criterion) {
    let rt = make_rt();
    let mut group = c.benchmark_group("resolve_single_requirement");

    let req_strings = ["python", "python-3.9", "maya-2024", "houdini>=19.5"];

    for req_str in &req_strings {
        group.bench_with_input(BenchmarkId::new("req", req_str), req_str, |b, &s| {
            b.iter(|| {
                let req: Requirement = s
                    .parse()
                    .unwrap_or_else(|_| Requirement::new(s.to_string()));
                let mut resolver = make_resolver(SolverConfig::default());
                rt.block_on(resolver.resolve(vec![black_box(req)])).unwrap()
            })
        });
    }

    group.finish();
}

/// Benchmark: multi-requirement resolution (no packages in repo → immediate fail path)
fn bench_resolve_multiple_requirements(c: &mut Criterion) {
    let rt = make_rt();
    let mut group = c.benchmark_group("resolve_multi_requirements");

    for n_reqs in [2usize, 5, 10] {
        group.bench_with_input(BenchmarkId::new("n_reqs", n_reqs), &n_reqs, |b, &n| {
            let reqs: Vec<Requirement> = (0..n)
                .map(|i| Requirement::new(format!("pkg{}", i)))
                .collect();
            b.iter(|| {
                let mut resolver = make_resolver(SolverConfig::default());
                rt.block_on(resolver.resolve(black_box(reqs.clone())))
                    .unwrap()
            })
        });
    }

    group.finish();
}

/// Benchmark: SolverConfig variations
fn bench_solver_configs(c: &mut Criterion) {
    let rt = make_rt();
    let mut group = c.benchmark_group("solver_config_variants");

    let configs = vec![
        ("default", SolverConfig::default()),
        (
            "parallel_4",
            SolverConfig {
                enable_parallel: true,
                max_workers: 4,
                ..Default::default()
            },
        ),
        (
            "no_cache",
            SolverConfig {
                enable_caching: false,
                ..Default::default()
            },
        ),
        (
            "latest_false",
            SolverConfig {
                prefer_latest: false,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        group.bench_with_input(BenchmarkId::new("config", name), &(), |b, _| {
            b.iter(|| {
                let mut resolver = make_resolver(config.clone());
                rt.block_on(resolver.resolve(vec![])).unwrap()
            })
        });
    }

    group.finish();
}

fn ci_criterion(sample: usize, measure_s: u64) -> Criterion {
    let ci = std::env::var("CRITERION_QUICK").is_ok();
    Criterion::default()
        .sample_size(if ci { 20 } else { sample })
        .measurement_time(Duration::from_secs(if ci { 2 } else { measure_s }))
        .warm_up_time(Duration::from_millis(if ci { 300 } else { 2000 }))
}

criterion_group!(
    name = resolver_basic;
    config = ci_criterion(200, 5);
    targets = bench_resolver_creation, bench_resolve_empty
);

criterion_group!(
    name = resolver_resolution;
    config = ci_criterion(100, 8);
    targets = bench_resolve_single_requirement, bench_resolve_multiple_requirements
);

criterion_group!(
    name = resolver_configs;
    config = ci_criterion(100, 5);
    targets = bench_solver_configs
);

criterion_main!(resolver_basic, resolver_resolution, resolver_configs);
