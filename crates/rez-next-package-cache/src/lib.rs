//! Package caching for rez-next.
//!
//! This module provides caching functionality for package definitions to avoid
//! re-scanning the filesystem on every rez command, improving performance
//! for large package repositories.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Error types ─────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Cache entry not found: {0}")]
    NotFound(String),

    #[error("Cache entry expired: {0}")]
    Expired(String),
}

pub type CacheResult<T> = Result<T, CacheError>;

// ── Cache entry ────────────────────────────────────────────────────────────────

/// A cached package definition with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPackage {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Path to the package definition file
    pub path: PathBuf,
    /// Last modification time of the package definition file
    pub mtime: SystemTime,
    /// Cached package data (serialized)
    pub data: String,
    /// When this entry was cached
    pub cached_at: SystemTime,
    /// TTL for this entry (None = no expiry)
    pub ttl: Option<Duration>,
}

impl CachedPackage {
    /// Check if the cache entry is still valid.
    /// Returns true if:
    /// 1. The source file hasn't been modified (mtime matches)
    /// 2. The cache entry hasn't expired (cached_at + ttl > now)
    pub fn is_valid(&self, source_mtime: SystemTime) -> bool {
        // Check if source file has been modified
        // Use duration_since to compare two SystemTime values
        let mtime_changed = source_mtime
            .duration_since(self.mtime)
            .is_ok_and(|dur| dur.as_secs() > 0)
            || self
                .mtime
                .duration_since(source_mtime)
                .is_ok_and(|dur| dur.as_secs() > 0);

        if mtime_changed {
            return false;
        }

        // Check TTL
        if let Some(ttl) = self.ttl {
            if let Ok(elapsed) = self.cached_at.elapsed() {
                if elapsed > ttl {
                    return false;
                }
            } else {
                // If we can't determine elapsed time, assume invalid
                return false;
            }
        }

        true
    }
}

// ── Cache backend trait ─────────────────────────────────────────────────────────

/// Trait for cache backends.
pub trait CacheBackend: Send + Sync {
    /// Get a cached package by path.
    fn get(&self, path: &Path) -> CacheResult<Option<CachedPackage>>;

    /// Put a package into the cache.
    fn put(&self, path: &Path, package: CachedPackage) -> CacheResult<()>;

    /// Remove a cached package by path.
    fn remove(&self, path: &Path) -> CacheResult<()>;

    /// Clear all cached packages.
    fn clear(&self) -> CacheResult<()>;

    /// Get cache statistics.
    fn stats(&self) -> CacheStats;
}

// ── Cache statistics ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of cache puts
    pub puts: u64,
    /// Number of cache removes
    pub removes: u64,
    /// Number of cache clears
    pub clears: u64,
}

// ── In-memory cache ────────────────────────────────────────────────────────────

/// In-memory cache backend using DashMap for concurrent access.
pub struct InMemoryCache {
    store: DashMap<PathBuf, CachedPackage>,
    stats: DashMap<&'static str, u64>,
}

