//! Cache configuration and settings
//!
//! This module provides unified configuration for all cache types,
//! integrating with existing cache configurations while adding
//! intelligent caching features.

use crate::{
    EvictionStrategy, DEFAULT_L1_CAPACITY, DEFAULT_L2_CAPACITY, DEFAULT_MEMORY_LIMIT_MB,
    DEFAULT_TTL_SECONDS,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Unified cache configuration
///
/// This configuration integrates settings for multi-level caching,
/// predictive preheating, and adaptive tuning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedCacheConfig {
    /// L1 cache configuration (memory cache)
    pub l1_config: L1CacheConfig,
    /// L2 cache configuration (disk cache)
    pub l2_config: L2CacheConfig,
    /// Predictive preheating configuration
    pub preheating_config: PreheatingConfig,
    /// Adaptive tuning configuration
    pub tuning_config: TuningConfig,
    /// Monitoring and statistics configuration
    pub monitoring_config: MonitoringConfig,
}

impl Default for UnifiedCacheConfig {
    fn default() -> Self {
        Self {
            l1_config: L1CacheConfig::default(),
            l2_config: L2CacheConfig::default(),
            preheating_config: PreheatingConfig::default(),
            tuning_config: TuningConfig::default(),
            monitoring_config: MonitoringConfig::default(),
        }
    }
}

/// L1 (memory) cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L1CacheConfig {
    /// Maximum number of entries in L1 cache
    pub max_entries: usize,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Default TTL for L1 entries (in seconds)
    pub default_ttl: u64,
    /// Eviction strategy
    pub eviction_strategy: EvictionStrategy,
    /// Enable concurrent access optimization
    pub enable_concurrent_access: bool,
    /// Shard count for DashMap (0 = auto-detect)
    pub shard_count: usize,
    /// Promotion threshold (access count to promote from L2 to L1)
    pub promotion_threshold: u64,
}

impl Default for L1CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_L1_CAPACITY,
            max_memory_bytes: DEFAULT_MEMORY_LIMIT_MB * 1024 * 1024,
            default_ttl: DEFAULT_TTL_SECONDS,
            eviction_strategy: EvictionStrategy::LRU,
            enable_concurrent_access: true,
            shard_count: 0,         // Auto-detect based on CPU cores
            promotion_threshold: 3, // Promote after 3 accesses
        }
    }
}

/// L2 (disk) cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct L2CacheConfig {
    /// Maximum number of entries in L2 cache
    pub max_entries: usize,
    /// Maximum disk usage in bytes
    pub max_disk_bytes: u64,
    /// Default TTL for L2 entries (in seconds)
    pub default_ttl: u64,
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Enable compression for disk storage
    pub enable_compression: bool,
    /// Cleanup interval (in seconds)
    pub cleanup_interval: u64,
    /// Enable background cleanup
    pub enable_background_cleanup: bool,
}

impl Default for L2CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_L2_CAPACITY,
            max_disk_bytes: 1024 * 1024 * 1024,    // 1 GB
            default_ttl: DEFAULT_TTL_SECONDS * 24, // 24 hours for disk cache
            cache_dir: PathBuf::from(".rez_intelligent_cache"),
            enable_compression: true,
            cleanup_interval: 300, // 5 minutes
            enable_background_cleanup: true,
        }
    }
}

/// Predictive preheating configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreheatingConfig {
    /// Enable predictive preheating
    pub enable_predictive_preheating: bool,
    /// Enable pattern learning
    pub enable_pattern_learning: bool,
    /// Minimum prediction confidence threshold (0.0 to 1.0)
    pub min_confidence_threshold: f64,
    /// Maximum number of entries to preheat per cycle
    pub max_preheat_entries: usize,
    /// Maximum concurrent preheating operations
    pub max_concurrent_preheats: usize,
    /// Maximum preheating queue size
    pub max_preheat_queue_size: usize,
    /// Preheating interval (in seconds)
    pub preheat_interval: u64,
    /// Preheating window (seconds before predicted access)
    pub preheat_window_seconds: u64,
    /// Pattern window for learning (in seconds)
    pub pattern_window_seconds: u64,
    /// Enable background preheating
    pub enable_background_preheat: bool,
    /// Maximum CPU usage for preheating (0.0 to 1.0)
    pub max_cpu_usage: f64,
    /// Access pattern history size
    pub history_size: usize,
}

