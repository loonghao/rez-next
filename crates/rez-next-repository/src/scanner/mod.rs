//! High-performance repository scanning utilities with optimised I/O.
//!
//! This module is split into focused sub-modules:
//!
//! - [`cache`]  — scan-result caching, eviction, and background refresh.
//! - [`path`]   — path normalisation, include/exclude pattern matching.
//! - [`scan`]   — core scanning: repository tree traversal and file parsing.
//!
//! The [`RepositoryScanner`] struct and its `new()` / `Default` impls live here;
//! all method implementations are spread across the sub-modules via `impl
//! RepositoryScanner` blocks.

// Re-export scanner types for backward compatibility.
use crate::scanner_types::ScanCacheEntry;
pub use crate::scanner_types::{
    CacheStatistics, PackageScanResult, ScanError, ScanErrorType, ScanPerformanceMetrics,
    ScanResult, ScannerConfig,
};

pub(super) mod cache;
pub(super) mod path;
pub(super) mod scan;

use dashmap::DashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};

/// High-performance repository scanner with advanced optimisations.
#[derive(Debug)]
pub struct RepositoryScanner {
    /// Scanner configuration.
    pub(super) config: ScannerConfig,
    /// Semaphore for limiting concurrent I/O operations.
    pub(super) semaphore: Arc<Semaphore>,
    /// Scan result cache (path → cached result).
    pub(super) scan_cache: Arc<DashMap<PathBuf, ScanCacheEntry>>,
    /// Accumulated I/O time across files scanned (milliseconds).
    pub(super) io_time: Arc<AtomicU64>,
    /// Accumulated parse time across files scanned (milliseconds).
    pub(super) parsing_time: Arc<AtomicU64>,
    /// Number of files read via memory-mapping.
    pub(super) memory_mapped_files: Arc<AtomicUsize>,
    /// Number of cache hits.
    pub(super) cache_hits: Arc<AtomicUsize>,
    /// Number of cache misses.
    pub(super) cache_misses: Arc<AtomicUsize>,
    /// Number of prefix-based cache hits.
    pub(super) prefix_hits: Arc<AtomicUsize>,
    /// Highest observed concurrency level.
    pub(super) peak_concurrency: Arc<AtomicUsize>,
    /// Current in-flight concurrency level.
    pub(super) current_concurrency: Arc<AtomicUsize>,
    /// Peak memory approximation: sum of file-content bytes held in memory.
    pub(super) peak_memory_bytes: Arc<AtomicU64>,
    /// Path-prefix → child-path index for prefix-based lookups.
    pub(super) prefix_cache: Arc<DashMap<PathBuf, Vec<PathBuf>>>,
    /// Background refresh task handle.
    pub(super) refresh_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    /// Pre-compiled exclude-pattern regexes (built once in `new()`).
    pub(super) exclude_regexes: Arc<Vec<regex::Regex>>,
    /// Exact filenames from `include_patterns` with no wildcards (O(1) lookup).
    pub(super) include_filenames: Arc<HashSet<String>>,
}

impl RepositoryScanner {
    /// Create a new high-performance repository scanner.
    pub fn new(config: ScannerConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_scans));

        // Pre-compile exclude patterns once so `should_exclude_path` never
        // re-compiles regexes on the hot path.
        let exclude_regexes: Vec<regex::Regex> = config
            .exclude_patterns
            .iter()
            .filter_map(|p| Self::glob_to_regex(p))
            .collect();

        // Build a HashSet of exact (wildcard-free) include filenames for O(1)
        // lookup in `is_package_file`.
        let include_filenames: HashSet<String> = config
            .include_patterns
            .iter()
            .filter(|p| !p.contains('*') && !p.contains('?'))
            .cloned()
            .collect();

        let scanner = Self {
            config: config.clone(),
            semaphore,
            scan_cache: Arc::new(DashMap::new()),
            io_time: Arc::new(AtomicU64::new(0)),
            parsing_time: Arc::new(AtomicU64::new(0)),
            memory_mapped_files: Arc::new(AtomicUsize::new(0)),
            cache_hits: Arc::new(AtomicUsize::new(0)),
            cache_misses: Arc::new(AtomicUsize::new(0)),
            prefix_hits: Arc::new(AtomicUsize::new(0)),
            peak_concurrency: Arc::new(AtomicUsize::new(0)),
            current_concurrency: Arc::new(AtomicUsize::new(0)),
            peak_memory_bytes: Arc::new(AtomicU64::new(0)),
            prefix_cache: Arc::new(DashMap::new()),
            refresh_handle: Arc::new(RwLock::new(None)),
            exclude_regexes: Arc::new(exclude_regexes),
            include_filenames: Arc::new(include_filenames),
        };

        if config.enable_background_refresh && config.cache_refresh_interval > 0 {
            scanner.start_background_refresh();
        }

        scanner
    }
}

impl Default for RepositoryScanner {
    fn default() -> Self {
        Self::new(ScannerConfig::default())
    }
}

#[cfg(test)]
#[path = "../scanner_tests.rs"]
mod scanner_tests;
