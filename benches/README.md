# Comprehensive Benchmark Suite

This directory contains the comprehensive benchmark suite for rez-core, providing unified performance testing across all core modules.

## 🏗️ Architecture

The benchmark suite is built around a modular architecture that allows for:

- **Unified Interface**: All modules implement the `ModuleBenchmark` trait
- **Configurable Testing**: Flexible configuration for different testing scenarios
- **Baseline Management**: Automatic baseline storage and comparison
- **Regression Detection**: Built-in performance regression detection
- **Multiple Output Formats**: HTML, JSON, Markdown, and CSV reports

## 📁 Files

### Core Framework
- `comprehensive_benchmark_suite.rs` - Core framework implementation
- `example_version_module.rs` - Example implementation showing how to use the framework

### Module-Specific Benchmarks
- `version_benchmark.rs` - Version parsing, comparison, and sorting benchmarks
- `solver_benchmark.rs` - ✨ **NEW** Comprehensive solver system benchmarks
- `solver_benchmark_main.rs` - ✨ **NEW** Main entry point for solver benchmarks
- `standalone_solver_benchmark.rs` - ✨ **NEW** Simplified standalone solver testing
- `context_benchmark.rs` - ✨ **NEW** Comprehensive context system benchmarks
- `context_benchmark_main.rs` - ✨ **NEW** Main entry point for context benchmarks
- `simple_context_benchmark.rs` - ✨ **NEW** Simplified standalone context testing
- `rex_benchmark.rs` - ✨ **NEW** Comprehensive Rex system benchmarks
- `rex_benchmark_main.rs` - ✨ **NEW** Main entry point for Rex benchmarks
- `simple_rex_benchmark.rs` - ✨ **NEW** Simplified standalone Rex testing
- `build_cache_benchmark.rs` - ✨ **NEW** Comprehensive Build and Cache system benchmarks
- `build_cache_benchmark_main.rs` - ✨ **NEW** Main entry point for Build and Cache benchmarks
- `simple_build_cache_benchmark.rs` - ✨ **NEW** Simplified standalone Build and Cache testing

### Legacy Benchmarks
- `performance_optimization_benchmark.rs` - Existing comprehensive performance benchmarks
- `package_benchmark.rs` - Package creation, serialization, and validation benchmarks
- `package_core_benchmark.rs` - Core package functionality benchmarks
- `simple_package_benchmark.rs` - Simplified package benchmarks
- `standalone_package_benchmark.rs` - Standalone package testing
- `unified_benchmark_test.rs` - Integrated testing across modules

## 🚀 Quick Start

### 1. Running All Benchmarks

```bash
# Run all registered benchmarks
cargo bench --bench comprehensive_benchmark_suite

# Run with specific configuration
cargo bench --bench comprehensive_benchmark_suite --features comprehensive-benchmarks
```

### 2. Running Specific Modules

```bash
# Run only version module benchmarks
cargo bench --bench comprehensive_benchmark_suite -- version

# Run multiple specific modules
cargo bench --bench comprehensive_benchmark_suite -- version package
```

### 3. Solver-Specific Benchmarks ✨ NEW

```bash
# Run comprehensive solver benchmarks
cargo bench --bench solver_benchmark_main

# Run quick solver tests for development
cargo bench --bench standalone_solver_benchmark

# Run specific solver benchmark groups
cargo bench --bench solver_benchmark_main solver_performance
cargo bench --bench solver_benchmark_main solver_algorithms
cargo bench --bench solver_benchmark_main quick_solver

# Performance validation (verify 3-5x improvements)
cargo bench --bench solver_benchmark_main solver_validation

# Establish new baseline
cargo bench --bench solver_benchmark_main solver_baseline

# Regression testing
cargo bench --bench solver_benchmark_main solver_regression
```

### 4. Context-Specific Benchmarks ✨ NEW

