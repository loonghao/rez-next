//! Unified cache interface and implementations
//!
//! This module provides a common trait for all cache types in rez-core,
//! enabling unified management and intelligent caching strategies.

use crate::{CacheError, UnifiedCacheStats};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

/// Unified cache interface for all rez-core cache types
///
/// This trait provides a common interface for SolverCache, RepositoryCache,
/// RexCache, and any future cache implementations. It enables unified
/// management through the IntelligentCacheManager.
#[async_trait]
pub trait UnifiedCache<K, V>: Send + Sync + Debug
where
    K: Clone + Hash + Eq + Send + Sync + Debug,
    V: Clone + Send + Sync + Debug,
{
    /// Get a value from the cache
    ///
    /// Returns `Some(value)` if the key exists and is valid,
    /// `None` if the key doesn't exist or has expired.
    async fn get(&self, key: &K) -> Option<V>;

    /// Put a value into the cache
    ///
    /// Stores the key-value pair in the cache. May trigger eviction
    /// if the cache is at capacity.
    async fn put(&self, key: K, value: V) -> Result<(), CacheError>;

    /// Remove a value from the cache
    ///
    /// Returns `true` if the key was present and removed,
    /// `false` if the key was not found.
    async fn remove(&self, key: &K) -> bool;

    /// Check if a key exists in the cache
    ///
    /// Returns `true` if the key exists and is valid,
    /// `false` otherwise.
    async fn contains_key(&self, key: &K) -> bool;

    /// Get cache statistics
    ///
    /// Returns comprehensive statistics about cache performance,
    /// including hit rates, memory usage, and entry counts.
    async fn get_stats(&self) -> UnifiedCacheStats;

    /// Clear all entries from the cache
    ///
    /// Removes all cached entries. This operation cannot be undone.
    async fn clear(&self) -> Result<(), CacheError>;

    /// Get the current number of entries in the cache
    async fn size(&self) -> usize;

    /// Check if the cache is empty
    async fn is_empty(&self) -> bool {
        self.size().await == 0
    }

    /// Get cache capacity (maximum number of entries)
    async fn capacity(&self) -> usize;

    /// Get cache type identifier
    fn cache_type(&self) -> &'static str;
}

/// Cache entry metadata for unified management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntryMetadata {
    /// Entry creation timestamp (Unix timestamp)
    pub created_at: u64,
    /// Last access timestamp (Unix timestamp)
    pub last_accessed: u64,
    /// Access count
    pub access_count: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Entry size in bytes (estimated)
    pub size_bytes: u64,
    /// Cache level (L1, L2, etc.)
    pub cache_level: CacheLevel,
    /// Priority score for eviction decisions
    pub priority_score: f64,
}

impl CacheEntryMetadata {
    /// Create new metadata for a cache entry
    pub fn new(ttl: u64, size_bytes: u64, cache_level: CacheLevel) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl,
            size_bytes,
            cache_level,
            priority_score: 1.0,
        }
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }

    /// Check if the entry is expired
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now > self.created_at + self.ttl
    }

    /// Calculate cache retention score for eviction decisions
    pub fn retention_score(&self) -> f64 {
        let age_factor = 1.0 / (1.0 + (self.last_accessed as f64 - self.created_at as f64) / 3600.0);
        let frequency_factor = (self.access_count as f64).ln().max(1.0);
        let size_factor = 1.0 / (1.0 + self.size_bytes as f64 / 1024.0);
        
        self.priority_score * frequency_factor * age_factor * size_factor
    }
}

/// Cache level enumeration for multi-level caching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheLevel {
    /// L1 memory cache (fastest access)
    L1,
    /// L2 disk cache (larger capacity)
    L2,
    /// L3 remote cache (highest capacity)
    L3,
}

impl CacheLevel {
    /// Get the access speed ranking (lower is faster)
    pub fn speed_rank(&self) -> u8 {
        match self {
            CacheLevel::L1 => 1,
            CacheLevel::L2 => 2,
            CacheLevel::L3 => 3,
        }
    }

    /// Get the capacity ranking (higher is larger)
    pub fn capacity_rank(&self) -> u8 {
        match self {
            CacheLevel::L1 => 1,
            CacheLevel::L2 => 2,
            CacheLevel::L3 => 3,
        }
    }
}

/// Wrapper for cache entries with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<V> {
    /// The cached value
    pub value: V,
    /// Entry metadata
    pub metadata: CacheEntryMetadata,
}

impl<V> CacheEntry<V> {
    /// Create a new cache entry
    pub fn new(value: V, ttl: u64, size_bytes: u64, cache_level: CacheLevel) -> Self {
        Self {
            value,
            metadata: CacheEntryMetadata::new(ttl, size_bytes, cache_level),
        }
    }

    /// Mark the entry as accessed and return the value
    pub fn access(&mut self) -> &V {
        self.metadata.mark_accessed();
        &self.value
    }

    /// Check if the entry is expired
    pub fn is_expired(&self) -> bool {
        self.metadata.is_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_entry_metadata() {
        let metadata = CacheEntryMetadata::new(3600, 1024, CacheLevel::L1);
        assert_eq!(metadata.ttl, 3600);
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.cache_level, CacheLevel::L1);
        assert!(!metadata.is_expired());
    }

    #[test]
    fn test_cache_level_rankings() {
        assert!(CacheLevel::L1.speed_rank() < CacheLevel::L2.speed_rank());
        assert!(CacheLevel::L2.speed_rank() < CacheLevel::L3.speed_rank());
        assert!(CacheLevel::L1.capacity_rank() < CacheLevel::L2.capacity_rank());
    }

    #[test]
    fn test_cache_entry() {
        let entry = CacheEntry::new("test_value".to_string(), 3600, 100, CacheLevel::L1);
        assert_eq!(entry.value, "test_value");
        assert!(!entry.is_expired());
    }
}
