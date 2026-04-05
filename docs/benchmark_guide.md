# Benchmark Guide

## Overview

This guide covers the Criterion benchmark targets registered in `rez-next`.

## Registered benchmarks

There are currently **10** benchmark targets registered in the root `Cargo.toml`.
The up-to-date file list is maintained in `benches/README.md`.

| Category | Bench targets |
|----------|---------------|
| Core version/package | `version_benchmark`, `package_benchmark`, `simple_package_benchmark` |
| Solver | `solver_real_repo_bench`, `solver_bench_v2` |
| Runtime and tooling | `rex_benchmark`, `pip_conversion_benchmark` |
| Context / dependency / cache | `context_operations_benchmark`, `depends_benchmark`, `cache_operations_benchmark` |

## Running

```bash
# All registered benchmarks
vx cargo bench

# Curated fast subset used in local development
vx just bench

# Individual targets
vx cargo bench --bench version_benchmark
vx cargo bench --bench solver_bench_v2

# Filter within a target
vx cargo bench --bench version_benchmark -- state_machine
```

## Interpreting results

Criterion outputs:

```
version_parsing         time:   [9.088 us  9.120 us  9.157 us]
                        change: [-0.44% +0.71% +1.68%] (p = 0.21 > 0.05)
                        No change in performance detected.
```

- `time:` 95% confidence interval `[lower  mean  upper]`
- `change:` relative to the previous run
- `p-value < 0.05` means the change is statistically significant

## Adding benchmarks

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn my_benchmark(c: &mut Criterion) {
    c.bench_function("my_op", |b| {
        b.iter(|| black_box(my_function(black_box(input))));
    });
}

criterion_group!(benches, my_benchmark);
criterion_main!(benches);
```

Register the target in the root `Cargo.toml`:

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

## Regression detection

```bash
# Save a baseline
vx cargo bench -- --save-baseline main

# Compare after changes
vx cargo bench -- --baseline main
```

## References

- [Criterion.rs](https://docs.rs/criterion/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