```bash
# Run comprehensive context benchmarks
cargo bench --bench context_benchmark_main

# Run quick context tests for development
cargo bench --bench simple_context_benchmark

# Run specific context benchmark groups
cargo bench --bench context_benchmark_main context_performance
cargo bench --bench context_benchmark_main context_environment
cargo bench --bench context_benchmark_main context_shell

# Performance validation (verify performance improvements)
cargo bench --bench context_benchmark_main context_validation

# Establish new baseline
cargo bench --bench context_benchmark_main context_baseline

# Regression testing
cargo bench --bench context_benchmark_main context_regression

# Serialization and I/O tests
cargo bench --bench context_benchmark_main context_serialization

# Caching and fingerprinting tests
cargo bench --bench context_benchmark_main context_caching
```

### 5. Rex-Specific Benchmarks ✨ NEW

```bash
# Run comprehensive Rex benchmarks
cargo bench --bench rex_benchmark_main

# Run quick Rex tests for development
cargo bench --bench simple_rex_benchmark

# Run specific Rex benchmark groups
cargo bench --bench rex_benchmark_main rex_performance
cargo bench --bench rex_benchmark_main rex_caching
cargo bench --bench rex_benchmark_main rex_interpreter

# Performance validation (verify 75x performance improvements)
cargo bench --bench rex_benchmark_main rex_validation

# Establish new baseline
cargo bench --bench rex_benchmark_main rex_baseline

# Regression testing
cargo bench --bench rex_benchmark_main rex_regression

# Serialization and I/O tests
cargo bench --bench rex_benchmark_main rex_serialization

# Caching detailed analysis
cargo bench --bench rex_benchmark_main rex_caching_detailed
```

### 6. Build and Cache-Specific Benchmarks ✨ NEW

```bash
# Run comprehensive Build and Cache benchmarks
cargo bench --bench build_cache_benchmark_main

# Run quick Build and Cache tests for development
cargo bench --bench simple_build_cache_benchmark

# Run specific Build and Cache benchmark groups
cargo bench --bench build_cache_benchmark_main build_performance
cargo bench --bench build_cache_benchmark_main cache_performance
cargo bench --bench build_cache_benchmark_main build_parallel

# Performance validation
cargo bench --bench build_cache_benchmark_main build_validation
cargo bench --bench build_cache_benchmark_main cache_validation

# Establish new baseline
cargo bench --bench build_cache_benchmark_main build_cache_baseline

# Regression testing
cargo bench --bench build_cache_benchmark_main build_cache_regression

# Advanced features
cargo bench --bench build_cache_benchmark_main cache_advanced
cargo bench --bench build_cache_benchmark_main build_cache_integration
```

### 7. Quick Development Testing

```bash
# Run quick benchmarks for development
cargo bench --bench comprehensive_benchmark_suite --features quick-benchmarks

# Quick solver testing
cargo bench --bench standalone_solver_benchmark

# Quick context testing
cargo bench --bench simple_context_benchmark

# Quick Rex testing
cargo bench --bench simple_rex_benchmark

# Quick Build and Cache testing
cargo bench --bench simple_build_cache_benchmark
```

## 🧩 Solver Benchmark Details ✨ NEW

The solver benchmark suite provides comprehensive testing for the rez-core dependency resolution system:

### Core Functionality Tests
- **Basic Resolution**: Simple, medium, and complex dependency scenarios
- **Conflict Resolution**: Different conflict strategies (LatestWins, EarliestWins, FindCompatible)
- **Cache Performance**: Cache hit ratios and performance comparison with/without caching
- **Parallel Solving**: Multi-threaded performance with different worker counts (1, 2, 4, 8)

### Advanced Algorithm Tests
- **A* Heuristic Algorithm**: Heuristic-guided dependency resolution performance
- **Optimized Solver**: Performance comparison between basic and optimized solvers
- **Scalability Testing**: Performance across different complexity levels (simple → complex)

### Resource Usage Tests
- **Memory Usage**: Memory consumption during solving operations
- **Statistics Collection**: Overhead of metrics and statistics collection

### Performance Validation
- **Baseline Establishment**: Create and manage performance baselines
- **Regression Testing**: Detect performance regressions against baselines
- **Target Validation**: Verify 3-5x performance improvements for complex scenarios

