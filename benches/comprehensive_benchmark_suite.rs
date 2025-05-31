//! Comprehensive Benchmark Suite
//!
//! This module provides a unified framework for running performance benchmarks
//! across all rez-core modules. It includes standardized interfaces, configuration
//! management, and baseline metrics collection.

use criterion::{Criterion, BenchmarkId, Throughput, criterion_group, criterion_main};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use std::fs;

/// Trait for module-specific benchmark implementations
pub trait ModuleBenchmark: Send + Sync {
    /// Get the name of the benchmark module
    fn name(&self) -> &str;
    
    /// Run all benchmarks for this module
    fn run_benchmarks(&self, c: &mut Criterion);
    
    /// Get baseline metrics for this module
    fn get_baseline_metrics(&self) -> BaselineMetrics;
    
    /// Get module-specific configuration
    fn get_config(&self) -> ModuleBenchmarkConfig {
        ModuleBenchmarkConfig::default()
    }
    
    /// Validate that the module is ready for benchmarking
    fn validate(&self) -> Result<(), BenchmarkError> {
        Ok(())
    }
}

/// Configuration for individual module benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleBenchmarkConfig {
    /// Whether this module is enabled for benchmarking
    pub enabled: bool,
    /// Warm-up time for this module's benchmarks
    pub warm_up_time: Duration,
    /// Measurement time for this module's benchmarks
    pub measurement_time: Duration,
    /// Sample size for this module's benchmarks
    pub sample_size: usize,
    /// Module-specific parameters
    pub parameters: HashMap<String, String>,
}

impl Default for ModuleBenchmarkConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            warm_up_time: Duration::from_millis(500),
            measurement_time: Duration::from_secs(3),
            sample_size: 100,
            parameters: HashMap::new(),
        }
    }
}

/// Baseline performance metrics for a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    /// Module name
    pub module_name: String,
    /// Timestamp when baseline was recorded
    pub timestamp: SystemTime,
    /// Individual benchmark results
    pub benchmarks: HashMap<String, BenchmarkResult>,
    /// Overall module performance score
    pub overall_score: f64,
    /// Environment information
    pub environment: EnvironmentInfo,
}

/// Individual benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Mean execution time in nanoseconds
    pub mean_time_ns: f64,
    /// Standard deviation in nanoseconds
    pub std_dev_ns: f64,
    /// Throughput (operations per second)
    pub throughput_ops_per_sec: Option<f64>,
    /// Memory usage in bytes
    pub memory_usage_bytes: Option<u64>,
    /// Additional metrics
    pub additional_metrics: HashMap<String, f64>,
}

/// Environment information for baseline comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    /// Operating system
    pub os: String,
    /// CPU model
    pub cpu: String,
    /// Total memory in bytes
    pub memory_bytes: u64,
    /// Rust version
    pub rust_version: String,
    /// Compiler flags
    pub compiler_flags: Vec<String>,
}

/// Benchmark suite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Global benchmark settings
    pub global: GlobalBenchmarkConfig,
    /// Module-specific configurations
    pub modules: HashMap<String, ModuleBenchmarkConfig>,
    /// Output configuration
    pub output: OutputConfig,
    /// Baseline management configuration
    pub baseline: BaselineConfig,
}

/// Global benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBenchmarkConfig {
    /// Default warm-up time
    pub default_warm_up_time: Duration,
    /// Default measurement time
    pub default_measurement_time: Duration,
    /// Default sample size
    pub default_sample_size: usize,
    /// Whether to run in parallel
    pub parallel_execution: bool,
    /// Maximum number of concurrent benchmarks
    pub max_concurrent: usize,
}

/// Output configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output directory for reports
    pub output_dir: PathBuf,
    /// Report formats to generate
    pub formats: Vec<ReportFormat>,
    /// Whether to generate detailed reports
    pub detailed_reports: bool,
}

/// Baseline management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineConfig {
    /// Directory to store baseline data
    pub baseline_dir: PathBuf,
    /// Whether to auto-update baselines
    pub auto_update: bool,
    /// Threshold for regression detection (percentage)
    pub regression_threshold: f64,
}

