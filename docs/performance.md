# Performance Analysis Guide

This document describes how to analyze and optimize performance in rez-core using various profiling tools.

## 🔥 Flamegraph Profiling

We use [flamegraph](https://github.com/flamegraph-rs/flamegraph) for detailed performance analysis, following the approach used by pydantic-core.

### Prerequisites

#### Linux (Recommended)
```bash
# Install perf (required for flamegraph)
sudo apt-get install linux-perf  # Ubuntu/Debian
sudo yum install perf            # CentOS/RHEL

# Install flamegraph
cargo install flamegraph
```

#### macOS
```bash
# Install DTrace (usually pre-installed)
# Install flamegraph
cargo install flamegraph
```

#### Windows
Flamegraph has limited support on Windows. Consider using:
- WSL2 with Linux setup
- Docker with Linux container
- Alternative profiling tools (see below)

### Running Flamegraph

#### Quick Start
```bash
# Unix/Linux/macOS
make flamegraph

# Windows (limited support)
.\scripts\build.ps1 flamegraph
```

#### Manual Steps
```bash
# 1. Build with profiling symbols
make build-profiling

# 2. Run flamegraph on specific benchmarks
flamegraph --output flamegraph.svg -- cargo bench --features flamegraph

# 3. Open the generated SVG
# The flamegraph.svg file can be opened in any web browser
```

### Interpreting Flamegraphs

- **Width**: Time spent in function (wider = more time)
- **Height**: Call stack depth
- **Color**: Different functions (no semantic meaning)
- **Interactive**: Click to zoom, hover for details

Look for:
- Wide bars at the top (hot functions)
- Unexpected call patterns
- Memory allocation hotspots

## 📊 Benchmark Analysis

### Running Benchmarks

```bash
# Python benchmarks with pytest-benchmark
make benchmark

# Rust benchmarks with criterion
make bench-rust

# All benchmarks
make test && make benchmark && make bench-rust
```

### Benchmark Categories

#### Version Creation Performance
- Target: >1000 versions/ms
- Measures: Parsing overhead, memory allocation
- Key metrics: Throughput, latency distribution

#### Version Comparison Performance  
- Target: >100 sorts of 100 versions/ms
- Measures: Comparison algorithm efficiency
- Key metrics: O(n log n) scaling verification

#### Memory Usage
- Target: Reasonable memory per version object
- Measures: Memory leaks, allocation patterns
- Key metrics: Peak memory, allocation rate

### Performance Targets

| Operation | Target Performance | Current Status |
|-----------|-------------------|----------------|
| Version Creation | >1000/ms | ~1000/ms ✅ |
| Version Comparison | >100 sorts/ms | ~100/ms ✅ |
| Memory per Version | <100 bytes | TBD 🔍 |
| Range Operations | >500/ms | TBD 🚧 |

## 🔧 Alternative Profiling Tools

### Windows-Specific Tools

#### Visual Studio Diagnostics
```bash
# Build with debug info
cargo build --profile profiling

# Use Visual Studio's built-in profiler
# File -> Open -> Project/Solution -> target/profiling/rez-core.exe
```

#### Intel VTune (if available)
```bash
# Build optimized with debug info
cargo build --profile profiling

# Run with VTune
vtune -collect hotspots target/profiling/deps/rez_core-*.exe
```

### Cross-Platform Tools

#### Valgrind (Linux/macOS)
```bash
# Memory profiling
cargo build --profile profiling
valgrind --tool=callgrind target/profiling/deps/rez_core-*

# View with kcachegrind
kcachegrind callgrind.out.*
```

#### perf (Linux)
```bash
# CPU profiling
cargo build --profile profiling
perf record target/profiling/deps/rez_core-*
perf report
```

## 📈 Performance Regression Testing

### Automated Benchmarks

We run benchmarks in CI to catch performance regressions:

```bash
# Run baseline benchmarks
cargo bench -- --save-baseline main

# After changes, compare
cargo bench -- --baseline main
```

### Performance Monitoring

Key metrics to monitor:
- Version creation throughput
- Memory usage growth
- Comparison operation scaling
- Python binding overhead

## 🎯 Optimization Strategies

### Hot Path Optimization
1. **Identify hot paths** using flamegraph
2. **Minimize allocations** in critical loops
3. **Use SIMD** for bulk operations where applicable
4. **Cache frequently used data**

### Memory Optimization
1. **Reduce struct sizes** with careful field ordering
2. **Use string interning** for repeated version strings
3. **Implement object pooling** for temporary objects
4. **Profile memory allocation patterns**

### Algorithm Optimization
1. **Choose optimal data structures** (Vec vs HashMap vs BTreeMap)
2. **Implement specialized comparison** for common cases
3. **Use parallel processing** with Rayon where beneficial
4. **Cache expensive computations**

## 📝 Performance Testing Checklist

Before releasing performance improvements:

- [ ] Run full benchmark suite
- [ ] Generate flamegraph profiles
- [ ] Check memory usage patterns
- [ ] Verify no performance regressions
- [ ] Test on multiple platforms
- [ ] Compare with Python baseline
- [ ] Document performance characteristics

## 🔗 References

- [Flamegraph Documentation](https://github.com/flamegraph-rs/flamegraph)
- [Criterion.rs Benchmarking](https://bheisler.github.io/criterion.rs/book/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [pydantic-core Performance](https://github.com/pydantic/pydantic-core#profiling)