### Test Scenarios
- **Simple Scenarios**: 1-5 packages (single package, linear chain, simple diamond)
- **Medium Scenarios**: 6-20 packages (web framework, data science stack)
- **Complex Scenarios**: 21+ packages (large application, enterprise stack)
- **Conflict Scenarios**: Version conflicts, incompatible requirements
- **Cache Scenarios**: Repeated requests, similar complex requests

### Performance Targets
- **Dependency Resolution**: 3-5x improvement for complex scenarios
- **Cache Hit Ratio**: >90% for repeated requests
- **Parallel Scaling**: Linear scaling up to 4-8 workers
- **Memory Efficiency**: Minimal memory overhead during resolution

## 🏗️ Context Benchmark Details ✨ NEW

The context benchmark suite provides comprehensive testing for the rez-core context management system:

### Core Functionality Tests
- **Context Creation**: Simple, medium, and complex context building scenarios
- **Environment Generation**: Package environment variable generation and management
- **Shell Execution**: Cross-platform shell command execution performance
- **Serialization**: Context serialization/deserialization in multiple formats (JSON, YAML, Binary)

### Advanced Features Tests
- **Context Caching**: Fingerprinting and context lookup performance
- **Execution Performance**: Context execution setup and statistics collection
- **Path Management**: Different path modification strategies (Prepend, Append, Replace)
- **Variable Expansion**: Environment variable expansion and substitution

### Resource Usage Tests
- **Memory Usage**: Memory consumption during context operations
- **Validation**: Context and environment variable validation overhead
- **I/O Performance**: File-based context serialization and loading

### Performance Validation
- **Baseline Establishment**: Create and manage context performance baselines
- **Regression Testing**: Detect performance regressions in context operations
- **Scalability Testing**: Performance across different context complexity levels

### Test Scenarios
- **Simple Contexts**: 1-5 packages (single package, basic dev environment, isolated environment)
- **Medium Contexts**: 6-20 packages (web development stack, data science environment)
- **Complex Contexts**: 21+ packages (enterprise development suite, CI/CD pipeline environment)
- **Environment Scenarios**: Basic env generation, complex path handling, variable expansion
- **Shell Scenarios**: Simple commands, environment-dependent commands, cross-platform commands
- **Serialization Scenarios**: JSON, YAML, and binary format testing

### Performance Targets
- **Context Creation**: Sub-millisecond creation for simple contexts
- **Environment Generation**: <10ms for medium complexity environments
- **Shell Execution**: Platform-native performance with minimal overhead
- **Serialization**: Efficient serialization with format-appropriate compression
- **Caching**: >95% cache hit rate for repeated context operations
- **Memory Efficiency**: Minimal memory overhead during context lifecycle

## ⚡ Rex Benchmark Details ✨ NEW

The Rex benchmark suite provides comprehensive testing for the rez-core Rex command system:

### Core Functionality Tests
- **Rex Parsing**: Basic vs optimized parser performance comparison
- **Command Execution**: Simple, medium, and complex Rex script execution
- **Interpreter Performance**: Rex interpreter creation and command processing
- **Script Generation**: Shell script generation for different shell types (bash, cmd, powershell)

### Advanced Features Tests
- **Rex Caching**: Parse and execution result caching with different eviction strategies
- **Binding Generation**: Package binding generation for different complexity levels
- **Statistics Collection**: Performance overhead of statistics and metrics collection
- **Memory Usage**: Memory consumption tracking during Rex operations

### Resource Usage Tests
- **Parser Performance**: Basic parser vs optimized parser comparison
- **Execution Scalability**: Performance across different script complexity levels
- **Cache Efficiency**: Cache hit ratios and performance with different cache configurations
- **Serialization**: Rex script and command serialization/deserialization performance

### Performance Validation
- **75x Target Validation**: Verify 75x performance improvement target for Rex execution
- **Baseline Establishment**: Create and manage Rex performance baselines
- **Regression Testing**: Detect performance regressions in Rex operations
- **Cache Hit Ratio**: Validate >90% cache hit rate target

