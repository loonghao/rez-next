//! Unified cache statistics and monitoring
//!
//! This module provides comprehensive statistics collection and monitoring
//! for the intelligent caching system, integrating with existing cache
//! statistics while adding new metrics for multi-level caching and
//! predictive preheating.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Unified cache statistics
///
/// Aggregates statistics from all cache levels and intelligent features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCacheStats {
    /// L1 cache statistics
    pub l1_stats: CacheLevelStats,
    /// L2 cache statistics
    pub l2_stats: CacheLevelStats,
    /// Overall cache statistics
    pub overall_stats: OverallCacheStats,
    /// Predictive preheating statistics
    pub preheating_stats: CachePreheatingStats,
    /// Adaptive tuning statistics
    pub tuning_stats: TuningStats,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    /// Statistics collection timestamp
    pub timestamp: u64,
}

impl Default for UnifiedCacheStats {
    fn default() -> Self {
        Self {
            l1_stats: CacheLevelStats::default(),
            l2_stats: CacheLevelStats::default(),
            overall_stats: OverallCacheStats::default(),
            preheating_stats: CachePreheatingStats::default(),
            tuning_stats: TuningStats::default(),
            performance_metrics: PerformanceMetrics::default(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Statistics for a single cache level (L1, L2, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLevelStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Current number of entries
    pub entries: usize,
    /// Maximum capacity
    pub capacity: usize,
    /// Estimated memory/disk usage in bytes
    pub usage_bytes: u64,
    /// Maximum allowed usage in bytes
    pub max_usage_bytes: u64,
    /// Cache evictions
    pub evictions: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Load factor (entries / capacity)
    pub load_factor: f64,
    /// Average entry size in bytes
    pub avg_entry_size: f64,
}

impl Default for CacheLevelStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            capacity: 0,
            usage_bytes: 0,
            max_usage_bytes: 0,
            evictions: 0,
            hit_rate: 0.0,
            load_factor: 0.0,
            avg_entry_size: 0.0,
        }
    }
}

impl CacheLevelStats {
    /// Update hit rate based on current hits and misses
    pub fn update_hit_rate(&mut self) {
        let total_requests = self.hits + self.misses;
        if total_requests > 0 {
            self.hit_rate = self.hits as f64 / total_requests as f64;
        }
    }

    /// Update load factor based on current entries and capacity
    pub fn update_load_factor(&mut self) {
        if self.capacity > 0 {
            self.load_factor = self.entries as f64 / self.capacity as f64;
        }
    }

    /// Update average entry size
    pub fn update_avg_entry_size(&mut self) {
        if self.entries > 0 {
            self.avg_entry_size = self.usage_bytes as f64 / self.entries as f64;
        }
    }

    /// Update all calculated fields
    pub fn update_calculated_fields(&mut self) {
        self.update_hit_rate();
        self.update_load_factor();
        self.update_avg_entry_size();
    }
}

/// Overall cache system statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallCacheStats {
    /// Total hits across all cache levels
    pub total_hits: u64,
    /// Total misses across all cache levels
    pub total_misses: u64,
    /// Overall hit rate
    pub overall_hit_rate: f64,
    /// Total entries across all levels
    pub total_entries: usize,
    /// Total memory usage across all levels
    pub total_memory_bytes: u64,
    /// Total disk usage
    pub total_disk_bytes: u64,
    /// Cache efficiency score (0.0 to 1.0)
    pub efficiency_score: f64,
    /// Data promotion count (L2 to L1)
    pub promotions: u64,
    /// Data demotion count (L1 to L2)
    pub demotions: u64,
}

impl Default for OverallCacheStats {
    fn default() -> Self {
        Self {
            total_hits: 0,
            total_misses: 0,
            overall_hit_rate: 0.0,
            total_entries: 0,
            total_memory_bytes: 0,
            total_disk_bytes: 0,
            efficiency_score: 0.0,
            promotions: 0,
            demotions: 0,
        }
    }
}

/// Predictive preheating statistics (cache level)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePreheatingStats {
    /// Total preheating attempts
    pub preheat_attempts: u64,
    /// Successful preheating operations
    pub preheat_successes: u64,
    /// Preheating hit rate (preheated entries that were accessed)
    pub preheat_hit_rate: f64,
    /// Total entries preheated
    pub entries_preheated: u64,
    /// Entries preheated that were actually accessed
    pub preheated_entries_accessed: u64,
    /// Average prediction confidence
    pub avg_prediction_confidence: f64,
    /// Time saved by preheating (in milliseconds)
    pub time_saved_ms: u64,
    /// CPU time used for preheating (in milliseconds)
    pub cpu_time_used_ms: u64,
}

impl Default for CachePreheatingStats {
    fn default() -> Self {
        Self {
            preheat_attempts: 0,
            preheat_successes: 0,
            preheat_hit_rate: 0.0,
            entries_preheated: 0,
            preheated_entries_accessed: 0,
            avg_prediction_confidence: 0.0,
            time_saved_ms: 0,
            cpu_time_used_ms: 0,
        }
    }
}

/// Adaptive tuning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningStats {
    /// Total tuning cycles performed
    pub tuning_cycles: u64,
    /// Successful tuning adjustments
    pub successful_adjustments: u64,
    /// TTL adjustments made
    pub ttl_adjustments: u64,
    /// Capacity adjustments made
    pub capacity_adjustments: u64,
    /// Eviction strategy changes
    pub eviction_strategy_changes: u64,
    /// Performance improvement from tuning (percentage)
    pub performance_improvement: f64,
    /// Last tuning timestamp
    pub last_tuning_timestamp: u64,
    /// Current tuning stability score (0.0 to 1.0)
    pub stability_score: f64,
}

