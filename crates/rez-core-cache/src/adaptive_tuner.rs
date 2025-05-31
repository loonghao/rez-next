//! Adaptive Cache Tuner
//!
//! This module provides real-time cache parameter optimization based on
//! performance metrics and workload characteristics.

use crate::{TuningConfig, UnifiedCacheStats};
use serde::{Serialize, Deserialize};
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
    time::SystemTime,
};
use tokio::time::Interval;


/// Cache performance metrics for tuning decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: SystemTime,
    /// Overall hit rate
    pub hit_rate: f64,
    /// L1 hit rate
    pub l1_hit_rate: f64,
    /// L2 hit rate
    pub l2_hit_rate: f64,
    /// Memory usage percentage
    pub memory_usage_percent: f64,
    /// Disk usage percentage
    pub disk_usage_percent: f64,
    /// Average get latency (microseconds)
    pub avg_get_latency_us: f64,
    /// Average put latency (microseconds)
    pub avg_put_latency_us: f64,
    /// Operations per second
    pub ops_per_second: f64,
    /// Eviction rate (evictions per minute)
    pub eviction_rate: f64,
    /// Promotion rate (L2 to L1 promotions per minute)
    pub promotion_rate: f64,
}

impl Default for PerformanceSnapshot {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            hit_rate: 0.0,
            l1_hit_rate: 0.0,
            l2_hit_rate: 0.0,
            memory_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            avg_get_latency_us: 0.0,
            avg_put_latency_us: 0.0,
            ops_per_second: 0.0,
            eviction_rate: 0.0,
            promotion_rate: 0.0,
        }
    }
}

/// Tuning recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningRecommendation {
    /// Parameter to tune
    pub parameter: String,
    /// Current value
    pub current_value: f64,
    /// Recommended value
    pub recommended_value: f64,
    /// Confidence in recommendation (0.0 to 1.0)
    pub confidence: f64,
    /// Expected improvement
    pub expected_improvement: f64,
    /// Reason for recommendation
    pub reason: String,
}

/// Adaptive tuning statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveTuningStats {
    /// Total tuning operations performed
    pub tuning_operations: u64,
    /// Successful tuning operations (improved performance)
    pub successful_tunings: u64,
    /// Failed tuning operations (degraded performance)
    pub failed_tunings: u64,
    /// Success rate
    pub success_rate: f64,
    /// Average performance improvement
    pub avg_improvement: f64,
    /// Current tuning confidence
    pub current_confidence: f64,
    /// Last tuning timestamp
    pub last_tuning: SystemTime,
}

impl Default for AdaptiveTuningStats {
    fn default() -> Self {
        Self {
            tuning_operations: 0,
            successful_tunings: 0,
            failed_tunings: 0,
            success_rate: 0.0,
            avg_improvement: 0.0,
            current_confidence: 0.5,
            last_tuning: SystemTime::UNIX_EPOCH,
        }
    }
}

/// Adaptive Cache Tuner
///
/// Continuously monitors cache performance and automatically adjusts parameters
/// to optimize hit rates, latency, and resource utilization.
#[derive(Debug)]
pub struct AdaptiveTuner {
    /// Configuration
    config: TuningConfig,
    /// Performance history
    performance_history: Arc<RwLock<VecDeque<PerformanceSnapshot>>>,
    /// Current tuning parameters
    current_params: Arc<RwLock<TuningParameters>>,
    /// Tuning statistics
    stats: Arc<RwLock<AdaptiveTuningStats>>,
    /// Background tuning interval
    _tuning_interval: Option<Interval>,
}

/// Current tuning parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningParameters {
    /// L1 cache size multiplier
    pub l1_size_multiplier: f64,
    /// L2 cache size multiplier
    pub l2_size_multiplier: f64,
    /// Promotion threshold
    pub promotion_threshold: u64,
    /// Eviction aggressiveness (0.0 to 1.0)
    pub eviction_aggressiveness: f64,
    /// TTL multiplier
    pub ttl_multiplier: f64,
    /// Preheating aggressiveness (0.0 to 1.0)
    pub preheating_aggressiveness: f64,
}

impl Default for TuningParameters {
    fn default() -> Self {
        Self {
            l1_size_multiplier: 1.0,
            l2_size_multiplier: 1.0,
            promotion_threshold: 3,
            eviction_aggressiveness: 0.5,
            ttl_multiplier: 1.0,
            preheating_aggressiveness: 0.3,
        }
    }
}

impl AdaptiveTuner {
    /// Create a new adaptive tuner
    pub fn new(config: TuningConfig) -> Self {
        Self {
            config,
            performance_history: Arc::new(RwLock::new(VecDeque::new())),
            current_params: Arc::new(RwLock::new(TuningParameters::default())),
            stats: Arc::new(RwLock::new(AdaptiveTuningStats::default())),
            _tuning_interval: None,
        }
    }

