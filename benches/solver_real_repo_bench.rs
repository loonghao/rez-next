//! Solver Real Repository Benchmark
//!
//! Performance benchmarks using actual filesystem-based package repositories.
//! Measures:
//! - Repository scan time
//! - Solver resolution time for common DCC pipeline scenarios
//! - Requirement parsing throughput
//! - Comparison of A* vs greedy resolution strategies

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_next_package::Requirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_repository::PackageRepository;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn make_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

/// Create a package.py file at `<repo_dir>/<name>/<version>/package.py`
fn create_package(repo_dir: &PathBuf, name: &str, version: &str, requires: &[&str]) {
    let pkg_dir = repo_dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();

    let requires_block = if requires.is_empty() {
        String::new()
    } else {
        let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
        format!("requires = [\n{}\n]\n", items.join("\n"))
    };

    fs::write(
        pkg_dir.join("package.py"),
        format!(
            "name = '{}'\nversion = '{}'\n{}",
            name, version, requires_block
        ),
    )
    .unwrap();
}

/// Build a small but realistic DCC pipeline repository
fn build_dcc_repo() -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Python versions
    create_package(&repo_dir, "python", "3.9.0", &[]);
    create_package(&repo_dir, "python", "3.10.0", &[]);
    create_package(&repo_dir, "python", "3.11.0", &[]);

    // Core libs
    create_package(&repo_dir, "numpy", "1.21.0", &["python-3.7+"]);
    create_package(&repo_dir, "numpy", "1.24.0", &["python-3.8+"]);
    create_package(&repo_dir, "numpy", "1.25.2", &["python-3.9+"]);
    create_package(&repo_dir, "scipy", "1.9.0", &["python-3.8+", "numpy-1.18+"]);
    create_package(&repo_dir, "scipy", "1.11.0", &["python-3.9+", "numpy-1.20+"]);

    // UI frameworks
    create_package(&repo_dir, "pyside2", "5.15.0", &["python-3+<4"]);
    create_package(&repo_dir, "pyside6", "6.5.0", &["python-3.9+"]);

    // DCC apps
    create_package(&repo_dir, "maya", "2023.0", &["python-3.9+<3.11", "pyside2-5+"]);
    create_package(&repo_dir, "maya", "2024.0", &["python-3.10+<3.12", "pyside2-5+"]);
    create_package(&repo_dir, "houdini", "19.5.0", &["python-3.9+<3.11"]);
    create_package(&repo_dir, "houdini", "20.0.547", &["python-3.10+<3.12"]);
    create_package(&repo_dir, "nuke", "14.0.0", &["python-3.9+<3.11", "pyside2-5+"]);
    create_package(&repo_dir, "nuke", "15.0.0", &["python-3.10+<3.12", "pyside2-5+"]);

    // Pipeline tools
    create_package(&repo_dir, "rez", "3.0.0", &["python-3.7+"]);
    create_package(&repo_dir, "pipeline_core", "1.0.0", &["python-3.9+", "numpy-1.20+"]);
    create_package(&repo_dir, "pipeline_core", "2.0.0", &["python-3.10+", "numpy-1.24+", "scipy-1.9+"]);

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "bench_repo".to_string(),
    )));

    (tmp, Arc::new(mgr))
}

/// Build a large repository with many packages for stress testing
fn build_large_repo(n_packages: usize, n_versions: usize) -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Base: always have python
    for minor in 7..13u32 {
        create_package(&repo_dir, "python", &format!("3.{}", minor), &[]);
    }

    // Generate n_packages * n_versions packages
    for i in 0..n_packages {
        let pkg_name = format!("package_{:03}", i);
        for v in 0..n_versions {
            let version = format!("1.{}.0", v);
            let requires = if i > 0 {
                let dep = format!("package_{:03}-1.0+", i - 1);
                vec![dep]
            } else {
                vec![]
            };
            let requires_ref: Vec<&str> = requires.iter().map(|s| s.as_str()).collect();
            create_package(&repo_dir, &pkg_name, &version, &requires_ref);
        }
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "large_repo".to_string(),
    )));

    (tmp, Arc::new(mgr))
}

// ─── Benchmarks ───────────────────────────────────────────────────────────────

/// Benchmark: repository scan speed
fn bench_repo_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("repo_scan");
    group.measurement_time(Duration::from_secs(5));

    let rt = make_rt();
    let (_tmp, repo) = build_dcc_repo();

    group.bench_function("scan_python", |b| {
        b.iter(|| {
            let r = Arc::clone(&repo);
            rt.block_on(r.find_packages(black_box("python"))).unwrap()
        })
    });

    group.bench_function("scan_maya", |b| {
        b.iter(|| {
            let r = Arc::clone(&repo);
            rt.block_on(r.find_packages(black_box("maya"))).unwrap()
        })
    });

    group.finish();
}

/// Benchmark: resolve single package from DCC repo
fn bench_resolve_single_dcc(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_single_dcc");
    group.measurement_time(Duration::from_secs(5));

    let rt = make_rt();
    let (_tmp, repo) = build_dcc_repo();
    let config = SolverConfig::default();

    let packages = ["python", "maya", "houdini", "numpy", "scipy"];

    for pkg in &packages {
        group.bench_with_input(BenchmarkId::new("resolve", pkg), pkg, |b, &p| {
            b.iter(|| {
                let req: Requirement = p.parse().unwrap();
                let mut resolver = DependencyResolver::new(Arc::clone(&repo), config.clone());
                rt.block_on(resolver.resolve(vec![black_box(req)])).unwrap()
            })
        });
    }

    group.finish();
}