### Test Scenarios
- **Simple Scripts**: 1-5 commands (single setenv, basic environment setup, path manipulation)
- **Medium Scripts**: 6-20 commands (development environment, build environment)
- **Complex Scripts**: 21+ commands (enterprise environment, CI/CD pipeline environment)
- **Parser Scenarios**: Basic commands, complex parsing, variable expansion
- **Cache Scenarios**: Different cache sizes, eviction strategies (LRU, LFU, FIFO, TTL)
- **Binding Scenarios**: Simple package binding, complex multi-package binding

### Performance Targets
- **Rex Parsing**: Sub-millisecond parsing for simple scripts
- **Command Execution**: <5ms execution for medium complexity scripts
- **Cache Performance**: >90% cache hit rate for repeated operations
- **Memory Efficiency**: Minimal memory overhead during Rex script lifecycle
- **Scalability**: Linear performance scaling with script complexity
- **75x Improvement**: Validate 75x performance improvement for complex Rex operations

## 🏗️ Build and Cache Benchmark Details ✨ NEW

The Build and Cache benchmark suite provides comprehensive testing for the rez-core Build and Cache systems:

### Build System Tests
- **Build Manager Performance**: Build manager creation, configuration, and request processing
- **Build System Detection**: Automatic detection of build systems (CMake, Make, Python, Node.js, Cargo)
- **Build Environment Setup**: Environment creation, variable setup, and context integration
- **Build Parallel Processing**: Concurrent build management and scalability testing

### Cache System Tests
- **Intelligent Cache Performance**: Multi-level cache operations and hit ratio optimization
- **Predictive Preheating**: Pattern recognition and predictive cache preheating
- **Adaptive Tuning**: Performance analysis and automatic cache parameter optimization
- **Performance Monitoring**: Real-time monitoring and statistics collection

### Resource Usage Tests
- **Build Memory Usage**: Memory consumption during build operations
- **Cache Memory Efficiency**: Cache memory usage and optimization
- **Build Statistics Collection**: Statistics collection overhead and performance impact
- **Scalability Testing**: Performance across different complexity levels

### Performance Validation
- **Build System Validation**: Validate build system detection and configuration performance
- **Cache Hit Ratio Validation**: Verify >90% cache hit rate targets
- **Baseline Establishment**: Create and manage Build and Cache performance baselines
- **Regression Testing**: Detect performance regressions in Build and Cache operations

### Test Scenarios
- **Simple Builds**: 1-3 packages (single Python package, simple CMake project, basic dependency chain)
- **Medium Builds**: 4-10 packages (web development stack, data science environment)
- **Complex Builds**: 11+ packages (enterprise application with 15+ modules)
- **Build Systems**: CMake, Make, Python, Node.js detection and configuration
- **Cache Scenarios**: Small (100 ops), medium (1K ops), large (10K ops), high-throughput (50K ops)
- **Build Configurations**: Single-threaded, parallel (4 workers), high-performance (8 workers)

### Performance Targets
- **Build Manager Creation**: Sub-millisecond creation time
- **Build System Detection**: <1ms detection for common build systems
- **Build Environment Setup**: <5ms setup for medium complexity projects
- **Cache Operations**: >1000 ops/sec for L1 cache, >500 ops/sec for L2 cache
- **Cache Hit Ratio**: >90% for repeated operations
- **Parallel Build Scaling**: Linear scaling up to 4-8 concurrent builds
- **Memory Efficiency**: Minimal memory overhead during build and cache operations

## 🚀 CI Integration and Performance Regression Detection ✨ NEW

The rez-core project includes comprehensive CI integration for automated performance monitoring:

### GitHub Actions Workflow
- **Automated Benchmarks**: Runs on every push and pull request
- **Multiple Benchmark Types**: Quick, comprehensive, validation, and regression tests
- **Scheduled Runs**: Weekly comprehensive benchmarks for trend analysis
- **Manual Triggers**: On-demand benchmark execution with configurable options

### Performance Regression Detection
- **Automatic Analysis**: Compares current results with established baselines
- **Severity Classification**: Minor (5-10%), Moderate (10-20%), Major (20-50%), Critical (>50%)
- **Confidence Scoring**: Statistical confidence in regression detection
- **Threshold Configuration**: Configurable regression thresholds (default: 10%)