impl InMemoryCache {
    /// Create a new in-memory cache.
    pub fn new() -> Self {
        Self {
            store: DashMap::new(),
            stats: DashMap::new(),
        }
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheBackend for InMemoryCache {
    fn get(&self, path: &Path) -> CacheResult<Option<CachedPackage>> {
        match self.store.get(path) {
            Some(entry) => {
                // Update stats
                *self.stats.entry("hits").or_insert(0) += 1;

                // Check if entry is still valid
                let cached = entry.clone();
                if let Ok(metadata) = std::fs::metadata(path) {
                    if let Ok(mtime) = metadata.modified() {
                        if cached.is_valid(mtime) {
                            return Ok(Some(cached));
                        }
                    }
                }

                // Entry is invalid, remove it
                drop(entry);
                self.store.remove(path);
                *self.stats.entry("misses").or_insert(0) += 1;
                Ok(None)
            }
            None => {
                *self.stats.entry("misses").or_insert(0) += 1;
                Ok(None)
            }
        }
    }

    fn put(&self, path: &Path, package: CachedPackage) -> CacheResult<()> {
        self.store.insert(path.to_path_buf(), package);
        *self.stats.entry("puts").or_insert(0) += 1;
        Ok(())
    }

    fn remove(&self, path: &Path) -> CacheResult<()> {
        self.store.remove(path);
        *self.stats.entry("removes").or_insert(0) += 1;
        Ok(())
    }

    fn clear(&self) -> CacheResult<()> {
        self.store.clear();
        *self.stats.entry("clears").or_insert(0) += 1;
        Ok(())
    }

    fn stats(&self) -> CacheStats {
        CacheStats {
            hits: *self.stats.entry("hits").or_insert(0),
            misses: *self.stats.entry("misses").or_insert(0),
            puts: *self.stats.entry("puts").or_insert(0),
            removes: *self.stats.entry("removes").or_insert(0),
            clears: *self.stats.entry("clears").or_insert(0),
        }
    }
}

// ── File-based cache ───────────────────────────────────────────────────────────

/// File-based cache backend that stores cache entries as JSON files.
pub struct FileCache {
    cache_dir: PathBuf,
    in_memory: InMemoryCache,
}

impl FileCache {
    /// Create a new file-based cache.
    pub fn new(cache_dir: impl Into<PathBuf>) -> CacheResult<Self> {
        let cache_dir = cache_dir.into();
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self {
            cache_dir,
            in_memory: InMemoryCache::new(),
        })
    }

    /// Get the cache file path for a package path.
    fn cache_file_path(&self, path: &Path) -> PathBuf {
        // Use a hash of the path as the cache file name
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        let hash = hasher.finish();

        self.cache_dir.join(format!("{:016x}.json", hash))
    }
}

impl CacheBackend for FileCache {
    fn get(&self, path: &Path) -> CacheResult<Option<CachedPackage>> {
        // Check in-memory cache first
        if let Some(cached) = self.in_memory.get(path)? {
            return Ok(Some(cached));
        }

        // Check file cache
        let cache_file = self.cache_file_path(path);
        if cache_file.exists() {
            let content = std::fs::read_to_string(&cache_file)?;
            let cached: CachedPackage = serde_json::from_str(&content)?;

            // Check if entry is still valid
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(mtime) = metadata.modified() {
                    if cached.is_valid(mtime) {
                        // Update in-memory cache
                        self.in_memory.put(path, cached.clone())?;
                        return Ok(Some(cached));
                    }
                }
            }

            // Entry is invalid, remove it
            std::fs::remove_file(&cache_file)?;
        }

        Ok(None)
    }

    fn put(&self, path: &Path, package: CachedPackage) -> CacheResult<()> {
        // Update in-memory cache
        self.in_memory.put(path, package.clone())?;

        // Update file cache
        let cache_file = self.cache_file_path(path);
        let content = serde_json::to_string_pretty(&package)?;
        std::fs::write(&cache_file, content)?;

        Ok(())
    }

    fn remove(&self, path: &Path) -> CacheResult<()> {
        // Remove from in-memory cache
        self.in_memory.remove(path)?;

        // Remove from file cache
        let cache_file = self.cache_file_path(path);
        if cache_file.exists() {
            std::fs::remove_file(&cache_file)?;
        }

        Ok(())
    }

    fn clear(&self) -> CacheResult<()> {
        // Clear in-memory cache
        self.in_memory.clear()?;

        // Clear file cache
        for entry in std::fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                std::fs::remove_file(&path)?;
            }
        }

        Ok(())
    }

    fn stats(&self) -> CacheStats {
        self.in_memory.stats()
    }
}

// ── Package cache manager ─────────────────────────────────────────────────────

/// Package cache manager that coordinates cache backends.
pub struct PackageCache {
    backend: Arc<dyn CacheBackend>,
    default_ttl: Option<Duration>,
}

impl Clone for PackageCache {
    fn clone(&self) -> Self {
        Self {
            backend: Arc::clone(&self.backend),
            default_ttl: self.default_ttl,
        }
    }
}

impl PackageCache {
    /// Create a new package cache with the given backend.
    pub fn new(backend: Arc<dyn CacheBackend>) -> Self {
        Self {
            backend,
            default_ttl: None,
        }
    }

    /// Create a new package cache with in-memory backend.
    pub fn new_in_memory() -> Self {
        Self::new(Arc::new(InMemoryCache::new()))
    }

    /// Create a new package cache with file backend.
    pub fn new_file_based(cache_dir: impl Into<PathBuf>) -> CacheResult<Self> {
        let backend = FileCache::new(cache_dir)?;
        Ok(Self::new(Arc::new(backend)))
    }

    /// Set the default TTL for cache entries.
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    /// Get a cached package by path.
    pub fn get(&self, path: &Path) -> CacheResult<Option<CachedPackage>> {
        self.backend.get(path)
    }

    /// Put a package into the cache.
    pub fn put(&self, path: &Path, mut package: CachedPackage) -> CacheResult<()> {
        // Set default TTL if not set
        if package.ttl.is_none() {
            package.ttl = self.default_ttl;
        }

        self.backend.put(path, package)
    }

    /// Put a package into the cache with the given data.
    pub fn put_package(
        &self,
        path: &Path,
        name: &str,
        version: &str,
        data: &str,
    ) -> CacheResult<()> {
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| SystemTime::now());

        let package = CachedPackage {
            name: name.to_string(),
            version: version.to_string(),
            path: path.to_path_buf(),
            mtime,
            data: data.to_string(),
            cached_at: SystemTime::now(),
            ttl: self.default_ttl,
        };