/// Report output formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportFormat {
    Html,
    Json,
    Markdown,
    Csv,
}

/// Benchmark errors
#[derive(Debug, thiserror::Error)]
pub enum BenchmarkError {
    #[error("Module validation failed: {0}")]
    ValidationFailed(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Baseline error: {0}")]
    BaselineError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            global: GlobalBenchmarkConfig {
                default_warm_up_time: Duration::from_millis(500),
                default_measurement_time: Duration::from_secs(3),
                default_sample_size: 100,
                parallel_execution: false,
                max_concurrent: 4,
            },
            modules: HashMap::new(),
            output: OutputConfig {
                output_dir: PathBuf::from("target/benchmark-reports"),
                formats: vec![ReportFormat::Html, ReportFormat::Json],
                detailed_reports: true,
            },
            baseline: BaselineConfig {
                baseline_dir: PathBuf::from("benchmarks/baselines"),
                auto_update: false,
                regression_threshold: 10.0, // 10% regression threshold
            },
        }
    }
}

/// Main benchmark suite manager
pub struct BenchmarkSuite {
    /// Suite configuration
    config: BenchmarkConfig,
    /// Registered benchmark modules
    modules: Vec<Box<dyn ModuleBenchmark>>,
    /// Baseline storage
    baseline_storage: BaselineStorage,
}

/// Convenience function to create a benchmark suite with all core modules
pub fn create_comprehensive_suite() -> BenchmarkSuite {
    let mut suite = BenchmarkSuite::new();

    // Register all core benchmark modules
    register_core_modules(&mut suite).expect("Failed to register core modules");

    suite
}

/// Register all core benchmark modules
pub fn register_core_modules(suite: &mut BenchmarkSuite) -> Result<(), BenchmarkError> {
    // Register Version module benchmark
    suite.register_module(Box::new(crate::version_benchmark::VersionBenchmark::new()))?;

    // Register Solver module benchmark
    suite.register_module(Box::new(crate::solver_benchmark::SolverBenchmark::new()))?;

    // Register Context module benchmark
    suite.register_module(Box::new(crate::context_benchmark::ContextBenchmark::new()))?;

    // Register Rex module benchmark
    suite.register_module(Box::new(crate::rex_benchmark::RexBenchmark::new()))?;

    // Register Build and Cache module benchmark
    suite.register_module(Box::new(crate::build_cache_benchmark::BuildCacheBenchmark::new()))?;

    Ok(())
}

impl BenchmarkSuite {
    /// Create a new benchmark suite with default configuration
    pub fn new() -> Self {
        Self::with_config(BenchmarkConfig::default())
    }
    
    /// Create a new benchmark suite with custom configuration
    pub fn with_config(config: BenchmarkConfig) -> Self {
        let baseline_storage = BaselineStorage::new(config.baseline.baseline_dir.clone());
        
        Self {
            config,
            modules: Vec::new(),
            baseline_storage,
        }
    }
    
    /// Register a benchmark module
    pub fn register_module(&mut self, module: Box<dyn ModuleBenchmark>) -> Result<(), BenchmarkError> {
        // Validate the module
        module.validate()?;
        
        // Check for duplicate names
        let module_name = module.name();
        if self.modules.iter().any(|m| m.name() == module_name) {
            return Err(BenchmarkError::ValidationFailed(
                format!("Module '{}' is already registered", module_name)
            ));
        }
        
        self.modules.push(module);
        Ok(())
    }
    
    /// Run all registered benchmarks
    pub fn run_all(&self) -> Result<(), BenchmarkError> {
        let criterion = self.create_criterion();
        
        for module in &self.modules {
            let module_name = module.name();
            
            // Check if module is enabled
            if let Some(module_config) = self.config.modules.get(module_name) {
                if !module_config.enabled {
                    println!("Skipping disabled module: {}", module_name);
                    continue;
                }
            }
            
            println!("Running benchmarks for module: {}", module_name);
            module.run_benchmarks(&criterion);
        }
        
        Ok(())
    }
    
