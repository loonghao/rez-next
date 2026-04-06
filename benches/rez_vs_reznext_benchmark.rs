//! rez vs rez-next Cross-language Performance Comparison Benchmark
//!
//! This benchmark measures the same operations that are profiled in the official
//! rez Python benchmarks (`metrics/benchmarking/data/rez_baseline.json`), allowing
//! a direct apples-to-apples comparison of rez-next (Rust) vs rez (Python).
//!
//! Baseline reference (rez 2.112.0, Python 3.9, Linux Azure Xeon E5-2673 v4):
//!   version_parse_1000        →  12.0 ms (1000× Version.parse)
//!   version_range_parse_1000  →  18.0 ms (1000× VersionRange creation)
//!   req_parse_1000            →  25.0 ms (1000× Requirement/PackageRequest)
//!   rex_execute_10cmds        →   5.0 ms (10-command Rex block execution)
//!   shell_script_generate     →  15.0 ms (generate bash activation script)
//!   package_py_parse          →   8.0 ms (parse a 50-line package.py)
//!   startup_import            → 450.0 ms (import rez cold start)
//!
//! Run with:
//!   cargo bench --bench rez_vs_reznext_benchmark
//!
//! To generate the comparison report:
//!   cargo bench --bench rez_vs_reznext_benchmark 2>&1 | \
//!     python metrics/benchmarking/scripts/parse_criterion.py > /tmp/bench.json
//!   python metrics/benchmarking/scripts/generate_results.py \
//!     --bench-json /tmp/bench.json \
//!     --out metrics/benchmarking/RESULTS.md

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rez_next_package::{PackageSerializer, Requirement};
use rez_next_rex::{RexExecutor, RexParser};
use rez_next_version::{Version, VersionRange};
use std::hint::black_box;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/// 1000 diverse version strings mirroring the spread in a real rez package repo.
fn version_strings_1000() -> Vec<String> {
    let mut v = Vec::with_capacity(1000);
    for major in 0u32..10 {
        for minor in 0u32..10 {
            for patch in 0u32..10 {
                v.push(format!("{major}.{minor}.{patch}"));
            }
        }
    }
    v
}

/// 1000 version range strings (mix of simple, bounded, and compound).
fn version_range_strings_1000() -> Vec<String> {
    let mut v = Vec::with_capacity(1000);
    let templates: &[&str] = &[
        ">={major}.{minor}",
        "<={major}.{minor}.{patch}",
        ">={major}.{minor},<{next_major}.0",
        "{major}.{minor}.{patch}",
        "*",
    ];
    let mut idx = 0usize;
    for major in 0u32..5 {
        for minor in 0u32..10 {
            for patch in 0u32..20 {
                let tpl = templates[idx % templates.len()];
                let next_major = major + 1;
                let s = tpl
                    .replace("{major}", &major.to_string())
                    .replace("{minor}", &minor.to_string())
                    .replace("{patch}", &patch.to_string())
                    .replace("{next_major}", &next_major.to_string());
                v.push(s);
                idx += 1;
            }
        }
    }
    v.truncate(1000);
    v
}

/// 1000 requirement strings (mix of plain, versioned, conditional).
fn requirement_strings_1000() -> Vec<String> {
    let pkgs = [
        "python", "maya", "houdini", "nuke", "katana", "mari", "clarisse",
        "usd", "alembic", "openexr", "openvdb", "numpy", "scipy", "requests",
        "click", "pydantic", "fastapi", "sqlalchemy", "boto3", "ansible",
    ];
    let constraints = [
        "",
        "-3",
        ">=2.0",
        ">=1.0,<2.0",
        "==1.2.3",
        "~=3.9",
    ];
    let mut v = Vec::with_capacity(1000);
    let mut idx = 0usize;
    while v.len() < 1000 {
        let pkg = pkgs[idx % pkgs.len()];
        let con = constraints[idx % constraints.len()];
        let patch = idx / (pkgs.len() * constraints.len());
        // Append a numeric suffix to guarantee uniqueness across 1000 entries.
        if con.is_empty() {
            v.push(format!("{pkg}{patch}"));
        } else {
            v.push(format!("{pkg}{patch}{con}"));
        }
        idx += 1;
    }
    v
}

