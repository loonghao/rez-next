//! Package caching system for improved performance

use crate::{dependency::DependencyResolutionResult, Package, PackageValidationResult};
use lru::LruCache;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// Cached data
    pub data: T,
    /// Cache timestamp
    pub timestamp: u64,
    /// Time to live in seconds
    pub ttl: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_access: u64,
    /// Cache key
    pub key: String,
    /// Data size estimate
    pub size_estimate: usize,
}

/// Cache statistics
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total entries
    pub total_entries: usize,
    /// Total memory usage estimate
    pub memory_usage_bytes: usize,
    /// Cache hit ratio
    pub hit_ratio: f64,
    /// Average access time in microseconds
    pub avg_access_time_us: u64,
}

/// Cache configuration
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Default TTL in seconds
    pub default_ttl: u64,
    /// Enable persistent cache
    pub persistent: bool,
    /// Cache directory for persistent storage
    pub cache_dir: Option<PathBuf>,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Enable compression for persistent cache
    pub compress: bool,
    /// Cache cleanup interval in seconds
    pub cleanup_interval: u64,
}

/// Package parsing cache
pub struct PackageParseCache {
    /// In-memory cache
    cache: Arc<RwLock<LruCache<String, CacheEntry<Package>>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStatistics>>,
    /// Last cleanup time
    last_cleanup: Arc<RwLock<SystemTime>>,
}

/// Package validation cache
pub struct PackageValidationCache {
    /// In-memory cache
    cache: Arc<RwLock<LruCache<String, CacheEntry<PackageValidationResult>>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStatistics>>,
    /// Last cleanup time
    last_cleanup: Arc<RwLock<SystemTime>>,
}

/// Dependency resolution cache
pub struct DependencyResolutionCache {
    /// In-memory cache
    cache: Arc<RwLock<LruCache<String, CacheEntry<DependencyResolutionResult>>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStatistics>>,
    /// Last cleanup time
    last_cleanup: Arc<RwLock<SystemTime>>,
}

/// Unified package cache manager
#[cfg_attr(feature = "python-bindings", pyclass)]
pub struct PackageCacheManager {
    /// Package parsing cache
    pub parse_cache: PackageParseCache,
    /// Package validation cache
    pub validation_cache: PackageValidationCache,
    /// Dependency resolution cache
    pub dependency_cache: DependencyResolutionCache,
    /// Global cache configuration
    config: CacheConfig,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            default_ttl: 3600, // 1 hour
            persistent: false,
            cache_dir: None,
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            compress: true,
            cleanup_interval: 300, // 5 minutes
        }
    }
}

#[cfg_attr(feature = "python-bindings", pymethods)]
impl CacheConfig {
    /// Create a new cache configuration
    #[cfg_attr(feature = "python-bindings", new)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration for development
    #[cfg_attr(feature = "python-bindings", staticmethod)]
    pub fn development() -> Self {
        Self {
            max_entries: 500,
            default_ttl: 1800, // 30 minutes
            persistent: false,
            cache_dir: None,
            max_memory_bytes: 50 * 1024 * 1024, // 50MB
            compress: false,
            cleanup_interval: 600, // 10 minutes
        }
    }

    /// Create configuration for production
    pub fn production() -> Self {
        Self {
            max_entries: 5000,
            default_ttl: 7200, // 2 hours
            persistent: true,
            cache_dir: Some(PathBuf::from("/tmp/rez_cache")),
            max_memory_bytes: 500 * 1024 * 1024, // 500MB
            compress: true,
            cleanup_interval: 300, // 5 minutes
        }
    }

    /// Set maximum entries
    pub fn with_max_entries(mut self, max_entries: usize) -> Self {
        self.max_entries = max_entries;
        self
    }

    /// Set default TTL
    pub fn with_default_ttl(mut self, ttl: u64) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Enable persistent cache
    pub fn with_persistent_cache(mut self, cache_dir: PathBuf) -> Self {
        self.persistent = true;
        self.cache_dir = Some(cache_dir);
        self
    }

    /// Set maximum memory usage
    pub fn with_max_memory(mut self, max_memory_bytes: usize) -> Self {
        self.max_memory_bytes = max_memory_bytes;
        self
    }
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry
    pub fn new(data: T, key: String, ttl: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            data,
            timestamp: now,
            ttl,
            access_count: 0,
            last_access: now,
            key,
            size_estimate: std::mem::size_of::<T>(),
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now > self.timestamp + self.ttl
    }

