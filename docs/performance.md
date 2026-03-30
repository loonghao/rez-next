# Performance Analysis Guide

How to profile and optimize rez-next.

## Flamegraph

```bash
# Install
cargo install flamegraph

# Build with debug symbols
vx cargo build --profile profiling

# Generate
flamegraph --output flamegraph.svg -- cargo bench --bench version_benchmark
```

The `profiling` profile is defined in `Cargo.toml`:

```toml
[profile.profiling]
inherits = "release"
debug = true
lto = false
```

### Reading flamegraphs

- Width = time spent (wider = hotter)
- Height = call depth
- Interactive: click to zoom, hover for detail

## Criterion HTML reports

```bash
vx cargo bench --bench version_benchmark
# Open target/criterion/report/index.html
```

## Alternative tools

### Valgrind (Linux)

```bash
vx cargo build --profile profiling
valgrind --tool=callgrind target/profiling/rez-next
kcachegrind callgrind.out.*
```

### perf (Linux)

```bash
vx cargo build --profile profiling
perf record target/profiling/rez-next
perf report
```

### Visual Studio (Windows)

Build with the profiling profile and use VS diagnostics tools.

## Regression testing

```bash
vx cargo bench -- --save-baseline main
vx cargo bench -- --baseline main
```

## References

- [flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [Criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
