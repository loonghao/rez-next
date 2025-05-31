//! Intelligent Cache Benchmarks
//!
//! Comprehensive benchmarking suite for the intelligent cache system.

use crate::{
    IntelligentCacheManager, UnifiedCacheConfig, UnifiedCache,
    PerformanceMonitor, BenchmarkResult,
};
use std::{
    sync::Arc,
    time::Duration,
};
use tokio::time::sleep;

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of operations per benchmark
    pub operations_count: usize,
    /// Number of concurrent workers
    pub worker_count: usize,
    /// Key space size (number of unique keys)
    pub key_space_size: usize,
    /// Value size in bytes
    pub value_size: usize,
    /// Read/write ratio (0.0 = all writes, 1.0 = all reads)
    pub read_write_ratio: f64,
    /// Benchmark duration limit
    pub max_duration: Duration,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            operations_count: 100_000,
            worker_count: 4,
            key_space_size: 10_000,
            value_size: 1024,
            read_write_ratio: 0.8, // 80% reads, 20% writes
            max_duration: Duration::from_secs(60),
        }
    }
}

/// Benchmark suite for intelligent cache
pub struct CacheBenchmarkSuite {
    /// Cache manager under test
    cache: Arc<IntelligentCacheManager<String, Vec<u8>>>,
    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,
    /// Benchmark configuration
    config: BenchmarkConfig,
}

impl CacheBenchmarkSuite {
    /// Create a new benchmark suite
    pub fn new(cache_config: UnifiedCacheConfig, bench_config: BenchmarkConfig) -> Self {
        let cache = Arc::new(IntelligentCacheManager::new(cache_config));
        let monitor = cache.monitor();
        
        Self {
            cache,
            monitor,
            config: bench_config,
        }
    }

    /// Run all benchmarks
    pub async fn run_all_benchmarks(&self) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();

        // Basic performance benchmarks
        results.push(self.benchmark_sequential_operations().await);
        results.push(self.benchmark_concurrent_operations().await);
        results.push(self.benchmark_mixed_workload().await);
        
        // Cache-specific benchmarks
        results.push(self.benchmark_hit_rate_optimization().await);
        results.push(self.benchmark_memory_efficiency().await);
        results.push(self.benchmark_predictive_preheating().await);
        results.push(self.benchmark_adaptive_tuning().await);

        // Stress tests
        results.push(self.benchmark_high_contention().await);
        results.push(self.benchmark_large_dataset().await);

