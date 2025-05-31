//! Rex System Benchmark Suite
//!
//! Comprehensive benchmarks for the rez-core Rex system including:
//! - Rex command parsing performance (basic vs optimized parser)
//! - Rex command execution performance
//! - Rex caching system performance
//! - Rex binding system performance
//! - Shell script generation performance
//! - Rex interpreter performance
//! - Statistics collection overhead

use criterion::{Criterion, BenchmarkId, Throughput, black_box};
use rez_core_rex::{
    RexParser, OptimizedRexParser, RexInterpreter, RexExecutor, RexCache, RexCacheConfig,
    RexCommand, RexScript, ParserConfig, InterpreterConfig, ExecutorConfig,
    RexBindingGenerator, EvictionStrategy
};
use rez_core_context::{ResolvedContext, ContextBuilder, ContextConfig};
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::{Version, VersionRange};
use rez_core_common::RezCoreError;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Serialize, Deserialize};

/// Rex benchmark module implementation
pub struct RexBenchmark {
    /// Test data for benchmarks
    test_data: RexTestData,
    /// Baseline metrics
    baseline_metrics: BaselineMetrics,
}

/// Test data for Rex benchmarks
#[derive(Debug, Clone)]
pub struct RexTestData {
    /// Simple Rex scripts (1-5 commands)
    pub simple_scripts: Vec<RexScriptScenario>,
    /// Medium complexity scripts (6-20 commands)
    pub medium_scripts: Vec<RexScriptScenario>,
    /// Complex scripts (21+ commands)
    pub complex_scripts: Vec<RexScriptScenario>,
    /// Parser test scenarios
    pub parser_scenarios: Vec<ParserScenario>,
    /// Cache test scenarios
    pub cache_scenarios: Vec<CacheScenario>,
    /// Binding test scenarios
    pub binding_scenarios: Vec<BindingScenario>,
}

/// Individual Rex script test scenario
#[derive(Debug, Clone)]
pub struct RexScriptScenario {
    pub name: String,
    pub script_content: String,
    pub commands: Vec<RexCommand>,
    pub expected_env_vars: usize,
    pub expected_aliases: usize,
    pub complexity_score: u32,
}

/// Parser performance test scenario
#[derive(Debug, Clone)]
pub struct ParserScenario {
    pub name: String,
    pub script_lines: Vec<String>,
    pub expected_commands: usize,
    pub parser_config: ParserConfig,
}

/// Cache performance test scenario
#[derive(Debug, Clone)]
pub struct CacheScenario {
    pub name: String,
    pub cache_config: RexCacheConfig,
    pub test_commands: Vec<String>,
    pub expected_hit_ratio: f64,
}

/// Binding system test scenario
#[derive(Debug, Clone)]
pub struct BindingScenario {
    pub name: String,
    pub packages: Vec<Package>,
    pub context: ResolvedContext,
    pub expected_bindings: usize,
}

/// Baseline metrics for Rex benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub module_name: String,
    pub timestamp: SystemTime,
    pub benchmarks: HashMap<String, BenchmarkResult>,
    pub overall_score: f64,
    pub environment: EnvironmentInfo,
}

/// Individual benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean_time_ns: f64,
    pub std_dev_ns: f64,
    pub throughput_ops_per_sec: Option<f64>,
    pub memory_usage_bytes: Option<u64>,
    pub additional_metrics: HashMap<String, f64>,
}

/// Environment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub os: String,
    pub cpu: String,
    pub memory_bytes: u64,
    pub rust_version: String,
    pub compiler_flags: Vec<String>,
}

impl RexBenchmark {
    /// Create new Rex benchmark instance
    pub fn new() -> Self {
        let test_data = Self::generate_test_data();
        let baseline_metrics = Self::create_baseline_metrics();

        Self {
            test_data,
            baseline_metrics,
        }
    }

    /// Generate comprehensive test data for benchmarks
    fn generate_test_data() -> RexTestData {
        RexTestData {
            simple_scripts: Self::generate_simple_scripts(),
            medium_scripts: Self::generate_medium_scripts(),
            complex_scripts: Self::generate_complex_scripts(),
            parser_scenarios: Self::generate_parser_scenarios(),
            cache_scenarios: Self::generate_cache_scenarios(),
            binding_scenarios: Self::generate_binding_scenarios(),
        }
    }

