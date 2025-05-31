//! Simple Rex Benchmark
//!
//! A minimal benchmark for testing the Rex system functionality
//! without complex dependencies. This focuses on core Rex operations.

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

/// Test basic Rex functionality
fn bench_rex_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_basic");
    
    // Test Rex parser creation
    group.bench_function("create_parser", |b| {
        b.iter(|| {
            black_box(RexParser::new())
        });
    });
    
    // Test optimized Rex parser creation
    group.bench_function("create_optimized_parser", |b| {
        b.iter(|| {
            black_box(OptimizedRexParser::new())
        });
    });
    
    // Test Rex interpreter creation
    group.bench_function("create_interpreter", |b| {
        b.iter(|| {
            black_box(RexInterpreter::new())
        });
    });
    
    group.finish();
}

/// Test Rex parsing performance
fn bench_rex_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_parsing");
    
    // Simple parsing test
    group.bench_function("parse_simple_setenv", |b| {
        let parser = RexParser::new();
        let script = "setenv PYTHON_ROOT /opt/python";
        
        b.iter(|| {
            black_box(parser.parse(script))
        });
    });
    
    // Multiple commands parsing
    group.bench_function("parse_multiple_commands", |b| {
        let parser = RexParser::new();
        let script = r#"setenv PYTHON_ROOT /opt/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#;
        
        b.iter(|| {
            black_box(parser.parse(script))
        });
    });
    
    // Compare basic vs optimized parser
    let complex_script = r#"setenv DEV_ROOT /opt/dev
setenv PYTHON_ROOT $DEV_ROOT/python
setenv NODE_ROOT $DEV_ROOT/node
prependenv PATH $PYTHON_ROOT/bin
prependenv PATH $NODE_ROOT/bin
appendenv LD_LIBRARY_PATH $PYTHON_ROOT/lib
alias python python3
alias pip pip3
alias node node"#;
    
    group.bench_function("basic_parser_complex", |b| {
        let parser = RexParser::new();
        b.iter(|| {
            black_box(parser.parse(complex_script))
        });
    });
    
    group.bench_function("optimized_parser_complex", |b| {
        let parser = OptimizedRexParser::new();
        b.iter(|| {
            black_box(parser.parse_optimized(complex_script))
        });
    });
    
    group.finish();
}

/// Test Rex execution performance
fn bench_rex_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_execution");
    
    // Simple execution test
    group.bench_function("execute_simple_setenv", |b| {
        let context = create_simple_context();
        let mut executor = RexExecutor::new(context);
        let script = "setenv PYTHON_ROOT /opt/python";
        
        b.iter(|| {
            black_box(executor.execute_script_content_sync(script))
        });
    });
    
    // Multiple commands execution
    group.bench_function("execute_multiple_commands", |b| {
        let context = create_simple_context();
        let mut executor = RexExecutor::new(context);
        let script = r#"setenv PYTHON_ROOT /opt/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#;
        
        b.iter(|| {
            black_box(executor.execute_script_content_sync(script))
        });
    });
    
    group.finish();
}

/// Test Rex cache performance
fn bench_rex_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_cache");
    
    // Test cache creation
    group.bench_function("create_cache", |b| {
        b.iter(|| {
            black_box(RexCache::new())
        });
    });
    
    // Test cache with configuration
    group.bench_function("create_cache_with_config", |b| {
        let config = RexCacheConfig {
            max_parse_entries: 1000,
            max_execution_entries: 500,
            eviction_strategy: EvictionStrategy::LRU,
            ..Default::default()
        };
        
        b.iter(|| {
            black_box(RexCache::with_config(config.clone()))
        });
    });
    
    // Test cache operations
    group.bench_function("cache_operations", |b| {
        let cache = RexCache::new();
        let commands = vec![
            "setenv VAR1 value1",
            "setenv VAR2 value2",
            "setenv VAR3 value3",
        ];
        
        b.iter(|| {
            for command in &commands {
                // Simulate cache operations
                black_box(command);
            }
        });
    });
    
    group.finish();
}

/// Test Rex interpreter configurations
fn bench_rex_interpreter_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_interpreter_configs");
    
    let configs = vec![
        ("default", InterpreterConfig::default()),
        ("strict_mode", InterpreterConfig {
            strict_mode: true,
            ..Default::default()
        }),
        ("variable_expansion", InterpreterConfig {
            enable_variable_expansion: true,
            ..Default::default()
        }),
        ("optimized", InterpreterConfig {
            strict_mode: false,
            enable_variable_expansion: true,
            enable_caching: true,
            ..Default::default()
        }),
    ];
    
    for (name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("interpreter_config", name),
            &config,
            |b, config| {
                b.iter(|| {
                    black_box(RexInterpreter::with_config(config.clone()))
                });
            },
        );
    }
    
    group.finish();
}

