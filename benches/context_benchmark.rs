//! Context System Benchmark Suite
//!
//! Comprehensive benchmarks for the rez-core context system including:
//! - Context creation and building
//! - Environment variable generation
//! - Shell execution and command running
//! - Serialization and deserialization
//! - Caching mechanisms
//! - Execution statistics and performance monitoring

use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use rez_core_common::RezCoreError;
use rez_core_context::{
    ContextBuilder, ContextConfig, ContextExecution, ContextFormat, ContextSerialization,
    ContextStatus, EnvironmentManager, ExecutionConfig, PathStrategy, ResolvedContext,
    ShellExecutor, ShellType,
};
use rez_core_package::{Package, PackageRequirement};
use rez_core_solver::{DependencySolver, SolverConfig, SolverRequest};
use rez_core_version::{Version, VersionRange};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Context benchmark module implementation
pub struct ContextBenchmark {
    /// Test data for benchmarks
    test_data: ContextTestData,
    /// Baseline metrics
    baseline_metrics: BaselineMetrics,
}

/// Test data for context benchmarks
#[derive(Debug, Clone)]
pub struct ContextTestData {
    /// Simple context scenarios (1-5 packages)
    pub simple_contexts: Vec<ContextScenario>,
    /// Medium complexity contexts (6-20 packages)
    pub medium_contexts: Vec<ContextScenario>,
    /// Complex contexts (21+ packages)
    pub complex_contexts: Vec<ContextScenario>,
    /// Environment generation scenarios
    pub env_scenarios: Vec<EnvironmentScenario>,
    /// Shell execution scenarios
    pub shell_scenarios: Vec<ShellScenario>,
    /// Serialization scenarios
    pub serialization_scenarios: Vec<SerializationScenario>,
}

/// Individual context test scenario
#[derive(Debug, Clone)]
pub struct ContextScenario {
    pub name: String,
    pub requirements: Vec<PackageRequirement>,
    pub packages: Vec<Package>,
    pub config: ContextConfig,
    pub expected_env_vars: usize,
    pub complexity_score: u32,
}

/// Environment generation test scenario
#[derive(Debug, Clone)]
pub struct EnvironmentScenario {
    pub name: String,
    pub packages: Vec<Package>,
    pub config: ContextConfig,
    pub expected_vars: usize,
    pub path_modifications: usize,
}

/// Shell execution test scenario
#[derive(Debug, Clone)]
pub struct ShellScenario {
    pub name: String,
    pub shell_type: ShellType,
    pub commands: Vec<String>,
    pub environment: HashMap<String, String>,
    pub expected_duration_ms: u64,
}

/// Serialization test scenario
#[derive(Debug, Clone)]
pub struct SerializationScenario {
    pub name: String,
    pub context: ResolvedContext,
    pub format: ContextFormat,
    pub expected_size_bytes: usize,
}

/// Baseline metrics for context benchmarks
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

impl ContextBenchmark {
    /// Create new context benchmark instance
    pub fn new() -> Self {
        let test_data = Self::generate_test_data();
        let baseline_metrics = Self::create_baseline_metrics();

        Self {
            test_data,
            baseline_metrics,
        }
    }

    /// Generate comprehensive test data for benchmarks
    fn generate_test_data() -> ContextTestData {
        ContextTestData {
            simple_contexts: Self::generate_simple_contexts(),
            medium_contexts: Self::generate_medium_contexts(),
            complex_contexts: Self::generate_complex_contexts(),
            env_scenarios: Self::generate_env_scenarios(),
            shell_scenarios: Self::generate_shell_scenarios(),
            serialization_scenarios: Self::generate_serialization_scenarios(),
        }
    }

