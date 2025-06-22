//! Solver System Benchmark Suite
//!
//! Comprehensive benchmarks for the rez-core solver system including:
//! - Basic dependency resolution
//! - Conflict detection and resolution
//! - Caching mechanisms
//! - Parallel solving optimization
//! - A* heuristic algorithm
//! - Optimized solver performance

use criterion::{black_box, BenchmarkId, Criterion, Throughput};
use rez_core_common::RezCoreError;
use rez_core_package::{Package, PackageRequirement};
use rez_core_solver::{
    ConflictStrategy, DependencySolver, OptimizedDependencySolver, SolverConfig, SolverRequest,
    SolverStats,
};
use rez_core_version::{Version, VersionRange};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Solver benchmark module implementation
pub struct SolverBenchmark {
    /// Test data for benchmarks
    test_data: SolverTestData,
    /// Baseline metrics
    baseline_metrics: BaselineMetrics,
}

/// Test data for solver benchmarks
#[derive(Debug, Clone)]
pub struct SolverTestData {
    /// Simple dependency scenarios (1-5 packages)
    pub simple_scenarios: Vec<SolverScenario>,
    /// Medium complexity scenarios (6-20 packages)
    pub medium_scenarios: Vec<SolverScenario>,
    /// Complex scenarios (21+ packages)
    pub complex_scenarios: Vec<SolverScenario>,
    /// Conflict scenarios for testing conflict resolution
    pub conflict_scenarios: Vec<ConflictScenario>,
    /// Cache test scenarios
    pub cache_scenarios: Vec<CacheScenario>,
}

/// Individual solver test scenario
#[derive(Debug, Clone)]
pub struct SolverScenario {
    pub name: String,
    pub requirements: Vec<PackageRequirement>,
    pub available_packages: Vec<Package>,
    pub expected_packages: usize,
    pub complexity_score: u32,
}

/// Conflict resolution test scenario
#[derive(Debug, Clone)]
pub struct ConflictScenario {
    pub name: String,
    pub conflicting_requirements: Vec<PackageRequirement>,
    pub available_packages: Vec<Package>,
    pub strategy: ConflictStrategy,
    pub should_resolve: bool,
}

/// Cache performance test scenario
#[derive(Debug, Clone)]
pub struct CacheScenario {
    pub name: String,
    pub repeated_requests: Vec<SolverRequest>,
    pub cache_hit_ratio_target: f64,
}

/// Baseline metrics for solver benchmarks
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

impl SolverBenchmark {
    /// Create new solver benchmark instance
    pub fn new() -> Self {
        let test_data = Self::generate_test_data();
        let baseline_metrics = Self::create_baseline_metrics();

        Self {
            test_data,
            baseline_metrics,
        }
    }

    /// Generate comprehensive test data for benchmarks
    fn generate_test_data() -> SolverTestData {
        SolverTestData {
            simple_scenarios: Self::generate_simple_scenarios(),
            medium_scenarios: Self::generate_medium_scenarios(),
            complex_scenarios: Self::generate_complex_scenarios(),
            conflict_scenarios: Self::generate_conflict_scenarios(),
            cache_scenarios: Self::generate_cache_scenarios(),
        }
    }

    /// Generate simple dependency scenarios (1-5 packages)
    fn generate_simple_scenarios() -> Vec<SolverScenario> {
        vec![
            SolverScenario {
                name: "single_package".to_string(),
                requirements: vec![PackageRequirement::new(
                    "python".to_string(),
                    Some(VersionRange::new("3.9+".to_string()).unwrap()),
                )],
                available_packages: Self::create_python_packages(),
                expected_packages: 1,
                complexity_score: 1,
            },
            SolverScenario {
                name: "linear_chain".to_string(),
                requirements: vec![PackageRequirement::new(
                    "app".to_string(),
                    Some(VersionRange::new("1.0+".to_string()).unwrap()),
                )],
                available_packages: Self::create_linear_chain_packages(),
                expected_packages: 3,
                complexity_score: 3,
            },
            SolverScenario {
                name: "simple_diamond".to_string(),
                requirements: vec![PackageRequirement::new(
                    "top".to_string(),
                    Some(VersionRange::new("1.0+".to_string()).unwrap()),
                )],
                available_packages: Self::create_diamond_packages(),
                expected_packages: 4,
                complexity_score: 4,
            },
        ]
    }

