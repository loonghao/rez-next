//! Cache management for the repository scanner.
//!
//! This module handles:
//! - Scan result caching, eviction and validation.
//! - Prefix-based cache lookup.
//! - Cache pre-loading and background refresh.
//! - Cache statistics reporting.

use super::RepositoryScanner;
use crate::scanner_types::{CacheStatistics, PackageScanResult, ScanCacheEntry};
use rez_next_common::RezCoreError;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::SystemTime;
use tokio::time::interval;
use tracing::warn;

impl RepositoryScanner {
    /// Clear the scan cache and reset all performance counters.
    pub fn clear_cache(&self) {
        self.scan_cache.clear();
        self.prefix_cache.clear();
        self.io_time.store(0, Ordering::Relaxed);
        self.parsing_time.store(0, Ordering::Relaxed);
        self.memory_mapped_files.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.prefix_hits.store(0, Ordering::Relaxed);
        self.peak_concurrency.store(0, Ordering::Relaxed);
        self.current_concurrency.store(0, Ordering::Relaxed);
        self.peak_memory_bytes.store(0, Ordering::Relaxed);
    }

    /// Return the number of entries currently held in the scan cache.
    pub fn cache_size(&self) -> usize {
        self.scan_cache.len()
    }

    /// Build a [`CacheStatistics`] snapshot from the current atomic counters.
    pub fn get_cache_statistics(&self) -> CacheStatistics {
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let prefix_hits = self.prefix_hits.load(Ordering::Relaxed);
        let total_entries = hits + misses;

        let hit_rate = if total_entries > 0 {
            hits as f64 / total_entries as f64
        } else {
            0.0
        };

        let prefix_hit_rate = if total_entries > 0 {
            prefix_hits as f64 / total_entries as f64
        } else {
            0.0
        };

        CacheStatistics {
            hits,
            misses,
            prefix_hits,
            hit_rate,
            prefix_hit_rate,
            cache_size: self.scan_cache.len(),
            total_entries,
        }
    }

    /// Retrieve a cached result using exact or prefix path matching.
    pub fn get_by_prefix(&self, path: &Path) -> Option<PackageScanResult> {
        if !self.config.enable_prefix_matching {
            return None;
        }

        let normalized_path = self.normalize_path(path);

        // Exact match first
        if let Some(mut entry) = self.scan_cache.get_mut(&normalized_path) {
            if self.is_cache_entry_valid(&entry) {
                entry.access_count += 1;
                entry.last_accessed = SystemTime::now();
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.result.clone());
            }
        }

        // Prefix match fallback
        for mut cached_entry in self.scan_cache.iter_mut() {
            let cached_path = cached_entry.key();
            if (normalized_path.starts_with(cached_path)
                || cached_path.starts_with(&normalized_path))
                && self.is_cache_entry_valid(cached_entry.value())
            {
                cached_entry.value_mut().access_count += 1;
                cached_entry.value_mut().last_accessed = SystemTime::now();
                self.prefix_hits.fetch_add(1, Ordering::Relaxed);
                return Some(cached_entry.value().result.clone());
            }
        }

        None
    }

    /// Pre-scan the given paths and populate the cache.
    pub async fn preload_common_paths(&self, paths: &[PathBuf]) -> Result<usize, RezCoreError> {
        if !self.config.enable_cache_preload {
            return Ok(0);
        }

        let mut preloaded_count = 0;

        for path in paths {
            if path.exists() && path.is_dir() {
                match self.scan_repository(path).await {
                    Ok(scan_result) => {
                        preloaded_count += scan_result.packages.len();

                        let mut prefix_paths = Vec::new();
                        for package_result in &scan_result.packages {
                            prefix_paths.push(package_result.package_file.clone());
                        }
                        self.prefix_cache.insert(path.clone(), prefix_paths);
                    }
                    Err(e) => {
                        warn!("Failed to preload path {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(preloaded_count)
    }

    /// Pre-scan the paths listed in the scanner configuration.
    pub async fn preload_default_paths(&self) -> Result<usize, RezCoreError> {
        let paths = self.config.preload_paths.clone();
        self.preload_common_paths(&paths).await
    }

    /// Stop the background cache-refresh task (if running).
    pub async fn stop_background_refresh(&self) {
        let mut refresh_handle = self.refresh_handle.write().await;
        if let Some(handle) = refresh_handle.take() {
            handle.abort();
        }
    }

    /// Spawn a background task that periodically evicts stale cache entries.
    pub(super) fn start_background_refresh(&self) {
        let scan_cache = self.scan_cache.clone();
        let prefix_cache = self.prefix_cache.clone();
        let refresh_interval = self.config.cache_refresh_interval;
        let preload_paths = self.config.preload_paths.clone();

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(refresh_interval));

            loop {
                ticker.tick().await;

                // Evict expired entries
                let mut expired_keys = Vec::new();
                for entry in scan_cache.iter() {
                    if !Self::is_cache_entry_valid_static(entry.value()) {
                        expired_keys.push(entry.key().clone());
                    }
                }
                for key in expired_keys {
                    scan_cache.remove(&key);
                }

                // Refresh prefix cache for pre-loaded paths
                for path in &preload_paths {
                    if path.exists() && path.is_dir() {
                        if let Some(mut entry) = prefix_cache.get_mut(path) {
                            entry.clear();
                        }
                    }
                }
            }
        });

        if let Ok(mut refresh_handle) = self.refresh_handle.try_write() {
            *refresh_handle = Some(handle);
        }
    }

    /// Return `true` if the cache entry is still consistent with the on-disk file.
    pub(super) fn is_cache_entry_valid(&self, entry: &ScanCacheEntry) -> bool {
        Self::is_cache_entry_valid_static(entry)
    }

    /// Static (no `&self`) version of [`is_cache_entry_valid`].
    pub(super) fn is_cache_entry_valid_static(entry: &ScanCacheEntry) -> bool {
        if let Ok(metadata) = std::fs::metadata(&entry.result.package_file) {
            if let Ok(mtime) = metadata.modified() {
                return mtime == entry.mtime && metadata.len() == entry.size;
            }
        }
        false
    }
}