    /// Update access statistics
    pub fn update_access(&mut self) {
        self.access_count += 1;
        self.last_access = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get age in seconds
    pub fn age_seconds(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        now.saturating_sub(self.timestamp)
    }
}

impl CacheStatistics {
    /// Update hit statistics
    pub fn record_hit(&mut self) {
        self.hits += 1;
        self.update_hit_ratio();
    }

    /// Update miss statistics
    pub fn record_miss(&mut self) {
        self.misses += 1;
        self.update_hit_ratio();
    }

    /// Update hit ratio
    fn update_hit_ratio(&mut self) {
        let total = self.hits + self.misses;
        if total > 0 {
            self.hit_ratio = self.hits as f64 / total as f64;
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.total_entries = 0;
        self.memory_usage_bytes = 0;
        self.hit_ratio = 0.0;
        self.avg_access_time_us = 0;
    }
}

impl PackageParseCache {
    /// Create a new package parse cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_entries)
                    .unwrap_or(std::num::NonZeroUsize::new(1000).unwrap()),
            ))),
            config,
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
            last_cleanup: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// Get package from cache
    pub fn get(&self, key: &str) -> Option<Package> {
        let start_time = std::time::Instant::now();

        let result = {
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(key) {
                if !entry.is_expired() {
                    entry.update_access();
                    Some(entry.data.clone())
                } else {
                    cache.pop(key);
                    None
                }
            } else {
                None
            }
        };

        let elapsed = start_time.elapsed().as_micros() as u64;
        let mut stats = self.stats.write().unwrap();

        if result.is_some() {
            stats.record_hit();
        } else {
            stats.record_miss();
        }

        stats.avg_access_time_us = (stats.avg_access_time_us + elapsed) / 2;

        result
    }

    /// Put package in cache
    pub fn put(&self, key: String, package: Package) {
        let entry = CacheEntry::new(package, key.clone(), self.config.default_ttl);

        {
            let mut cache = self.cache.write().unwrap();
            cache.put(key, entry);
        }

        self.update_memory_stats();
        self.cleanup_if_needed();
    }

    /// Generate cache key for package
    pub fn generate_key(&self, path: &Path, content_hash: Option<&str>) -> String {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);

        if let Some(hash) = content_hash {
            hash.hash(&mut hasher);
        }

        format!("parse_{:x}", hasher.finish())
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        let mut stats = self.stats.write().unwrap();
        stats.reset();
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        self.stats.read().unwrap().clone()
    }

    /// Update memory usage statistics
    fn update_memory_stats(&self) {
        let cache = self.cache.read().unwrap();
        let mut stats = self.stats.write().unwrap();

        stats.total_entries = cache.len();
        // Estimate memory usage (simplified)
        stats.memory_usage_bytes = cache.len() * 1024; // Rough estimate
    }

    /// Cleanup expired entries if needed
    fn cleanup_if_needed(&self) {
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.read().unwrap();
            let elapsed = last_cleanup.elapsed().unwrap_or_default();
            elapsed > Duration::from_secs(self.config.cleanup_interval)
        };

        if should_cleanup {
            self.cleanup_expired();
            *self.last_cleanup.write().unwrap() = SystemTime::now();
        }
    }

    /// Remove expired entries
    fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        let mut expired_keys = Vec::new();

        // Collect expired keys (we can't modify while iterating)
        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        for key in expired_keys {
            cache.pop(&key);
        }

        self.update_memory_stats();
    }
}