        self.put(path, package)
    }

    /// Remove a cached package by path.
    pub fn remove(&self, path: &Path) -> CacheResult<()> {
        self.backend.remove(path)
    }

    /// Clear all cached packages.
    pub fn clear(&self) -> CacheResult<()> {
        self.backend.clear()
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        self.backend.stats()
    }

    /// Invalidate cache entries for packages that have been modified.
    pub fn invalidate_stale(&self) -> CacheResult<u64> {
        // TODO(cleanup): This is a simplified implementation that just clears the entire cache.
        // A more sophisticated implementation would check each entry's mtime.
        self.clear()?;
        Ok(0)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_cached_package_validity() {
        let mtime = SystemTime::now();

        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("test_package.py"),
            mtime,
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };

        // Valid when mtime matches
        assert!(cached.is_valid(mtime));

        // Invalid when mtime differs
        let different_mtime = mtime - Duration::from_secs(1);
        assert!(!cached.is_valid(different_mtime));
    }

    #[test]
    fn test_cached_package_ttl() {
        let mtime = SystemTime::now();

        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: PathBuf::from("test_package.py"),
            mtime,
            data: "{}".to_string(),
            cached_at: SystemTime::now() - Duration::from_secs(10),
            ttl: Some(Duration::from_secs(5)),
        };

        // Invalid because TTL has expired (elapsed > ttl)
        assert!(!cached.is_valid(mtime));
    }

    #[test]
    fn test_in_memory_cache_basic() {
        let cache = InMemoryCache::new();
        let temp_file = create_test_file("{}");
        let path = temp_file.path().to_path_buf();

        // Miss on empty cache
        let result = cache.get(&path).unwrap();
        assert!(result.is_none());

        // Put and get
        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: path.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path, cached.clone()).unwrap();

        let result = cache.get(&path).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.version, "1.0.0");
    }

    #[test]
    fn test_in_memory_cache_remove() {
        let cache = InMemoryCache::new();
        let temp_file = create_test_file("{}");
        let path = temp_file.path().to_path_buf();

        // Put
        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: path.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path, cached).unwrap();

        // Remove
        cache.remove(&path).unwrap();

        // Miss after remove
        let result = cache.get(&path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_in_memory_cache_clear() {
        let cache = InMemoryCache::new();
        let temp_file1 = create_test_file("{}");
        let temp_file2 = create_test_file("{}");
        let path1 = temp_file1.path().to_path_buf();
        let path2 = temp_file2.path().to_path_buf();

        // Put two entries
        let cached1 = CachedPackage {
            name: "test1".to_string(),
            version: "1.0.0".to_string(),
            path: path1.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        let cached2 = CachedPackage {
            name: "test2".to_string(),
            version: "2.0.0".to_string(),
            path: path2.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path1, cached1).unwrap();
        cache.put(&path2, cached2).unwrap();

        // Clear
        cache.clear().unwrap();

        // Both should miss
        assert!(cache.get(&path1).unwrap().is_none());
        assert!(cache.get(&path2).unwrap().is_none());
    }

    #[test]
    fn test_in_memory_cache_stats() {
        let cache = InMemoryCache::new();
        let temp_file = create_test_file("{}");
        let path = temp_file.path().to_path_buf();

        // Miss
        cache.get(&path).unwrap();
        assert_eq!(cache.stats().misses, 1);
        assert_eq!(cache.stats().hits, 0);

        // Put
        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: path.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path, cached).unwrap();
        assert_eq!(cache.stats().puts, 1);

        // Hit
        cache.get(&path).unwrap();
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_package_cache_in_memory() {
        let cache = PackageCache::new_in_memory();

        let temp_file = create_test_file("{}");
        let path = temp_file.path().to_path_buf();

        // Miss
        assert!(cache.get(&path).unwrap().is_none());

        // Put using put directly
        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: path.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path, cached).unwrap();

        // Hit
        let result = cache.get(&path).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.name, "test");
        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.data, "{}");
    }

    #[test]
    fn test_package_cache_stats() {
        let cache = PackageCache::new_in_memory();

        let temp_file = create_test_file("{}");
        let path = temp_file.path().to_path_buf();

        // Miss
        cache.get(&path).unwrap();
        assert_eq!(cache.stats().misses, 1);

        // Put
        let cached = CachedPackage {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: path.clone(),
            mtime: SystemTime::now(),
            data: "{}".to_string(),
            cached_at: SystemTime::now(),
            ttl: None,
        };
        cache.put(&path, cached).unwrap();
        assert_eq!(cache.stats().puts, 1);

        // Hit
        cache.get(&path).unwrap();
        assert_eq!(cache.stats().hits, 1);
    }
}
