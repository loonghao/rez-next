//! Benchmarks for reverse dependency (rez depends) operations
//!
//! Measures the cost of scanning a synthetic package set to find
//! all packages that depend on a given target package name.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;
use rez_next_package::Package;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a package with given name and a list of requirement strings.
/// `Package.requires` is `Vec<String>`.
fn make_package(name: &str, requires: &[&str]) -> Package {
    let mut pkg = Package::new(name.to_string());
    pkg.requires = requires.iter().map(|s| s.to_string()).collect();
    pkg
}

/// Build a synthetic repository of `n` packages.
/// Every third package has a dependency on "python-3+" to simulate a realistic ratio.
fn build_package_set(n: usize) -> Vec<Package> {
    (0..n)
        .map(|i| {
            if i % 3 == 0 {
                make_package(&format!("pkg_{}", i), &["python-3+"])
            } else if i % 7 == 0 {
                make_package(&format!("pkg_{}", i), &["python-3+", "numpy-1+"])
            } else {
                make_package(&format!("pkg_{}", i), &[])
            }
        })
        .collect()
}

/// Count how many packages in `set` depend on `target` (by prefix match on requirement string).
fn count_dependents(set: &[Package], target: &str) -> usize {
    set.iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with(target)))
        .count()
}

// ─── Benchmarks ──────────────────────────────────────────────────────────────

/// Benchmark: reverse dependency scan at various repository sizes.
fn bench_depends_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("depends_reverse_scan");
    for &size in &[50usize, 200, 500, 1000, 5000] {
        let packages = build_package_set(size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &packages,
            |b, pkgs| {
                b.iter(|| count_dependents(black_box(pkgs), black_box("python")));
            },
        );
    }
    group.finish();
}

/// Benchmark: package construction cost for dependency graph building.
fn bench_package_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("depends_package_construction");
    for &size in &[10usize, 50, 100, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |b, &n| {
                b.iter(|| build_package_set(black_box(n)));
            },
        );
    }
    group.finish();
}

/// Benchmark: requirement string parsing (core of depends analysis).
fn bench_requirement_string_ops(c: &mut Criterion) {
    let req_strings = ["python-3+",
        "numpy-1.20+<2",
        "maya-2024",
        "houdini-20+",
        "nuke-13.2+<14"];
    c.bench_function("depends_requirement_string_match_batch", |b| {
        b.iter(|| {
            req_strings
                .iter()
                .filter(|s| black_box(s).starts_with("python"))
                .count()
        });
    });
}

/// Benchmark: multi-target depends query (find all packages depending on any of N targets).
fn bench_multi_target_depends(c: &mut Criterion) {
    let packages = build_package_set(1000);
    let targets = ["python", "numpy", "maya", "houdini"];
    c.bench_function("depends_multi_target_1000_pkgs", |b| {
        b.iter(|| {
            targets
                .iter()
                .map(|t| count_dependents(black_box(&packages), black_box(t)))
                .sum::<usize>()
        });
    });
}

/// Benchmark: building a name→dependents index over the entire repository.
fn bench_build_depends_index(c: &mut Criterion) {
    let mut group = c.benchmark_group("depends_build_index");
    for &size in &[100usize, 500, 2000] {
        let packages = build_package_set(size);
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &packages,
            |b, pkgs| {
                b.iter(|| {
                    // Build a map: "target_name" -> [dependent_pkg_name, ...]
                    // Requirement strings may be "python-3+" so we extract the
                    // alphabetic prefix as the package name.
                    let mut index: std::collections::HashMap<String, Vec<&str>> =
                        std::collections::HashMap::new();
                    for pkg in black_box(pkgs) {
                        for req in &pkg.requires {
                            // Extract package name prefix (take chars up to first non-alpha/digit/underscore)
                            let name_end = req
                                .find(['-', '+', '<', '>', '='])
                                .unwrap_or(req.len());
                            let pkg_name = &req[..name_end];
                            index
                                .entry(pkg_name.to_string())
                                .or_default()
                                .push(pkg.name.as_str());
                        }
                    }
                    index
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_depends_scan,
    bench_package_construction,
    bench_requirement_string_ops,
    bench_multi_target_depends,
    bench_build_depends_index,
);
criterion_main!(benches);

