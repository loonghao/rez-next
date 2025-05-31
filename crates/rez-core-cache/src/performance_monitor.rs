//! Performance Monitor
//!
//! This module provides comprehensive performance monitoring and benchmarking
//! for the intelligent cache system.

use crate::{MonitoringConfig, PerformanceMetrics};
use serde::{Serialize, Deserialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock, atomic::{AtomicU64, Ordering}},
    time::{Duration, Instant, SystemTime},
};
use tokio::time::Interval;


/// Real-time performance counters
#[derive(Debug)]
pub struct PerformanceCounters {
    /// Total get operations
    pub get_operations: AtomicU64,
    /// Total put operations
    pub put_operations: AtomicU64,
    /// Total remove operations
    pub remove_operations: AtomicU64,
    /// Total get latency (microseconds)
    pub total_get_latency_us: AtomicU64,
    /// Total put latency (microseconds)
    pub total_put_latency_us: AtomicU64,
    /// Total remove latency (microseconds)
    pub total_remove_latency_us: AtomicU64,
    /// Peak memory usage
    pub peak_memory_usage: AtomicU64,
    /// Current memory usage
    pub current_memory_usage: AtomicU64,
    /// Total bytes read from disk
    pub disk_bytes_read: AtomicU64,
    /// Total bytes written to disk
    pub disk_bytes_written: AtomicU64,
}

impl Default for PerformanceCounters {
    fn default() -> Self {
        Self {
            get_operations: AtomicU64::new(0),
            put_operations: AtomicU64::new(0),
            remove_operations: AtomicU64::new(0),
            total_get_latency_us: AtomicU64::new(0),
            total_put_latency_us: AtomicU64::new(0),
            total_remove_latency_us: AtomicU64::new(0),
            peak_memory_usage: AtomicU64::new(0),
            current_memory_usage: AtomicU64::new(0),
            disk_bytes_read: AtomicU64::new(0),
            disk_bytes_written: AtomicU64::new(0),
        }
    }
}

/// Performance event for detailed logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceEvent {
    /// Event timestamp
    pub timestamp: SystemTime,
    /// Event type
    pub event_type: PerformanceEventType,
    /// Operation latency (microseconds)
    pub latency_us: u64,
    /// Memory usage at time of event
    pub memory_usage: u64,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Types of performance events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceEventType {
    /// Cache get operation
    CacheGet,
    /// Cache put operation
    CachePut,
    /// Cache remove operation
    CacheRemove,
    /// Cache eviction
    CacheEviction,
    /// Cache promotion (L2 to L1)
    CachePromotion,
    /// Cache demotion (L1 to L2)
    CacheDemotion,
    /// Predictive preheating
    PredictivePreheating,
    /// Adaptive tuning
    AdaptiveTuning,
}

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Operations per second
    pub ops_per_second: f64,
    /// Average latency (microseconds)
    pub avg_latency_us: f64,
    /// 95th percentile latency (microseconds)
    pub p95_latency_us: f64,
    /// 99th percentile latency (microseconds)
    pub p99_latency_us: f64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
    /// Hit rate achieved
    pub hit_rate: f64,
    /// Test duration
    pub duration: Duration,
    /// Timestamp
    pub timestamp: SystemTime,
}