/// Benchmark: resolve with transitive dependencies
fn bench_resolve_transitive_dcc(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_transitive_dcc");
    group.measurement_time(Duration::from_secs(5));

    let rt = make_rt();
    let (_tmp, repo) = build_dcc_repo();
    let config = SolverConfig::default();

    // Scenarios with varying depth of transitive dependencies
    let scenarios: &[(&str, &[&str])] = &[
        ("python_only", &["python"]),
        ("maya_full", &["maya"]),          // maya → python + pyside2
        ("pipeline", &["pipeline_core"]),  // pipeline_core → python + numpy + scipy
        ("dcc_suite", &["maya", "houdini", "numpy"]),
    ];

    for (name, reqs) in scenarios {
        group.bench_with_input(BenchmarkId::new("scenario", name), reqs, |b, reqs| {
            b.iter(|| {
                let requirements: Vec<Requirement> =
                    reqs.iter().map(|s| s.parse().unwrap()).collect();
                let mut resolver = DependencyResolver::new(Arc::clone(&repo), config.clone());
                rt.block_on(resolver.resolve(black_box(requirements))).unwrap()
            })
        });
    }

    group.finish();
}

/// Benchmark: requirement string parsing throughput
fn bench_requirement_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("requirement_parsing");

    let formats = [
        "python",
        "python-3",
        "python-3.9",
        "python-3.9+",
        "python-3.9+<4",
        "python-3.9+<3.12",
        "numpy-1.20+",
        "scipy-1.9+<2",
        "maya-2024",
        "houdini-20.0.547",
        "rez>=3.0",
        "rez-3.0.0",
    ];

    group.bench_function("parse_all_formats", |b| {
        b.iter(|| {
            for f in &formats {
                let _ = black_box(f.parse::<Requirement>().unwrap_or_else(|_| {
                    Requirement::new(f.to_string())
                }));
            }
        })
    });

    for fmt in &formats {
        group.bench_with_input(BenchmarkId::new("parse", fmt), fmt, |b, &f| {
            b.iter(|| {
                black_box(f.parse::<Requirement>().unwrap_or_else(|_| {
                    Requirement::new(f.to_string())
                }))
            })
        });
    }

    group.finish();
}

/// Benchmark: version constraint satisfaction check
fn bench_version_constraint_check(c: &mut Criterion) {
    use rez_next_package::requirement::VersionConstraint;
    use rez_next_version::Version;

    let mut group = c.benchmark_group("version_constraint");

    let version = Version::parse("3.11.0").unwrap();

    group.bench_function("GreaterThanOrEqual_shallow", |b| {
        let v = Version::parse("3").unwrap();
        let c = VersionConstraint::GreaterThanOrEqual(v);
        b.iter(|| c.is_satisfied_by(black_box(&version)))
    });

    group.bench_function("GreaterThanOrEqual_deep", |b| {
        let v = Version::parse("3.9.0").unwrap();
        let c = VersionConstraint::GreaterThanOrEqual(v);
        b.iter(|| c.is_satisfied_by(black_box(&version)))
    });

    group.bench_function("LessThan_shallow", |b| {
        let v = Version::parse("4").unwrap();
        let c = VersionConstraint::LessThan(v);
        b.iter(|| c.is_satisfied_by(black_box(&version)))
    });

    group.bench_function("Multiple_ge_lt", |b| {
        let min = Version::parse("3").unwrap();
        let max = Version::parse("4").unwrap();
        let c = VersionConstraint::Multiple(vec![
            VersionConstraint::GreaterThanOrEqual(min),
            VersionConstraint::LessThan(max),
        ]);
        b.iter(|| c.is_satisfied_by(black_box(&version)))
    });

    group.bench_function("Prefix", |b| {
        let v = Version::parse("3.11").unwrap();
        let c = VersionConstraint::Prefix(v);
        b.iter(|| c.is_satisfied_by(black_box(&version)))
    });

    group.finish();
}

/// Benchmark: large repo resolution
fn bench_resolve_large_repo(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolve_large_repo");
    group.measurement_time(Duration::from_secs(10));
    group.sample_size(10);

    let rt = make_rt();

    // Small repo (10 packages, 3 versions each)
    let (_tmp_s, repo_small) = build_large_repo(10, 3);
    group.bench_function("small_repo_linear_chain", |b| {
        let req: Requirement = "package_009".parse().unwrap();
        b.iter(|| {
            let mut resolver = DependencyResolver::new(
                Arc::clone(&repo_small),
                SolverConfig::default(),
            );
            rt.block_on(resolver.resolve(vec![black_box(req.clone())])).unwrap()
        })
    });

    // Medium repo (20 packages, 5 versions each)
    let (_tmp_m, repo_medium) = build_large_repo(20, 5);
    group.bench_function("medium_repo_linear_chain", |b| {
        let req: Requirement = "package_019".parse().unwrap();
        b.iter(|| {
            let mut resolver = DependencyResolver::new(
                Arc::clone(&repo_medium),
                SolverConfig::default(),
            );
            rt.block_on(resolver.resolve(vec![black_box(req.clone())])).unwrap()
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_repo_scan,
    bench_resolve_single_dcc,
    bench_resolve_transitive_dcc,
    bench_requirement_parsing,
    bench_version_constraint_check,
    bench_resolve_large_repo,
);
criterion_main!(benches);
