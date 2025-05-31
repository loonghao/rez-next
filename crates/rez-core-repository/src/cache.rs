//! Repository caching system

use rez_core_common::RezCoreError;
use rez_core_package::Package;
use rez_core_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock;

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Package data
    pub package: Package,
    /// Cache timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Source file path
    pub source_path: PathBuf,
    /// Source file modification time
    pub source_mtime: u64,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(package: Package, source_path: PathBuf, ttl: u64) -> Result<Self, RezCoreError> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RezCoreError::Cache(format!("Failed to get current time: {}", e)))?
            .as_secs();

        let source_mtime = std::fs::metadata(&source_path)
            .map_err(|e| RezCoreError::Cache(format!("Failed to get file metadata: {}", e)))?
            .modified()
            .map_err(|e| RezCoreError::Cache(format!("Failed to get modification time: {}", e)))?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RezCoreError::Cache(format!("Failed to convert modification time: {}", e)))?
            .as_secs();

        Ok(Self {
            package,
            timestamp,
            ttl,
            source_path,
            source_mtime,
        })
    }

    /// Check if the cache entry is valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Check TTL
        if now > self.timestamp + self.ttl {
            return false;
        }

        // Check if source file has been modified
        if let Ok(metadata) = std::fs::metadata(&self.source_path) {
            if let Ok(mtime) = metadata.modified() {
                if let Ok(mtime_secs) = mtime.duration_since(UNIX_EPOCH) {
                    return mtime_secs.as_secs() <= self.source_mtime;
                }
            }
        }

        true
    }

    /// Get the age of the cache entry in seconds
    pub fn age(&self) -> u64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now.saturating_sub(self.timestamp)
    }
}

/// Repository cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache directory path
    pub cache_dir: PathBuf,
    /// Default TTL for cache entries (in seconds)
    pub default_ttl: u64,
    /// Maximum cache size in bytes
    pub max_size_bytes: u64,
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Cache cleanup interval (in seconds)
    pub cleanup_interval: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: PathBuf::from(".rez_cache"),
            default_ttl: 3600, // 1 hour
            max_size_bytes: 100 * 1024 * 1024, // 100 MB
            max_entries: 10000,
            enable_compression: true,
            cleanup_interval: 300, // 5 minutes
        }
    }
}

/// Repository cache implementation
#[derive(Debug)]
pub struct RepositoryCache {
    /// Cache configuration
    config: CacheConfig,
    /// In-memory cache index
    index: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cache hits
    pub hits: u64,
    /// Total cache misses
    pub misses: u64,
    /// Total cache entries
    pub entries: usize,
    /// Total cache size in bytes
    pub size_bytes: u64,
    /// Last cleanup time
    pub last_cleanup: Option<u64>,
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            entries: 0,
            size_bytes: 0,
            last_cleanup: None,
        }
    }
}