### Baseline Management
- **Automatic Updates**: Updates baselines on main branch with approval
- **Backup System**: Maintains historical baselines with automatic cleanup
- **Metadata Tracking**: Tracks baseline creation, updates, and commit hashes
- **Validation**: Ensures baseline data integrity and format compliance

### Performance Reporting
- **Multiple Formats**: HTML, JSON, and Markdown reports
- **Trend Analysis**: Historical performance trends with visualizations
- **Performance Badges**: Shields.io badges for README and documentation
- **PR Comments**: Automatic performance reports on pull requests

### Usage Examples

#### Manual Benchmark Execution
```bash
# Trigger comprehensive benchmarks
gh workflow run benchmark.yml -f benchmark_type=comprehensive

# Trigger validation benchmarks
gh workflow run benchmark.yml -f benchmark_type=validation

# Update baselines (requires approval)
gh workflow run benchmark.yml -f benchmark_type=comprehensive -f baseline_update=true
```

#### Local Performance Analysis
```bash
# Analyze performance regressions
python scripts/analyze_performance_regression.py \
  --current-dir benchmark-results \
  --baseline-dir benchmark-baselines \
  --threshold 10.0

# Generate performance report
python scripts/generate_performance_report.py \
  --input-dir benchmark-results \
  --output-dir benchmark-reports \
  --format html,json,markdown

# Update baselines
python scripts/update_baselines.py \
  --benchmark-dir benchmark-results \
  --baseline-dir benchmark-baselines \
  --commit-hash $(git rev-parse HEAD)

# Create performance badges
python scripts/create_performance_badge.py \
  --benchmark-dir benchmark-results \
  --output-file performance-badge.json

# Generate trend analysis
python scripts/generate_trend_analysis.py \
  --benchmark-dir benchmark-results \
  --output-dir trend-analysis \
  --lookback-days 30
```

### Performance Targets and Monitoring
- **Solver System**: 3-5x improvement for complex scenarios
- **Context System**: Sub-millisecond creation for simple contexts
- **Rex System**: 75x performance improvement for complex operations
- **Build System**: >90% cache hit rate for repeated operations
- **Overall Score**: Maintain >75/100 performance score across all modules

## 🔧 Creating a New Module Benchmark

### Step 1: Implement the ModuleBenchmark Trait

```rust
use comprehensive_benchmark_suite::*;

pub struct MyModuleBenchmark {
    // Module-specific data
}

impl ModuleBenchmark for MyModuleBenchmark {
    fn name(&self) -> &str {
        "my_module"
    }
    
    fn run_benchmarks(&self, c: &mut Criterion) {
        // Implement your benchmarks here
        c.bench_function("my_benchmark", |b| {
            b.iter(|| {
                // Your benchmark code
            });
        });
    }
    
    fn get_baseline_metrics(&self) -> BaselineMetrics {
        // Return baseline metrics for comparison
        BaselineMetrics {
            module_name: "my_module".to_string(),
            timestamp: SystemTime::now(),
            benchmarks: HashMap::new(),
            overall_score: 90.0,
            environment: environment::detect_environment(),
        }
    }
}
```

### Step 2: Register Your Module

```rust
fn main() {
    let mut suite = BenchmarkSuite::new();
    let my_module = Box::new(MyModuleBenchmark::new());
    suite.register_module(my_module).unwrap();
    suite.run_all().unwrap();
}
```

## ⚙️ Configuration

### Benchmark Configurations

The framework provides several pre-configured benchmark setups:

```rust
use comprehensive_benchmark_suite::config_helpers::*;

// High-performance configuration (longer, more accurate)
let config = high_performance_config();

// Quick configuration (faster, for development)
let config = quick_config();

// Comprehensive configuration (most detailed)
let config = comprehensive_config();
```

### Custom Configuration

