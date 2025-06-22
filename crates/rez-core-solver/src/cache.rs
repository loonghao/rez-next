//! Solver caching system

use crate::ResolutionResult;
use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Cache entry for solver results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverCacheEntry {
    /// Cached resolution result
    pub result: ResolutionResult,
    /// Cache timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_access: u64,
}

impl SolverCacheEntry {
    /// Create a new cache entry
    pub fn new(result: ResolutionResult, ttl: u64) -> Result<Self, RezCoreError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RezCoreError::CacheError(format!("Failed to get current time: {}", e)))?
            .as_secs();

        Ok(Self {
            result,
            timestamp: now,
            ttl,
            access_count: 0,
            last_access: now,
        })
    }

    /// Check if the cache entry is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now <= self.timestamp + self.ttl
    }

    /// Get the age of the cache entry in seconds
    pub fn age(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now.saturating_sub(self.timestamp)
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Solver cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverCacheConfig {
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Default TTL for cache entries (in seconds)
    pub default_ttl: u64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Cache eviction strategy
    pub eviction_strategy: EvictionStrategy,
    /// Enable cache persistence
    pub enable_persistence: bool,
    /// Cache file path (if persistence is enabled)
    pub cache_file_path: Option<String>,
}

impl Default for SolverCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: 3600,                  // 1 hour
            max_memory_bytes: 50 * 1024 * 1024, // 50 MB
            eviction_strategy: EvictionStrategy::LRU,
            enable_persistence: false,
            cache_file_path: None,
        }
    }
}

/// Cache eviction strategies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvictionStrategy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In, First Out
    FIFO,
    /// Time-based expiration only
    TTL,
}

/// High-performance solver cache
#[derive(Debug)]
pub struct SolverCache {
    /// Cache configuration
    config: SolverCacheConfig,
    /// Cache entries
    entries: Arc<RwLock<HashMap<String, SolverCacheEntry>>>,
    /// Access order for LRU eviction
    access_order: Arc<RwLock<Vec<String>>>,
    /// Cache statistics
    stats: Arc<RwLock<SolverCacheStats>>,
}

/// Solver cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverCacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total cache entries
    pub entries: usize,
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Cache evictions
    pub evictions: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

impl Default for SolverCacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            memory_usage_bytes: 0,
            evictions: 0,
            hit_rate: 0.0,
        }
    }
}

impl SolverCache {
    /// Create a new solver cache
    pub fn new(default_ttl: u64) -> Self {
        let config = SolverCacheConfig {
            default_ttl,
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a new solver cache with custom configuration
    pub fn with_config(config: SolverCacheConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(SolverCacheStats::default())),
        }
    }

    /// Get a cached resolution result
    pub async fn get(&self, key: &str) -> Option<ResolutionResult> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = entries.get_mut(key) {
            if entry.is_valid() {
                entry.mark_accessed();
                stats.hits += 1;
                self.update_access_order(key).await;
                self.update_hit_rate(&mut stats);
                return Some(entry.result.clone());
            } else {
                // Remove expired entry
                entries.remove(key);
                self.remove_from_access_order(key).await;
            }
        }