impl PackageValidationCache {
    /// Create a new package validation cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_entries)
                    .unwrap_or(std::num::NonZeroUsize::new(1000).unwrap()),
            ))),
            config,
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
            last_cleanup: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// Get validation result from cache
    pub fn get(&self, key: &str) -> Option<PackageValidationResult> {
        let start_time = std::time::Instant::now();

        let result = {
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(key) {
                if !entry.is_expired() {
                    entry.update_access();
                    Some(entry.data.clone())
                } else {
                    cache.pop(key);
                    None
                }
            } else {
                None
            }
        };

        let elapsed = start_time.elapsed().as_micros() as u64;
        let mut stats = self.stats.write().unwrap();

        if result.is_some() {
            stats.record_hit();
        } else {
            stats.record_miss();
        }

        stats.avg_access_time_us = (stats.avg_access_time_us + elapsed) / 2;

        result
    }

    /// Put validation result in cache
    pub fn put(&self, key: String, result: PackageValidationResult) {
        let entry = CacheEntry::new(result, key.clone(), self.config.default_ttl);

        {
            let mut cache = self.cache.write().unwrap();
            cache.put(key, entry);
        }

        self.update_memory_stats();
        self.cleanup_if_needed();
    }

    /// Generate cache key for validation
    pub fn generate_key(&self, package_hash: &str, options_hash: &str) -> String {
        format!("validation_{}_{}", package_hash, options_hash)
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        let mut stats = self.stats.write().unwrap();
        stats.reset();
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        self.stats.read().unwrap().clone()
    }

    /// Update memory usage statistics
    fn update_memory_stats(&self) {
        let cache = self.cache.read().unwrap();
        let mut stats = self.stats.write().unwrap();

        stats.total_entries = cache.len();
        stats.memory_usage_bytes = cache.len() * 2048; // Rough estimate for validation results
    }

    /// Cleanup expired entries if needed
    fn cleanup_if_needed(&self) {
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.read().unwrap();
            let elapsed = last_cleanup.elapsed().unwrap_or_default();
            elapsed > Duration::from_secs(self.config.cleanup_interval)
        };

        if should_cleanup {
            self.cleanup_expired();
            *self.last_cleanup.write().unwrap() = SystemTime::now();
        }
    }

    /// Remove expired entries
    fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        let mut expired_keys = Vec::new();

        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            cache.pop(&key);
        }

        self.update_memory_stats();
    }
}

impl DependencyResolutionCache {
    /// Create a new dependency resolution cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(config.max_entries)
                    .unwrap_or(std::num::NonZeroUsize::new(1000).unwrap()),
            ))),
            config,
            stats: Arc::new(RwLock::new(CacheStatistics::default())),
            last_cleanup: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// Get dependency resolution result from cache
    pub fn get(&self, key: &str) -> Option<DependencyResolutionResult> {
        let start_time = std::time::Instant::now();

        let result = {
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(key) {
                if !entry.is_expired() {
                    entry.update_access();
                    Some(entry.data.clone())
                } else {
                    cache.pop(key);
                    None
                }
            } else {
                None
            }
        };

        let elapsed = start_time.elapsed().as_micros() as u64;
        let mut stats = self.stats.write().unwrap();

        if result.is_some() {
            stats.record_hit();
        } else {
            stats.record_miss();
        }

        stats.avg_access_time_us = (stats.avg_access_time_us + elapsed) / 2;

        result
    }

    /// Put dependency resolution result in cache
    pub fn put(&self, key: String, result: DependencyResolutionResult) {
        let entry = CacheEntry::new(result, key.clone(), self.config.default_ttl);

        {
            let mut cache = self.cache.write().unwrap();
            cache.put(key, entry);
        }

        self.update_memory_stats();
        self.cleanup_if_needed();
    }

    /// Generate cache key for dependency resolution
    pub fn generate_key(&self, package_hash: &str, options_hash: &str, repo_hash: &str) -> String {
        format!("dependency_{}_{}_{}", package_hash, options_hash, repo_hash)
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();

        let mut stats = self.stats.write().unwrap();
        stats.reset();
    }

    /// Get cache statistics
    pub fn get_statistics(&self) -> CacheStatistics {
        self.stats.read().unwrap().clone()
    }

    /// Update memory usage statistics
    fn update_memory_stats(&self) {
        let cache = self.cache.read().unwrap();
        let mut stats = self.stats.write().unwrap();

        stats.total_entries = cache.len();
        stats.memory_usage_bytes = cache.len() * 4096; // Rough estimate for dependency results
    }

    /// Cleanup expired entries if needed
    fn cleanup_if_needed(&self) {
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.read().unwrap();
            let elapsed = last_cleanup.elapsed().unwrap_or_default();
            elapsed > Duration::from_secs(self.config.cleanup_interval)
        };

        if should_cleanup {
            self.cleanup_expired();
            *self.last_cleanup.write().unwrap() = SystemTime::now();
        }
    }

    /// Remove expired entries
    fn cleanup_expired(&self) {
        let mut cache = self.cache.write().unwrap();
        let mut expired_keys = Vec::new();

        for (key, entry) in cache.iter() {
            if entry.is_expired() {
                expired_keys.push(key.clone());
            }
        }

        for key in expired_keys {
            cache.pop(&key);
        }

        self.update_memory_stats();
    }
}