        results
    }

    /// Benchmark sequential operations
    pub async fn benchmark_sequential_operations(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("sequential_operations", || async {
            let test_data = self.generate_test_data();
            
            // Sequential writes
            for (key, value) in &test_data {
                let _ = self.cache.put(key.clone(), value.clone()).await;
            }
            
            // Sequential reads
            for (key, _) in &test_data {
                let _ = self.cache.get(key).await;
            }
        }).await
    }

    /// Benchmark concurrent operations
    pub async fn benchmark_concurrent_operations(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("concurrent_operations", || async {
            let test_data = Arc::new(self.generate_test_data());
            let cache = Arc::clone(&self.cache);
            
            let mut handles = Vec::new();
            
            for worker_id in 0..self.config.worker_count {
                let cache = Arc::clone(&cache);
                let test_data = Arc::clone(&test_data);
                let ops_per_worker = self.config.operations_count / self.config.worker_count;
                
                let handle = tokio::spawn(async move {
                    for i in 0..ops_per_worker {
                        let key_index = (worker_id * ops_per_worker + i) % test_data.len();
                        let (key, value) = &test_data[key_index];
                        
                        if i % 5 == 0 {
                            // Write operation
                            let _ = cache.put(key.clone(), value.clone()).await;
                        } else {
                            // Read operation
                            let _ = cache.get(key).await;
                        }
                    }
                });
                
                handles.push(handle);
            }
            
            // Wait for all workers to complete
            for handle in handles {
                let _ = handle.await;
            }
        }).await
    }

    /// Benchmark mixed workload (reads and writes)
    pub async fn benchmark_mixed_workload(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("mixed_workload", || async {
            let test_data = self.generate_test_data();
            
            for (i, (key, value)) in test_data.iter().enumerate() {
                let operation_ratio = i as f64 / test_data.len() as f64;
                
                if operation_ratio < self.config.read_write_ratio {
                    // Read operation
                    let _ = self.cache.get(key).await;
                } else {
                    // Write operation
                    let _ = self.cache.put(key.clone(), value.clone()).await;
                }
            }
        }).await
    }

    /// Benchmark hit rate optimization
    pub async fn benchmark_hit_rate_optimization(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("hit_rate_optimization", || async {
            let test_data = self.generate_test_data();
            
            // Phase 1: Populate cache with hot data
            let hot_data_size = test_data.len() / 4; // 25% hot data
            for (key, value) in test_data.iter().take(hot_data_size) {
                let _ = self.cache.put(key.clone(), value.clone()).await;
            }
            
            // Phase 2: Access pattern simulation (80/20 rule)
            for _ in 0..self.config.operations_count {
                let key_index = if rand::random::<f64>() < 0.8 {
                    // 80% access to hot data
                    rand::random::<usize>() % hot_data_size
                } else {
                    // 20% access to cold data
                    hot_data_size + (rand::random::<usize>() % (test_data.len() - hot_data_size))
                };
                
                let (key, value) = &test_data[key_index];
                
                if self.cache.get(key).await.is_none() {
                    // Cache miss - populate
                    let _ = self.cache.put(key.clone(), value.clone()).await;
                }
            }
        }).await
    }

    /// Benchmark memory efficiency
    pub async fn benchmark_memory_efficiency(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("memory_efficiency", || async {
            let large_value = vec![0u8; self.config.value_size * 10]; // 10x larger values
            
            // Fill cache to capacity
            for i in 0..self.config.key_space_size {
                let key = format!("large_key_{}", i);
                let _ = self.cache.put(key, large_value.clone()).await;
            }
            
            // Trigger evictions with new data
            for i in 0..self.config.key_space_size / 2 {
                let key = format!("new_key_{}", i);
                let _ = self.cache.put(key, large_value.clone()).await;
            }
        }).await
    }

    /// Benchmark predictive preheating
    pub async fn benchmark_predictive_preheating(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("predictive_preheating", || async {
            let test_data = self.generate_test_data();
            
            // Create predictable access patterns
            for cycle in 0..10 {
                for (i, (key, value)) in test_data.iter().enumerate() {
                    if i % 10 == cycle % 10 {
                        // Access pattern: every 10th item in rotation
                        if self.cache.get(key).await.is_none() {
                            let _ = self.cache.put(key.clone(), value.clone()).await;
                        }
                    }
                }
                
                // Wait for pattern learning
                sleep(Duration::from_millis(100)).await;
            }
            
            // Test prediction accuracy
            for (i, (key, _)) in test_data.iter().enumerate() {
                if i % 10 == 0 {
                    // These should be preheated
                    let _ = self.cache.get(key).await;
                }
            }
        }).await
    }

    /// Benchmark adaptive tuning
    pub async fn benchmark_adaptive_tuning(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("adaptive_tuning", || async {
            let test_data = self.generate_test_data();
            
            // Phase 1: Low hit rate workload
            for _ in 0..1000 {
                let key_index = rand::random::<usize>() % test_data.len();
                let (key, value) = &test_data[key_index];
                
                if self.cache.get(key).await.is_none() {
                    let _ = self.cache.put(key.clone(), value.clone()).await;
                }
            }
            
            // Wait for tuning
            sleep(Duration::from_secs(1)).await;
            
            // Phase 2: High hit rate workload
            let hot_keys: Vec<_> = test_data.iter().take(100).collect();
            for _ in 0..1000 {
                let (key, value) = hot_keys[rand::random::<usize>() % hot_keys.len()];
                
                if self.cache.get(key).await.is_none() {
                    let _ = self.cache.put(key.clone(), value.clone()).await;
                }
            }
        }).await
    }

    /// Benchmark high contention scenarios
    pub async fn benchmark_high_contention(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("high_contention", || async {
            let hot_keys = vec!["hot_key_1", "hot_key_2", "hot_key_3"];
            let test_value = vec![0u8; self.config.value_size];
            
            let cache = Arc::clone(&self.cache);
            let mut handles = Vec::new();
            
            // Multiple workers accessing the same hot keys
            for _ in 0..self.config.worker_count * 2 {
                let cache = Arc::clone(&cache);
                let hot_keys = hot_keys.clone();
                let test_value = test_value.clone();
                
                let handle = tokio::spawn(async move {
                    for _ in 0..1000 {
                        let key = &hot_keys[rand::random::<usize>() % hot_keys.len()];
                        
                        if rand::random::<bool>() {
                            let _ = cache.get(&key.to_string()).await;
                        } else {
                            let _ = cache.put(key.to_string(), test_value.clone()).await;
                        }
                    }
                });
                
                handles.push(handle);
            }
            
            for handle in handles {
                let _ = handle.await;
            }
        }).await
    }

    /// Benchmark large dataset handling
    pub async fn benchmark_large_dataset(&self) -> BenchmarkResult {
        self.monitor.run_benchmark("large_dataset", || async {
            let large_dataset_size = self.config.key_space_size * 10;
            let test_value = vec![0u8; self.config.value_size];
            
            // Populate with large dataset
            for i in 0..large_dataset_size {
                let key = format!("large_dataset_key_{}", i);
                let _ = self.cache.put(key, test_value.clone()).await;
                
                // Periodic reads to test cache behavior under load
                if i % 1000 == 0 {
                    for j in 0..100 {
                        let read_key = format!("large_dataset_key_{}", j);
                        let _ = self.cache.get(&read_key).await;
                    }
                }
            }
        }).await
    }

    /// Generate test data for benchmarks
    fn generate_test_data(&self) -> Vec<(String, Vec<u8>)> {
        (0..self.config.key_space_size)
            .map(|i| {
                let key = format!("test_key_{}", i);
                let value = vec![i as u8; self.config.value_size];
                (key, value)
            })
            .collect()
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> crate::UnifiedCacheStats {
        self.cache.get_stats().await
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> crate::PerformanceMetrics {
        self.monitor.get_performance_metrics().await
    }
}

/// Run a comprehensive benchmark suite
pub async fn run_comprehensive_benchmarks() -> Vec<BenchmarkResult> {
    println!("🚀 Starting Intelligent Cache Comprehensive Benchmarks");
    
    // Test different configurations
    let configs = vec![
        ("High Performance", UnifiedCacheConfig::high_performance()),
        ("Low Memory", UnifiedCacheConfig::low_memory()),
        ("Default", UnifiedCacheConfig::default()),
    ];
    
    let mut all_results = Vec::new();
    
    for (config_name, cache_config) in configs {
        println!("\n📊 Testing configuration: {}", config_name);
        
        let bench_config = BenchmarkConfig::default();
        let suite = CacheBenchmarkSuite::new(cache_config, bench_config);
        
        let results = suite.run_all_benchmarks().await;
        
        println!("✅ Completed {} benchmarks for {}", results.len(), config_name);
        for result in &results {
            println!("  {} - {:.2} ops/sec, {:.2}μs avg latency", 
                result.name, result.ops_per_second, result.avg_latency_us);
        }
        
        all_results.extend(results);
    }
    
    println!("\n🎯 Benchmark Summary:");
    println!("Total benchmarks completed: {}", all_results.len());
    
    all_results
}
