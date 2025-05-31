//! Simple Context Benchmark
//!
//! A minimal benchmark for testing the context system functionality
//! without complex dependencies. This focuses on core context operations.

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, black_box};
use rez_core_context::{
    ResolvedContext, ContextBuilder, ContextConfig, EnvironmentManager, 
    ShellExecutor, ShellType, PathStrategy, ContextSerialization, ContextFormat
};
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::{Version, VersionRange};
use std::collections::HashMap;
use std::time::Duration;

/// Test basic context functionality
fn bench_context_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_basic");
    
    // Test context creation
    group.bench_function("create_context", |b| {
        let requirements = vec![
            PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
        ];
        
        b.iter(|| {
            black_box(ResolvedContext::from_requirements(requirements.clone()))
        });
    });
    
    // Test context builder
    group.bench_function("context_builder", |b| {
        let requirements = vec![
            PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
        ];
        
        b.iter(|| {
            let builder = ContextBuilder::new()
                .requirements(requirements.clone())
                .config(ContextConfig::default());
            black_box(builder.build())
        });
    });
    
    group.finish();
}

/// Test context configuration variations
fn bench_context_configs(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_configs");
    
    let requirements = vec![
        PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
    ];
    
    let configs = vec![
        ("default", ContextConfig::default()),
        ("inherit_env", ContextConfig {
            inherit_parent_env: true,
            ..Default::default()
        }),
        ("isolated", ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        }),
        ("with_additional_vars", ContextConfig {
            additional_env_vars: {
                let mut vars = HashMap::new();
                vars.insert("CUSTOM_VAR".to_string(), "value".to_string());
                vars
            },
            ..Default::default()
        }),
    ];
    
    for (name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("config", name),
            &config,
            |b, config| {
                b.iter(|| {
                    let builder = ContextBuilder::new()
                        .requirements(requirements.clone())
                        .config(config.clone());
                    black_box(builder.build())
                });
            },
        );
    }
    
    group.finish();
}

/// Test environment generation
fn bench_environment_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("environment_generation");
    
    // Simple environment generation
    group.bench_function("simple_env_gen", |b| {
        let packages = vec![
            Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
        ];
        let env_manager = EnvironmentManager::new(ContextConfig::default());
        
        b.iter(|| {
            black_box(env_manager.generate_environment_sync(&packages))
        });
    });
    
    // Multiple packages environment generation
    group.bench_function("multi_package_env_gen", |b| {
        let packages = vec![
            Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
            Package::new("git".to_string(), Version::new("2.40.0".to_string()).unwrap()),
            Package::new("node".to_string(), Version::new("18.17.0".to_string()).unwrap()),
        ];
        let env_manager = EnvironmentManager::new(ContextConfig::default());
        
        b.iter(|| {
            black_box(env_manager.generate_environment_sync(&packages))
        });
    });
    
    // Test different path strategies
    let packages = vec![
        Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
    ];
    
    for strategy in &[PathStrategy::Prepend, PathStrategy::Append, PathStrategy::Replace] {
        group.bench_with_input(
            BenchmarkId::new("path_strategy", format!("{:?}", strategy)),
            strategy,
            |b, strategy| {
                let config = ContextConfig {
                    path_strategy: strategy.clone(),
                    ..Default::default()
                };
                let env_manager = EnvironmentManager::new(config);
                
                b.iter(|| {
                    black_box(env_manager.generate_environment_sync(&packages))
                });
            },
        );
    }
    
    group.finish();
}

/// Test shell execution
fn bench_shell_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("shell_execution");
    
    // Simple command execution
    group.bench_function("simple_command", |b| {
        let executor = ShellExecutor::new();
        
        b.iter(|| {
            black_box(executor.execute_sync("echo 'test'"))
        });
    });
    
    // Test different shell types
    let simple_command = "echo 'test'";
    for shell_type in &[ShellType::Bash, ShellType::Cmd, ShellType::PowerShell] {
        group.bench_with_input(
            BenchmarkId::new("shell_type", format!("{:?}", shell_type)),
            shell_type,
            |b, shell_type| {
                let executor = ShellExecutor::with_shell(shell_type.clone());
                b.iter(|| {
                    black_box(executor.execute_sync(simple_command))
                });
            },
        );
    }
    
    // Test with environment variables
    group.bench_function("with_environment", |b| {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());
        let executor = ShellExecutor::new().with_environment(env);
        
        b.iter(|| {
            black_box(executor.execute_sync("echo $TEST_VAR"))
        });
    });
    
    group.finish();
}

