//! Intelligent Cache Manager
//!
//! This module provides the IntelligentCacheManager, which coordinates
//! multi-level caching, predictive preheating, and adaptive tuning.

use crate::{
    UnifiedCache, UnifiedCacheConfig, UnifiedCacheStats, CacheError,
    PredictivePreheater, AdaptiveTuner, PerformanceMonitor,
};
use dashmap::DashMap;

use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, RwLock},
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::RwLock as AsyncRwLock;
use async_trait::async_trait;

/// Multi-level cache entry with metadata
#[derive(Debug, Clone)]
pub struct MultiLevelCacheEntry<V> {
    /// The cached value
    pub value: V,
    /// Creation timestamp
    pub created_at: SystemTime,
    /// Last access timestamp
    pub last_accessed: SystemTime,
    /// Access count
    pub access_count: u64,
    /// Cache level (1 for L1, 2 for L2)
    pub level: u8,
    /// Size in bytes (estimated)
    pub size_bytes: u64,
    /// Time to live (in seconds)
    pub ttl: u64,
    /// Prediction score for preheating
    pub prediction_score: f64,
}

impl<V> MultiLevelCacheEntry<V> {
    /// Create a new cache entry
    pub fn new(value: V, ttl: u64, level: u8, size_bytes: u64) -> Self {
        let now = SystemTime::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 1,
            level,
            size_bytes,
            ttl,
            prediction_score: 0.0,
        }
    }

    /// Check if the entry is still valid
    pub fn is_valid(&self) -> bool {
        if self.ttl == 0 {
            return true; // No expiration
        }
        
        let elapsed = self.created_at
            .elapsed()
            .unwrap_or(Duration::from_secs(u64::MAX))
            .as_secs();
        elapsed < self.ttl
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.last_accessed = SystemTime::now();
        self.access_count += 1;
    }

    /// Calculate entry priority for eviction
    pub fn calculate_priority(&self) -> f64 {
        let age_factor = self.last_accessed
            .elapsed()
            .unwrap_or(Duration::from_secs(0))
            .as_secs() as f64;
        let frequency_factor = self.access_count as f64;
        let size_factor = 1.0 / (self.size_bytes as f64 + 1.0);
        
        // Higher score = higher priority to keep
        (frequency_factor * size_factor) / (age_factor + 1.0)
    }
}

/// Intelligent Cache Manager
///
/// Coordinates multi-level caching with predictive preheating and adaptive tuning.
/// Provides unified interface for all cache operations while optimizing performance.
#[derive(Debug)]
pub struct IntelligentCacheManager<K, V> 
where
    K: Clone + Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
    V: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Configuration
    config: UnifiedCacheConfig,
    /// L1 cache (memory) - high-speed concurrent access
    l1_cache: Arc<DashMap<K, MultiLevelCacheEntry<V>>>,
    /// L2 cache (disk/persistent) - larger capacity
    l2_cache: Arc<AsyncRwLock<HashMap<K, MultiLevelCacheEntry<V>>>>,
    /// Predictive preheater
    preheater: Arc<PredictivePreheater<K>>,
    /// Adaptive tuner
    tuner: Arc<AdaptiveTuner>,
    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,
    /// Cache statistics
    stats: Arc<RwLock<UnifiedCacheStats>>,
    /// Access pattern tracking
    access_patterns: Arc<RwLock<HashMap<K, Vec<SystemTime>>>>,
}

