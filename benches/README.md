# Benchmark Suite

Benchmarks for rez-next using [Criterion.rs](https://docs.rs/criterion/).

## Enabled

| File | Description |
|------|-------------|
| `version_benchmark.rs` | Version parsing, comparison, sorting, state-machine parser |
| `package_benchmark.rs` | Package creation, serialization, deserialization, validation, variants |
| `simple_package_benchmark.rs` | Standalone package benchmarks |
| `solver_real_repo_bench.rs` | Solver resolution on real filesystem repos; A* vs greedy comparison |
| `solver_bench_v2.rs` | Solver v2 micro-benchmarks: single/multi-package, conflict detection |
| `rex_benchmark.rs` | Rex command interpreter execution throughput |
| `pip_conversion_benchmark.rs` | pip→rez requirement conversion throughput |
| `context_operations_benchmark.rs` | Context creation, env-var generation, PATH prepend |
| `depends_benchmark.rs` | Dependency graph traversal and ranking |
| `cache_operations_benchmark.rs` | Cache put/get/eviction, hot-path pkg lookup (7 scenarios) |
| `rez_vs_reznext_benchmark.rs` | **Cross-language comparison**: mirrors rez Python baseline operations (version/range/req parse ×1000, rex execute, shell script generate, package.py parse, startup) |

## Recent results (Windows, release profile)

```
version_parsing                  ~9.1 us
version_comparison               ~6.8 ns
version_sorting/10               ~1.96 us
version_sorting/100              ~19.4 us
version_sorting/1000             ~176 us
version_creation_scale/1000      ~9.0 ms
state_machine_token_parsing      ~535 ns

package_creation/simple          ~35 ns
package_creation/with_version    ~8.4 us
package_creation/complex         ~8.9 us
package_serialization/yaml       ~7.0 us
```

## rez vs rez-next Comparison (Windows, debug profile)

> Baseline: rez 2.112.0, Python 3.9, Linux Azure Xeon E5-2673 v4 (`metrics/benchmarking/data/rez_baseline.json`)
>
> rez-next numbers measured on Windows with `CRITERION_QUICK=1` (debug profile).

> Release profile figures are typically 3–10× faster than debug.

| Operation | rez Python (baseline) | rez-next Rust (debug) | Speedup (debug) |
|-----------|----------------------|----------------------|-----------------|
| version_parse ×1000 | 12.0 ms | 9.3 ms | **~1.3×** |
| version_range_parse ×1000 | 18.0 ms | 9.4 ms | **~1.9×** |
| req_parse ×1000 | 25.0 ms | 22.9 µs | **~1090×** |
| rex_execute (10 cmds) | 5.0 ms | 836 µs | **~6×** |
| shell_script_generate | 15.0 ms | 13.0 µs | **~1150×** |
| package.py parse (50 lines) | 8.0 ms | ~29 µs | **~276×** |
| startup (`import rez`) | 450.0 ms | 30 ns | **~15000000×** |

> **Note**: `req_parse` and `shell_generate` speedups are especially large because
> rez-next's `Requirement::new` is a lightweight struct constructor (no regex parsing),
> and `RexParser::parse` avoids the Python interpreter overhead.
> The startup comparison is not directly equivalent — rez-next's figure only measures
> `RepositoryManager + DependencyResolver` construction, not a full module import chain.


## Writing benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn my_bench(c: &mut Criterion) {
    c.bench_function("op", |b| {
        b.iter(|| black_box(my_fn(black_box(input))));
    });
}

criterion_group!(benches, my_bench);
criterion_main!(benches);
```

Register in `Cargo.toml`:

```toml
[[bench]]
name = "my_bench"
harness = false
```

## Tips

- Use `black_box()` to prevent dead-code elimination
- Vary input sizes to understand scaling
- Run on a quiet machine for stable results