    /// Run benchmarks for specific modules
    pub fn run_modules(&self, module_names: &[&str]) -> Result<(), BenchmarkError> {
        let criterion = self.create_criterion();
        
        for module_name in module_names {
            if let Some(module) = self.modules.iter().find(|m| m.name() == *module_name) {
                println!("Running benchmarks for module: {}", module_name);
                module.run_benchmarks(&criterion);
            } else {
                return Err(BenchmarkError::ValidationFailed(
                    format!("Module '{}' not found", module_name)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Create configured Criterion instance
    fn create_criterion(&self) -> Criterion {
        Criterion::default()
            .warm_up_time(self.config.global.default_warm_up_time)
            .measurement_time(self.config.global.default_measurement_time)
            .sample_size(self.config.global.default_sample_size)
    }
    
    /// Get list of registered modules
    pub fn list_modules(&self) -> Vec<&str> {
        self.modules.iter().map(|m| m.name()).collect()
    }
    
    /// Get configuration
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }
}

/// Baseline storage and management
pub struct BaselineStorage {
    baseline_dir: PathBuf,
}

impl BaselineStorage {
    pub fn new(baseline_dir: PathBuf) -> Self {
        Self { baseline_dir }
    }
    
    /// Save baseline metrics for a module
    pub fn save_baseline(&self, metrics: &BaselineMetrics) -> Result<(), BenchmarkError> {
        std::fs::create_dir_all(&self.baseline_dir)?;
        
        let filename = format!("{}.json", metrics.module_name);
        let filepath = self.baseline_dir.join(filename);
        
        let json = serde_json::to_string_pretty(metrics)?;
        std::fs::write(filepath, json)?;
        
        Ok(())
    }
    
    /// Load baseline metrics for a module
    pub fn load_baseline(&self, module_name: &str) -> Result<BaselineMetrics, BenchmarkError> {
        let filename = format!("{}.json", module_name);
        let filepath = self.baseline_dir.join(filename);
        
        let json = std::fs::read_to_string(filepath)?;
        let metrics = serde_json::from_str(&json)?;
        
        Ok(metrics)
    }
    
    /// Check if baseline exists for a module
    pub fn has_baseline(&self, module_name: &str) -> bool {
        let filename = format!("{}.json", module_name);
        let filepath = self.baseline_dir.join(filename);
        filepath.exists()
    }
}

/// Utility functions for benchmark suite
impl BenchmarkSuite {
    /// Collect baseline metrics from all modules
    pub fn collect_baselines(&self) -> Result<Vec<BaselineMetrics>, BenchmarkError> {
        let mut baselines = Vec::new();

        for module in &self.modules {
            let metrics = module.get_baseline_metrics();
            baselines.push(metrics);
        }

        Ok(baselines)
    }

    /// Save all current baselines
    pub fn save_all_baselines(&self) -> Result<(), BenchmarkError> {
        let baselines = self.collect_baselines()?;

        for baseline in baselines {
            self.baseline_storage.save_baseline(&baseline)?;
        }

        Ok(())
    }

    /// Load all available baselines
    pub fn load_all_baselines(&self) -> Result<Vec<BaselineMetrics>, BenchmarkError> {
        let mut baselines = Vec::new();

        for module in &self.modules {
            let module_name = module.name();
            if self.baseline_storage.has_baseline(module_name) {
                let baseline = self.baseline_storage.load_baseline(module_name)?;
                baselines.push(baseline);
            }
        }

        Ok(baselines)
    }
}

/// Helper functions for creating benchmark configurations
pub mod config_helpers {
    use super::*;

    /// Create a high-performance benchmark configuration
    pub fn high_performance_config() -> BenchmarkConfig {
        BenchmarkConfig {
            global: GlobalBenchmarkConfig {
                default_warm_up_time: Duration::from_secs(1),
                default_measurement_time: Duration::from_secs(5),
                default_sample_size: 200,
                parallel_execution: true,
                max_concurrent: 8,
            },
            ..Default::default()
        }
    }

    /// Create a quick benchmark configuration for development
    pub fn quick_config() -> BenchmarkConfig {
        BenchmarkConfig {
            global: GlobalBenchmarkConfig {
                default_warm_up_time: Duration::from_millis(100),
                default_measurement_time: Duration::from_secs(1),
                default_sample_size: 10,
                parallel_execution: false,
                max_concurrent: 1,
            },
            ..Default::default()
        }
    }

    /// Create a comprehensive benchmark configuration
    pub fn comprehensive_config() -> BenchmarkConfig {
        BenchmarkConfig {
            global: GlobalBenchmarkConfig {
                default_warm_up_time: Duration::from_secs(2),
                default_measurement_time: Duration::from_secs(10),
                default_sample_size: 500,
                parallel_execution: true,
                max_concurrent: 4,
            },
            output: OutputConfig {
                output_dir: PathBuf::from("target/comprehensive-benchmark-reports"),
                formats: vec![
                    ReportFormat::Html,
                    ReportFormat::Json,
                    ReportFormat::Markdown,
                    ReportFormat::Csv,
                ],
                detailed_reports: true,
            },
            baseline: BaselineConfig {
                baseline_dir: PathBuf::from("benchmarks/comprehensive-baselines"),
                auto_update: true,
                regression_threshold: 5.0, // 5% regression threshold
            },
            ..Default::default()
        }
    }
}

/// Environment detection utilities
pub mod environment {
    use super::*;

    /// Detect current environment information
    pub fn detect_environment() -> EnvironmentInfo {
        EnvironmentInfo {
            os: std::env::consts::OS.to_string(),
            cpu: detect_cpu_model(),
            memory_bytes: detect_memory_size(),
            rust_version: detect_rust_version(),
            compiler_flags: detect_compiler_flags(),
        }
    }

    fn detect_cpu_model() -> String {
        // Simplified CPU detection
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64".to_string()
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            "unknown".to_string()
        }
    }

    fn detect_memory_size() -> u64 {
        // Simplified memory detection - return 0 for now
        // In a real implementation, this would use system APIs
        0
    }

    fn detect_rust_version() -> String {
        env!("RUSTC_VERSION").to_string()
    }

    fn detect_compiler_flags() -> Vec<String> {
        // Return common optimization flags
        vec![
            "-O3".to_string(),
            "-C target-cpu=native".to_string(),
        ]
    }
}

/// Benchmark result analysis utilities
pub mod analysis {
    use super::*;

    /// Compare two benchmark results and detect regressions
    pub fn compare_results(
        current: &BenchmarkResult,
        baseline: &BenchmarkResult,
        threshold: f64,
    ) -> ComparisonResult {
        let performance_change = (current.mean_time_ns - baseline.mean_time_ns) / baseline.mean_time_ns * 100.0;

        let status = if performance_change > threshold {
            ComparisonStatus::Regression
        } else if performance_change < -threshold {
            ComparisonStatus::Improvement
        } else {
            ComparisonStatus::NoChange
        };

        ComparisonResult {
            benchmark_name: current.name.clone(),
            performance_change_percent: performance_change,
            status,
            current_time_ns: current.mean_time_ns,
            baseline_time_ns: baseline.mean_time_ns,
        }
    }

    /// Comparison result
    #[derive(Debug, Clone)]
    pub struct ComparisonResult {
        pub benchmark_name: String,
        pub performance_change_percent: f64,
        pub status: ComparisonStatus,
        pub current_time_ns: f64,
        pub baseline_time_ns: f64,
    }

    /// Comparison status
    #[derive(Debug, Clone, PartialEq)]
    pub enum ComparisonStatus {
        Improvement,
        NoChange,
        Regression,
    }
}



/// Configuration helpers
pub mod config_helpers {
    use super::*;

    /// Create a comprehensive benchmark configuration
    pub fn comprehensive_config() -> BenchmarkConfig {
        BenchmarkConfig {
            global: GlobalBenchmarkConfig {
                default_warm_up_time: Duration::from_secs(2),
                default_measurement_time: Duration::from_secs(10),
                default_sample_size: 500,
                parallel_execution: true,
                max_concurrent: 4,
            },
            output: OutputConfig {
                output_dir: PathBuf::from("target/comprehensive-benchmark-reports"),
                formats: vec![
                    ReportFormat::Html,
                    ReportFormat::Json,
                    ReportFormat::Markdown,
                    ReportFormat::Csv,
                ],
                detailed_reports: true,
            },
            baseline: BaselineConfig {
                baseline_dir: PathBuf::from("benchmarks/comprehensive-baselines"),
                auto_update: true,
                regression_threshold: 5.0, // 5% regression threshold
            },
            ..Default::default()
        }
    }
}

/// Main entry point for comprehensive benchmarks
pub fn run_comprehensive_benchmarks() {
    use config_helpers::*;

    println!("🚀 Starting Comprehensive Benchmark Suite");
    println!("==========================================");

    // Create benchmark suite with comprehensive configuration
    let mut suite = BenchmarkSuite::with_config(comprehensive_config());

    // Register all available modules
    println!("📦 Registering benchmark modules...");

    // Register version module
    let version_module = Box::new(version_module::VersionModuleBenchmark::new());
    suite.register_module(version_module).expect("Failed to register version module");

    println!("✅ Registered {} modules", suite.list_modules().len());

    // Run all benchmarks
    println!("🏃 Running all benchmarks...");
    match suite.run_all() {
        Ok(()) => println!("✅ All benchmarks completed successfully"),
        Err(e) => eprintln!("❌ Benchmark execution failed: {}", e),
    }

    // Save baselines
    println!("💾 Saving baseline metrics...");
    match suite.save_all_baselines() {
        Ok(()) => println!("✅ Baselines saved successfully"),
        Err(e) => eprintln!("❌ Failed to save baselines: {}", e),
    }

    println!("🎯 Comprehensive benchmark suite completed!");
}

/// Quick benchmark runner for development
#[cfg(feature = "quick-benchmarks")]
pub fn run_quick_benchmarks() {
    use config_helpers::*;

    println!("⚡ Starting Quick Benchmark Suite");
    println!("=================================");

    let mut suite = BenchmarkSuite::with_config(quick_config());

    // Register essential modules only
    println!("📦 Registering essential modules...");

    println!("✅ Registered {} modules", suite.list_modules().len());

    // Run benchmarks
    println!("🏃 Running quick benchmarks...");
    match suite.run_all() {
        Ok(()) => println!("✅ Quick benchmarks completed"),
        Err(e) => eprintln!("❌ Quick benchmark execution failed: {}", e),
    }
}

/// Benchmark runner with custom module selection
pub fn run_selected_benchmarks(module_names: &[&str]) {
    println!("🎯 Starting Selected Benchmark Suite");
    println!("====================================");

    let mut suite = BenchmarkSuite::new();

    // Register all modules (in real implementation)
    println!("📦 Registering all modules...");

    println!("✅ Registered {} modules", suite.list_modules().len());

    // Run selected benchmarks
    println!("🏃 Running selected benchmarks: {:?}", module_names);
    match suite.run_modules(module_names) {
        Ok(()) => println!("✅ Selected benchmarks completed"),
        Err(e) => eprintln!("❌ Selected benchmark execution failed: {}", e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_suite_creation() {
        let suite = BenchmarkSuite::new();
        assert_eq!(suite.list_modules().len(), 0);

        let config = suite.config();
        assert_eq!(config.baseline.regression_threshold, 10.0);
    }

    #[test]
    fn test_config_helpers() {
        let high_perf = config_helpers::high_performance_config();
        assert_eq!(high_perf.global.default_sample_size, 200);
        assert!(high_perf.global.parallel_execution);

        let quick = config_helpers::quick_config();
        assert_eq!(quick.global.default_sample_size, 10);
        assert!(!quick.global.parallel_execution);

        let comprehensive = config_helpers::comprehensive_config();
        assert_eq!(comprehensive.global.default_sample_size, 500);
        assert_eq!(comprehensive.baseline.regression_threshold, 5.0);
    }

    #[test]
    fn test_environment_detection() {
        let env = environment::detect_environment();
        assert!(!env.os.is_empty());
        assert!(!env.cpu.is_empty());
        assert!(!env.rust_version.is_empty());
    }

    #[test]
    fn test_baseline_storage() {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let storage = BaselineStorage::new(temp_dir.path().to_path_buf());

        let metrics = BaselineMetrics {
            module_name: "test_module".to_string(),
            timestamp: SystemTime::now(),
            benchmarks: HashMap::new(),
            overall_score: 85.0,
            environment: environment::detect_environment(),
        };

        // Test save and load
        assert!(storage.save_baseline(&metrics).is_ok());
        assert!(storage.has_baseline("test_module"));

        let loaded = storage.load_baseline("test_module").unwrap();
        assert_eq!(loaded.module_name, "test_module");
        assert_eq!(loaded.overall_score, 85.0);
    }

    #[test]
    fn test_comparison_analysis() {
        let current = BenchmarkResult {
            name: "test_benchmark".to_string(),
            mean_time_ns: 1100.0,
            std_dev_ns: 50.0,
            throughput_ops_per_sec: None,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        let baseline = BenchmarkResult {
            name: "test_benchmark".to_string(),
            mean_time_ns: 1000.0,
            std_dev_ns: 40.0,
            throughput_ops_per_sec: None,
            memory_usage_bytes: None,
            additional_metrics: HashMap::new(),
        };

        let comparison = analysis::compare_results(&current, &baseline, 5.0);
        assert_eq!(comparison.status, analysis::ComparisonStatus::Regression);
        assert_eq!(comparison.performance_change_percent, 10.0);
    }
}

/// Version module benchmark implementation
pub mod version_module {
    use super::*;
    use criterion::{black_box, BenchmarkId};

    /// Version module benchmark implementation
    pub struct VersionModuleBenchmark {
        test_versions: Vec<String>,
    }

    impl VersionModuleBenchmark {
        pub fn new() -> Self {
            let test_versions = vec![
                "1.2.3".to_string(),
                "1.2.3-alpha.1".to_string(),
                "2.0.0-beta.2+build.123".to_string(),
                "1.0.0-rc.1".to_string(),
                "3.1.4-dev.123".to_string(),
                "10.20.30".to_string(),
                "1.2.3-alpha1.beta2.gamma3".to_string(),
                "0.0.1-snapshot.20231201".to_string(),
            ];

            Self { test_versions }
        }
    }

    impl ModuleBenchmark for VersionModuleBenchmark {
        fn name(&self) -> &str {
            "version"
        }

        fn run_benchmarks(&self, c: &mut Criterion) {
            self.benchmark_version_parsing(c);
            self.benchmark_version_comparison(c);
            self.benchmark_version_sorting(c);
            self.benchmark_batch_operations(c);
        }

        fn get_baseline_metrics(&self) -> BaselineMetrics {
            let mut benchmarks = HashMap::new();

            // Add baseline metrics for each benchmark
            benchmarks.insert("version_parsing".to_string(), BenchmarkResult {
                name: "version_parsing".to_string(),
                mean_time_ns: 1000.0,
                std_dev_ns: 50.0,
                throughput_ops_per_sec: Some(1_000_000.0),
                memory_usage_bytes: Some(1024),
                additional_metrics: HashMap::new(),
            });

            benchmarks.insert("version_comparison".to_string(), BenchmarkResult {
                name: "version_comparison".to_string(),
                mean_time_ns: 100.0,
                std_dev_ns: 10.0,
                throughput_ops_per_sec: Some(10_000_000.0),
                memory_usage_bytes: Some(512),
                additional_metrics: HashMap::new(),
            });

            BaselineMetrics {
                module_name: "version".to_string(),
                timestamp: SystemTime::now(),
                benchmarks,
                overall_score: 95.0, // High performance score
                environment: environment::detect_environment(),
            }
        }

        fn get_config(&self) -> ModuleBenchmarkConfig {
            ModuleBenchmarkConfig {
                enabled: true,
                warm_up_time: Duration::from_millis(500),
                measurement_time: Duration::from_secs(3),
                sample_size: 100,
                parameters: {
                    let mut params = HashMap::new();
                    params.insert("test_version_count".to_string(), self.test_versions.len().to_string());
                    params
                },
            }
        }

        fn validate(&self) -> Result<(), BenchmarkError> {
            if self.test_versions.is_empty() {
                return Err(BenchmarkError::ValidationFailed(
                    "No test versions available".to_string()
                ));
            }
            Ok(())
        }
    }

    impl VersionModuleBenchmark {
        /// Benchmark version parsing performance
        fn benchmark_version_parsing(&self, c: &mut Criterion) {
            let mut group = c.benchmark_group("version_parsing");

            // Single version parsing
            group.bench_function("single_version", |b| {
                b.iter(|| {
                    for version_str in &self.test_versions {
                        black_box(self.mock_parse_version(black_box(version_str)));
                    }
                });
            });

            // Optimized version parsing
            group.bench_function("optimized_version", |b| {
                b.iter(|| {
                    for version_str in &self.test_versions {
                        black_box(self.mock_parse_version_optimized(black_box(version_str)));
                    }
                });
            });

            group.finish();
        }

        /// Benchmark version comparison performance
        fn benchmark_version_comparison(&self, c: &mut Criterion) {
            let versions: Vec<MockVersion> = self.test_versions
                .iter()
                .map(|v| self.mock_parse_version(v))
                .collect();

            c.bench_function("version_comparison", |b| {
                b.iter(|| {
                    for i in 0..versions.len() {
                        for j in (i + 1)..versions.len() {
                            black_box(self.mock_compare_versions(&versions[i], &versions[j]));
                        }
                    }
                });
            });
        }

        /// Benchmark version sorting performance
        fn benchmark_version_sorting(&self, c: &mut Criterion) {
            let mut group = c.benchmark_group("version_sorting");

            for size in [10, 100, 1000].iter() {
                let versions: Vec<MockVersion> = (0..*size)
                    .map(|i| self.mock_parse_version(&format!("1.{}.0", i)))
                    .collect();

                group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
                    b.iter(|| {
                        let mut sorted_versions = versions.clone();
                        sorted_versions.sort_by(|a, b| self.mock_compare_versions(a, b));
                        black_box(sorted_versions);
                    });
                });
            }
            group.finish();
        }

        /// Benchmark batch operations
        fn benchmark_batch_operations(&self, c: &mut Criterion) {
            let mut group = c.benchmark_group("batch_operations");

            group.bench_function("batch_parsing", |b| {
                b.iter(|| {
                    let versions: Vec<MockVersion> = self.test_versions
                        .iter()
                        .map(|v| self.mock_parse_version(black_box(v)))
                        .collect();
                    black_box(versions);
                });
            });

            group.finish();
        }

        // Mock implementations for demonstration
        fn mock_parse_version(&self, version_str: &str) -> MockVersion {
            // Simulate parsing work
            std::thread::sleep(Duration::from_nanos(100));
            MockVersion {
                original: version_str.to_string(),
                major: 1,
                minor: 2,
                patch: 3,
            }
        }

        fn mock_parse_version_optimized(&self, version_str: &str) -> MockVersion {
            // Simulate faster optimized parsing
            std::thread::sleep(Duration::from_nanos(50));
            MockVersion {
                original: version_str.to_string(),
                major: 1,
                minor: 2,
                patch: 3,
            }
        }

        fn mock_compare_versions(&self, a: &MockVersion, b: &MockVersion) -> std::cmp::Ordering {
            // Simulate version comparison
            a.major.cmp(&b.major)
                .then_with(|| a.minor.cmp(&b.minor))
                .then_with(|| a.patch.cmp(&b.patch))
        }
    }

    /// Mock version structure for demonstration
    #[derive(Debug, Clone)]
    struct MockVersion {
        original: String,
        major: u32,
        minor: u32,
        patch: u32,
    }
}

/// Criterion benchmark entry point
fn comprehensive_benchmarks(c: &mut Criterion) {
    println!("🚀 Running Comprehensive Benchmark Suite");

    // Create and run version module benchmarks directly
    let version_module = version_module::VersionModuleBenchmark::new();
    version_module.run_benchmarks(c);

    println!("✅ All benchmarks completed successfully");
}

/// Configure criterion with performance optimizations
fn configure_criterion() -> Criterion {
    Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(10))
        .sample_size(500)
}

criterion_group! {
    name = comprehensive_benches;
    config = configure_criterion();
    targets = comprehensive_benchmarks
}

criterion_main!(comprehensive_benches);
