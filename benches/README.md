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

```bash
vx just bench
```

## Disabled

The following exist but are commented out in `Cargo.toml` due to API changes:

- `comprehensive_benchmark_suite.rs`
- `solver_benchmark.rs` / `solver_benchmark_main.rs`
- `context_benchmark.rs` / `context_benchmark_main.rs` / `simple_context_benchmark.rs`
- `build_cache_benchmark.rs` / `build_cache_benchmark_main.rs` / `simple_build_cache_benchmark.rs`
- `performance_validation_benchmark.rs` / `performance_validation_main.rs`

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