impl Default for TuningStats {
    fn default() -> Self {
        Self {
            tuning_cycles: 0,
            successful_adjustments: 0,
            ttl_adjustments: 0,
            capacity_adjustments: 0,
            eviction_strategy_changes: 0,
            performance_improvement: 0.0,
            last_tuning_timestamp: 0,
            stability_score: 1.0,
        }
    }
}

/// Performance metrics for cache operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average get operation latency (in microseconds)
    pub avg_get_latency_us: f64,
    /// Average put operation latency (in microseconds)
    pub avg_put_latency_us: f64,
    /// Average eviction latency (in microseconds)
    pub avg_eviction_latency_us: f64,
    /// Operations per second
    pub ops_per_second: f64,
    /// Memory allocation rate (bytes per second)
    pub memory_allocation_rate: f64,
    /// Disk I/O rate (bytes per second)
    pub disk_io_rate: f64,
    /// CPU usage percentage for cache operations
    pub cpu_usage_percent: f64,
    /// Peak memory usage (in bytes)
    pub peak_memory_usage: u64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_get_latency_us: 0.0,
            avg_put_latency_us: 0.0,
            avg_eviction_latency_us: 0.0,
            ops_per_second: 0.0,
            memory_allocation_rate: 0.0,
            disk_io_rate: 0.0,
            cpu_usage_percent: 0.0,
            peak_memory_usage: 0,
        }
    }
}

impl UnifiedCacheStats {
    /// Create a new statistics instance with current timestamp
    pub fn new() -> Self {
        Self::default()
    }

    /// Update overall statistics from individual cache level stats
    pub fn update_overall_stats(&mut self) {
        self.overall_stats.total_hits = self.l1_stats.hits + self.l2_stats.hits;
        self.overall_stats.total_misses = self.l1_stats.misses + self.l2_stats.misses;

        let total_requests = self.overall_stats.total_hits + self.overall_stats.total_misses;
        if total_requests > 0 {
            self.overall_stats.overall_hit_rate =
                self.overall_stats.total_hits as f64 / total_requests as f64;
        }

        self.overall_stats.total_entries = self.l1_stats.entries + self.l2_stats.entries;
        self.overall_stats.total_memory_bytes = self.l1_stats.usage_bytes;
        self.overall_stats.total_disk_bytes = self.l2_stats.usage_bytes;

        // Calculate efficiency score based on hit rate and resource usage
        let hit_rate_score = self.overall_stats.overall_hit_rate;
        let memory_efficiency = if self.l1_stats.max_usage_bytes > 0 {
            1.0 - (self.l1_stats.usage_bytes as f64 / self.l1_stats.max_usage_bytes as f64)
        } else {
            1.0
        };
        self.overall_stats.efficiency_score = (hit_rate_score + memory_efficiency) / 2.0;

        // Update timestamp
        self.timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Get a summary report as a formatted string
    pub fn summary_report(&self) -> String {
        format!(
            "Cache Statistics Summary:\n\
             Overall Hit Rate: {:.2}%\n\
             Total Entries: {}\n\
             Memory Usage: {:.2} MB\n\
             Disk Usage: {:.2} MB\n\
             Efficiency Score: {:.2}\n\
             Preheating Hit Rate: {:.2}%\n\
             Tuning Cycles: {}",
            self.overall_stats.overall_hit_rate * 100.0,
            self.overall_stats.total_entries,
            self.overall_stats.total_memory_bytes as f64 / (1024.0 * 1024.0),
            self.overall_stats.total_disk_bytes as f64 / (1024.0 * 1024.0),
            self.overall_stats.efficiency_score,
            self.preheating_stats.preheat_hit_rate * 100.0,
            self.tuning_stats.tuning_cycles
        )
    }

    /// Check if the cache is performing well based on target metrics
    pub fn is_performing_well(&self, target_hit_rate: f64) -> bool {
        self.overall_stats.overall_hit_rate >= target_hit_rate
            && self.overall_stats.efficiency_score >= 0.7
            && self.tuning_stats.stability_score >= 0.8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_level_stats() {
        let mut stats = CacheLevelStats::default();
        stats.hits = 80;
        stats.misses = 20;
        stats.entries = 100;
        stats.capacity = 200;
        stats.usage_bytes = 1024;

        stats.update_calculated_fields();

        assert_eq!(stats.hit_rate, 0.8);
        assert_eq!(stats.load_factor, 0.5);
        assert_eq!(stats.avg_entry_size, 10.24);
    }

    #[test]
    fn test_unified_cache_stats() {
        let mut stats = UnifiedCacheStats::new();
        stats.l1_stats.hits = 70;
        stats.l1_stats.misses = 10;
        stats.l2_stats.hits = 20;
        stats.l2_stats.misses = 5;

        stats.update_overall_stats();

        assert_eq!(stats.overall_stats.total_hits, 90);
        assert_eq!(stats.overall_stats.total_misses, 15);
        assert!((stats.overall_stats.overall_hit_rate - 0.857).abs() < 0.01);
    }

    #[test]
    fn test_performance_check() {
        let mut stats = UnifiedCacheStats::new();
        stats.overall_stats.overall_hit_rate = 0.95;
        stats.overall_stats.efficiency_score = 0.8;
        stats.tuning_stats.stability_score = 0.9;

        assert!(stats.is_performing_well(0.9));
        assert!(!stats.is_performing_well(0.96));
    }
}