impl PackageCacheManager {
    /// Create a new package cache manager
    pub fn new(config: CacheConfig) -> Self {
        Self {
            parse_cache: PackageParseCache::new(config.clone()),
            validation_cache: PackageValidationCache::new(config.clone()),
            dependency_cache: DependencyResolutionCache::new(config.clone()),
            config,
        }
    }

    /// Create cache manager with default configuration
    pub fn default() -> Self {
        Self::new(CacheConfig::default())
    }

    /// Create cache manager for development
    pub fn development() -> Self {
        Self::new(CacheConfig::development())
    }

    /// Create cache manager for production
    pub fn production() -> Self {
        Self::new(CacheConfig::production())
    }

    /// Get combined cache statistics
    pub fn get_combined_statistics(&self) -> HashMap<String, CacheStatistics> {
        let mut stats = HashMap::new();
        stats.insert("parse".to_string(), self.parse_cache.get_statistics());
        stats.insert(
            "validation".to_string(),
            self.validation_cache.get_statistics(),
        );
        stats.insert(
            "dependency".to_string(),
            self.dependency_cache.get_statistics(),
        );
        stats
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.parse_cache.clear();
        self.validation_cache.clear();
        self.dependency_cache.clear();
    }

    /// Get total memory usage across all caches
    pub fn get_total_memory_usage(&self) -> usize {
        let parse_stats = self.parse_cache.get_statistics();
        let validation_stats = self.validation_cache.get_statistics();
        let dependency_stats = self.dependency_cache.get_statistics();

        parse_stats.memory_usage_bytes
            + validation_stats.memory_usage_bytes
            + dependency_stats.memory_usage_bytes
    }

    /// Get total cache entries across all caches
    pub fn get_total_entries(&self) -> usize {
        let parse_stats = self.parse_cache.get_statistics();
        let validation_stats = self.validation_cache.get_statistics();
        let dependency_stats = self.dependency_cache.get_statistics();

        parse_stats.total_entries + validation_stats.total_entries + dependency_stats.total_entries
    }

    /// Get overall hit ratio across all caches
    pub fn get_overall_hit_ratio(&self) -> f64 {
        let parse_stats = self.parse_cache.get_statistics();
        let validation_stats = self.validation_cache.get_statistics();
        let dependency_stats = self.dependency_cache.get_statistics();

        let total_hits = parse_stats.hits + validation_stats.hits + dependency_stats.hits;
        let total_misses = parse_stats.misses + validation_stats.misses + dependency_stats.misses;
        let total_requests = total_hits + total_misses;

        if total_requests > 0 {
            total_hits as f64 / total_requests as f64
        } else {
            0.0
        }
    }

