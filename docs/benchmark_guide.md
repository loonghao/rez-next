# Benchmark Guide

## Overview

This guide covers the benchmark suite for rez-next.

## Enabled benchmarks

Three benchmarks are registered in `Cargo.toml`:

| File | What it measures |
|------|------------------|
| `version_benchmark.rs` | Version parsing, comparison, sorting, state-machine parser |
| `package_benchmark.rs` | Package creation, serialization, deserialization, validation, variants |
| `simple_package_benchmark.rs` | Standalone package benchmarks |

Several other benchmark files exist in `benches/` but are disabled due to API changes in dependent crates. They will be re-enabled as APIs stabilize.

## Running

```bash
# All enabled benchmarks
vx just bench

# Individual
vx cargo bench --bench version_benchmark
vx cargo bench --bench package_benchmark

# Filter
vx cargo bench --bench version_benchmark -- state_machine
```

## Interpreting results

Criterion outputs:

```
version_parsing         time:   [9.088 us  9.120 us  9.157 us]
                        change: [-0.44% +0.71% +1.68%] (p = 0.21 > 0.05)
                        No change in performance detected.
```

- `time:` 95% confidence interval [lower  mean  upper]
- `change:` relative to previous run
- `p-value < 0.05` = statistically significant

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

Register in root `Cargo.toml`:

```toml
[[bench]]
name = "my_benchmark"
harness = false
```

## Regression detection

```bash
vx cargo bench -- --save-baseline main
# after changes:
vx cargo bench -- --baseline main
```

## References

- [Criterion.rs](https://docs.rs/criterion/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
