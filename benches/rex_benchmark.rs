//! Rex Command Language Benchmarks
//!
//! Measures performance of the Rex parser and executor —
//! key components used whenever a package's `commands` block is processed.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_next_rex::{RexExecutor, RexParser};
use std::hint::black_box;
use std::time::Duration;

// ── Typical real-world commands blocks ────────────────────────────────────────

const MAYA_COMMANDS: &str = r#"
env.setenv('MAYA_VERSION', '{version}')
env.setenv('MAYA_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
alias('maya', '{root}/bin/maya')
alias('mayabatch', '{root}/bin/mayabatch')
"#;

const PYTHON_COMMANDS: &str = r#"
env.setenv('PYTHONHOME', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
env.setenv_if_empty('PYTHON_VERSION', '{version}')
alias('python3', '{root}/bin/python3')
"#;

const HOUDINI_COMMANDS: &str = r#"
env.setenv('HFS', '{root}')
env.setenv('HB', '{root}/bin')
env.setenv('HOUDINI_MAJOR_RELEASE', '20')
env.setenv('HOUDINI_MINOR_RELEASE', '0')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/dsolib')
env.prepend_path('PYTHONPATH', '{root}/houdini/python3.11libs')
alias('houdini', '{root}/bin/houdini')
alias('hython', '{root}/bin/hython')
alias('hbatch', '{root}/bin/hbatch')
info("Houdini {version} loaded from {root}")
"#;

const LARGE_PKG_COMMANDS: &str = r#"
env.setenv('PKG_ROOT', '{root}')
env.setenv('PKG_VERSION', '{version}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
env.prepend_path('PYTHONPATH', '{root}/python/site-packages')
env.prepend_path('CMAKE_PREFIX_PATH', '{root}')
env.prepend_path('PKG_CONFIG_PATH', '{root}/lib/pkgconfig')
env.setenv_if_empty('BUILD_TYPE', 'Release')
env.setenv_if_empty('COMPILER', 'gcc')
alias('pkg', '{root}/bin/pkg')
alias('pkg-config-wrapper', '{root}/bin/pkg-config-wrapper')
source('{root}/etc/env.sh')
info("pkg {version} activated")
command('{root}/bin/pkg-postactivate')
"#;

// ── Parser benchmarks ─────────────────────────────────────────────────────────

fn bench_parser_simple(c: &mut Criterion) {
    let parser = RexParser::new();
    c.bench_function("rex_parse_simple_setenv", |b| {
        b.iter(|| {
            black_box(
                parser
                    .parse(black_box(r#"env.setenv('MY_VAR', '/opt/pkg/1.0')"#))
                    .unwrap(),
            )
        })
    });
}

fn bench_parser_commands_blocks(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_parse_commands_block");
    let parser = RexParser::new();

    let cases = [
        ("maya", MAYA_COMMANDS),
        ("python", PYTHON_COMMANDS),
        ("houdini", HOUDINI_COMMANDS),
        ("large_pkg", LARGE_PKG_COMMANDS),
    ];

    for (name, commands) in &cases {
        group.bench_with_input(BenchmarkId::new("parse", name), commands, |b, cmds| {
            b.iter(|| black_box(parser.parse(black_box(cmds)).unwrap()))
        });
    }

    group.finish();
}

// ── Executor benchmarks ───────────────────────────────────────────────────────

fn bench_executor_execute(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_execute_commands_block");

    let cases = [
        ("maya", MAYA_COMMANDS, "/opt/maya/2024.1", "2024.1"),
        ("python", PYTHON_COMMANDS, "/usr/local", "3.11.0"),
        ("houdini", HOUDINI_COMMANDS, "/opt/houdini/20.0", "20.0"),
        ("large_pkg", LARGE_PKG_COMMANDS, "/opt/pkg/3.2.1", "3.2.1"),
    ];

    for (name, commands, root, version) in &cases {
        group.bench_with_input(BenchmarkId::new("execute", name), &(), |b, _| {
            b.iter(|| {
                let mut exec = RexExecutor::new();
                black_box(
                    exec.execute_commands(
                        black_box(commands),
                        black_box(name),
                        Some(black_box(root)),
                        Some(black_box(version)),
                    )
                    .unwrap(),
                )
            })
        });
    }

    group.finish();
}

fn bench_executor_multi_package(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_execute_multi_package");

    // Simulate a context with N packages applied sequentially
    for pkg_count in [2usize, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("n_packages", pkg_count),
            &pkg_count,
            |b, &n| {
                b.iter(|| {
                    let mut exec = RexExecutor::new();
                    for i in 0..n {
                        let pkg_name = format!("pkg{}", i);
                        let root = format!("/opt/pkg{}/1.0", i);
                        let commands = format!(
                            "env.setenv('PKG{0}_ROOT', '{{root}}')\n\
                             env.prepend_path('PATH', '{{root}}/bin')\n\
                             alias('pkg{0}', '{{root}}/bin/pkg{0}')",
                            i
                        );
                        exec.execute_commands(&commands, &pkg_name, Some(&root), Some("1.0"))
                            .unwrap();
                    }
                    black_box(())
                })
            },
        );
    }

    group.finish();
}

fn bench_parser_new(c: &mut Criterion) {
    c.bench_function("rex_parser_construction", |b| {
        b.iter(|| black_box(RexParser::new()))
    });
}

fn bench_parser_cached_access(c: &mut Criterion) {
    c.bench_function("rex_parser_cached_access", |b| {
        b.iter(|| black_box(rez_next_rex::parser::get_cached_parser()))
    });
}

// ── Groups ────────────────────────────────────────────────────────────────────

criterion_group!(
    name = rex_parser_benches;
    config = Criterion::default()
        .sample_size(200)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_parser_simple, bench_parser_commands_blocks, bench_parser_new, bench_parser_cached_access
);

criterion_group!(
    name = rex_executor_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_executor_execute, bench_executor_multi_package
);

criterion_main!(rex_parser_benches, rex_executor_benches);