/// Performance Monitor
///
/// Provides comprehensive monitoring, metrics collection, and benchmarking
/// for the intelligent cache system.
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Configuration
    config: MonitoringConfig,
    /// Real-time performance counters
    counters: Arc<PerformanceCounters>,
    /// Performance event log
    event_log: Arc<RwLock<VecDeque<PerformanceEvent>>>,
    /// Latency histogram buckets (microseconds)
    latency_histogram: Arc<RwLock<HashMap<u64, u64>>>,
    /// Benchmark results history
    benchmark_history: Arc<RwLock<Vec<BenchmarkResult>>>,
    /// Monitoring start time
    start_time: Instant,
    /// Background monitoring interval
    _monitoring_interval: Option<Interval>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            counters: Arc::new(PerformanceCounters::default()),
            event_log: Arc::new(RwLock::new(VecDeque::new())),
            latency_histogram: Arc::new(RwLock::new(HashMap::new())),
            benchmark_history: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
            _monitoring_interval: None,
        }
    }

    /// Record a get operation latency
    pub async fn record_get_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        
        self.counters.get_operations.fetch_add(1, Ordering::Relaxed);
        self.counters.total_get_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        self.update_latency_histogram(latency_us).await;
        
        if self.config.enable_event_logging {
            self.log_event(PerformanceEvent {
                timestamp: SystemTime::now(),
                event_type: PerformanceEventType::CacheGet,
                latency_us,
                memory_usage: self.counters.current_memory_usage.load(Ordering::Relaxed),
                metadata: HashMap::new(),
            }).await;
        }
    }

    /// Record a put operation latency
    pub async fn record_put_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        
        self.counters.put_operations.fetch_add(1, Ordering::Relaxed);
        self.counters.total_put_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        self.update_latency_histogram(latency_us).await;
        
        if self.config.enable_event_logging {
            self.log_event(PerformanceEvent {
                timestamp: SystemTime::now(),
                event_type: PerformanceEventType::CachePut,
                latency_us,
                memory_usage: self.counters.current_memory_usage.load(Ordering::Relaxed),
                metadata: HashMap::new(),
            }).await;
        }
    }

    /// Record a remove operation latency
    pub async fn record_remove_latency(&self, latency: Duration) {
        let latency_us = latency.as_micros() as u64;
        
        self.counters.remove_operations.fetch_add(1, Ordering::Relaxed);
        self.counters.total_remove_latency_us.fetch_add(latency_us, Ordering::Relaxed);
        
        self.update_latency_histogram(latency_us).await;
        
        if self.config.enable_event_logging {
            self.log_event(PerformanceEvent {
                timestamp: SystemTime::now(),
                event_type: PerformanceEventType::CacheRemove,
                latency_us,
                memory_usage: self.counters.current_memory_usage.load(Ordering::Relaxed),
                metadata: HashMap::new(),
            }).await;
        }
    }

    /// Update memory usage
    pub async fn update_memory_usage(&self, current_usage: u64) {
        self.counters.current_memory_usage.store(current_usage, Ordering::Relaxed);
        
        // Update peak if necessary
        let current_peak = self.counters.peak_memory_usage.load(Ordering::Relaxed);
        if current_usage > current_peak {
            self.counters.peak_memory_usage.store(current_usage, Ordering::Relaxed);
        }
    }

    /// Record disk I/O
    pub async fn record_disk_io(&self, bytes_read: u64, bytes_written: u64) {
        self.counters.disk_bytes_read.fetch_add(bytes_read, Ordering::Relaxed);
        self.counters.disk_bytes_written.fetch_add(bytes_written, Ordering::Relaxed);
    }

    /// Log a performance event
    async fn log_event(&self, event: PerformanceEvent) {
        if !self.config.enable_event_logging {
            return;
        }

        let mut log = self.event_log.write().unwrap();
        log.push_back(event);
        
        // Keep log size manageable
        while log.len() > self.config.max_events_in_memory {
            log.pop_front();
        }
    }

    /// Update latency histogram
    async fn update_latency_histogram(&self, latency_us: u64) {
        // Round to nearest bucket (powers of 2)
        let bucket = if latency_us == 0 {
            0
        } else {
            1u64 << (64 - latency_us.leading_zeros())
        };

        let mut histogram = self.latency_histogram.write().unwrap();
        *histogram.entry(bucket).or_insert(0) += 1;
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        let get_ops = self.counters.get_operations.load(Ordering::Relaxed);
        let put_ops = self.counters.put_operations.load(Ordering::Relaxed);
        let remove_ops = self.counters.remove_operations.load(Ordering::Relaxed);
        
        let total_get_latency = self.counters.total_get_latency_us.load(Ordering::Relaxed);
        let total_put_latency = self.counters.total_put_latency_us.load(Ordering::Relaxed);
        
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        let total_ops = get_ops + put_ops + remove_ops;

        PerformanceMetrics {
            avg_get_latency_us: if get_ops > 0 {
                total_get_latency as f64 / get_ops as f64
            } else {
                0.0
            },
            avg_put_latency_us: if put_ops > 0 {
                total_put_latency as f64 / put_ops as f64
            } else {
                0.0
            },
            avg_eviction_latency_us: 0.0, // TODO: Track eviction latency
            ops_per_second: if elapsed_secs > 0.0 {
                total_ops as f64 / elapsed_secs
            } else {
                0.0
            },
            memory_allocation_rate: 0.0, // TODO: Track allocation rate
            disk_io_rate: if elapsed_secs > 0.0 {
                (self.counters.disk_bytes_read.load(Ordering::Relaxed) + 
                 self.counters.disk_bytes_written.load(Ordering::Relaxed)) as f64 / elapsed_secs
            } else {
                0.0
            },
            cpu_usage_percent: 0.0, // TODO: Track CPU usage
            peak_memory_usage: self.counters.peak_memory_usage.load(Ordering::Relaxed),
        }
    }

    /// Run a benchmark
    pub async fn run_benchmark<F, Fut>(&self, name: &str, benchmark_fn: F) -> BenchmarkResult
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = ()>,
    {
        let start_time = Instant::now();
        let start_ops = self.counters.get_operations.load(Ordering::Relaxed) +
                       self.counters.put_operations.load(Ordering::Relaxed);
        
        // Reset latency histogram for this benchmark
        {
            let mut histogram = self.latency_histogram.write().unwrap();
            histogram.clear();
        }

        // Run the benchmark
        benchmark_fn().await;

        let duration = start_time.elapsed();
        let end_ops = self.counters.get_operations.load(Ordering::Relaxed) +
                     self.counters.put_operations.load(Ordering::Relaxed);
        
        let ops_performed = end_ops - start_ops;
        let ops_per_second = if duration.as_secs_f64() > 0.0 {
            ops_performed as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        // Calculate latency percentiles
        let (avg_latency, p95_latency, p99_latency) = self.calculate_latency_percentiles().await;

        let result = BenchmarkResult {
            name: name.to_string(),
            ops_per_second,
            avg_latency_us: avg_latency,
            p95_latency_us: p95_latency,
            p99_latency_us: p99_latency,
            memory_usage: self.counters.current_memory_usage.load(Ordering::Relaxed),
            hit_rate: 0.0, // TODO: Calculate hit rate for benchmark
            duration,
            timestamp: SystemTime::now(),
        };

        // Store benchmark result
        {
            let mut history = self.benchmark_history.write().unwrap();
            history.push(result.clone());
        }

        result
    }

    /// Calculate latency percentiles from histogram
    async fn calculate_latency_percentiles(&self) -> (f64, f64, f64) {
        let histogram = self.latency_histogram.read().unwrap();
        
        if histogram.is_empty() {
            return (0.0, 0.0, 0.0);
        }

        // Convert histogram to sorted vector
        let mut latencies: Vec<(u64, u64)> = histogram.iter()
            .map(|(&latency, &count)| (latency, count))
            .collect();
        latencies.sort_by_key(|&(latency, _)| latency);

        let total_samples: u64 = latencies.iter().map(|(_, count)| count).sum();
        if total_samples == 0 {
            return (0.0, 0.0, 0.0);
        }

        // Calculate weighted average
        let weighted_sum: u64 = latencies.iter()
            .map(|(latency, count)| latency * count)
            .sum();
        let avg_latency = weighted_sum as f64 / total_samples as f64;

        // Calculate percentiles
        let p95_target = (total_samples as f64 * 0.95) as u64;
        let p99_target = (total_samples as f64 * 0.99) as u64;

        let mut cumulative = 0u64;
        let mut p95_latency = 0.0;
        let mut p99_latency = 0.0;

        for &(latency, count) in &latencies {
            cumulative += count;
            
            if p95_latency == 0.0 && cumulative >= p95_target {
                p95_latency = latency as f64;
            }
            
            if p99_latency == 0.0 && cumulative >= p99_target {
                p99_latency = latency as f64;
                break;
            }
        }

        (avg_latency, p95_latency, p99_latency)
    }

    /// Get recent performance events
    pub async fn get_recent_events(&self, limit: usize) -> Vec<PerformanceEvent> {
        let log = self.event_log.read().unwrap();
        log.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// Get benchmark history
    pub async fn get_benchmark_history(&self) -> Vec<BenchmarkResult> {
        self.benchmark_history.read().unwrap().clone()
    }

    /// Reset all counters and statistics
    pub async fn reset(&self) {
        // Reset counters
        self.counters.get_operations.store(0, Ordering::Relaxed);
        self.counters.put_operations.store(0, Ordering::Relaxed);
        self.counters.remove_operations.store(0, Ordering::Relaxed);
        self.counters.total_get_latency_us.store(0, Ordering::Relaxed);
        self.counters.total_put_latency_us.store(0, Ordering::Relaxed);
        self.counters.total_remove_latency_us.store(0, Ordering::Relaxed);
        self.counters.peak_memory_usage.store(0, Ordering::Relaxed);
        self.counters.current_memory_usage.store(0, Ordering::Relaxed);
        self.counters.disk_bytes_read.store(0, Ordering::Relaxed);
        self.counters.disk_bytes_written.store(0, Ordering::Relaxed);

        // Clear logs and histograms
        {
            let mut log = self.event_log.write().unwrap();
            log.clear();
        }
        
        {
            let mut histogram = self.latency_histogram.write().unwrap();
            histogram.clear();
        }
    }
}
