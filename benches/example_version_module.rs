//! Example Version Module Benchmark
//!
//! This module demonstrates how to implement the ModuleBenchmark trait
//! for the Version system using the comprehensive benchmark suite framework.

use criterion::{black_box, Criterion, BenchmarkId, Throughput};
use std::collections::HashMap;
use std::time::SystemTime;

// Import the comprehensive benchmark suite framework
mod comprehensive_benchmark_suite;
use comprehensive_benchmark_suite::*;

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
        // Create mock baseline metrics for demonstration
        let mut benchmarks = HashMap::new();
        
        benchmarks.insert("version_parsing".to_string(), BenchmarkResult {
            name: "version_parsing".to_string(),
            mean_time_ns: 1700.0, // 1.7 microseconds based on actual performance
            std_dev_ns: 50.0,
            throughput_ops_per_sec: Some(586_633.0),
            memory_usage_bytes: Some(1024),
            additional_metrics: HashMap::new(),
        });
        
        benchmarks.insert("version_comparison".to_string(), BenchmarkResult {
            name: "version_comparison".to_string(),
            mean_time_ns: 100.0, // 100 nanoseconds
            std_dev_ns: 10.0,
            throughput_ops_per_sec: Some(10_000_000.0),
            memory_usage_bytes: Some(0),
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
            warm_up_time: std::time::Duration::from_millis(500),
            measurement_time: std::time::Duration::from_secs(3),
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
        
        // Standard parsing benchmark
        group.bench_function("standard_parsing", |b| {
            b.iter(|| {
                for version_str in &self.test_versions {
                    // Mock version parsing - in real implementation, use actual Version::parse
                    black_box(self.mock_parse_version(black_box(version_str)));
                }
            });
        });
        
        // Optimized parsing benchmark
        group.bench_function("optimized_parsing", |b| {
            b.iter(|| {
                for version_str in &self.test_versions {
                    // Mock optimized parsing
                    black_box(self.mock_parse_version_optimized(black_box(version_str)));
                }
            });
        });
        
        group.finish();
    }
    
    /// Benchmark version comparison performance
    fn benchmark_version_comparison(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("version_comparison");
        
        // Create parsed versions for comparison
        let parsed_versions: Vec<_> = self.test_versions.iter()
            .map(|v| self.mock_parse_version(v))
            .collect();
        
        group.bench_function("version_comparison", |b| {
            b.iter(|| {
                for i in 0..parsed_versions.len() {
                    for j in (i+1)..parsed_versions.len() {
                        black_box(self.mock_compare_versions(&parsed_versions[i], &parsed_versions[j]));
                    }
                }
            });
        });
        
        group.finish();
    }
    
    /// Benchmark version sorting performance
    fn benchmark_version_sorting(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("version_sorting");
        
        for size in [10, 100, 1000].iter() {
            let versions: Vec<String> = (0..*size)
                .map(|i| format!("1.{}.{}", i % 100, i % 10))
                .collect();
            
            group.throughput(Throughput::Elements(*size as u64));
            group.bench_with_input(BenchmarkId::new("sort_versions", size), size, |b, _| {
                b.iter(|| {
                    let mut parsed: Vec<_> = versions.iter()
                        .map(|v| self.mock_parse_version(v))
                        .collect();
                    parsed.sort_by(|a, b| self.mock_compare_versions(a, b));
                    black_box(parsed);
                });
            });
        }
        
        group.finish();
    }
    
    /// Benchmark batch operations
    fn benchmark_batch_operations(&self, c: &mut Criterion) {
        let mut group = c.benchmark_group("batch_operations");
        
        let large_version_set: Vec<String> = (0..1000)
            .map(|i| format!("1.{}.{}", i / 100, i % 100))
            .collect();
        
        group.bench_function("batch_parse", |b| {
            b.iter(|| {
                let parsed: Vec<_> = large_version_set.iter()
                    .map(|v| self.mock_parse_version(v))
                    .collect();
                black_box(parsed);
            });
        });
        
        group.bench_function("batch_parse_and_sort", |b| {
            b.iter(|| {
                let mut parsed: Vec<_> = large_version_set.iter()
                    .map(|v| self.mock_parse_version(v))
                    .collect();
                parsed.sort_by(|a, b| self.mock_compare_versions(a, b));
                black_box(parsed);
            });
        });
        
        group.finish();
    }
    
    // Mock implementations for demonstration
    fn mock_parse_version(&self, version_str: &str) -> MockVersion {
        // Simulate parsing work
        std::thread::sleep(std::time::Duration::from_nanos(100));
        MockVersion {
            original: version_str.to_string(),
            major: 1,
            minor: 2,
            patch: 3,
        }
    }
    
    fn mock_parse_version_optimized(&self, version_str: &str) -> MockVersion {
        // Simulate faster optimized parsing
        std::thread::sleep(std::time::Duration::from_nanos(50));
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

/// Example of how to use the comprehensive benchmark suite
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_version_benchmark_module() {
        let module = VersionModuleBenchmark::new();
        
        // Test validation
        assert!(module.validate().is_ok());
        
        // Test configuration
        let config = module.get_config();
        assert!(config.enabled);
        assert_eq!(config.parameters.get("test_version_count").unwrap(), "8");
        
        // Test baseline metrics
        let baseline = module.get_baseline_metrics();
        assert_eq!(baseline.module_name, "version");
        assert!(baseline.benchmarks.contains_key("version_parsing"));
        assert!(baseline.benchmarks.contains_key("version_comparison"));
    }
    
    #[test]
    fn test_benchmark_suite_integration() {
        let mut suite = BenchmarkSuite::new();
        let module = Box::new(VersionModuleBenchmark::new());
        
        // Register the module
        assert!(suite.register_module(module).is_ok());
        
        // Check module list
        let modules = suite.list_modules();
        assert_eq!(modules, vec!["version"]);
        
        // Test baseline collection
        let baselines = suite.collect_baselines().unwrap();
        assert_eq!(baselines.len(), 1);
        assert_eq!(baselines[0].module_name, "version");
    }
}