    /// Record performance snapshot
    pub async fn record_performance(&self, stats: &UnifiedCacheStats) {
        let snapshot = PerformanceSnapshot {
            timestamp: SystemTime::now(),
            hit_rate: stats.overall_stats.overall_hit_rate,
            l1_hit_rate: if stats.l1_stats.hits + stats.l1_stats.misses > 0 {
                stats.l1_stats.hits as f64 / (stats.l1_stats.hits + stats.l1_stats.misses) as f64
            } else {
                0.0
            },
            l2_hit_rate: if stats.l2_stats.hits + stats.l2_stats.misses > 0 {
                stats.l2_stats.hits as f64 / (stats.l2_stats.hits + stats.l2_stats.misses) as f64
            } else {
                0.0
            },
            memory_usage_percent: if stats.l1_stats.max_usage_bytes > 0 {
                stats.l1_stats.usage_bytes as f64 / stats.l1_stats.max_usage_bytes as f64 * 100.0
            } else {
                0.0
            },
            disk_usage_percent: if stats.l2_stats.max_usage_bytes > 0 {
                stats.l2_stats.usage_bytes as f64 / stats.l2_stats.max_usage_bytes as f64 * 100.0
            } else {
                0.0
            },
            avg_get_latency_us: stats.performance_metrics.avg_get_latency_us,
            avg_put_latency_us: stats.performance_metrics.avg_put_latency_us,
            ops_per_second: stats.performance_metrics.ops_per_second,
            eviction_rate: stats.l1_stats.evictions as f64 + stats.l2_stats.evictions as f64,
            promotion_rate: stats.overall_stats.promotions as f64,
        };

        let mut history = self.performance_history.write().unwrap();
        history.push_back(snapshot);

        // Keep only recent history
        let max_history = self.config.performance_window_size;
        while history.len() > max_history {
            history.pop_front();
        }
    }

    /// Analyze performance and generate tuning recommendations
    pub async fn analyze_and_tune(&self) -> Vec<TuningRecommendation> {
        if !self.config.enable_adaptive_tuning {
            return Vec::new();
        }

        // Clone the history to avoid holding the lock across await
        let history = {
            let history_guard = self.performance_history.read().unwrap();
            if history_guard.len() < self.config.min_samples_for_tuning {
                return Vec::new();
            }
            history_guard.clone()
        };

        let mut recommendations = Vec::new();

        // Analyze hit rate trends
        if let Some(hit_rate_rec) = self.analyze_hit_rate(&history) {
            recommendations.push(hit_rate_rec);
        }

        // Analyze memory usage
        if let Some(memory_rec) = self.analyze_memory_usage(&history) {
            recommendations.push(memory_rec);
        }

        // Analyze latency trends
        if let Some(latency_rec) = self.analyze_latency(&history) {
            recommendations.push(latency_rec);
        }

        // Analyze eviction patterns
        if let Some(eviction_rec) = self.analyze_eviction_patterns(&history) {
            recommendations.push(eviction_rec);
        }

        // Apply high-confidence recommendations
        self.apply_recommendations(&recommendations).await;

        recommendations
    }

    /// Analyze hit rate trends
    fn analyze_hit_rate(&self, history: &VecDeque<PerformanceSnapshot>) -> Option<TuningRecommendation> {
        if history.len() < 3 {
            return None;
        }

        let recent_hit_rate = history.iter().rev().take(3).map(|s| s.hit_rate).sum::<f64>() / 3.0;
        let older_hit_rate = history.iter().take(3).map(|s| s.hit_rate).sum::<f64>() / 3.0;

        // If hit rate is declining, recommend increasing cache size
        if recent_hit_rate < older_hit_rate - 0.05 && recent_hit_rate < self.config.target_hit_rate {
            let current_params = self.current_params.read().unwrap();
            let new_multiplier = (current_params.l1_size_multiplier * 1.2).min(2.0);
            
            return Some(TuningRecommendation {
                parameter: "l1_size_multiplier".to_string(),
                current_value: current_params.l1_size_multiplier,
                recommended_value: new_multiplier,
                confidence: 0.8,
                expected_improvement: 0.1,
                reason: "Hit rate declining, increasing L1 cache size".to_string(),
            });
        }

        None
    }

    /// Analyze memory usage patterns
    fn analyze_memory_usage(&self, history: &VecDeque<PerformanceSnapshot>) -> Option<TuningRecommendation> {
        let avg_memory_usage = history.iter()
            .map(|s| s.memory_usage_percent)
            .sum::<f64>() / history.len() as f64;

        // If memory usage is consistently high, recommend more aggressive eviction
        if avg_memory_usage > 90.0 {
            let current_params = self.current_params.read().unwrap();
            let new_aggressiveness = (current_params.eviction_aggressiveness + 0.1).min(1.0);
            
            return Some(TuningRecommendation {
                parameter: "eviction_aggressiveness".to_string(),
                current_value: current_params.eviction_aggressiveness,
                recommended_value: new_aggressiveness,
                confidence: 0.9,
                expected_improvement: 0.05,
                reason: "High memory usage, increasing eviction aggressiveness".to_string(),
            });
        }

        // If memory usage is low, we can be less aggressive
        if avg_memory_usage < 50.0 {
            let current_params = self.current_params.read().unwrap();
            let new_aggressiveness = (current_params.eviction_aggressiveness - 0.1).max(0.1);
            
            return Some(TuningRecommendation {
                parameter: "eviction_aggressiveness".to_string(),
                current_value: current_params.eviction_aggressiveness,
                recommended_value: new_aggressiveness,
                confidence: 0.7,
                expected_improvement: 0.03,
                reason: "Low memory usage, reducing eviction aggressiveness".to_string(),
            });
        }

        None
    }