```rust
let config = BenchmarkConfig {
    global: GlobalBenchmarkConfig {
        default_warm_up_time: Duration::from_secs(1),
        default_measurement_time: Duration::from_secs(5),
        default_sample_size: 100,
        parallel_execution: true,
        max_concurrent: 4,
    },
    output: OutputConfig {
        output_dir: PathBuf::from("target/my-benchmark-reports"),
        formats: vec![ReportFormat::Html, ReportFormat::Json],
        detailed_reports: true,
    },
    baseline: BaselineConfig {
        baseline_dir: PathBuf::from("benchmarks/my-baselines"),
        auto_update: false,
        regression_threshold: 10.0, // 10% regression threshold
    },
    modules: HashMap::new(),
};
```

## 📊 Baseline Management

### Saving Baselines

```rust
let suite = BenchmarkSuite::new();
// ... register modules ...
suite.save_all_baselines().unwrap();
```

### Loading and Comparing Baselines

```rust
let baselines = suite.load_all_baselines().unwrap();
for baseline in baselines {
    println!("Module: {}, Score: {}", baseline.module_name, baseline.overall_score);
}
```

### Regression Detection

```rust
use comprehensive_benchmark_suite::analysis::*;

let comparison = compare_results(&current_result, &baseline_result, 5.0);
match comparison.status {
    ComparisonStatus::Regression => println!("Performance regression detected!"),
    ComparisonStatus::Improvement => println!("Performance improved!"),
    ComparisonStatus::NoChange => println!("No significant change"),
}
```

## 📈 Output Formats

The framework supports multiple output formats:

- **HTML**: Interactive reports with charts (default Criterion format)
- **JSON**: Machine-readable structured data
- **Markdown**: Documentation-friendly format
- **CSV**: Spreadsheet-compatible format

Reports are generated in the configured output directory (default: `target/benchmark-reports`).

## 🔍 Validation

Use the validation script to ensure your benchmark implementation is correct:

```bash
python scripts/validate_benchmark_framework.py
```

This script validates:
- File structure
- Cargo configuration
- Rust syntax
- Framework structure
- Example implementations
- Baseline storage functionality

## 🎯 Best Practices

### 1. Benchmark Design

- **Use `black_box()`** to prevent compiler optimizations
- **Test realistic scenarios** that match actual usage patterns
- **Include both micro and macro benchmarks**
- **Test different input sizes** to understand scaling behavior

### 2. Baseline Management

- **Establish baselines** on a stable, representative system
- **Update baselines** when making intentional performance changes
- **Use appropriate regression thresholds** (5-10% is often reasonable)
- **Document baseline conditions** (hardware, compiler version, etc.)

### 3. CI/CD Integration

- **Run benchmarks regularly** but not on every commit (they're expensive)
- **Use quick benchmarks** for development, comprehensive for releases
- **Store baseline data** in version control or dedicated storage
- **Alert on regressions** but allow for manual review

## 🚀 Advanced Usage

### Custom Metrics

```rust
let mut additional_metrics = HashMap::new();
additional_metrics.insert("memory_allocations".to_string(), 1024.0);
additional_metrics.insert("cache_misses".to_string(), 42.0);

let result = BenchmarkResult {
    name: "my_benchmark".to_string(),
    mean_time_ns: 1000.0,
    std_dev_ns: 50.0,
    throughput_ops_per_sec: Some(1_000_000.0),
    memory_usage_bytes: Some(2048),
    additional_metrics,
};
```

### Environment Detection

```rust
use comprehensive_benchmark_suite::environment::*;

let env = detect_environment();
println!("Running on: {} {}", env.os, env.cpu);
println!("Rust version: {}", env.rust_version);
```

## 🐛 Troubleshooting

### Common Issues

1. **Compilation Errors**: Ensure all dependencies are properly configured in `Cargo.toml`
2. **Missing Baselines**: Run `save_all_baselines()` to create initial baselines
3. **Inconsistent Results**: Use longer measurement times and more samples
4. **Memory Issues**: Reduce sample sizes or run benchmarks sequentially

### Debug Mode

Enable debug logging for troubleshooting:

```rust
env_logger::init();
// Your benchmark code here
```

## 📚 References

- [Criterion.rs Documentation](https://docs.rs/criterion/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Benchmarking Best Practices](https://github.com/rust-lang/rfcs/blob/master/text/2544-benchmarking.md)