impl Default for PreheatingConfig {
    fn default() -> Self {
        Self {
            enable_predictive_preheating: true,
            enable_pattern_learning: true,
            min_confidence_threshold: 0.7,
            max_preheat_entries: 100,
            max_concurrent_preheats: 10,
            max_preheat_queue_size: 1000,
            preheat_interval: 60,         // 1 minute
            preheat_window_seconds: 300,  // 5 minutes
            pattern_window_seconds: 3600, // 1 hour
            enable_background_preheat: true,
            max_cpu_usage: 0.1, // 10% CPU usage limit
            history_size: 10000,
        }
    }
}

/// Adaptive tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningConfig {
    /// Enable adaptive tuning
    pub enable_adaptive_tuning: bool,
    /// Tuning interval (in seconds)
    pub tuning_interval: u64,
    /// Target hit rate for optimization
    pub target_hit_rate: f64,
    /// Minimum hit rate threshold for tuning
    pub min_hit_rate: f64,
    /// Maximum adjustment factor per tuning cycle
    pub max_adjustment_factor: f64,
    /// Minimum confidence for automatic tuning
    pub min_confidence_for_auto_tuning: f64,
    /// Performance window size for analysis
    pub performance_window_size: usize,
    /// Minimum samples required for tuning decisions
    pub min_samples_for_tuning: usize,
    /// Enable TTL adaptation
    pub enable_ttl_adaptation: bool,
    /// Enable capacity adaptation
    pub enable_capacity_adaptation: bool,
    /// Enable eviction strategy adaptation
    pub enable_eviction_adaptation: bool,
    /// Stability window size (number of cycles)
    pub stability_window: usize,
}

impl Default for TuningConfig {
    fn default() -> Self {
        Self {
            enable_adaptive_tuning: true,
            tuning_interval: 300,       // 5 minutes
            target_hit_rate: 0.9,       // 90% target hit rate
            min_hit_rate: 0.8,          // 80% minimum hit rate
            max_adjustment_factor: 0.1, // 10% maximum adjustment
            min_confidence_for_auto_tuning: 0.8,
            performance_window_size: 100,
            min_samples_for_tuning: 10,
            enable_ttl_adaptation: true,
            enable_capacity_adaptation: true,
            enable_eviction_adaptation: false, // Conservative default
            stability_window: 5,
        }
    }
}

/// Monitoring and statistics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable detailed statistics collection
    pub enable_detailed_stats: bool,
    /// Statistics collection interval (in seconds)
    pub stats_interval: u64,
    /// Enable performance metrics
    pub enable_performance_metrics: bool,
    /// Enable cache event logging
    pub enable_event_logging: bool,
    /// Maximum number of events to keep in memory
    pub max_events_in_memory: usize,
    /// Enable real-time monitoring
    pub enable_realtime_monitoring: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_detailed_stats: true,
            stats_interval: 60, // 1 minute
            enable_performance_metrics: true,
            enable_event_logging: false, // Disabled by default for performance
            max_events_in_memory: 1000,
            enable_realtime_monitoring: false, // Disabled by default
        }
    }
}

/// Cache operation timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    /// Timeout for cache get operations
    pub get_timeout: Duration,
    /// Timeout for cache put operations
    pub put_timeout: Duration,
    /// Timeout for cache cleanup operations
    pub cleanup_timeout: Duration,
    /// Timeout for preheating operations
    pub preheat_timeout: Duration,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            get_timeout: Duration::from_millis(100),
            put_timeout: Duration::from_millis(500),
            cleanup_timeout: Duration::from_secs(30),
            preheat_timeout: Duration::from_secs(10),
        }
    }
}