    /// Analyze latency trends
    fn analyze_latency(&self, history: &VecDeque<PerformanceSnapshot>) -> Option<TuningRecommendation> {
        if history.len() < 5 {
            return None;
        }

        let recent_latency = history.iter().rev().take(3).map(|s| s.avg_get_latency_us).sum::<f64>() / 3.0;
        let baseline_latency = history.iter().take(3).map(|s| s.avg_get_latency_us).sum::<f64>() / 3.0;

        // If latency is increasing significantly, recommend reducing cache size or TTL
        if recent_latency > baseline_latency * 1.5 && recent_latency > 1000.0 {
            let current_params = self.current_params.read().unwrap();
            let new_ttl_multiplier = (current_params.ttl_multiplier * 0.8).max(0.5);
            
            return Some(TuningRecommendation {
                parameter: "ttl_multiplier".to_string(),
                current_value: current_params.ttl_multiplier,
                recommended_value: new_ttl_multiplier,
                confidence: 0.7,
                expected_improvement: 0.2,
                reason: "High latency detected, reducing TTL to improve cache freshness".to_string(),
            });
        }

        None
    }

    /// Analyze eviction patterns
    fn analyze_eviction_patterns(&self, history: &VecDeque<PerformanceSnapshot>) -> Option<TuningRecommendation> {
        let avg_eviction_rate = history.iter()
            .map(|s| s.eviction_rate)
            .sum::<f64>() / history.len() as f64;

        // If eviction rate is very high, recommend increasing cache size
        if avg_eviction_rate > 100.0 {
            let current_params = self.current_params.read().unwrap();
            let new_multiplier = (current_params.l2_size_multiplier * 1.3).min(3.0);
            
            return Some(TuningRecommendation {
                parameter: "l2_size_multiplier".to_string(),
                current_value: current_params.l2_size_multiplier,
                recommended_value: new_multiplier,
                confidence: 0.8,
                expected_improvement: 0.15,
                reason: "High eviction rate, increasing L2 cache size".to_string(),
            });
        }

        None
    }

    /// Apply high-confidence recommendations
    async fn apply_recommendations(&self, recommendations: &[TuningRecommendation]) {
        let mut params = self.current_params.write().unwrap();
        let mut stats = self.stats.write().unwrap();

        for rec in recommendations {
            if rec.confidence >= self.config.min_confidence_for_auto_tuning {
                match rec.parameter.as_str() {
                    "l1_size_multiplier" => params.l1_size_multiplier = rec.recommended_value,
                    "l2_size_multiplier" => params.l2_size_multiplier = rec.recommended_value,
                    "promotion_threshold" => params.promotion_threshold = rec.recommended_value as u64,
                    "eviction_aggressiveness" => params.eviction_aggressiveness = rec.recommended_value,
                    "ttl_multiplier" => params.ttl_multiplier = rec.recommended_value,
                    "preheating_aggressiveness" => params.preheating_aggressiveness = rec.recommended_value,
                    _ => continue,
                }

                stats.tuning_operations += 1;
                stats.last_tuning = SystemTime::now();
            }
        }
    }

    /// Get current tuning parameters
    pub fn get_current_parameters(&self) -> TuningParameters {
        self.current_params.read().unwrap().clone()
    }

    /// Get tuning statistics
    pub fn get_stats(&self) -> AdaptiveTuningStats {
        let mut stats = self.stats.read().unwrap().clone();
        
        // Calculate success rate
        if stats.tuning_operations > 0 {
            stats.success_rate = stats.successful_tunings as f64 / stats.tuning_operations as f64;
        }

        stats
    }

    /// Record tuning outcome
    pub async fn record_tuning_outcome(&self, improved: bool, improvement: f64) {
        let mut stats = self.stats.write().unwrap();
        
        if improved {
            stats.successful_tunings += 1;
            stats.avg_improvement = (stats.avg_improvement * (stats.successful_tunings - 1) as f64 + improvement) 
                / stats.successful_tunings as f64;
        } else {
            stats.failed_tunings += 1;
        }

        // Update confidence based on recent success rate
        if stats.tuning_operations > 0 {
            stats.current_confidence = stats.successful_tunings as f64 / stats.tuning_operations as f64;
        }
    }
}
