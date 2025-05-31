//! Unified Benchmark Suite for rez-core
//!
//! This is a comprehensive benchmark framework that demonstrates the unified approach
//! for testing all rez-core modules. This standalone version serves as a proof of concept
//! for the comprehensive benchmark architecture.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Benchmark configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub warm_up_time: Duration,
    pub measurement_time: Duration,
    pub sample_size: usize,
    pub output_dir: String,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warm_up_time: Duration::from_secs(2),
            measurement_time: Duration::from_secs(10),
            sample_size: 500,
            output_dir: "target/benchmark-reports".to_string(),
        }
    }
}

/// Benchmark result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean_time_ns: f64,
    pub std_dev_ns: f64,
    pub throughput_ops_per_sec: Option<f64>,
    pub timestamp: SystemTime,
}

/// Version module simulation
pub mod version_module {
    use super::*;
    
    #[derive(Debug, Clone)]
    pub struct MockVersion {
        pub major: u32,
        pub minor: u32,
        pub patch: u32,
        pub prerelease: Option<String>,
    }
    
    impl MockVersion {
        pub fn parse(version_str: &str) -> Result<Self, String> {
            // Simulate version parsing work
            std::thread::sleep(Duration::from_nanos(100));
            
            let parts: Vec<&str> = version_str.split('.').collect();
            if parts.len() < 3 {
                return Err("Invalid version format".to_string());
            }
            
            Ok(MockVersion {
                major: parts[0].parse().map_err(|_| "Invalid major version")?,
                minor: parts[1].parse().map_err(|_| "Invalid minor version")?,
                patch: parts[2].parse().map_err(|_| "Invalid patch version")?,
                prerelease: None,
            })
        }
        
        pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
            self.major.cmp(&other.major)
                .then_with(|| self.minor.cmp(&other.minor))
                .then_with(|| self.patch.cmp(&other.patch))
        }
    }
    
    pub fn benchmark_version_parsing(c: &mut Criterion) {
        let test_versions = vec![
            "1.2.3", "1.2.4", "1.2.5", "2.0.0", "2.0.1",
            "3.1.4", "10.20.30", "0.0.1", "1.0.0", "2.1.0"
        ];
        
        let mut group = c.benchmark_group("version_parsing");
        
        group.bench_function("single_version_parsing", |b| {
            b.iter(|| {
                for version_str in &test_versions {
                    black_box(MockVersion::parse(black_box(version_str)).unwrap());
                }
            });
        });
        
        group.bench_function("batch_version_parsing", |b| {
            b.iter(|| {
                let versions: Vec<MockVersion> = test_versions
                    .iter()
                    .map(|v| MockVersion::parse(v).unwrap())
                    .collect();
                black_box(versions);
            });
        });
        
        group.finish();
    }
    
    pub fn benchmark_version_comparison(c: &mut Criterion) {
        let versions: Vec<MockVersion> = vec![
            "1.2.3", "1.2.4", "1.2.5", "2.0.0", "2.0.1"
        ].iter().map(|v| MockVersion::parse(v).unwrap()).collect();
        
        c.bench_function("version_comparison", |b| {
            b.iter(|| {
                for i in 0..versions.len() {
                    for j in (i + 1)..versions.len() {
                        black_box(versions[i].compare(&versions[j]));
                    }
                }
            });
        });
    }
    
    pub fn benchmark_version_sorting(c: &mut Criterion) {
        let mut group = c.benchmark_group("version_sorting");
        
        for size in [10, 100, 1000].iter() {
            let versions: Vec<MockVersion> = (0..*size)
                .map(|i| MockVersion::parse(&format!("1.{}.0", i)).unwrap())
                .collect();
            
            group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
                b.iter(|| {
                    let mut sorted_versions = versions.clone();
                    sorted_versions.sort_by(|a, b| a.compare(b));
                    black_box(sorted_versions);
                });
            });
        }
        group.finish();
    }
}

/// Package module simulation
pub mod package_module {
    use super::*;
    
    #[derive(Debug, Clone)]
    pub struct MockPackage {
        pub name: String,
        pub version: String,
        pub dependencies: Vec<String>,
    }
    
    impl MockPackage {
        pub fn new(name: &str, version: &str) -> Self {
            // Simulate package creation work
            std::thread::sleep(Duration::from_nanos(50));
            
            Self {
                name: name.to_string(),
                version: version.to_string(),
                dependencies: Vec::new(),
            }
        }
        
        pub fn add_dependency(&mut self, dep: &str) {
            self.dependencies.push(dep.to_string());
        }
    }
    
    pub fn benchmark_package_creation(c: &mut Criterion) {
        let package_names = vec![
            "package_a", "package_b", "package_c", "package_d", "package_e"
        ];
        
        c.bench_function("package_creation", |b| {
            b.iter(|| {
                for name in &package_names {
                    black_box(MockPackage::new(black_box(name), "1.0.0"));
                }
            });
        });
    }
    
    pub fn benchmark_dependency_management(c: &mut Criterion) {
        let mut group = c.benchmark_group("dependency_management");
        
        for dep_count in [5, 10, 50].iter() {
            group.bench_with_input(BenchmarkId::from_parameter(dep_count), dep_count, |b, &dep_count| {
                b.iter(|| {
                    let mut package = MockPackage::new("test_package", "1.0.0");
                    for i in 0..dep_count {
                        package.add_dependency(&format!("dep_{}", i));
                    }
                    black_box(package);
                });
            });
        }
        group.finish();
    }
}

/// Performance optimization benchmarks
pub mod performance_module {
    use super::*;
    
    pub fn benchmark_memory_allocation(c: &mut Criterion) {
        let mut group = c.benchmark_group("memory_allocation");
        
        group.bench_function("vector_allocation", |b| {
            b.iter(|| {
                let mut vec: Vec<i32> = Vec::new();
                for i in 0..1000 {
                    vec.push(i);
                }
                black_box(vec);
            });
        });
        
        group.bench_function("hashmap_allocation", |b| {
            b.iter(|| {
                let mut map: HashMap<String, i32> = HashMap::new();
                for i in 0..100 {
                    map.insert(format!("key_{}", i), i);
                }
                black_box(map);
            });
        });
        
        group.finish();
    }
    
    pub fn benchmark_string_operations(c: &mut Criterion) {
        let test_strings = vec![
            "package_name_1", "package_name_2", "package_name_3",
            "very_long_package_name_with_many_characters",
            "short", "medium_length_name"
        ];
        
        c.bench_function("string_concatenation", |b| {
            b.iter(|| {
                let mut result = String::new();
                for s in &test_strings {
                    result.push_str(s);
                    result.push('_');
                }
                black_box(result);
            });
        });
    }
}

/// Main benchmark entry point
fn comprehensive_benchmarks(c: &mut Criterion) {
    println!("🚀 Running Unified Benchmark Suite");
    println!("==================================");
    
    // Run version module benchmarks
    println!("📦 Running Version Module Benchmarks...");
    version_module::benchmark_version_parsing(c);
    version_module::benchmark_version_comparison(c);
    version_module::benchmark_version_sorting(c);
    
    // Run package module benchmarks
    println!("📦 Running Package Module Benchmarks...");
    package_module::benchmark_package_creation(c);
    package_module::benchmark_dependency_management(c);
    
    // Run performance benchmarks
    println!("⚡ Running Performance Benchmarks...");
    performance_module::benchmark_memory_allocation(c);
    performance_module::benchmark_string_operations(c);
    
    println!("✅ All benchmarks completed successfully");
}

criterion_group!(unified_benches, comprehensive_benchmarks);
criterion_main!(unified_benches);