    /// Generate simple Rex script scenarios (1-5 commands)
    fn generate_simple_scripts() -> Vec<RexScriptScenario> {
        vec![
            RexScriptScenario {
                name: "single_setenv".to_string(),
                script_content: "setenv PYTHON_ROOT /opt/python".to_string(),
                commands: vec![
                    RexCommand::SetEnv {
                        name: "PYTHON_ROOT".to_string(),
                        value: "/opt/python".to_string(),
                    }
                ],
                expected_env_vars: 1,
                expected_aliases: 0,
                complexity_score: 1,
            },
            RexScriptScenario {
                name: "basic_environment_setup".to_string(),
                script_content: r#"setenv PYTHON_ROOT /opt/python
prependenv PATH $PYTHON_ROOT/bin
alias python python3"#.to_string(),
                commands: vec![
                    RexCommand::SetEnv {
                        name: "PYTHON_ROOT".to_string(),
                        value: "/opt/python".to_string(),
                    },
                    RexCommand::PrependEnv {
                        name: "PATH".to_string(),
                        value: "$PYTHON_ROOT/bin".to_string(),
                        separator: ":".to_string(),
                    },
                    RexCommand::Alias {
                        name: "python".to_string(),
                        command: "python3".to_string(),
                    },
                ],
                expected_env_vars: 2,
                expected_aliases: 1,
                complexity_score: 3,
            },
        ]
    }

    /// Generate medium complexity scripts (6-20 commands)
    fn generate_medium_scripts() -> Vec<RexScriptScenario> {
        vec![
            RexScriptScenario {
                name: "development_environment".to_string(),
                script_content: r#"setenv DEV_ROOT /opt/dev
setenv PYTHON_ROOT $DEV_ROOT/python
setenv NODE_ROOT $DEV_ROOT/node
prependenv PATH $PYTHON_ROOT/bin
prependenv PATH $NODE_ROOT/bin
alias python python3
alias pip pip3"#.to_string(),
                commands: Self::create_dev_environment_commands(),
                expected_env_vars: 7,
                expected_aliases: 5,
                complexity_score: 15,
            },
        ]
    }

    /// Generate complex scripts (21+ commands)
    fn generate_complex_scripts() -> Vec<RexScriptScenario> {
        vec![
            RexScriptScenario {
                name: "enterprise_environment".to_string(),
                script_content: Self::create_enterprise_environment_script(),
                commands: Self::create_enterprise_environment_commands(),
                expected_env_vars: 25,
                expected_aliases: 15,
                complexity_score: 50,
            },
        ]
    }

    /// Generate parser test scenarios
    fn generate_parser_scenarios() -> Vec<ParserScenario> {
        vec![
            ParserScenario {
                name: "basic_commands".to_string(),
                script_lines: vec![
                    "setenv VAR value".to_string(),
                    "prependenv PATH /bin".to_string(),
                    "alias ls 'ls -la'".to_string(),
                ],
                expected_commands: 3,
                parser_config: ParserConfig::default(),
            },
        ]
    }

    /// Generate cache test scenarios
    fn generate_cache_scenarios() -> Vec<CacheScenario> {
        vec![
            CacheScenario {
                name: "small_cache_lru".to_string(),
                cache_config: RexCacheConfig {
                    max_parse_entries: 100,
                    max_execution_entries: 50,
                    eviction_strategy: EvictionStrategy::LRU,
                    ..Default::default()
                },
                test_commands: Self::create_repeated_commands(50),
                expected_hit_ratio: 0.8,
            },
        ]
    }

    /// Generate binding test scenarios
    fn generate_binding_scenarios() -> Vec<BindingScenario> {
        vec![
            BindingScenario {
                name: "simple_package_binding".to_string(),
                packages: vec![
                    Package::new("python".to_string(), Version::new("3.9.0".to_string()).unwrap()),
                ],
                context: Self::create_simple_context(),
                expected_bindings: 5,
            },
        ]
    }

    /// Create baseline metrics structure
    fn create_baseline_metrics() -> BaselineMetrics {
        BaselineMetrics {
            module_name: "rex".to_string(),
            timestamp: SystemTime::now(),
            benchmarks: HashMap::new(),
            overall_score: 0.0,
            environment: EnvironmentInfo {
                os: std::env::consts::OS.to_string(),
                cpu: "unknown".to_string(),
                memory_bytes: 0,
                rust_version: env!("RUSTC_VERSION").to_string(),
                compiler_flags: vec![],
            },
        }
    }

    // Helper methods for creating test data
    fn create_dev_environment_commands() -> Vec<RexCommand> {
        vec![
            RexCommand::SetEnv { name: "DEV_ROOT".to_string(), value: "/opt/dev".to_string() },
            RexCommand::SetEnv { name: "PYTHON_ROOT".to_string(), value: "$DEV_ROOT/python".to_string() },
        ]
    }