impl<K, V> IntelligentCacheManager<K, V>
where
    K: Clone + Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
    V: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new intelligent cache manager
    pub fn new(config: UnifiedCacheConfig) -> Self {
        let preheater = Arc::new(PredictivePreheater::new(config.preheating_config.clone()));
        let tuner = Arc::new(AdaptiveTuner::new(config.tuning_config.clone()));
        let monitor = Arc::new(PerformanceMonitor::new(config.monitoring_config.clone()));
        
        Self {
            config,
            l1_cache: Arc::new(DashMap::new()),
            l2_cache: Arc::new(AsyncRwLock::new(HashMap::new())),
            preheater,
            tuner,
            monitor,
            stats: Arc::new(RwLock::new(UnifiedCacheStats::default())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get cache configuration
    pub fn config(&self) -> &UnifiedCacheConfig {
        &self.config
    }

    /// Get predictive preheater
    pub fn preheater(&self) -> Arc<PredictivePreheater<K>> {
        Arc::clone(&self.preheater)
    }

    /// Get adaptive tuner
    pub fn tuner(&self) -> Arc<AdaptiveTuner> {
        Arc::clone(&self.tuner)
    }

    /// Get performance monitor
    pub fn monitor(&self) -> Arc<PerformanceMonitor> {
        Arc::clone(&self.monitor)
    }

    /// Record access pattern for predictive preheating
    async fn record_access_pattern(&self, key: &K) {
        if !self.config.preheating_config.enable_pattern_learning {
            return;
        }

        let mut patterns = self.access_patterns.write().unwrap();
        let now = SystemTime::now();
        
        patterns
            .entry(key.clone())
            .or_insert_with(Vec::new)
            .push(now);

        // Keep only recent accesses for pattern analysis
        let cutoff = now - Duration::from_secs(self.config.preheating_config.pattern_window_seconds);
        if let Some(times) = patterns.get_mut(key) {
            times.retain(|&time| time > cutoff);
        }
    }

    /// Promote data from L2 to L1 cache
    async fn promote_to_l1(&self, key: K, mut entry: MultiLevelCacheEntry<V>) -> Result<(), CacheError> {
        // Check L1 capacity
        if self.l1_cache.len() >= self.config.l1_config.max_entries {
            self.evict_l1_entries().await?;
        }

        // Update entry metadata for L1
        entry.level = 1;
        entry.mark_accessed();

        // Insert into L1
        self.l1_cache.insert(key.clone(), entry);

        // Remove from L2
        let mut l2_cache = self.l2_cache.write().await;
        l2_cache.remove(&key);

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.overall_stats.promotions += 1;
        }

        Ok(())
    }

    /// Demote data from L1 to L2 cache
    async fn demote_to_l2(&self, key: K, mut entry: MultiLevelCacheEntry<V>) -> Result<(), CacheError> {
        // Check L2 capacity
        {
            let l2_cache = self.l2_cache.read().await;
            if l2_cache.len() >= self.config.l2_config.max_entries {
                drop(l2_cache);
                self.evict_l2_entries().await?;
            }
        }

        // Update entry metadata for L2
        entry.level = 2;

        // Insert into L2
        {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.insert(key.clone(), entry);
        }

        // Remove from L1
        self.l1_cache.remove(&key);

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.overall_stats.demotions += 1;
        }

        Ok(())
    }

    /// Evict entries from L1 cache
    async fn evict_l1_entries(&self) -> Result<(), CacheError> {
        let eviction_count = (self.l1_cache.len() as f64 * 0.1).max(1.0) as usize;
        
        // Collect entries with their priorities
        let mut entries: Vec<(K, f64)> = self.l1_cache
            .iter()
            .map(|entry| {
                let priority = entry.value().calculate_priority();
                (entry.key().clone(), priority)
            })
            .collect();

        // Sort by priority (lowest first for eviction)
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Evict lowest priority entries
        for (key, _) in entries.into_iter().take(eviction_count) {
            if let Some((_, entry)) = self.l1_cache.remove(&key) {
                // Try to demote to L2 if valuable enough
                if entry.access_count > 1 {
                    self.demote_to_l2(key, entry).await?;
                }
            }
        }

        Ok(())
    }

    /// Evict entries from L2 cache
    async fn evict_l2_entries(&self) -> Result<(), CacheError> {
        let mut l2_cache = self.l2_cache.write().await;
        let eviction_count = (l2_cache.len() as f64 * 0.1).max(1.0) as usize;
        
        // Collect entries with their priorities
        let mut entries: Vec<(K, f64)> = l2_cache
            .iter()
            .map(|(key, entry)| {
                let priority = entry.calculate_priority();
                (key.clone(), priority)
            })
            .collect();

        // Sort by priority (lowest first for eviction)
        entries.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Remove lowest priority entries
        for (key, _) in entries.into_iter().take(eviction_count) {
            l2_cache.remove(&key);
        }

        Ok(())
    }

    /// Update cache statistics
    async fn update_statistics(&self) {
        // Collect L1 statistics
        let l1_entries = self.l1_cache.len();
        let l1_usage_bytes = self.l1_cache
            .iter()
            .map(|entry| entry.value().size_bytes)
            .sum();

        // Collect L2 statistics
        let (l2_entries, l2_usage_bytes) = {
            let l2_cache = self.l2_cache.read().await;
            let entries = l2_cache.len();
            let usage_bytes = l2_cache
                .values()
                .map(|entry| entry.size_bytes)
                .sum();
            (entries, usage_bytes)
        };

        // Update statistics in a separate scope
        {
            let mut stats = self.stats.write().unwrap();

            // Update L1 statistics
            stats.l1_stats.entries = l1_entries;
            stats.l1_stats.usage_bytes = l1_usage_bytes;

            // Update L2 statistics
            stats.l2_stats.entries = l2_entries;
            stats.l2_stats.usage_bytes = l2_usage_bytes;

            // Update overall statistics
            stats.update_overall_stats();
        }
    }
}