impl UnifiedCacheConfig {
    /// Create a configuration optimized for high performance
    pub fn high_performance() -> Self {
        Self {
            l1_config: L1CacheConfig {
                max_entries: 50000,
                max_memory_bytes: 500 * 1024 * 1024, // 500 MB
                enable_concurrent_access: true,
                shard_count: num_cpus::get() * 2,
                ..Default::default()
            },
            l2_config: L2CacheConfig {
                max_entries: 500000,
                max_disk_bytes: 5 * 1024 * 1024 * 1024, // 5 GB
                enable_compression: true,
                enable_background_cleanup: true,
                ..Default::default()
            },
            preheating_config: PreheatingConfig {
                enable_predictive_preheating: true,
                min_confidence_threshold: 0.6,
                max_preheat_entries: 500,
                max_cpu_usage: 0.2, // 20% CPU usage
                ..Default::default()
            },
            tuning_config: TuningConfig {
                enable_adaptive_tuning: true,
                tuning_interval: 120,        // 2 minutes
                max_adjustment_factor: 0.15, // 15% adjustment
                ..Default::default()
            },
            monitoring_config: MonitoringConfig {
                enable_detailed_stats: true,
                enable_performance_metrics: true,
                enable_realtime_monitoring: true,
                ..Default::default()
            },
        }
    }

    /// Create a configuration optimized for low memory usage
    pub fn low_memory() -> Self {
        Self {
            l1_config: L1CacheConfig {
                max_entries: 1000,
                max_memory_bytes: 10 * 1024 * 1024, // 10 MB
                ..Default::default()
            },
            l2_config: L2CacheConfig {
                max_entries: 10000,
                max_disk_bytes: 100 * 1024 * 1024, // 100 MB
                enable_compression: true,
                ..Default::default()
            },
            preheating_config: PreheatingConfig {
                enable_predictive_preheating: false, // Disable preheating to save memory
                ..Default::default()
            },
            tuning_config: TuningConfig {
                enable_adaptive_tuning: false, // Disable tuning to save CPU
                ..Default::default()
            },
            monitoring_config: MonitoringConfig {
                enable_detailed_stats: false,
                enable_performance_metrics: false,
                ..Default::default()
            },
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.l1_config.max_entries == 0 {
            return Err("L1 max_entries must be greater than 0".to_string());
        }

        if self.l2_config.max_entries == 0 {
            return Err("L2 max_entries must be greater than 0".to_string());
        }

        if self.preheating_config.min_confidence_threshold < 0.0
            || self.preheating_config.min_confidence_threshold > 1.0
        {
            return Err(
                "Preheating min_confidence_threshold must be between 0.0 and 1.0".to_string(),
            );
        }

        if self.tuning_config.min_hit_rate < 0.0 || self.tuning_config.min_hit_rate > 1.0 {
            return Err("Tuning min_hit_rate must be between 0.0 and 1.0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UnifiedCacheConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_high_performance_config() {
        let config = UnifiedCacheConfig::high_performance();
        assert!(config.validate().is_ok());
        assert!(config.l1_config.max_entries > DEFAULT_L1_CAPACITY);
    }

    #[test]
    fn test_low_memory_config() {
        let config = UnifiedCacheConfig::low_memory();
        assert!(config.validate().is_ok());
        assert!(config.l1_config.max_entries < DEFAULT_L1_CAPACITY);
        assert!(!config.preheating_config.enable_predictive_preheating);
    }

    #[test]
    fn test_config_validation() {
        let mut config = UnifiedCacheConfig::default();
        config.l1_config.max_entries = 0;
        assert!(config.validate().is_err());
    }
}