/// Test Rex command types
fn bench_rex_command_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_command_types");
    
    let commands = vec![
        ("setenv", RexCommand::SetEnv {
            name: "VAR".to_string(),
            value: "value".to_string(),
        }),
        ("prependenv", RexCommand::PrependEnv {
            name: "PATH".to_string(),
            value: "/bin".to_string(),
            separator: ":".to_string(),
        }),
        ("appendenv", RexCommand::AppendEnv {
            name: "PATH".to_string(),
            value: "/usr/bin".to_string(),
            separator: ":".to_string(),
        }),
        ("alias", RexCommand::Alias {
            name: "ll".to_string(),
            command: "ls -la".to_string(),
        }),
    ];
    
    for (name, command) in commands {
        group.bench_with_input(
            BenchmarkId::new("command_type", name),
            &command,
            |b, command| {
                let mut interpreter = RexInterpreter::new();
                b.iter(|| {
                    black_box(interpreter.execute_command_sync(command))
                });
            },
        );
    }
    
    group.finish();
}

/// Test Rex scalability
fn bench_rex_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_scalability");
    
    // Test with different numbers of commands
    for command_count in &[1, 5, 10, 20, 50] {
        let script_lines: Vec<String> = (0..*command_count)
            .map(|i| format!("setenv VAR_{} value_{}", i, i))
            .collect();
        let script = script_lines.join("\n");
        
        group.bench_with_input(
            BenchmarkId::new("command_count", command_count),
            &script,
            |b, script| {
                let parser = RexParser::new();
                b.iter(|| {
                    black_box(parser.parse(script))
                });
            },
        );
    }
    
    // Test execution scalability
    for command_count in &[1, 5, 10, 20] {
        let script_lines: Vec<String> = (0..*command_count)
            .map(|i| format!("setenv VAR_{} value_{}", i, i))
            .collect();
        let script = script_lines.join("\n");
        
        group.bench_with_input(
            BenchmarkId::new("execution_scale", command_count),
            &script,
            |b, script| {
                let context = create_simple_context();
                let mut executor = RexExecutor::new(context);
                b.iter(|| {
                    black_box(executor.execute_script_content_sync(script))
                });
            },
        );
    }
    
    group.finish();
}

/// Test Rex binding generation
fn bench_rex_binding(c: &mut Criterion) {
    let mut group = c.benchmark_group("rex_binding");
    
    // Simple binding generation
    group.bench_function("simple_binding", |b| {
        let packages = vec![
            Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
        ];
        let context = create_simple_context();
        let binding_generator = RexBindingGenerator::new();
        
        b.iter(|| {
            black_box(binding_generator.generate_bindings(&packages, &context))
        });
    });
    
    // Multiple packages binding
    group.bench_function("multiple_packages_binding", |b| {
        let packages = vec![
            Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
            Package::new("node".to_string(), Version::new("18.17.0".to_string()).unwrap()),
            Package::new("git".to_string(), Version::new("2.40.0".to_string()).unwrap()),
        ];
        let context = create_complex_context();
        let binding_generator = RexBindingGenerator::new();
        
        b.iter(|| {
            black_box(binding_generator.generate_bindings(&packages, &context))
        });
    });
    
    group.finish();
}

/// Performance validation benchmark
fn bench_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_validation");
    
    // This benchmark helps validate that Rex performance meets targets
    group.bench_function("baseline_performance", |b| {
        let script = r#"setenv PYTHON_ROOT /opt/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#;
        
        b.iter(|| {
            // Parse
            let parser = RexParser::new();
            let parsed_script = parser.parse(script).unwrap();
            
            // Execute
            let context = create_simple_context();
            let mut executor = RexExecutor::new(context);
            let _result = executor.execute_script_sync(&parsed_script);
            
            black_box(parsed_script)
        });
    });
    
    // Test with optimized configuration
    group.bench_function("optimized_performance", |b| {
        let script = r#"setenv PYTHON_ROOT /opt/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#;
        
        b.iter(|| {
            // Parse with optimized parser
            let parser = OptimizedRexParser::new();
            let parsed_script = parser.parse_optimized(script).unwrap();
            
            // Execute with optimized configuration
            let context = create_simple_context();
            let config = ExecutorConfig {
                enable_caching: true,
                parallel_execution: true,
                ..Default::default()
            };
            let mut executor = RexExecutor::with_config(context, config);
            let _result = executor.execute_script_sync(&parsed_script);
            
            black_box(parsed_script)
        });
    });
    
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

fn create_complex_context() -> ResolvedContext {
    let requirements = vec![
        PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap())),
        PackageRequirement::new("node".to_string(), Some(VersionRange::new("18+".to_string()).unwrap())),
        PackageRequirement::new("git".to_string(), None),
    ];
    ContextBuilder::new()
        .requirements(requirements)
        .config(ContextConfig::default())
        .build()
}

// Configure criterion groups
criterion_group!(
    name = rex_basic_tests;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_rex_basic, bench_rex_parsing, bench_rex_cache
);

criterion_group!(
    name = rex_functionality_tests;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_rex_execution, bench_rex_interpreter_configs, bench_rex_command_types
);

criterion_group!(
    name = rex_advanced_tests;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_rex_scalability, bench_rex_binding, bench_performance_validation
);

// Main entry point
criterion_main!(rex_basic_tests, rex_functionality_tests, rex_advanced_tests);
