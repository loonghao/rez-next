//! Rex Benchmark Main Entry Point
//!
//! Comprehensive Rex system benchmarks with multiple configurations and scenarios.
//! This provides the main entry point for running all Rex-related benchmarks.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use rez_core_rex::{
    RexParser, OptimizedRexParser, RexInterpreter, RexExecutor, RexCache, RexCacheConfig,
    RexCommand, RexScript, ParserConfig, InterpreterConfig, ExecutorConfig,
    RexBindingGenerator, EvictionStrategy
};
use rez_core_context::{ResolvedContext, ContextBuilder, ContextConfig};
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::{Version, VersionRange};
use std::collections::HashMap;
use std::time::Duration;

mod rex_benchmark;
use rex_benchmark::RexBenchmark;

/// Rex performance benchmarks - Core functionality
fn rex_performance(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_parser_performance(c);
    benchmark.benchmark_execution_performance(c);
}

/// Rex cache benchmarks - Caching system performance
fn rex_caching(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_cache_performance(c);
}

/// Rex interpreter benchmarks - Interpreter and command execution
fn rex_interpreter(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_interpreter_performance(c);
}

/// Rex binding benchmarks - Package binding generation
fn rex_binding(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_binding_performance(c);
}

/// Rex script generation benchmarks - Shell script generation
fn rex_script_generation(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_script_generation(c);
}

/// Rex scalability benchmarks - Performance across complexity levels
fn rex_scalability(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_scalability(c);
}

/// Rex memory benchmarks - Memory usage tracking
fn rex_memory(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_memory_usage(c);
}

/// Rex statistics benchmarks - Statistics collection overhead
fn rex_statistics(c: &mut Criterion) {
    let benchmark = RexBenchmark::new();
    benchmark.benchmark_statistics_collection(c);
}

/// Rex validation benchmarks - Performance target validation
fn rex_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_validation");
    
    // Validate 75x performance improvement target for Rex execution
    group.bench_function("rex_75x_target_validation", |b| {
        let script = r#"setenv ENTERPRISE_ROOT /opt/enterprise
setenv JAVA_HOME $ENTERPRISE_ROOT/java
setenv MAVEN_HOME $ENTERPRISE_ROOT/maven
setenv GRADLE_HOME $ENTERPRISE_ROOT/gradle
prependenv PATH $JAVA_HOME/bin
prependenv PATH $MAVEN_HOME/bin
prependenv PATH $GRADLE_HOME/bin
appendenv LD_LIBRARY_PATH $JAVA_HOME/lib
setenv MAVEN_OPTS '-Xmx2g -XX:ReservedCodeCacheSize=512m'
setenv GRADLE_OPTS '-Xmx2g -XX:MaxMetaspaceSize=512m'"#;
        
        b.iter(|| {
            // Use optimized Rex components for maximum performance
            let parser = OptimizedRexParser::new();
            let parsed_script = parser.parse_optimized(script).unwrap();
            
            let context = create_enterprise_context();
            let config = ExecutorConfig {
                enable_caching: true,
                parallel_execution: true,
                enable_statistics: false, // Disable for pure performance
                ..Default::default()
            };
            let mut executor = RexExecutor::with_config(context, config);
            let _result = executor.execute_script_sync(&parsed_script);
            
            black_box(parsed_script)
        });
    });
    
    // Baseline performance measurement
    group.bench_function("rex_baseline_measurement", |b| {
        let simple_script = "setenv PYTHON_ROOT /opt/python";
        
        b.iter(|| {
            let parser = RexParser::new();
            let parsed_script = parser.parse(simple_script).unwrap();
            
            let context = create_simple_context();
            let mut executor = RexExecutor::new(context);
            let _result = executor.execute_script_sync(&parsed_script);
            
            black_box(parsed_script)
        });
    });
    
    // Cache hit ratio validation (target: >90%)
    group.bench_function("rex_cache_hit_ratio_validation", |b| {
        let cache = RexCache::with_config(RexCacheConfig {
            max_parse_entries: 1000,
            max_execution_entries: 500,
            eviction_strategy: EvictionStrategy::LRU,
            ..Default::default()
        });
        
        let repeated_commands = (0..100)
            .map(|i| format!("setenv VAR_{} value_{}", i % 10, i))
            .collect::<Vec<_>>();
        
        b.iter(|| {
            for command in &repeated_commands {
                // Simulate cache operations
                let parser = RexParser::new();
                let _script = parser.parse(command);
                black_box(command);
            }
        });
    });
    
    group.finish();
}