    /// Generate hash for package content
    pub fn generate_package_hash(&self, package: &Package) -> String {
        let mut hasher = DefaultHasher::new();
        package.name.hash(&mut hasher);
        package.version.hash(&mut hasher);
        package.requires.hash(&mut hasher);
        package.build_requires.hash(&mut hasher);
        package.private_build_requires.hash(&mut hasher);
        package.variants.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Generate hash for validation options
    pub fn generate_validation_options_hash(&self, options: &str) -> String {
        let mut hasher = DefaultHasher::new();
        options.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Generate hash for dependency resolution options
    pub fn generate_dependency_options_hash(&self, options: &str) -> String {
        let mut hasher = DefaultHasher::new();
        options.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if memory usage is within limits
    pub fn is_memory_usage_within_limits(&self) -> bool {
        self.get_total_memory_usage() <= self.config.max_memory_bytes
    }

    /// Perform cache maintenance
    pub fn perform_maintenance(&self) {
        // Force cleanup on all caches
        self.parse_cache.cleanup_expired();
        self.validation_cache.cleanup_expired();
        self.dependency_cache.cleanup_expired();

        // If still over memory limit, clear least recently used entries
        if !self.is_memory_usage_within_limits() {
            self.evict_lru_entries();
        }
    }

    /// Evict least recently used entries to free memory
    fn evict_lru_entries(&self) {
        // This is a simplified implementation
        // In a real implementation, you'd implement a more sophisticated LRU eviction
        let target_memory = self.config.max_memory_bytes * 80 / 100; // Target 80% of max
        let current_memory = self.get_total_memory_usage();

        if current_memory > target_memory {
            // Clear some entries from each cache
            // This is a simplified approach - a better implementation would
            // track access patterns and evict truly least recently used items
            let entries_to_remove = (current_memory - target_memory) / 1024; // Rough estimate

            // For now, just clear some entries from each cache
            // A real implementation would be more sophisticated
            if entries_to_remove > 10 {
                // Clear oldest entries from each cache
                // This would require additional tracking in a real implementation
            }
        }
    }

    /// Save cache to persistent storage (if enabled)
    pub fn save_to_disk(&self) -> Result<(), RezCoreError> {
        if !self.config.persistent {
            return Ok(());
        }

        let cache_dir = self.config.cache_dir.as_ref().ok_or_else(|| {
            RezCoreError::CacheError("Cache directory not configured".to_string())
        })?;

        // Create cache directory if it doesn't exist
        fs::create_dir_all(cache_dir).map_err(|e| {
            RezCoreError::CacheError(format!("Failed to create cache directory: {}", e))
        })?;

        // In a real implementation, you would serialize and save cache contents
        // This is a placeholder for the actual implementation
        Ok(())
    }

    /// Load cache from persistent storage (if available)
    pub fn load_from_disk(&mut self) -> Result<(), RezCoreError> {
        if !self.config.persistent {
            return Ok(());
        }

        let cache_dir = self.config.cache_dir.as_ref().ok_or_else(|| {
            RezCoreError::CacheError("Cache directory not configured".to_string())
        })?;

        if !cache_dir.exists() {
            return Ok(());
        }

        // In a real implementation, you would deserialize and load cache contents
        // This is a placeholder for the actual implementation
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config() {
        let config = CacheConfig::new();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.default_ttl, 3600);
        assert!(!config.persistent);

        let dev_config = CacheConfig::development();
        assert_eq!(dev_config.max_entries, 500);
        assert!(!dev_config.compress);

        let prod_config = CacheConfig::production();
        assert_eq!(prod_config.max_entries, 5000);
        assert!(prod_config.persistent);
        assert!(prod_config.compress);
    }

    #[test]
    fn test_cache_entry() {
        let data = "test_data".to_string();
        let key = "test_key".to_string();
        let ttl = 3600;

        let mut entry = CacheEntry::new(data.clone(), key.clone(), ttl);
        assert_eq!(entry.data, data);
        assert_eq!(entry.key, key);
        assert_eq!(entry.ttl, ttl);
        assert_eq!(entry.access_count, 0);
        assert!(!entry.is_expired());

        entry.update_access();
        assert_eq!(entry.access_count, 1);
    }

    #[test]
    fn test_cache_statistics() {
        let mut stats = CacheStatistics::default();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_ratio, 0.0);

        stats.record_hit();
        stats.record_hit();
        stats.record_miss();

        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_ratio - 0.6666666666666666).abs() < f64::EPSILON);
    }

    #[test]
    fn test_package_parse_cache() {
        let config = CacheConfig::development();
        let cache = PackageParseCache::new(config);

        let package = Package::new("test_package".to_string());
        let key = "test_key".to_string();

        // Cache miss
        assert!(cache.get(&key).is_none());

        // Cache put and hit
        cache.put(key.clone(), package.clone());
        let cached_package = cache.get(&key);
        assert!(cached_package.is_some());
        assert_eq!(cached_package.unwrap().name, package.name);

        // Test statistics
        let stats = cache.get_statistics();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_cache_manager() {
        let manager = PackageCacheManager::development();

        // Test initial state
        assert_eq!(manager.get_total_entries(), 0);
        assert_eq!(manager.get_total_memory_usage(), 0);
        assert_eq!(manager.get_overall_hit_ratio(), 0.0);

        // Test memory usage check
        assert!(manager.is_memory_usage_within_limits());

        // Test combined statistics
        let stats = manager.get_combined_statistics();
        assert!(stats.contains_key("parse"));
        assert!(stats.contains_key("validation"));
        assert!(stats.contains_key("dependency"));
    }

    #[test]
    fn test_hash_generation() {
        let manager = PackageCacheManager::default();
        let package = Package::new("test_package".to_string());

        let hash1 = manager.generate_package_hash(&package);
        let hash2 = manager.generate_package_hash(&package);
        assert_eq!(hash1, hash2);

        let mut package2 = Package::new("test_package".to_string());
        package2.requires.push("dependency".to_string());
        let hash3 = manager.generate_package_hash(&package2);
        assert_ne!(hash1, hash3);
    }
}
