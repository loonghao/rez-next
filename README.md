# rez-next

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![CI](https://img.shields.io/github/actions/workflow/status/loonghao/rez-next/ci.yml?branch=main)](https://github.com/loonghao/rez-next/actions)

An experimental Rust rewrite of core components from the [Rez](https://github.com/AcademySoftwareFoundation/rez) package manager.

[English](README.md) | [中文](README_zh.md)

---

## Warning

**This is a personal experiment. It is not production-ready and is not intended for general use.**

Most features are incomplete, APIs are unstable, and there is no guarantee of correctness or compatibility with the official Rez. If you need a package manager, use [Rez](https://github.com/AcademySoftwareFoundation/rez).

---

## What is this

A learning project that explores rewriting Rez's performance-critical subsystems in Rust — version parsing, package representation, dependency solving, context management, and the Rex command language.

The goal is not to replace Rez. It is to understand what a native implementation of these subsystems looks like, and whether meaningful performance gains are achievable for specific hot paths.

---

## Benchmark results

Measured with [Criterion.rs](https://github.com/bheisler/criterion.rs) in release mode (opt-level=3, LTO). These are Rust-internal micro-benchmarks only — they do not represent comparisons with Python Rez.

### Version operations

| Operation | Time |
|-----------|------|
| Parse single version (`1.2.3-alpha.1`) | ~9.1 us |
| State-machine tokenizer (5 versions) | ~535 ns |
| Compare two versions | ~6.8 ns |
| Sort 100 versions | ~19 us |
| Sort 1000 versions | ~176 us |
| Batch parse 1000 versions | ~9.0 ms |

### Package operations

| Operation | Time |
|-----------|------|
| Create empty package | ~35 ns |
| Create package with version | ~8.4 us |
| Create complex package (deps + tools + variants) | ~8.9 us |
| Serialize to YAML | ~7.0 us |
| Serialize to JSON | ~3.4 us |

<details>
<summary>Reproduce</summary>

```bash
vx cargo bench --bench version_benchmark
vx cargo bench --bench simple_package_benchmark
```

</details>

---

## Architecture

11 crates in a Cargo workspace:

```
rez-next-common       Shared error types, config, utilities
rez-next-version      Version parsing, comparison, ranges
rez-next-package      Package definition, package.py parsing (via RustPython AST)
rez-next-solver       Dependency resolution
rez-next-repository   Repository scanning and caching
rez-next-context      Resolved context and environment management
rez-next-build        Build system integration
rez-next-cache        Multi-level caching
rez-next-rex          Rex command language
rez-next-suites       Suite management (collections of resolved contexts)
rez-next-python       Python bindings via PyO3 (scaffolding only)
```

### Component status

| Crate | Status | Notes |
|-------|--------|-------|
| `rez-next-version` | Functional | Parsing, comparison, ranges, state-machine parser |
| `rez-next-package` | Functional | package.py parsing, serialization (YAML/JSON/Python) |
| `rez-next-common` | Functional | Error types, config |
| `rez-next-rex` | Partial | Command structures, shell generators, executor |
| `rez-next-solver` | Partial | Basic resolution, backtracking, cycle detection |
| `rez-next-context` | Partial | Context creation, environment generation, activation scripts |
| `rez-next-repository` | Partial | Scanning scaffolded |
| `rez-next-build` | Partial | Build system detection |
| `rez-next-cache` | Partial | Cache framework |
| `rez-next-suites` | Partial | Suite management basics |
| `rez-next-python` | Scaffolding | PyO3 bindings exist but are not usable |

---

## Building from source

```bash
git clone https://github.com/loonghao/rez-next
cd rez-next
cargo build --release
```

### Prerequisites

- Rust 1.70+
- [just](https://github.com/casey/just) (optional, for convenience commands)
- [vx](https://github.com/loonghao/vx) (optional, environment manager)

### Common commands

```bash
vx just build           # Dev build
vx just build-release   # Release build
vx just test            # Run all tests
vx just lint            # Clippy
vx just fmt             # Format
vx just ci              # Full CI check
vx just bench           # Benchmarks
```

---

## Tests

All tests pass:

```bash
vx just test
```

---

## Documentation

- [Contributing](docs/contributing.md) — development workflow and CI
- [Benchmark Guide](docs/benchmark_guide.md) — running and interpreting benchmarks
- [Performance Guide](docs/performance.md) — profiling tools
- [Python Integration](docs/python-integration.md) — planned Python bindings (not implemented)
- [Pre-commit Setup](docs/PRE_COMMIT_SETUP.md) — code quality hooks

---

## License

[Apache License 2.0](LICENSE)

## Acknowledgments

- [Rez](https://github.com/AcademySoftwareFoundation/rez) — the package manager this project studies
- [Rust](https://www.rust-lang.org/) — language and ecosystem