/// Rex regression testing - Detect performance regressions
fn rex_regression(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_regression");
    
    // Standard regression test scenarios
    let scenarios = vec![
        ("simple_setenv", "setenv PYTHON_ROOT /opt/python"),
        ("path_manipulation", r#"setenv TOOL_ROOT /opt/tools
prependenv PATH $TOOL_ROOT/bin
appendenv LD_LIBRARY_PATH $TOOL_ROOT/lib"#),
        ("development_environment", r#"setenv DEV_ROOT /opt/dev
setenv PYTHON_ROOT $DEV_ROOT/python
setenv NODE_ROOT $DEV_ROOT/node
prependenv PATH $PYTHON_ROOT/bin
prependenv PATH $NODE_ROOT/bin
alias python python3
alias pip pip3"#),
    ];
    
    for (name, script) in scenarios {
        group.bench_with_input(
            BenchmarkId::new("regression_test", name),
            &script,
            |b, script| {
                b.iter(|| {
                    let parser = RexParser::new();
                    let parsed_script = parser.parse(script).unwrap();
                    
                    let context = create_simple_context();
                    let mut executor = RexExecutor::new(context);
                    let _result = executor.execute_script_sync(&parsed_script);
                    
                    black_box(parsed_script)
                });
            },
        );
    }
    
    group.finish();
}

/// Rex baseline establishment - Create performance baselines
fn rex_baseline(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_baseline");
    
    // Establish baselines for different Rex operations
    group.bench_function("parser_baseline", |b| {
        let script = "setenv PYTHON_ROOT /opt/python";
        let parser = RexParser::new();
        
        b.iter(|| {
            black_box(parser.parse(script))
        });
    });
    
    group.bench_function("optimized_parser_baseline", |b| {
        let script = "setenv PYTHON_ROOT /opt/python";
        let parser = OptimizedRexParser::new();
        
        b.iter(|| {
            black_box(parser.parse_optimized(script))
        });
    });
    
    group.bench_function("executor_baseline", |b| {
        let script = "setenv PYTHON_ROOT /opt/python";
        let context = create_simple_context();
        let mut executor = RexExecutor::new(context);
        
        b.iter(|| {
            black_box(executor.execute_script_content_sync(script))
        });
    });
    
    group.bench_function("interpreter_baseline", |b| {
        let mut interpreter = RexInterpreter::new();
        let command = RexCommand::SetEnv {
            name: "PYTHON_ROOT".to_string(),
            value: "/opt/python".to_string(),
        };
        
        b.iter(|| {
            black_box(interpreter.execute_command_sync(&command))
        });
    });
    
    group.finish();
}

/// Rex serialization benchmarks - Context and command serialization
fn rex_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_serialization");
    
    // Test Rex script serialization
    group.bench_function("script_serialization", |b| {
        let script = r#"setenv DEV_ROOT /opt/dev
setenv PYTHON_ROOT $DEV_ROOT/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#;
        
        let parser = RexParser::new();
        let parsed_script = parser.parse(script).unwrap();
        
        b.iter(|| {
            // Serialize and deserialize the script
            let serialized = serde_json::to_string(&parsed_script).unwrap();
            let _deserialized: RexScript = serde_json::from_str(&serialized).unwrap();
            black_box(serialized)
        });
    });
    
    // Test Rex command serialization
    group.bench_function("command_serialization", |b| {
        let command = RexCommand::SetEnv {
            name: "PYTHON_ROOT".to_string(),
            value: "/opt/python".to_string(),
        };
        
        b.iter(|| {
            let serialized = serde_json::to_string(&command).unwrap();
            let _deserialized: RexCommand = serde_json::from_str(&serialized).unwrap();
            black_box(serialized)
        });
    });
    
    group.finish();
}

/// Rex caching detailed benchmarks - Detailed cache performance analysis
fn rex_caching_detailed(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_caching_detailed");
    
    // Test different cache sizes
    let cache_sizes = vec![100, 500, 1000, 5000];
    for size in cache_sizes {
        group.bench_with_input(
            BenchmarkId::new("cache_size", size),
            &size,
            |b, &size| {
                let config = RexCacheConfig {
                    max_parse_entries: size,
                    max_execution_entries: size / 2,
                    eviction_strategy: EvictionStrategy::LRU,
                    ..Default::default()
                };
                let cache = RexCache::with_config(config);
                
                b.iter(|| {
                    black_box(&cache);
                });
            },
        );
    }
    
    // Test different eviction strategies
    let strategies = vec![
        ("LRU", EvictionStrategy::LRU),
        ("LFU", EvictionStrategy::LFU),
        ("FIFO", EvictionStrategy::FIFO),
        ("TTL", EvictionStrategy::TTL),
    ];
    
    for (name, strategy) in strategies {
        group.bench_with_input(
            BenchmarkId::new("eviction_strategy", name),
            &strategy,
            |b, strategy| {
                let config = RexCacheConfig {
                    max_parse_entries: 1000,
                    max_execution_entries: 500,
                    eviction_strategy: strategy.clone(),
                    ..Default::default()
                };
                let cache = RexCache::with_config(config);
                
                b.iter(|| {
                    black_box(&cache);
                });
            },
        );
    }
    
    group.finish();
}

// Helper functions
fn create_simple_context() -> ResolvedContext {
    let requirements = vec![
        PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
    ];
    ContextBuilder::new()
        .requirements(requirements)
        .config(ContextConfig::default())
        .build()
}

fn create_enterprise_context() -> ResolvedContext {
    let requirements = vec![
        PackageRequirement::new("java".to_string(), Some(VersionRange::new("11+".to_string()).unwrap())),
        PackageRequirement::new("maven".to_string(), Some(VersionRange::new("3.8+".to_string()).unwrap())),
        PackageRequirement::new("gradle".to_string(), Some(VersionRange::new("7+".to_string()).unwrap())),
    ];
    ContextBuilder::new()
        .requirements(requirements)
        .config(ContextConfig::default())
        .build()
}

// Configure criterion groups with different settings for different test types
criterion_group!(
    name = rex_core_benchmarks;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = rex_performance, rex_interpreter, rex_scalability
);

criterion_group!(
    name = rex_advanced_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(15))
        .warm_up_time(Duration::from_secs(5));
    targets = rex_caching, rex_binding, rex_script_generation, rex_memory
);

criterion_group!(
    name = rex_validation_benchmarks;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(20))
        .warm_up_time(Duration::from_secs(5));
    targets = rex_validation, rex_baseline, rex_regression
);

criterion_group!(
    name = rex_specialized_benchmarks;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(2));
    targets = rex_statistics, rex_serialization, rex_caching_detailed
);

// Main entry point
criterion_main!(
    rex_core_benchmarks,
    rex_advanced_benchmarks,
    rex_validation_benchmarks,
    rex_specialized_benchmarks
);