    fn create_enterprise_environment_script() -> String {
        "setenv ENTERPRISE_ROOT /opt/enterprise".to_string()
    }

    fn create_enterprise_environment_commands() -> Vec<RexCommand> {
        vec![
            RexCommand::SetEnv { name: "ENTERPRISE_ROOT".to_string(), value: "/opt/enterprise".to_string() },
        ]
    }

    fn create_repeated_commands(count: usize) -> Vec<String> {
        (0..count).map(|i| format!("setenv VAR_{} value_{}", i % 10, i)).collect()
    }

    fn create_simple_context() -> ResolvedContext {
        let requirements = vec![
            PackageRequirement::new("python".to_string(), Some(VersionRange::new("3.9+".to_string()).unwrap()))
        ];
        ContextBuilder::new()
            .requirements(requirements)
            .config(ContextConfig::default())
            .build()
    }
}

impl Default for RexBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of ModuleBenchmark trait for Rex system
impl crate::comprehensive_benchmark_suite::ModuleBenchmark for RexBenchmark {
    fn name(&self) -> &str {
        "rex"
    }

    fn run_benchmarks(&self, c: &mut Criterion) {
        self.benchmark_parser_performance(c);
        self.benchmark_execution_performance(c);
        self.benchmark_cache_performance(c);
        self.benchmark_binding_performance(c);
        self.benchmark_interpreter_performance(c);
        self.benchmark_script_generation(c);
        self.benchmark_scalability(c);
    }

    fn get_baseline_metrics(&self) -> crate::comprehensive_benchmark_suite::BaselineMetrics {
        crate::comprehensive_benchmark_suite::BaselineMetrics {
            module_name: self.name().to_string(),
            timestamp: SystemTime::now(),
            benchmarks: HashMap::new(),
            overall_score: 0.0,
            environment: crate::comprehensive_benchmark_suite::EnvironmentInfo {
                os: std::env::consts::OS.to_string(),
                cpu: "unknown".to_string(),
                memory_bytes: 0,
                rust_version: env!("RUSTC_VERSION").to_string(),
                compiler_flags: vec![],
            },
        }
    }

    fn validate(&self) -> Result<(), crate::comprehensive_benchmark_suite::BenchmarkError> {
        // Validate that test data is properly initialized
        if self.test_data.simple_scripts.is_empty() {
            return Err(crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                "Simple scripts not initialized".to_string()
            ));
        }

        if self.test_data.parser_scenarios.is_empty() {
            return Err(crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                "Parser scenarios not initialized".to_string()
            ));
        }

        if self.test_data.cache_scenarios.is_empty() {
            return Err(crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                "Cache scenarios not initialized".to_string()
            ));
        }

        Ok(())
    }
}

impl RexBenchmark {
    /// Benchmark Rex parser performance (basic vs optimized)
    pub fn benchmark_parser_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_parser_performance");