/// Test context serialization
fn bench_context_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_serialization");
    
    let context = ResolvedContext::from_requirements(vec![
        PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
    ]);
    
    // Test different serialization formats
    for format in &[ContextFormat::Json, ContextFormat::Yaml, ContextFormat::Binary] {
        // Serialization
        group.bench_with_input(
            BenchmarkId::new("serialize", format!("{:?}", format)),
            format,
            |b, format| {
                b.iter(|| {
                    black_box(ContextSerialization::serialize(&context, *format))
                });
            },
        );
        
        // Deserialization
        let serialized = ContextSerialization::serialize(&context, *format).unwrap();
        group.bench_with_input(
            BenchmarkId::new("deserialize", format!("{:?}", format)),
            &(format, serialized),
            |b, (format, data)| {
                b.iter(|| {
                    black_box(ContextSerialization::deserialize(data, **format))
                });
            },
        );
    }
    
    group.finish();
}

/// Test context fingerprinting and caching
fn bench_context_caching(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_caching");
    
    // Test fingerprint generation
    group.bench_function("fingerprint_generation", |b| {
        let context = ResolvedContext::from_requirements(vec![
            PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
        ]);
        
        b.iter(|| {
            black_box(context.get_fingerprint())
        });
    });
    
    // Test context validation
    group.bench_function("context_validation", |b| {
        let context = ResolvedContext::from_requirements(vec![
            PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
        ]);
        
        b.iter(|| {
            black_box(context.validate())
        });
    });
    
    group.finish();
}

/// Test context scalability
fn bench_context_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_scalability");
    
    // Test with different numbers of packages
    for package_count in &[1, 5, 10, 20] {
        let requirements: Vec<_> = (0..*package_count)
            .map(|i| PackageRequirement::new(format!("pkg_{}", i), None))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("package_count", package_count),
            &requirements,
            |b, requirements| {
                b.iter(|| {
                    let builder = ContextBuilder::new()
                        .requirements(requirements.clone())
                        .config(ContextConfig::default());
                    black_box(builder.build())
                });
            },
        );
    }
    
    // Test environment generation scalability
    for package_count in &[1, 5, 10, 20] {
        let packages: Vec<_> = (0..*package_count)
            .map(|i| Package::new(format!("pkg_{}", i), Version::new("1.0.0".to_string()).unwrap()))
            .collect();
        
        group.bench_with_input(
            BenchmarkId::new("env_gen_scale", package_count),
            &packages,
            |b, packages| {
                let env_manager = EnvironmentManager::new(ContextConfig::default());
                b.iter(|| {
                    black_box(env_manager.generate_environment_sync(packages))
                });
            },
        );
    }
    
    group.finish();
}

/// Performance validation benchmark
fn bench_performance_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("performance_validation");
    
    // This benchmark helps validate that context performance meets targets
    group.bench_function("baseline_performance", |b| {
        let requirements = vec![
            PackageRequirement::new("test_package".to_string(), Some(VersionRange::new("1.0+".to_string()).unwrap()))
        ];
        
        b.iter(|| {
            let builder = ContextBuilder::new()
                .requirements(requirements.clone())
                .config(ContextConfig::default());
            let context = builder.build();
            
            // Also test environment generation
            let packages = vec![
                Package::new("test_package".to_string(), Version::new("1.0.0".to_string()).unwrap())
            ];
            let env_manager = EnvironmentManager::new(ContextConfig::default());
            let _env = env_manager.generate_environment_sync(&packages);
            
            black_box(context)
        });
    });
    
    // Test with optimized configuration
    group.bench_function("optimized_performance", |b| {
        let requirements = vec![
            PackageRequirement::new("test_package".to_string(), Some(VersionRange::new("1.0+".to_string()).unwrap()))
        ];
        let config = ContextConfig {
            inherit_parent_env: false,  // Faster startup
            path_strategy: PathStrategy::Replace,  // Simpler path handling
            ..Default::default()
        };
        
        b.iter(|| {
            let builder = ContextBuilder::new()
                .requirements(requirements.clone())
                .config(config.clone());
            let context = builder.build();
            
            let packages = vec![
                Package::new("test_package".to_string(), Version::new("1.0.0".to_string()).unwrap())
            ];
            let env_manager = EnvironmentManager::new(config.clone());
            let _env = env_manager.generate_environment_sync(&packages);
            
            black_box(context)
        });
    });
    
    group.finish();
}

// Configure criterion groups
criterion_group!(
    name = context_basic_tests;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_context_basic, bench_context_configs, bench_context_caching
);

criterion_group!(
    name = context_functionality_tests;
    config = Criterion::default()
        .sample_size(50)
        .measurement_time(Duration::from_secs(8))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_environment_generation, bench_shell_execution, bench_context_serialization
);

criterion_group!(
    name = context_scalability_tests;
    config = Criterion::default()
        .sample_size(30)
        .measurement_time(Duration::from_secs(10))
        .warm_up_time(Duration::from_secs(3));
    targets = bench_context_scalability, bench_performance_validation
);

// Main entry point
criterion_main!(context_basic_tests, context_functionality_tests, context_scalability_tests);