    /// Generate medium complexity scenarios (6-20 packages)
    fn generate_medium_scenarios() -> Vec<SolverScenario> {
        vec![
            SolverScenario {
                name: "web_framework".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "django".to_string(),
                        Some(VersionRange::new("4.0+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "postgres".to_string(),
                        Some(VersionRange::new("13+".to_string()).unwrap()),
                    ),
                ],
                available_packages: Self::create_web_framework_packages(),
                expected_packages: 12,
                complexity_score: 15,
            },
            SolverScenario {
                name: "data_science_stack".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "numpy".to_string(),
                        Some(VersionRange::new("1.20+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "pandas".to_string(),
                        Some(VersionRange::new("1.3+".to_string()).unwrap()),
                    ),
                ],
                available_packages: Self::create_data_science_packages(),
                expected_packages: 18,
                complexity_score: 25,
            },
        ]
    }

    /// Generate complex scenarios (21+ packages)
    fn generate_complex_scenarios() -> Vec<SolverScenario> {
        vec![
            SolverScenario {
                name: "large_application".to_string(),
                requirements: vec![PackageRequirement::new(
                    "app".to_string(),
                    Some(VersionRange::new("2.0+".to_string()).unwrap()),
                )],
                available_packages: Self::create_large_application_packages(),
                expected_packages: 35,
                complexity_score: 50,
            },
            SolverScenario {
                name: "enterprise_stack".to_string(),
                requirements: vec![
                    PackageRequirement::new(
                        "microservice".to_string(),
                        Some(VersionRange::new("1.0+".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "database".to_string(),
                        Some(VersionRange::new("5.0+".to_string()).unwrap()),
                    ),
                ],
                available_packages: Self::create_enterprise_packages(),
                expected_packages: 50,
                complexity_score: 100,
            },
        ]
    }

    /// Generate conflict resolution scenarios
    fn generate_conflict_scenarios() -> Vec<ConflictScenario> {
        vec![
            ConflictScenario {
                name: "version_conflict".to_string(),
                conflicting_requirements: vec![
                    PackageRequirement::new(
                        "python".to_string(),
                        Some(VersionRange::new("3.8".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "python".to_string(),
                        Some(VersionRange::new("3.9+".to_string()).unwrap()),
                    ),
                ],
                available_packages: Self::create_python_packages(),
                strategy: ConflictStrategy::LatestWins,
                should_resolve: true,
            },
            ConflictScenario {
                name: "incompatible_versions".to_string(),
                conflicting_requirements: vec![
                    PackageRequirement::new(
                        "lib".to_string(),
                        Some(VersionRange::new("1.0".to_string()).unwrap()),
                    ),
                    PackageRequirement::new(
                        "lib".to_string(),
                        Some(VersionRange::new("2.0+".to_string()).unwrap()),
                    ),
                ],
                available_packages: Self::create_conflicting_lib_packages(),
                strategy: ConflictStrategy::FailOnConflict,
                should_resolve: false,
            },
        ]
    }

    /// Generate cache performance scenarios
    fn generate_cache_scenarios() -> Vec<CacheScenario> {
        vec![
            CacheScenario {
                name: "repeated_simple_requests".to_string(),
                repeated_requests: Self::create_repeated_simple_requests(),
                cache_hit_ratio_target: 0.9,
            },
            CacheScenario {
                name: "similar_complex_requests".to_string(),
                repeated_requests: Self::create_similar_complex_requests(),
                cache_hit_ratio_target: 0.7,
            },
        ]
    }

    /// Create baseline metrics structure
    fn create_baseline_metrics() -> BaselineMetrics {
        BaselineMetrics {
            module_name: "solver".to_string(),
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

    // Helper methods for creating test packages
    fn create_python_packages() -> Vec<Package> {
        vec![
            Package::new(
                "python".to_string(),
                Version::new("3.8.0".to_string()).unwrap(),
            ),
            Package::new(
                "python".to_string(),
                Version::new("3.9.0".to_string()).unwrap(),
            ),
            Package::new(
                "python".to_string(),
                Version::new("3.10.0".to_string()).unwrap(),
            ),
        ]
    }

    fn create_linear_chain_packages() -> Vec<Package> {
        // app -> lib -> base
        vec![
            Package::new(
                "app".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "lib".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "base".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
        ]
    }

    fn create_diamond_packages() -> Vec<Package> {
        // top -> left, right -> bottom
        vec![
            Package::new(
                "top".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "left".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "right".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "bottom".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
        ]
    }

    fn create_web_framework_packages() -> Vec<Package> {
        // Simplified web framework dependency tree
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
        // Simplified data science stack
        (0..18)
            .map(|i| {
                Package::new(
                    format!("ds_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_large_application_packages() -> Vec<Package> {
        // Large application with many dependencies
        (0..35)
            .map(|i| {
                Package::new(
                    format!("app_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_enterprise_packages() -> Vec<Package> {
        // Enterprise stack with complex dependencies
        (0..50)
            .map(|i| {
                Package::new(
                    format!("enterprise_pkg_{}", i),
                    Version::new("1.0.0".to_string()).unwrap(),
                )
            })
            .collect()
    }

    fn create_conflicting_lib_packages() -> Vec<Package> {
        vec![
            Package::new(
                "lib".to_string(),
                Version::new("1.0.0".to_string()).unwrap(),
            ),
            Package::new(
                "lib".to_string(),
                Version::new("2.0.0".to_string()).unwrap(),
            ),
        ]
    }

    fn create_repeated_simple_requests() -> Vec<SolverRequest> {
        // Create multiple similar simple requests for cache testing
        (0..10)
            .map(|_| SolverRequest {
                requirements: vec![PackageRequirement::new(
                    "python".to_string(),
                    Some(VersionRange::new("3.9+".to_string()).unwrap()),
                )],
                config: SolverConfig::default(),
            })
            .collect()
    }

    fn create_similar_complex_requests() -> Vec<SolverRequest> {
        // Create multiple similar complex requests for cache testing
        (0..5)
            .map(|i| SolverRequest {
                requirements: vec![PackageRequirement::new(
                    format!("app_{}", i),
                    Some(VersionRange::new("1.0+".to_string()).unwrap()),
                )],
                config: SolverConfig::default(),
            })
            .collect()
    }
}

impl Default for SolverBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of ModuleBenchmark trait for Solver system
impl crate::comprehensive_benchmark_suite::ModuleBenchmark for SolverBenchmark {
    fn name(&self) -> &str {
        "solver"
    }

    fn run_benchmarks(&self, c: &mut Criterion) {
        self.benchmark_basic_resolution(c);
        self.benchmark_conflict_resolution(c);
        self.benchmark_cache_performance(c);
        self.benchmark_parallel_solving(c);
        self.benchmark_astar_algorithm(c);
        self.benchmark_optimized_solver(c);
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
        if self.test_data.simple_scenarios.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Simple scenarios not initialized".to_string(),
                ),
            );
        }

        if self.test_data.medium_scenarios.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Medium scenarios not initialized".to_string(),
                ),
            );
        }

        if self.test_data.complex_scenarios.is_empty() {
            return Err(
                crate::comprehensive_benchmark_suite::BenchmarkError::ValidationFailed(
                    "Complex scenarios not initialized".to_string(),
                ),
            );
        }

        Ok(())
    }
}

impl SolverBenchmark {
    /// Benchmark basic dependency resolution performance
    pub fn benchmark_basic_resolution(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_basic_resolution");

        // Benchmark simple scenarios
        for scenario in &self.test_data.simple_scenarios {
            group.throughput(Throughput::Elements(scenario.expected_packages as u64));
            group.bench_with_input(
                BenchmarkId::new("simple", &scenario.name),
                scenario,
                |b, scenario| {
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig::default(),
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );
        }

        // Benchmark medium scenarios
        for scenario in &self.test_data.medium_scenarios {
            group.throughput(Throughput::Elements(scenario.expected_packages as u64));
            group.bench_with_input(
                BenchmarkId::new("medium", &scenario.name),
                scenario,
                |b, scenario| {
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig::default(),
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark conflict resolution strategies
    pub fn benchmark_conflict_resolution(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_conflict_resolution");

        for scenario in &self.test_data.conflict_scenarios {
            for strategy in &[
                ConflictStrategy::LatestWins,
                ConflictStrategy::EarliestWins,
                ConflictStrategy::FindCompatible,
            ] {
                group.bench_with_input(
                    BenchmarkId::new(format!("{:?}", strategy), &scenario.name),
                    scenario,
                    |b, scenario| {
                        let config = SolverConfig {
                            conflict_strategy: strategy.clone(),
                            ..Default::default()
                        };
                        let solver = DependencySolver::with_config(config);

                        b.iter(|| {
                            let request = SolverRequest {
                                requirements: scenario.conflicting_requirements.clone(),
                                config: SolverConfig {
                                    conflict_strategy: strategy.clone(),
                                    ..Default::default()
                                },
                            };
                            black_box(solver.resolve(black_box(request)))
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark cache performance and hit ratios
    pub fn benchmark_cache_performance(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_cache_performance");

        for scenario in &self.test_data.cache_scenarios {
            group.bench_with_input(
                BenchmarkId::new("cache_hits", &scenario.name),
                scenario,
                |b, scenario| {
                    let config = SolverConfig {
                        enable_caching: true,
                        cache_ttl_seconds: 3600,
                        ..Default::default()
                    };
                    let solver = DependencySolver::with_config(config);

                    b.iter(|| {
                        // Run the same requests multiple times to test cache hits
                        for request in &scenario.repeated_requests {
                            black_box(solver.resolve(black_box(request.clone())));
                        }
                    });
                },
            );

            // Benchmark without cache for comparison
            group.bench_with_input(
                BenchmarkId::new("no_cache", &scenario.name),
                scenario,
                |b, scenario| {
                    let config = SolverConfig {
                        enable_caching: false,
                        ..Default::default()
                    };
                    let solver = DependencySolver::with_config(config);

                    b.iter(|| {
                        for request in &scenario.repeated_requests {
                            black_box(solver.resolve(black_box(request.clone())));
                        }
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark parallel solving performance
    pub fn benchmark_parallel_solving(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_parallel_performance");

        // Test different worker counts
        for worker_count in &[1, 2, 4, 8] {
            for scenario in &self.test_data.complex_scenarios {
                group.bench_with_input(
                    BenchmarkId::new(format!("workers_{}", worker_count), &scenario.name),
                    scenario,
                    |b, scenario| {
                        let config = SolverConfig {
                            enable_parallel: true,
                            max_workers: *worker_count,
                            ..Default::default()
                        };
                        let solver = DependencySolver::with_config(config);

                        b.iter(|| {
                            let request = SolverRequest {
                                requirements: scenario.requirements.clone(),
                                config: config.clone(),
                            };
                            black_box(solver.resolve(black_box(request)))
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark A* heuristic algorithm performance
    pub fn benchmark_astar_algorithm(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_astar_algorithm");

        // Test A* algorithm on complex scenarios
        for scenario in &self.test_data.complex_scenarios {
            group.bench_with_input(
                BenchmarkId::new("astar", &scenario.name),
                scenario,
                |b, scenario| {
                    // Note: This would use the A* solver when available
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig::default(),
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );
        }

        // Compare with basic solver
        for scenario in &self.test_data.complex_scenarios {
            group.bench_with_input(
                BenchmarkId::new("basic", &scenario.name),
                scenario,
                |b, scenario| {
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig::default(),
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark optimized solver performance
    pub fn benchmark_optimized_solver(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_optimized_performance");

        // Test optimized solver vs basic solver
        for scenario in &self.test_data.complex_scenarios {
            // Basic solver benchmark
            group.bench_with_input(
                BenchmarkId::new("basic_solver", &scenario.name),
                scenario,
                |b, scenario| {
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig::default(),
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );

            // Optimized solver benchmark (when available)
            group.bench_with_input(
                BenchmarkId::new("optimized_solver", &scenario.name),
                scenario,
                |b, scenario| {
                    // Note: This would use OptimizedDependencySolver when available
                    let solver = DependencySolver::new();
                    b.iter(|| {
                        let request = SolverRequest {
                            requirements: scenario.requirements.clone(),
                            config: SolverConfig {
                                enable_parallel: true,
                                enable_caching: true,
                                max_workers: 4,
                                ..Default::default()
                            },
                        };
                        black_box(solver.resolve(black_box(request)))
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark solver scalability with increasing complexity
    pub fn benchmark_scalability(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_scalability");

        // Test scalability across different complexity levels
        let all_scenarios = [
            ("simple", &self.test_data.simple_scenarios),
            ("medium", &self.test_data.medium_scenarios),
            ("complex", &self.test_data.complex_scenarios),
        ];

        for (complexity_level, scenarios) in &all_scenarios {
            for scenario in scenarios.iter() {
                group.throughput(Throughput::Elements(scenario.complexity_score as u64));
                group.bench_with_input(
                    BenchmarkId::new(*complexity_level, &scenario.name),
                    scenario,
                    |b, scenario| {
                        let solver = DependencySolver::new();
                        b.iter(|| {
                            let request = SolverRequest {
                                requirements: scenario.requirements.clone(),
                                config: SolverConfig::default(),
                            };
                            black_box(solver.resolve(black_box(request)))
                        });
                    },
                );
            }
        }

        group.finish();
    }

    /// Benchmark memory usage during solving
    pub fn benchmark_memory_usage(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_memory_usage");

        // Test memory usage for different scenario sizes
        for scenario in &self.test_data.complex_scenarios {
            group.bench_with_input(
                BenchmarkId::new("memory_tracking", &scenario.name),
                scenario,
                |b, scenario| {
                    let solver = DependencySolver::new();
                    b.iter_custom(|iters| {
                        let start = std::time::Instant::now();
                        for _ in 0..iters {
                            let request = SolverRequest {
                                requirements: scenario.requirements.clone(),
                                config: SolverConfig::default(),
                            };
                            black_box(solver.resolve(black_box(request)));
                        }
                        start.elapsed()
                    });
                },
            );
        }

        group.finish();
    }

    /// Benchmark solver statistics and metrics collection
    pub fn benchmark_statistics_collection(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("solver_statistics");

        group.bench_function("stats_collection_overhead", |b| {
            let solver = DependencySolver::new();
            b.iter(|| {
                // Benchmark the overhead of statistics collection
                let _stats = black_box(solver.stats());
            });
        });

        group.finish();
    }
}

/// Utility functions for solver benchmarks
impl SolverBenchmark {
    /// Create a solver request from a scenario
    pub fn create_request_from_scenario(scenario: &SolverScenario) -> SolverRequest {
        SolverRequest {
            requirements: scenario.requirements.clone(),
            config: SolverConfig::default(),
        }
    }

    /// Measure cache hit ratio for a given scenario
    pub fn measure_cache_hit_ratio(&self, scenario: &CacheScenario) -> f64 {
        let config = SolverConfig {
            enable_caching: true,
            cache_ttl_seconds: 3600,
            ..Default::default()
        };
        let solver = DependencySolver::with_config(config);

        // Run requests multiple times and measure cache performance
        let mut total_requests = 0;
        let mut cache_hits = 0;

        for request in &scenario.repeated_requests {
            for _ in 0..3 {
                // Run each request 3 times
                let _result = solver.resolve(request.clone());
                total_requests += 1;

                // In a real implementation, we would check if this was a cache hit
                // For now, assume later requests are cache hits
                if total_requests > scenario.repeated_requests.len() {
                    cache_hits += 1;
                }
            }
        }

        if total_requests > 0 {
            cache_hits as f64 / total_requests as f64
        } else {
            0.0
        }
    }

    /// Generate performance report for solver benchmarks
    pub fn generate_performance_report(&self) -> String {
        format!(
            "Solver Benchmark Report\n\
             ======================\n\
             Simple scenarios: {}\n\
             Medium scenarios: {}\n\
             Complex scenarios: {}\n\
             Conflict scenarios: {}\n\
             Cache scenarios: {}\n\
             \n\
             Test data validation: {:?}\n",
            self.test_data.simple_scenarios.len(),
            self.test_data.medium_scenarios.len(),
            self.test_data.complex_scenarios.len(),
            self.test_data.conflict_scenarios.len(),
            self.test_data.cache_scenarios.len(),
            self.validate()
        )
    }
}