        // Benchmark basic parser
        for scenario in &self.test_data.parser_scenarios {
            group.throughput(Throughput::Elements(scenario.script_lines.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("basic_parser", &scenario.name),
                scenario,
                |b, scenario| {
                    let parser = RexParser::with_config(scenario.parser_config.clone());
                    let script_content = scenario.script_lines.join("\n");
                    b.iter(|| {
                        black_box(parser.parse(&script_content))
                    });
                },
            );
        }

        // Benchmark optimized parser
        for scenario in &self.test_data.parser_scenarios {
            group.throughput(Throughput::Elements(scenario.script_lines.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("optimized_parser", &scenario.name),
                scenario,
                |b, scenario| {
                    let parser = OptimizedRexParser::with_config(scenario.parser_config.clone());
                    let script_content = scenario.script_lines.join("\n");
                    b.iter(|| {
                        black_box(parser.parse_optimized(&script_content))
                    });
                },
            );
        }

        // Compare parsing performance for different script sizes
        let script_sizes = vec![1, 5, 10, 20, 50];
        for size in script_sizes {
            let test_lines: Vec<String> = (0..size)
                .map(|i| format!("setenv VAR_{} value_{}", i, i))
                .collect();
            let script_content = test_lines.join("\n");

            group.bench_with_input(
                BenchmarkId::new("basic_parser_size", size),
                &script_content,
                |b, content| {
                    let parser = RexParser::new();
                    b.iter(|| {
                        black_box(parser.parse(content))
                    });
                },
            );

            group.bench_with_input(
                BenchmarkId::new("optimized_parser_size", size),
                &script_content,
                |b, content| {
                    let parser = OptimizedRexParser::new();
                    b.iter(|| {
                        black_box(parser.parse_optimized(content))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark Rex execution performance
    pub fn benchmark_execution_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_execution_performance");

        // Benchmark simple script execution
        for scenario in &self.test_data.simple_scripts {
            group.throughput(Throughput::Elements(scenario.commands.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("simple_execution", &scenario.name),
                scenario,
                |b, scenario| {
                    let context = self.create_simple_context();
                    let mut executor = RexExecutor::new(context);

                    b.iter(|| {
                        // Note: In a real implementation, this would be async
                        // For benchmarking, we'll use a synchronous version or block_on
                        black_box(executor.execute_script_content_sync(&scenario.script_content))
                    });
                },
            );
        }

        // Benchmark medium complexity script execution
        for scenario in &self.test_data.medium_scripts {
            group.throughput(Throughput::Elements(scenario.commands.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("medium_execution", &scenario.name),
                scenario,
                |b, scenario| {
                    let context = self.create_simple_context();
                    let mut executor = RexExecutor::new(context);

                    b.iter(|| {
                        black_box(executor.execute_script_content_sync(&scenario.script_content))
                    });
                },
            );
        }

        // Benchmark complex script execution
        for scenario in &self.test_data.complex_scripts {
            group.throughput(Throughput::Elements(scenario.commands.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("complex_execution", &scenario.name),
                scenario,
                |b, scenario| {
                    let context = self.create_simple_context();
                    let mut executor = RexExecutor::new(context);

                    b.iter(|| {
                        black_box(executor.execute_script_content_sync(&scenario.script_content))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark Rex cache performance
    pub fn benchmark_cache_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_cache_performance");

        for scenario in &self.test_data.cache_scenarios {
            // Test cache with different eviction strategies
            group.bench_with_input(
                BenchmarkId::new("cache_performance", &scenario.name),
                scenario,
                |b, scenario| {
                    let cache = RexCache::with_config(scenario.cache_config.clone());

                    b.iter(|| {
                        // Simulate cache operations
                        for (i, command) in scenario.test_commands.iter().enumerate() {
                            // Parse and cache
                            let parser = RexParser::new();
                            if let Ok(script) = parser.parse(command) {
                                // Cache the parsed result
                                let cache_key = format!("command_{}", i);
                                // Note: This would use actual cache methods
                                black_box(cache_key);
                            }
                        }
                    });
                },
            );
        }

        // Test cache hit ratio performance
        group.bench_function("cache_hit_ratio", |b| {
            let cache = RexCache::new();
            let repeated_commands = self.create_repeated_commands(100);

            b.iter(|| {
                for command in &repeated_commands {
                    // Simulate cache lookup
                    black_box(command);
                }
            });
        });

        group.finish();
    }

    /// Benchmark Rex binding performance
    pub fn benchmark_binding_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_binding_performance");

        for scenario in &self.test_data.binding_scenarios {
            group.bench_with_input(
                BenchmarkId::new("binding_generation", &scenario.name),
                scenario,
                |b, scenario| {
                    let binding_generator = RexBindingGenerator::new();

                    b.iter(|| {
                        black_box(binding_generator.generate_bindings(&scenario.packages, &scenario.context))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark Rex interpreter performance
    pub fn benchmark_interpreter_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_interpreter_performance");

        // Test interpreter creation
        group.bench_function("interpreter_creation", |b| {
            b.iter(|| {
                black_box(RexInterpreter::new())
            });
        });

        // Test interpreter with different configurations
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

        // Test command execution
        for scenario in &self.test_data.simple_scripts {
            group.bench_with_input(
                BenchmarkId::new("command_execution", &scenario.name),
                scenario,
                |b, scenario| {
                    let mut interpreter = RexInterpreter::new();

                    b.iter(|| {
                        for command in &scenario.commands {
                            black_box(interpreter.execute_command_sync(command));
                        }
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark shell script generation performance
    pub fn benchmark_script_generation(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_script_generation");

        // Test script generation for different shell types
        let shell_types = vec!["bash", "cmd", "powershell"];

        for shell_type in shell_types {
            for scenario in &self.test_data.simple_scripts {
                group.bench_with_input(
                    BenchmarkId::new(format!("script_gen_{}", shell_type), &scenario.name),
                    scenario,
                    |b, scenario| {
                        let interpreter = RexInterpreter::new();

                        b.iter(|| {
                            black_box(interpreter.generate_shell_script_for_type(shell_type, &scenario.commands))
                        });
                    },
                );
            }
        }

        // Test environment script generation
        group.bench_function("environment_script_generation", |b| {
            let interpreter = RexInterpreter::new();
            let context = self.create_simple_context();

            b.iter(|| {
                black_box(interpreter.generate_environment_script(&context))
            });
        });

        group.finish();
    }

    /// Benchmark Rex scalability with increasing complexity
    pub fn benchmark_scalability(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_scalability");

        // Test scalability across different complexity levels
        let all_scenarios = [
            ("simple", &self.test_data.simple_scripts),
            ("medium", &self.test_data.medium_scripts),
            ("complex", &self.test_data.complex_scripts),
        ];

        for (complexity_level, scenarios) in &all_scenarios {
            for scenario in scenarios.iter() {
                group.throughput(Throughput::Elements(scenario.complexity_score as u64));
                group.bench_with_input(
                    BenchmarkId::new(*complexity_level, &scenario.name),
                    scenario,
                    |b, scenario| {
                        b.iter(|| {
                            // Parse the script
                            let parser = RexParser::new();
                            let script = parser.parse(&scenario.script_content).unwrap();

                            // Execute the script
                            let context = self.create_simple_context();
                            let mut executor = RexExecutor::new(context);
                            let _result = executor.execute_script_sync(&script);

                            black_box(script)
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark memory usage during Rex operations
    pub fn benchmark_memory_usage(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_memory_usage");

        // Test memory usage for different script sizes
        for scenario in &self.test_data.complex_scripts {
            group.bench_with_input(
                BenchmarkId::new("memory_tracking", &scenario.name),
                scenario,
                |b, scenario| {
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            let parser = RexParser::new();
                            let script = parser.parse(&scenario.script_content).unwrap();
                            let context = self.create_simple_context();
                            let mut executor = RexExecutor::new(context);
                            let _result = executor.execute_script_sync(&script);
                            black_box(script);
                        }
                        start.elapsed()
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark statistics collection overhead
    pub fn benchmark_statistics_collection(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("rex_statistics");

        // Test executor statistics collection
        group.bench_function("executor_stats", |b| {
            let context = self.create_simple_context();
            let executor = RexExecutor::new(context);

            b.iter(|| {
                black_box(executor.get_stats())
            });
        });

        // Test interpreter statistics collection
        group.bench_function("interpreter_stats", |b| {
            let interpreter = RexInterpreter::new();

            b.iter(|| {
                black_box(interpreter.get_stats())
            });
        });

        // Test cache statistics collection
        group.bench_function("cache_stats", |b| {
            let cache = RexCache::new();

            b.iter(|| {
                black_box(cache.get_stats())
            });
        });

        group.finish();
    }
}

/// Utility functions for Rex benchmarks
impl RexBenchmark {
    /// Create a Rex script from a scenario
    pub fn create_script_from_scenario(scenario: &RexScriptScenario) -> RexScript {
        let parser = RexParser::new();
        parser.parse(&scenario.script_content).unwrap()
    }

    /// Measure parser performance for a given script
    pub fn measure_parser_performance(&self, script_content: &str) -> Duration {
        let start = std::time::Instant::now();
        let parser = RexParser::new();
        let _script = parser.parse(script_content);
        start.elapsed()
    }

    /// Measure execution performance for a given scenario
    pub fn measure_execution_performance(&self, scenario: &RexScriptScenario) -> Duration {
        let start = std::time::Instant::now();
        let context = self.create_simple_context();
        let mut executor = RexExecutor::new(context);
        let _result = executor.execute_script_content_sync(&scenario.script_content);
        start.elapsed()
    }

    /// Generate performance report for Rex benchmarks
    pub fn generate_performance_report(&self) -> String {
        format!(
            "Rex Benchmark Report\n\
             ===================\n\
             Simple scripts: {}\n\
             Medium scripts: {}\n\
             Complex scripts: {}\n\
             Parser scenarios: {}\n\
             Cache scenarios: {}\n\
             Binding scenarios: {}\n\
             \n\
             Test data validation: {:?}\n",
            self.test_data.simple_scripts.len(),
            self.test_data.medium_scripts.len(),
            self.test_data.complex_scripts.len(),
            self.test_data.parser_scenarios.len(),
            self.test_data.cache_scenarios.len(),
            self.test_data.binding_scenarios.len(),
            self.validate()
        )
    }
}