    /// Generate simple context scenarios (1-5 packages)
    fn generate_simple_contexts() -> Vec<ContextScenario> {
        vec![
            ContextScenario {
                name: "single_package_context".to_string(),
                requirements: vec![PackageRequirement::new(
                    "python".to_string(),
                    Some(VersionRange::new("3.9+".to_string()).unwrap()),
                )],
                packages: Self::create_python_packages(),
                config: ContextConfig::default(),
                expected_env_vars: 5,
                complexity_score: 1,
            },
            ContextScenario {
                name: "basic_dev_environment".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "python".to_string(),
                        Some(VersionRange::new("3.9+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new("git".to_string(), None),
                ],
                packages: Self::create_dev_packages(),
                config: ContextConfig {
                    inherit_parent_env: true,
                    shell_type: ShellType::Bash,
                    path_strategy: PathStrategy::Prepend,
                    ..Default::default()
                },
                expected_env_vars: 12,
                complexity_score: 3,
            },
            ContextScenario {
                name: "isolated_environment".to_string(),
                requirements: vec![PackageRequirement::new(
                    "node".to_string(),
                    Some(VersionRange::new("18+".to_string()).unwrap()),
                )],
                packages: Self::create_node_packages(),
                config: ContextConfig {
                    inherit_parent_env: false,
                    shell_type: ShellType::Bash,
                    path_strategy: PathStrategy::Replace,
                    ..Default::default()
                },
                expected_env_vars: 8,
                complexity_score: 2,
            },
        ]
    }

    /// Generate medium complexity contexts (6-20 packages)
    fn generate_medium_contexts() -> Vec<ContextScenario> {
        vec![
            ContextScenario {
                name: "web_development_stack".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "python".to_string(),
                        Some(VersionRange::new("3.9+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "node".to_string(),
                        Some(VersionRange::new("18+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new("git".to_string(), None),
                    PackageRequirement::new("docker".to_string(), None),
                ],
                packages: Self::create_web_dev_packages(),
                config: ContextConfig::default(),
                expected_env_vars: 35,
                complexity_score: 15,
            },
            ContextScenario {
                name: "data_science_environment".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "python".to_string(),
                        Some(VersionRange::new("3.9+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new("jupyter".to_string(), None),
                    PackageRequirement::new(
                        "numpy".to_string(),
                        Some(VersionRange::new("1.20+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new("pandas".to_string(), None),
                ],
                packages: Self::create_data_science_packages(),
                config: ContextConfig {
                    additional_env_vars: {
                        let mut vars = HashMap::new();
                        vars.insert("JUPYTER_CONFIG_DIR".to_string(), "/tmp/jupyter".to_string());
                        vars.insert("PYTHONPATH".to_string(), "/opt/data-science".to_string());
                        vars
                    },
                    ..Default::default()
                },
                expected_env_vars: 42,
                complexity_score: 20,
            },
        ]
    }

    /// Generate complex contexts (21+ packages)
    fn generate_complex_contexts() -> Vec<ContextScenario> {
        vec![
            ContextScenario {
                name: "enterprise_development_suite".to_string(),
                requirements: Self::create_enterprise_requirements(),
                packages: Self::create_enterprise_packages(),
                config: ContextConfig {
                    inherit_parent_env: true,
                    additional_env_vars: Self::create_enterprise_env_vars(),
                    unset_vars: vec!["TEMP_VAR".to_string(), "DEBUG_MODE".to_string()],
                    ..Default::default()
                },
                expected_env_vars: 85,
                complexity_score: 50,
            },
            ContextScenario {
                name: "ci_cd_pipeline_environment".to_string(),
                requirements: Self::create_cicd_requirements(),
                packages: Self::create_cicd_packages(),
                config: ContextConfig {
                    inherit_parent_env: false,
                    shell_type: ShellType::Bash,
                    path_strategy: PathStrategy::Prepend,
                    additional_env_vars: Self::create_cicd_env_vars(),
                    ..Default::default()
                },
                expected_env_vars: 65,
                complexity_score: 40,
            },
        ]
    }

    /// Generate environment generation scenarios
    fn generate_env_scenarios() -> Vec<EnvironmentScenario> {
        vec![
            EnvironmentScenario {
                name: "basic_env_generation".to_string(),
                packages: Self::create_python_packages(),
                config: ContextConfig::default(),
                expected_vars: 5,
                path_modifications: 1,
            },
            EnvironmentScenario {
                name: "complex_path_handling".to_string(),
                packages: Self::create_path_heavy_packages(),
                config: ContextConfig {
                    path_strategy: PathStrategy::Prepend,
                    ..Default::default()
                },
                expected_vars: 25,
                path_modifications: 8,
            },
            EnvironmentScenario {
                name: "variable_expansion".to_string(),
                packages: Self::create_variable_expansion_packages(),
                config: ContextConfig {
                    additional_env_vars: {
                        let mut vars = HashMap::new();
                        vars.insert("BASE_PATH".to_string(), "/opt/base".to_string());
                        vars.insert("EXPANDED_PATH".to_string(), "${BASE_PATH}/bin".to_string());
                        vars
                    },
                    ..Default::default()
                },
                expected_vars: 15,
                path_modifications: 3,
            },
        ]
    }

    /// Generate shell execution scenarios
    fn generate_shell_scenarios() -> Vec<ShellScenario> {
        vec![
            ShellScenario {
                name: "simple_command_execution".to_string(),
                shell_type: ShellType::Bash,
                commands: vec!["echo 'Hello World'".to_string(), "pwd".to_string()],
                environment: HashMap::new(),
                expected_duration_ms: 100,
            },
            ShellScenario {
                name: "environment_dependent_commands".to_string(),
                shell_type: ShellType::Bash,
                commands: vec!["python --version".to_string(), "which python".to_string()],
                environment: {
                    let mut env = HashMap::new();
                    env.insert("PATH".to_string(), "/opt/python/bin:/usr/bin".to_string());
                    env.insert("PYTHON_ROOT".to_string(), "/opt/python".to_string());
                    env
                },
                expected_duration_ms: 200,
            },
            ShellScenario {
                name: "cross_platform_commands".to_string(),
                shell_type: ShellType::detect(),
                commands: vec!["echo $HOME".to_string(), "ls -la".to_string()],
                environment: HashMap::new(),
                expected_duration_ms: 150,
            },
        ]
    }

    /// Generate serialization scenarios
    fn generate_serialization_scenarios() -> Vec<SerializationScenario> {
        vec![
            SerializationScenario {
                name: "json_serialization".to_string(),
                context: Self::create_sample_context(),
                format: ContextFormat::Json,
                expected_size_bytes: 2048,
            },
            SerializationScenario {
                name: "yaml_serialization".to_string(),
                context: Self::create_sample_context(),
                format: ContextFormat::Yaml,
                expected_size_bytes: 1800,
            },
            SerializationScenario {
                name: "binary_serialization".to_string(),
                context: Self::create_sample_context(),
                format: ContextFormat::Binary,
                expected_size_bytes: 1200,
            },
        ]
    }

    /// Create baseline metrics structure
    fn create_baseline_metrics() -> BaselineMetrics {
        BaselineMetrics {
            module_name: "context".to_string(),
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
    fn create_python_packages() -> Vec<Package> {
        vec![Package::new(
            "python".to_string(),
            Version::new("3.9.0".to_string()).unwrap(),
        )]
    }

    fn create_dev_packages() -> Vec<Package> {
        vec![
            Package::new(
                "python".to_string(),
                Version::new("3.9.0".to_string()).unwrap(),
            ),
            Package::new(
                "git".to_string(),
                Version::new("2.40.0".to_string()).unwrap(),
            ),
        ]
    }

    fn create_node_packages() -> Vec<Package> {
        vec![Package::new(
            "node".to_string(),
            Version::new("18.17.0".to_string()).unwrap(),
        )]
    }

    fn create_web_dev_packages() -> Vec<Package> {
        (0..12)
            .map(|i| {
                Package::new(
                    format!("web_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_data_science_packages() -> Vec<Package> {
        (0..18)
            .map(|i| {
                Package::new(
                    format!("ds_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_enterprise_packages() -> Vec<Package> {
        (0..35)
            .map(|i| {
                Package::new(
                    format!("enterprise_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_cicd_packages() -> Vec<Package> {
        (0..25)
            .map(|i| {
                Package::new(
                    format!("cicd_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_path_heavy_packages() -> Vec<Package> {
        (0..10)
            .map(|i| {
                Package::new(
                    format!("path_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_variable_expansion_packages() -> Vec<Package> {
        (0..8)
            .map(|i| {
                Package::new(
                    format!("var_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_enterprise_requirements() -> Vec<PackageRequirement> {
        vec![
            PackageRequirement::new(
                "java".to_string(),
                Some(VersionRange::new("11+".to_string()).unwrap()),
            ),
            PackageRequirement::new(
                "maven".to_string(),
                Some(VersionRange::new("3.8+".to_string()).unwrap()),
            ),
            PackageRequirement::new("docker".to_string(), None),
            PackageRequirement::new("kubernetes".to_string(), None),
        ]
    }

    fn create_cicd_requirements() -> Vec<PackageRequirement> {
        vec![
            PackageRequirement::new("jenkins".to_string(), None),
            PackageRequirement::new("git".to_string(), None),
            PackageRequirement::new("docker".to_string(), None),
        ]
    }

    fn create_enterprise_env_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("JAVA_HOME".to_string(), "/opt/java".to_string());
        vars.insert("MAVEN_HOME".to_string(), "/opt/maven".to_string());
        vars.insert(
            "DOCKER_HOST".to_string(),
            "unix:///var/run/docker.sock".to_string(),
        );
        vars
    }

    fn create_cicd_env_vars() -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("CI".to_string(), "true".to_string());
        vars.insert("BUILD_NUMBER".to_string(), "123".to_string());
        vars.insert("GIT_BRANCH".to_string(), "main".to_string());
        vars
    }

    fn create_sample_context() -> ResolvedContext {
        let requirements = vec![PackageRequirement::new(
            "python".to_string(),
            Some(VersionRange::new("3.9+".to_string()).unwrap()),
        )];
        ResolvedContext::from_requirements(requirements)
    }
}

impl Default for ContextBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of ModuleBenchmark trait for Context system
impl crate::comprehensive_benchmark_suite::ModuleBenchmark for ContextBenchmark {
    fn name(&self) -> &str {
        "context"
    }

    fn run_benchmarks(&self, c: &mut Criterion) {
        self.benchmark_context_creation(c);
        self.benchmark_environment_generation(c);
        self.benchmark_shell_execution(c);
        self.benchmark_serialization(c);
        self.benchmark_context_caching(c);
        self.benchmark_execution_performance(c);
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
        if self.test_data.simple_contexts.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Simple contexts not initialized".to_string(),
                ),
            );
        }

        if self.test_data.env_scenarios.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Environment scenarios not initialized".to_string(),
                ),
            );
        }

        if self.test_data.shell_scenarios.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Shell scenarios not initialized".to_string(),
                ),
            );
        }

        Ok(())
    }
}

impl ContextBenchmark {
    /// Benchmark context creation and building performance
    pub fn benchmark_context_creation(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_creation");

        // Benchmark simple context creation
        for scenario in &self.test_data.simple_contexts {
            group.throughput(Throughput::Elements(scenario.packages.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("simple", &scenario.name),
                scenario,
                |b, scenario| {
                    b.iter(|| {
                        let builder = ContextBuilder::new()
                            .requirements(scenario.requirements.clone())
                            .config(scenario.config.clone());
                        black_box(builder.build())
                    });
                },
            );
        }

        // Benchmark medium complexity contexts
        for scenario in &self.test_data.medium_contexts {
            group.throughput(Throughput::Elements(scenario.packages.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("medium", &scenario.name),
                scenario,
                |b, scenario| {
                    b.iter(|| {
                        let builder = ContextBuilder::new()
                            .requirements(scenario.requirements.clone())
                            .config(scenario.config.clone());
                        black_box(builder.build())
                    });
                },
            );
        }

        // Benchmark complex contexts
        for scenario in &self.test_data.complex_contexts {
            group.throughput(Throughput::Elements(scenario.packages.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("complex", &scenario.name),
                scenario,
                |b, scenario| {
                    b.iter(|| {
                        let builder = ContextBuilder::new()
                            .requirements(scenario.requirements.clone())
                            .config(scenario.config.clone());
                        black_box(builder.build())
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark environment variable generation performance
    pub fn benchmark_environment_generation(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("environment_generation");

        for scenario in &self.test_data.env_scenarios {
            group.throughput(Throughput::Elements(scenario.packages.len() as u64));
            group.bench_with_input(
                BenchmarkId::new("env_gen", &scenario.name),
                scenario,
                |b, scenario| {
                    let env_manager = EnvironmentManager::new(scenario.config.clone());
                    b.iter(|| {
                        // Note: In a real implementation, this would be async
                        // For benchmarking, we'll use a synchronous version or block_on
                        black_box(env_manager.generate_environment_sync(&scenario.packages))
                    });
                },
            );
        }

        // Benchmark different path strategies
        let packages = self.test_data.env_scenarios[0].packages.clone();
        for strategy in &[
            PathStrategy::Prepend,
            PathStrategy::Append,
            PathStrategy::Replace,
        ] {
            group.bench_with_input(
                BenchmarkId::new("path_strategy", format!("{:?}", strategy)),
                strategy,
                |b, strategy| {
                    let config = ContextConfig {
                        path_strategy: strategy.clone(),
                        ..Default::default()
                    };
                    let env_manager = EnvironmentManager::new(config);
                    b.iter(|| black_box(env_manager.generate_environment_sync(&packages)));
                },
            );
        }

        group.finish();
    }

    /// Benchmark shell execution performance
    pub fn benchmark_shell_execution(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("shell_execution");

        for scenario in &self.test_data.shell_scenarios {
            group.bench_with_input(
                BenchmarkId::new("shell_exec", &scenario.name),
                scenario,
                |b, scenario| {
                    let executor = ShellExecutor::with_shell(scenario.shell_type.clone())
                        .with_environment(scenario.environment.clone());

                    b.iter(|| {
                        for command in &scenario.commands {
                            // Note: In a real implementation, this would be async
                            // For benchmarking, we'll use a synchronous version
                            black_box(executor.execute_sync(command));
                        }
                    });
                },
            );
        }

        // Benchmark different shell types
        let simple_command = "echo 'test'";
        for shell_type in &[ShellType::Bash, ShellType::Cmd, ShellType::PowerShell] {
            group.bench_with_input(
                BenchmarkId::new("shell_type", format!("{:?}", shell_type)),
                shell_type,
                |b, shell_type| {
                    let executor = ShellExecutor::with_shell(shell_type.clone());
                    b.iter(|| black_box(executor.execute_sync(simple_command)));
                },
            );
        }

        group.finish();
    }

    /// Benchmark serialization and deserialization performance
    pub fn benchmark_serialization(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_serialization");

        for scenario in &self.test_data.serialization_scenarios {
            // Benchmark serialization
            group.bench_with_input(
                BenchmarkId::new(
                    "serialize",
                    format!("{}_{:?}", scenario.name, scenario.format),
                ),
                scenario,
                |b, scenario| {
                    b.iter(|| {
                        black_box(ContextSerialization::serialize(
                            &scenario.context,
                            scenario.format,
                        ))
                    });
                },
            );

            // Benchmark deserialization
            let serialized_data =
                ContextSerialization::serialize(&scenario.context, scenario.format).unwrap();
            group.bench_with_input(
                BenchmarkId::new(
                    "deserialize",
                    format!("{}_{:?}", scenario.name, scenario.format),
                ),
                &(scenario, serialized_data),
                |b, (scenario, data)| {
                    b.iter(|| black_box(ContextSerialization::deserialize(data, scenario.format)));
                },
            );
        }

        group.finish();
    }

    /// Benchmark context caching performance
    pub fn benchmark_context_caching(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_caching");

        // Test context fingerprint generation
        group.bench_function("fingerprint_generation", |b| {
            let context = self.test_data.simple_contexts[0].clone();
            let resolved_context = ContextBuilder::new()
                .requirements(context.requirements)
                .config(context.config)
                .build();

            b.iter(|| black_box(resolved_context.get_fingerprint()));
        });

        // Test context lookup performance
        group.bench_function("context_lookup", |b| {
            let contexts: Vec<_> = self
                .test_data
                .simple_contexts
                .iter()
                .map(|scenario| {
                    ContextBuilder::new()
                        .requirements(scenario.requirements.clone())
                        .config(scenario.config.clone())
                        .build()
                })
                .collect();

            b.iter(|| {
                for context in &contexts {
                    black_box(context.get_fingerprint());
                }
            });
        });

        group.finish();
    }

    /// Benchmark execution performance
    pub fn benchmark_execution_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("execution_performance");

        // Test context execution setup
        group.bench_function("execution_setup", |b| {
            let context = ContextBuilder::new()
                .requirements(self.test_data.simple_contexts[0].requirements.clone())
                .config(self.test_data.simple_contexts[0].config.clone())
                .build();

            b.iter(|| {
                let config = ExecutionConfig::default();
                black_box(ContextExecution::new(context.clone(), config))
            });
        });

        // Test execution statistics collection
        group.bench_function("execution_stats", |b| {
            let context = ContextBuilder::new()
                .requirements(self.test_data.simple_contexts[0].requirements.clone())
                .config(self.test_data.simple_contexts[0].config.clone())
                .build();
            let config = ExecutionConfig::default();
            let execution = ContextExecution::new(context, config);

            b.iter(|| black_box(execution.get_execution_stats()));
        });

        // Test tool availability checking
        group.bench_function("tool_availability", |b| {
            let context = ContextBuilder::new()
                .requirements(self.test_data.simple_contexts[0].requirements.clone())
                .config(self.test_data.simple_contexts[0].config.clone())
                .build();
            let config = ExecutionConfig::default();
            let execution = ContextExecution::new(context, config);

            b.iter(|| black_box(execution.get_available_tools()));
        });

        group.finish();
    }

    /// Benchmark context scalability with increasing complexity
    pub fn benchmark_scalability(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_scalability");

        // Test scalability across different complexity levels
        let all_scenarios = [
            ("simple", &self.test_data.simple_contexts),
            ("medium", &self.test_data.medium_contexts),
            ("complex", &self.test_data.complex_contexts),
        ];

        for (complexity_level, scenarios) in &all_scenarios {
            for scenario in scenarios.iter() {
                group.throughput(Throughput::Elements(scenario.complexity_score as u64));
                group.bench_with_input(
                    BenchmarkId::new(*complexity_level, &scenario.name),
                    scenario,
                    |b, scenario| {
                        b.iter(|| {
                            let builder = ContextBuilder::new()
                                .requirements(scenario.requirements.clone())
                                .config(scenario.config.clone());
                            let context = builder.build();

                            // Also test environment generation for scalability
                            let env_manager = EnvironmentManager::new(scenario.config.clone());
                            let _env = env_manager.generate_environment_sync(&scenario.packages);

                            black_box(context)
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark memory usage during context operations
    pub fn benchmark_memory_usage(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_memory_usage");

        // Test memory usage for different context sizes
        for scenario in &self.test_data.complex_contexts {
            group.bench_with_input(
                BenchmarkId::new("memory_tracking", &scenario.name),
                scenario,
                |b, scenario| {
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            let builder = ContextBuilder::new()
                                .requirements(scenario.requirements.clone())
                                .config(scenario.config.clone());
                            let context = builder.build();
                            let env_manager = EnvironmentManager::new(scenario.config.clone());
                            let _env = env_manager.generate_environment_sync(&scenario.packages);
                            black_box(context);
                        }
                        start.elapsed()
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark context validation and error handling
    pub fn benchmark_validation(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("context_validation");

        // Test context validation
        group.bench_function("context_validation", |b| {
            let context = ContextBuilder::new()
                .requirements(self.test_data.simple_contexts[0].requirements.clone())
                .config(self.test_data.simple_contexts[0].config.clone())
                .build();

            b.iter(|| black_box(context.validate()));
        });

        // Test environment variable validation
        group.bench_function("env_var_validation", |b| {
            let env_manager = EnvironmentManager::new(ContextConfig::default());
            let packages = &self.test_data.env_scenarios[0].packages;

            b.iter(|| {
                let env = env_manager.generate_environment_sync(packages);
                black_box(env)
            });
        });

        group.finish();
    }
}

/// Utility functions for context benchmarks
impl ContextBenchmark {
    /// Create a context from a scenario
    pub fn create_context_from_scenario(scenario: &ContextScenario) -> ResolvedContext {
        ContextBuilder::new()
            .requirements(scenario.requirements.clone())
            .config(scenario.config.clone())
            .build()
    }

    /// Measure environment generation performance
    pub fn measure_env_generation_performance(&self, scenario: &EnvironmentScenario) -> Duration {
        let start = std::time::Instant::now();
        let env_manager = EnvironmentManager::new(scenario.config.clone());
        let _env = env_manager.generate_environment_sync(&scenario.packages);
        start.elapsed()
    }

    /// Measure shell execution performance
    pub fn measure_shell_execution_performance(&self, scenario: &ShellScenario) -> Duration {
        let start = std::time::Instant::now();
        let executor = ShellExecutor::with_shell(scenario.shell_type.clone())
            .with_environment(scenario.environment.clone());

        for command in &scenario.commands {
            let _result = executor.execute_sync(command);
        }
        start.elapsed()
    }

    /// Generate performance report for context benchmarks
    pub fn generate_performance_report(&self) -> String {
        format!(
            "Context Benchmark Report\n\
             ========================\n\
             Simple contexts: {}\n\
             Medium contexts: {}\n\
             Complex contexts: {}\n\
             Environment scenarios: {}\n\
             Shell scenarios: {}\n\
             Serialization scenarios: {}\n\
             \n\
             Test data validation: {:?}\n",
            self.test_data.simple_contexts.len(),
            self.test_data.medium_contexts.len(),
            self.test_data.complex_contexts.len(),
            self.test_data.env_scenarios.len(),
            self.test_data.shell_scenarios.len(),
            self.test_data.serialization_scenarios.len(),
            self.validate()
        )
    }
}