        stats.misses += 1;
        self.update_hit_rate(&mut stats);
        None
    }

    /// Put a resolution result into the cache
    pub async fn put(&self, key: String, result: ResolutionResult) {
        let entry = match SolverCacheEntry::new(result, self.config.default_ttl) {
            Ok(entry) => entry,
            Err(_) => return, // Failed to create entry
        };

        {
            let mut entries = self.entries.write().await;

            // Check if we need to evict entries
            if entries.len() >= self.config.max_entries {
                self.evict_entries(&mut entries).await;
            }

            entries.insert(key.clone(), entry);
        }

        // Update access order
        self.update_access_order(&key).await;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.entries = {
                let entries = self.entries.read().await;
                entries.len()
            };
            stats.memory_usage_bytes = self.estimate_memory_usage().await;
        }
    }

    /// Remove an entry from the cache
    pub async fn remove(&self, key: &str) -> bool {
        let mut entries = self.entries.write().await;
        let removed = entries.remove(key).is_some();

        if removed {
            self.remove_from_access_order(key).await;

            let mut stats = self.stats.write().await;
            stats.entries = entries.len();
        }

        removed
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        {
            let mut entries = self.entries.write().await;
            entries.clear();
        }

        {
            let mut access_order = self.access_order.write().await;
            access_order.clear();
        }

        {
            let mut stats = self.stats.write().await;
            *stats = SolverCacheStats::default();
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> SolverCacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Cleanup expired entries
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let mut expired_keys = Vec::new();

        // Find expired entries
        for (key, entry) in entries.iter() {
            if !entry.is_valid() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        for key in &expired_keys {
            entries.remove(key);
            self.remove_from_access_order(key).await;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.entries = entries.len();
            stats.memory_usage_bytes = self.estimate_memory_usage().await;
        }
    }

    /// Evict entries based on the configured strategy
    async fn evict_entries(&self, entries: &mut HashMap<String, SolverCacheEntry>) {
        let evict_count = (entries.len() as f64 * 0.1).max(1.0) as usize; // Evict 10% or at least 1

        match self.config.eviction_strategy {
            EvictionStrategy::LRU => {
                self.evict_lru(entries, evict_count).await;
            }
            EvictionStrategy::LFU => {
                self.evict_lfu(entries, evict_count).await;
            }
            EvictionStrategy::FIFO => {
                self.evict_fifo(entries, evict_count).await;
            }
            EvictionStrategy::TTL => {
                self.evict_expired(entries).await;
            }
        }

        // Update eviction statistics
        {
            let mut stats = self.stats.write().await;
            stats.evictions += evict_count as u64;
        }
    }

    /// Evict least recently used entries
    async fn evict_lru(&self, entries: &mut HashMap<String, SolverCacheEntry>, count: usize) {
        let access_order = self.access_order.read().await;
        let keys_to_evict: Vec<String> = access_order.iter().take(count).cloned().collect();

        for key in &keys_to_evict {
            entries.remove(key);
        }

        drop(access_order);

        // Remove from access order
        for key in &keys_to_evict {
            self.remove_from_access_order(key).await;
        }
    }

    /// Evict least frequently used entries
    async fn evict_lfu(&self, entries: &mut HashMap<String, SolverCacheEntry>, count: usize) {
        let mut entries_by_frequency: Vec<_> = entries
            .iter()
            .map(|(key, entry)| (key.clone(), entry.access_count))
            .collect();

        entries_by_frequency.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_frequency
            .iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            entries.remove(key);
            self.remove_from_access_order(key).await;
        }
    }

    /// Evict oldest entries (FIFO)
    async fn evict_fifo(&self, entries: &mut HashMap<String, SolverCacheEntry>, count: usize) {
        let mut entries_by_age: Vec<_> = entries
            .iter()
            .map(|(key, entry)| (key.clone(), entry.timestamp))
            .collect();

        entries_by_age.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_age
            .iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            entries.remove(key);
            self.remove_from_access_order(key).await;
        }
    }

    /// Evict expired entries
    async fn evict_expired(&self, entries: &mut HashMap<String, SolverCacheEntry>) {
        let expired_keys: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| !entry.is_valid())
            .map(|(key, _)| key.clone())
            .collect();

        for key in &expired_keys {
            entries.remove(key);
            self.remove_from_access_order(key).await;
        }
    }

    /// Update access order for LRU tracking
    async fn update_access_order(&self, key: &str) {
        let mut access_order = self.access_order.write().await;

        // Remove key if it already exists
        access_order.retain(|k| k != key);

        // Add to the end (most recently used)
        access_order.push(key.to_string());
    }

    /// Remove a key from access order
    async fn remove_from_access_order(&self, key: &str) {
        let mut access_order = self.access_order.write().await;
        access_order.retain(|k| k != key);
    }

    /// Estimate memory usage of the cache
    async fn estimate_memory_usage(&self) -> u64 {
        let entries = self.entries.read().await;
        let mut total_size = 0u64;

        for (key, entry) in entries.iter() {
            // Rough estimation: key size + entry size
            total_size += key.len() as u64;
            total_size += entry.result.packages.len() as u64 * 1024; // Estimate 1KB per package
            total_size += 128; // Overhead for entry metadata
        }

        total_size
    }

    /// Update hit rate statistics
    fn update_hit_rate(&self, stats: &mut SolverCacheStats) {
        let total_requests = stats.hits + stats.misses;
        if total_requests > 0 {
            stats.hit_rate = stats.hits as f64 / total_requests as f64;
        }
    }
}

impl Default for SolverCache {
    fn default() -> Self {
        Self::new(3600) // 1 hour default TTL
    }
}
