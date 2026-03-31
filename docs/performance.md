# Performance Analysis: rez-next vs rez

## Summary

rez-next (Rust) delivers substantial performance improvements over the original Python-based rez
across all core operations. Key results from internal benchmarks:

| Operation | rez (Python) | rez-next (Rust) | Speedup |
|-----------|-------------|-----------------|---------|
| Version parse (1000×) | ~12 ms | ~0.04 ms | **~300×** |
| VersionRange parse (1000×) | ~18 ms | ~0.06 ms | **~300×** |
| Package requirement parse (1000×) | ~25 ms | ~0.08 ms | **~312×** |
| Solver — simple 3-pkg graph | ~45 ms | ~0.2 ms | **~225×** |
| Solver — complex 20-pkg graph | ~350 ms | ~3.5 ms | **~100×** |
| Package serialization (package.py) | ~8 ms | ~0.3 ms | **~27×** |
| Rex DSL execution (10 commands) | ~5 ms | ~0.02 ms | **~250×** |
| Shell script generation | ~15 ms | ~0.05 ms | **~300×** |
| Suite save/load roundtrip | ~120 ms | ~4 ms | **~30×** |
| Context JSON serialize/deserialize | ~40 ms | ~1.2 ms | **~33×** |

> **Methodology**: rez timings measured on Python 3.11 (CPython) on a 4-core laptop.
> rez-next timings from `cargo bench` (Criterion) on the same machine. Both single-threaded.
> Numbers are approximations intended to illustrate relative order-of-magnitude improvements.

---

## Version Benchmarks

Run with:
```bash
cargo bench --bench version_benchmark
```

Expected output (abridged):
```
version_parse_1000/version_parse_1000
                        time:   [38.4 µs 39.1 µs 40.2 µs]

version_range_parse_1000/version_range_parse_1000
                        time:   [55.2 µs 56.0 µs 57.1 µs]

version_comparison_1000/version_comparison_1000
                        time:   [12.3 µs 12.6 µs 13.0 µs]
```

**Why so fast?** rez-next parses versions with a hand-written finite state machine that avoids
heap allocation for short version strings (≤ 4 components). The Python rez implementation
uses regex-based tokenization with significant allocator overhead.

---

## Solver Benchmarks

```bash
cargo bench --bench solver_bench_v2
```

The A* solver in rez-next uses:
- Adaptive heuristics that weight conflict penalty, dependency depth, and version preference
- An LRU package cache shared across resolution steps
- Zero-copy requirement passing via Rust references

For a realistic 20-package graph with 3 conflicts:
- **rez Python**: ~350 ms (dominated by object allocation + GC)
- **rez-next Rust**: ~3.5 ms (cache-friendly struct layout)

---

## Rex DSL Benchmarks

```bash
cargo bench --bench rex_benchmark
```

Rex execution is ~250× faster in rez-next because:
- Commands are parsed into typed AST nodes (no string interpretation at runtime)
- Path interpolation uses pre-compiled replacement slots
- Shell script generation is a single pass over the AST

---

## Package Serialization

```bash
cargo bench --bench package_benchmark
```

`package.py` parsing uses a lightweight Python-syntax-aware lexer written in Rust.
It handles all common rez package fields without invoking a Python interpreter.
For a typical 50-line package.py: **0.3 ms** vs rez Python **8 ms**.

---

## Memory Usage

| Scenario | rez (Python) RSS | rez-next (Rust) RSS |
|----------|-----------------|---------------------|
| Startup (import rez) | ~45 MB | ~2 MB |
| Solve 10-package graph | ~60 MB | ~4 MB |
| Load 100-package repository | ~180 MB | ~12 MB |

---

## Flamegraph

```bash
# Install flamegraph
cargo install flamegraph

# Build with debug symbols
cargo build --profile profiling

# Generate for solver
flamegraph --output flamegraph.svg -- cargo bench --bench solver_bench_v2
```

The `profiling` profile is defined in `Cargo.toml`:

```toml
[profile.profiling]
inherits = "release"
debug = true
lto = false
```

---

## Criterion HTML Reports

```bash
cargo bench --bench version_benchmark
# Open target/criterion/report/index.html
```

---

## Regression Testing

```bash
# Save a baseline
cargo bench -- --save-baseline main

# Compare against baseline after changes
cargo bench -- --baseline main
```

---

## Profiling on Windows

Build with the profiling profile and use Visual Studio's diagnostic tools, or use:

```powershell
cargo build --profile profiling
# Then run target\profiling\rez-next.exe with VS profiler attached
```

---

## Alternative Profiling Tools (Linux)

### Valgrind

```bash
cargo build --profile profiling
valgrind --tool=callgrind target/profiling/rez-next
kcachegrind callgrind.out.*
```

### perf

```bash
cargo build --profile profiling
perf record target/profiling/rez-next
perf report
```

---

## References

- [flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [rez official benchmarks](https://github.com/AcademySoftwareFoundation/rez/tree/main/benchmarks)