impl RepositoryCache {
    /// Create a new repository cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            index: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Initialize the cache (create directories, load existing cache)
    pub async fn initialize(&self) -> Result<(), RezCoreError> {
        // Create cache directory if it doesn't exist
        if !self.config.cache_dir.exists() {
            fs::create_dir_all(&self.config.cache_dir).await
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to create cache directory: {}", e)
                ))?;
        }

        // Load existing cache index
        self.load_cache_index().await?;

        Ok(())
    }

    /// Get a package from cache
    pub async fn get(&self, key: &str) -> Result<Option<Package>, RezCoreError> {
        let mut index = self.index.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = index.get(key) {
            if entry.is_valid() {
                stats.hits += 1;
                return Ok(Some(entry.package.clone()));
            } else {
                // Remove invalid entry
                index.remove(key);
                self.remove_cache_file(key).await?;
            }
        }

        stats.misses += 1;
        Ok(None)
    }

    /// Put a package into cache
    pub async fn put(&self, key: &str, package: Package, source_path: PathBuf) -> Result<(), RezCoreError> {
        let entry = CacheEntry::new(package, source_path, self.config.default_ttl)?;
        
        // Write to disk
        self.write_cache_file(key, &entry).await?;

        // Update in-memory index
        {
            let mut index = self.index.write().await;
            index.insert(key.to_string(), entry);
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.entries = {
                let index = self.index.read().await;
                index.len()
            };
        }

        // Check if cleanup is needed
        self.cleanup_if_needed().await?;

        Ok(())
    }

    /// Remove a package from cache
    pub async fn remove(&self, key: &str) -> Result<bool, RezCoreError> {
        let mut index = self.index.write().await;
        let removed = index.remove(key).is_some();

        if removed {
            self.remove_cache_file(key).await?;
            
            let mut stats = self.stats.write().await;
            stats.entries = index.len();
        }

        Ok(removed)
    }

    /// Clear all cache entries
    pub async fn clear(&self) -> Result<(), RezCoreError> {
        {
            let mut index = self.index.write().await;
            index.clear();
        }

        // Remove all cache files
        if self.config.cache_dir.exists() {
            fs::remove_dir_all(&self.config.cache_dir).await
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to clear cache directory: {}", e)
                ))?;
            
            fs::create_dir_all(&self.config.cache_dir).await
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to recreate cache directory: {}", e)
                ))?;
        }

        // Reset statistics
        {
            let mut stats = self.stats.write().await;
            *stats = CacheStats::default();
        }

        Ok(())
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Perform cache cleanup (remove expired entries)
    pub async fn cleanup(&self) -> Result<(), RezCoreError> {
        let mut index = self.index.write().await;
        let mut expired_keys = Vec::new();

        // Find expired entries
        for (key, entry) in index.iter() {
            if !entry.is_valid() {
                expired_keys.push(key.clone());
            }
        }

        // Remove expired entries
        for key in &expired_keys {
            index.remove(key);
            self.remove_cache_file(key).await?;
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.entries = index.len();
            stats.last_cleanup = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0)
            );
        }

        Ok(())
    }

    /// Check if cleanup is needed and perform it
    async fn cleanup_if_needed(&self) -> Result<(), RezCoreError> {
        let stats = self.stats.read().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let should_cleanup = match stats.last_cleanup {
            Some(last) => now > last + self.config.cleanup_interval,
            None => true,
        };

        drop(stats); // Release the read lock

        if should_cleanup {
            self.cleanup().await?;
        }

        Ok(())
    }

    /// Load cache index from disk
    async fn load_cache_index(&self) -> Result<(), RezCoreError> {
        let index_path = self.config.cache_dir.join("index.json");
        
        if index_path.exists() {
            let content = fs::read_to_string(&index_path).await
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to read cache index: {}", e)
                ))?;

            let cache_index: HashMap<String, CacheEntry> = serde_json::from_str(&content)
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to parse cache index: {}", e)
                ))?;

            let mut index = self.index.write().await;
            *index = cache_index;
        }

        Ok(())
    }

    /// Save cache index to disk
    async fn save_cache_index(&self) -> Result<(), RezCoreError> {
        let index_path = self.config.cache_dir.join("index.json");
        let index = self.index.read().await;
        
        let content = serde_json::to_string_pretty(&*index)
            .map_err(|e| RezCoreError::Cache(
                format!("Failed to serialize cache index: {}", e)
            ))?;

        fs::write(&index_path, content).await
            .map_err(|e| RezCoreError::Cache(
                format!("Failed to write cache index: {}", e)
            ))?;

        Ok(())
    }

    /// Write a cache entry to disk
    async fn write_cache_file(&self, key: &str, entry: &CacheEntry) -> Result<(), RezCoreError> {
        let cache_file = self.config.cache_dir.join(format!("{}.json", key));
        
        let content = serde_json::to_string_pretty(entry)
            .map_err(|e| RezCoreError::Cache(
                format!("Failed to serialize cache entry: {}", e)
            ))?;

        fs::write(&cache_file, content).await
            .map_err(|e| RezCoreError::Cache(
                format!("Failed to write cache file: {}", e)
            ))?;

        Ok(())
    }

    /// Remove a cache file from disk
    async fn remove_cache_file(&self, key: &str) -> Result<(), RezCoreError> {
        let cache_file = self.config.cache_dir.join(format!("{}.json", key));
        
        if cache_file.exists() {
            fs::remove_file(&cache_file).await
                .map_err(|e| RezCoreError::Cache(
                    format!("Failed to remove cache file: {}", e)
                ))?;
        }

        Ok(())
    }
}