#[async_trait]
impl<K, V> UnifiedCache<K, V> for IntelligentCacheManager<K, V>
where
    K: Clone + Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
    V: Clone + Send + Sync + std::fmt::Debug + 'static,
{
    /// Get a value from the cache
    async fn get(&self, key: &K) -> Option<V> {
        let start_time = Instant::now();

        // Record access pattern
        self.record_access_pattern(key).await;

        // Try L1 cache first
        if let Some(mut entry) = self.l1_cache.get_mut(key) {
            if entry.is_valid() {
                entry.mark_accessed();

                // Update statistics
                {
                    let mut stats = self.stats.write().unwrap();
                    stats.l1_stats.hits += 1;
                }

                // Record performance metrics
                self.monitor.record_get_latency(start_time.elapsed()).await;

                return Some(entry.value.clone());
            } else {
                // Remove expired entry
                drop(entry);
                self.l1_cache.remove(key);
            }
        }

        // Try L2 cache
        {
            let mut l2_cache = self.l2_cache.write().await;
            if let Some(entry) = l2_cache.get_mut(key) {
                if entry.is_valid() {
                    entry.mark_accessed();
                    let value = entry.value.clone();

                    // Promote to L1 if frequently accessed
                    if entry.access_count >= self.config.l1_config.promotion_threshold {
                        let promoted_entry = entry.clone();
                        l2_cache.remove(key);
                        drop(l2_cache);

                        if let Err(e) = self.promote_to_l1(key.clone(), promoted_entry).await {
                            eprintln!("Failed to promote to L1: {:?}", e);
                        }
                    }

                    // Update statistics
                    {
                        let mut stats = self.stats.write().unwrap();
                        stats.l2_stats.hits += 1;
                    }

                    // Record performance metrics
                    self.monitor.record_get_latency(start_time.elapsed()).await;

                    return Some(value);
                } else {
                    // Remove expired entry
                    l2_cache.remove(key);
                }
            }
        }

        // Cache miss
        {
            let mut stats = self.stats.write().unwrap();
            stats.l1_stats.misses += 1;
            stats.l2_stats.misses += 1;
        }

        // Trigger predictive preheating
        if self.config.preheating_config.enable_predictive_preheating {
            self.preheater.predict_and_preheat(key).await;
        }

        // Record performance metrics
        self.monitor.record_get_latency(start_time.elapsed()).await;

        None
    }

    /// Put a value into the cache
    async fn put(&self, key: K, value: V) -> Result<(), CacheError> {
        let start_time = Instant::now();

        // Estimate size (simplified)
        let size_bytes = std::mem::size_of::<V>() as u64;

        // Create cache entry for L1
        let entry = MultiLevelCacheEntry::new(
            value,
            self.config.l1_config.default_ttl,
            1,
            size_bytes,
        );

        // Check L1 capacity and evict if necessary
        if self.l1_cache.len() >= self.config.l1_config.max_entries {
            self.evict_l1_entries().await?;
        }

        // Insert into L1 cache
        self.l1_cache.insert(key.clone(), entry);

        // Update statistics
        self.update_statistics().await;

        // Record performance metrics
        self.monitor.record_put_latency(start_time.elapsed()).await;

        // Trigger adaptive tuning
        if self.config.tuning_config.enable_adaptive_tuning {
            self.tuner.analyze_and_tune().await;
        }

        Ok(())
    }

    /// Remove a value from the cache
    async fn remove(&self, key: &K) -> bool {
        let l1_removed = self.l1_cache.remove(key).is_some();

        let l2_removed = {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.remove(key).is_some()
        };

        // Update statistics if removed
        if l1_removed || l2_removed {
            self.update_statistics().await;
        }

        l1_removed || l2_removed
    }

    /// Check if a key exists in the cache
    async fn contains_key(&self, key: &K) -> bool {
        // Check L1 first
        if let Some(entry) = self.l1_cache.get(key) {
            if entry.is_valid() {
                return true;
            }
        }

        // Check L2
        let l2_cache = self.l2_cache.read().await;
        if let Some(entry) = l2_cache.get(key) {
            return entry.is_valid();
        }

        false
    }

    /// Get cache statistics
    async fn get_stats(&self) -> UnifiedCacheStats {
        self.update_statistics().await;
        self.stats.read().unwrap().clone()
    }

    /// Clear all cache entries
    async fn clear(&self) -> Result<(), CacheError> {
        self.l1_cache.clear();

        {
            let mut l2_cache = self.l2_cache.write().await;
            l2_cache.clear();
        }

        // Reset statistics
        {
            let mut stats = self.stats.write().unwrap();
            *stats = UnifiedCacheStats::default();
        }

        Ok(())
    }

    /// Get cache size (total entries across all levels)
    async fn size(&self) -> usize {
        let l1_size = self.l1_cache.len();
        let l2_size = {
            let l2_cache = self.l2_cache.read().await;
            l2_cache.len()
        };
        l1_size + l2_size
    }

    /// Check if cache is empty
    async fn is_empty(&self) -> bool {
        self.size().await == 0
    }

    /// Get cache capacity (total across all levels)
    async fn capacity(&self) -> usize {
        self.config.l1_config.max_entries + self.config.l2_config.max_entries
    }

    /// Get cache type identifier
    fn cache_type(&self) -> &'static str {
        "IntelligentCacheManager"
    }
}