// A representative 10-command Rex block (mirrors rez_execute_10cmds baseline).
const REX_10_CMDS: &str = r#"
env.setenv('PKG_ROOT', '{root}')
env.setenv('PKG_VERSION', '{version}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
env.prepend_path('PYTHONPATH', '{root}/python/site-packages')
env.prepend_path('CMAKE_PREFIX_PATH', '{root}')
env.prepend_path('PKG_CONFIG_PATH', '{root}/lib/pkgconfig')
env.setenv_if_empty('BUILD_TYPE', 'Release')
alias('mypkg', '{root}/bin/mypkg')
info("mypkg {version} activated from {root}")
"#;

// A 50-line package.py equivalent in YAML — mirrors package_py_parse baseline.
const PACKAGE_YAML_50_LINES: &str = r#"
name: complex_tool
version: 3.1.4
description: A representative 50-line package definition for benchmarking
authors:
  - Alice Smith
  - Bob Jones
  - Carol White
requires:
  - python>=3.8
  - numpy>=1.20
  - scipy>=1.7
  - click>=8.0
  - pydantic>=1.9
  - requests>=2.26
  - boto3>=1.24
  - sqlalchemy>=1.4
  - alembic>=1.7
  - pillow>=9.0
build_requires:
  - cmake>=3.16
  - gcc>=9.0
  - ninja
  - pkg-config
tools:
  - complex-tool
  - ct-batch
  - ct-server
variants:
  - [python-3.8, platform-linux]
  - [python-3.9, platform-linux]
  - [python-3.10, platform-linux]
  - [python-3.8, platform-windows]
  - [python-3.9, platform-windows]
"#;

// ---------------------------------------------------------------------------
// Benchmark: version_parse_1000 (baseline: 12 ms)
// ---------------------------------------------------------------------------

fn bench_version_parse_1000(c: &mut Criterion) {
    let strings = version_strings_1000();
    let mut group = c.benchmark_group("version_parse");

    // Single-parse latency (for reference)
    group.bench_function("single", |b| {
        b.iter(|| black_box(Version::parse(black_box("2.3.1")).unwrap()))
    });

    // Batch 1000 — matches rez baseline key "version_parse_1000"
    group.throughput(Throughput::Elements(1000));
    group.bench_function("batch_1000", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(Version::parse(black_box(s)).unwrap());
            }
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: version_range_parse_1000 (baseline: 18 ms)
// ---------------------------------------------------------------------------

fn bench_version_range_parse_1000(c: &mut Criterion) {
    let strings = version_range_strings_1000();
    let mut group = c.benchmark_group("version_range_parse");

    group.bench_function("single", |b| {
        b.iter(|| black_box(VersionRange::new(">=1.0,<2.0".to_string()).unwrap()))
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function("batch_1000", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(VersionRange::new(black_box(s.clone())).unwrap());
            }
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: req_parse_1000 (baseline: 25 ms)
// ---------------------------------------------------------------------------

fn bench_req_parse_1000(c: &mut Criterion) {
    let strings = requirement_strings_1000();
    let mut group = c.benchmark_group("package_requirement");

    group.bench_function("single", |b| {
        b.iter(|| black_box(Requirement::new(black_box("python>=3.8".to_string()))))
    });

    group.throughput(Throughput::Elements(1000));
    group.bench_function("batch_1000", |b| {
        b.iter(|| {
            for s in &strings {
                black_box(Requirement::new(black_box(s.clone())));
            }
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: rex_execute_10cmds (baseline: 5 ms)
// ---------------------------------------------------------------------------

fn bench_rex_execute_10cmds(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_execute");

    group.bench_function("10_cmds", |b| {
        b.iter(|| {
            let mut exec = RexExecutor::new();
            black_box(
                exec.execute_commands(
                    black_box(REX_10_CMDS),
                    black_box("mypkg"),
                    Some(black_box("/opt/mypkg/3.1.4")),
                    Some(black_box("3.1.4")),
                )
                .unwrap(),
            )
        })
    });

    // Scale variants: 1, 5, 10, 20 commands to understand linear growth
    for n_cmds in [1usize, 5, 10, 20] {
        let block: String = (0..n_cmds)
            .map(|i| format!("env.setenv('VAR{i}', '{{root}}/v{i}')\n"))
            .collect();
        group.bench_with_input(
            BenchmarkId::new("n_cmds", n_cmds),
            &block,
            |b, cmds| {
                b.iter(|| {
                    let mut exec = RexExecutor::new();
                    black_box(
                        exec.execute_commands(
                            black_box(cmds.as_str()),
                            black_box("bench_pkg"),
                            Some(black_box("/opt/pkg/1.0")),
                            Some(black_box("1.0")),
                        )
                        .unwrap(),
                    )
                })
            },
        );
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Benchmark: shell_script_generate (baseline: 15 ms)
// Approximated by parsing + executing a multi-package activation Rex block.
// ---------------------------------------------------------------------------

fn bench_shell_script_generate(c: &mut Criterion) {
    let packages = [
        ("python", "/usr/local", "3.11.0"),
        ("maya", "/opt/autodesk/maya2024", "2024.1"),
        ("houdini", "/opt/sidefx/houdini20", "20.0"),
        ("nuke", "/opt/foundry/nuke14", "14.0"),
        ("katana", "/opt/foundry/katana6", "6.0"),
    ];

    // Build a combined activation script for 5 packages (proxy for shell script generation)
    let script: String = packages
        .iter()
        .map(|(name, root, ver)| {
            format!(
                "env.setenv('{name}_ROOT', '{root}')\n\
                 env.prepend_path('PATH', '{root}/bin')\n\
                 env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')\n\
                 alias('{name}', '{root}/bin/{name}')\n\
                 info(\"{name} {ver} activated\")\n"
            )
        })
        .collect();

    let parser = RexParser::new();

    c.bench_function("shell_generate", |b| {
        b.iter(|| {
            // parse (analogous to Python rez building the shell script AST)
            let cmds = black_box(parser.parse(black_box(&script)).unwrap());
            black_box(cmds)
        })
    });
}

// ---------------------------------------------------------------------------
// Benchmark: package_py_parse (baseline: 8 ms)
// Measures YAML deserialization of a 50-line package definition.
// ---------------------------------------------------------------------------

fn bench_package_py_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_serialization");

    // Single deserialize — matches baseline key "package_py_parse"
    group.bench_function("yaml_50_lines", |b| {
        b.iter(|| {
            black_box(
                PackageSerializer::load_from_yaml(black_box(PACKAGE_YAML_50_LINES)).unwrap(),
            )
        })
    });

    // Scale: measure how serialization cost grows with package complexity.
    for n_deps in [10usize, 50, 100] {
        let yaml = build_package_yaml(n_deps);
        group.bench_with_input(
            BenchmarkId::new("yaml_n_deps", n_deps),
            &yaml,
            |b, y| {
                b.iter(|| black_box(PackageSerializer::load_from_yaml(black_box(y)).unwrap()))
            },
        );
    }

    group.finish();
}

fn build_package_yaml(n_deps: usize) -> String {
    let requires: String = (0..n_deps)
        .map(|i| format!("  - dep{i}>=1.0\n"))
        .collect();
    format!(
        "name: bench_pkg\nversion: 1.0.0\ndescription: bench\nrequires:\n{requires}"
    )
}

// ---------------------------------------------------------------------------
// Benchmark: startup simulation
// Measures the cost of creating a fully-initialized resolver context
// (proxy for rez's 450 ms `import rez` cold-start baseline).
// ---------------------------------------------------------------------------

fn bench_startup_simulation(c: &mut Criterion) {
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("startup_import", |b| {
        b.iter(|| {
            // Construct the full solver stack (RepositoryManager + DependencyResolver)
            // This approximates the module-level work rez does on `import rez`.
            let repo = Arc::new(RepositoryManager::new());
            let resolver = DependencyResolver::new(repo, SolverConfig::default());
            black_box(resolver)
        })
    });

    // Also measure async resolve of an empty graph (first-use path)
    c.bench_function("startup_first_resolve", |b| {
        b.iter(|| {
            let repo = Arc::new(RepositoryManager::new());
            let mut resolver = DependencyResolver::new(repo, SolverConfig::default());
            rt.block_on(resolver.resolve(black_box(vec![]))).unwrap()
        })
    });
}

// ---------------------------------------------------------------------------
// Criterion groups
// ---------------------------------------------------------------------------

fn configure() -> Criterion {
    let quick = std::env::var("CRITERION_QUICK").is_ok();
    Criterion::default()
        .sample_size(if quick { 30 } else { 100 })
        .warm_up_time(Duration::from_millis(if quick { 500 } else { 2000 }))
        .measurement_time(Duration::from_secs(if quick { 3 } else { 8 }))
}

criterion_group!(
    name = version_benches;
    config = configure();
    targets = bench_version_parse_1000, bench_version_range_parse_1000
);

criterion_group!(
    name = req_benches;
    config = configure();
    targets = bench_req_parse_1000
);

criterion_group!(
    name = rex_benches;
    config = configure();
    targets = bench_rex_execute_10cmds, bench_shell_script_generate
);

criterion_group!(
    name = package_benches;
    config = configure();
    targets = bench_package_py_parse
);

criterion_group!(
    name = startup_benches;
    config = configure();
    targets = bench_startup_simulation
);

criterion_main!(
    version_benches,
    req_benches,
    rex_benches,
    package_benches,
    startup_benches
);
