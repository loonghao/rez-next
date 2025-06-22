//! High-performance repository scanning utilities with optimized I/O

use ahash::AHashMap;
use dashmap::DashMap;
use futures::stream::{self, StreamExt};
use memmap2::Mmap;
use rez_next_common::RezCoreError;
use rez_next_package::Package;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinSet;
use tokio::time::{interval, Instant};

/// Enhanced scanner configuration with performance optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    /// Maximum number of concurrent scan operations
    pub max_concurrent_scans: usize,
    /// Maximum depth to scan directories
    pub max_depth: usize,
    /// File patterns to include (glob patterns)
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (glob patterns)
    pub exclude_patterns: Vec<String>,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Scan timeout in seconds
    pub timeout_seconds: u64,
    /// Enable memory-mapped file reading for large files
    pub use_memory_mapping: bool,
    /// Minimum file size (bytes) to use memory mapping
    pub memory_mapping_threshold: u64,
    /// Batch size for concurrent directory processing
    pub directory_batch_size: usize,
    /// Enable intelligent file type detection
    pub smart_file_detection: bool,
    /// Cache scan results for faster subsequent scans
    pub enable_scan_cache: bool,
    /// Maximum cache size in MB
    pub max_cache_size_mb: usize,
    /// Enable path prefix matching for cache optimization
    pub enable_prefix_matching: bool,
    /// Enable intelligent cache preloading
    pub enable_cache_preload: bool,
    /// Common paths to preload into cache
    pub preload_paths: Vec<PathBuf>,
    /// Cache refresh interval in seconds (0 = disabled)
    pub cache_refresh_interval: u64,
    /// Enable background cache refresh
    pub enable_background_refresh: bool,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_scans: 20, // Increased for better parallelism
            max_depth: 10,
            include_patterns: vec![
                "package.py".to_string(),
                "package.yaml".to_string(),
                "package.yml".to_string(),
                "package.json".to_string(),
            ],
            exclude_patterns: vec![
                ".git/**".to_string(),
                ".svn/**".to_string(),
                "__pycache__/**".to_string(),
                "*.pyc".to_string(),
                ".DS_Store".to_string(),
                "node_modules/**".to_string(),
                ".vscode/**".to_string(),
                ".idea/**".to_string(),
            ],
            follow_symlinks: false,
            timeout_seconds: 300, // 5 minutes
            use_memory_mapping: true,
            memory_mapping_threshold: 1024 * 1024, // 1MB
            directory_batch_size: 50,
            smart_file_detection: true,
            enable_scan_cache: true,
            max_cache_size_mb: 100,
            enable_prefix_matching: true,
            enable_cache_preload: true,
            preload_paths: vec![
                PathBuf::from("/usr/local/packages"),
                PathBuf::from("/opt/packages"),
                PathBuf::from("C:\\packages"),
            ],
            cache_refresh_interval: 300, // 5 minutes
            enable_background_refresh: true,
        }
    }
}

/// Scan result for a single package
#[derive(Debug, Clone)]
pub struct PackageScanResult {
    /// The discovered package
    pub package: Package,
    /// Path to the package definition file
    pub package_file: PathBuf,
    /// Package directory path
    pub package_dir: PathBuf,
    /// File size in bytes
    pub file_size: u64,
    /// Scan duration in milliseconds
    pub scan_duration_ms: u64,
}

/// Enhanced scan result with performance metrics
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// All discovered packages
    pub packages: Vec<PackageScanResult>,
    /// Total scan duration in milliseconds
    pub total_duration_ms: u64,
    /// Number of directories scanned
    pub directories_scanned: usize,
    /// Number of files examined
    pub files_examined: usize,
    /// Number of errors encountered
    pub errors: Vec<ScanError>,
    /// Performance metrics
    pub performance_metrics: ScanPerformanceMetrics,
}

/// Performance metrics for scan operations
#[derive(Debug, Clone)]
pub struct ScanPerformanceMetrics {
    /// Total I/O time in milliseconds
    pub io_time_ms: u64,
    /// Total parsing time in milliseconds
    pub parsing_time_ms: u64,
    /// Number of files read using memory mapping
    pub memory_mapped_files: usize,
    /// Number of cache hits
    pub cache_hits: usize,
    /// Number of cache misses
    pub cache_misses: usize,
    /// Average file size processed (bytes)
    pub avg_file_size: u64,
    /// Peak memory usage during scan (bytes)
    pub peak_memory_usage: u64,
    /// Number of concurrent operations peak
    pub peak_concurrency: usize,
}

/// Scan error information
#[derive(Debug, Clone)]
pub struct ScanError {
    /// Path where the error occurred
    pub path: PathBuf,
    /// Error message
    pub message: String,
    /// Error type
    pub error_type: ScanErrorType,
}

/// Types of scan errors
#[derive(Debug, Clone, PartialEq)]
pub enum ScanErrorType {
    /// File system access error
    FileSystemError,
    /// Package parsing error
    PackageParseError,
    /// Permission denied
    PermissionDenied,
    /// Timeout error
    Timeout,
    /// Other error
    Other,
}

/// Cache entry for scan results
#[derive(Debug, Clone)]
struct ScanCacheEntry {
    /// Cached package scan result
    result: PackageScanResult,
    /// File modification time when cached
    mtime: std::time::SystemTime,
    /// File size when cached
    size: u64,
    /// Cache creation time
    cached_at: SystemTime,
    /// Access count for LRU tracking
    access_count: u64,
    /// Last access time
    last_accessed: SystemTime,
}

/// Enhanced cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Total cache hits
    pub hits: usize,
    /// Total cache misses
    pub misses: usize,
    /// Prefix match hits
    pub prefix_hits: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Prefix match hit rate (0.0 to 1.0)
    pub prefix_hit_rate: f64,
    /// Current cache size
    pub cache_size: usize,
    /// Total entries processed
    pub total_entries: usize,
}

/// High-performance repository scanner with advanced optimizations
#[derive(Debug)]
pub struct RepositoryScanner {
    /// Scanner configuration
    config: ScannerConfig,
    /// Semaphore for limiting concurrent operations
    semaphore: Arc<Semaphore>,
    /// Scan result cache (path -> cached result)
    scan_cache: Arc<DashMap<PathBuf, ScanCacheEntry>>,
    /// Performance metrics tracking
    io_time: Arc<AtomicU64>,
    parsing_time: Arc<AtomicU64>,
    memory_mapped_files: Arc<AtomicUsize>,
    cache_hits: Arc<AtomicUsize>,
    cache_misses: Arc<AtomicUsize>,
    prefix_hits: Arc<AtomicUsize>,
    peak_concurrency: Arc<AtomicUsize>,
    current_concurrency: Arc<AtomicUsize>,
    /// Path prefix cache for faster lookups
    prefix_cache: Arc<DashMap<PathBuf, Vec<PathBuf>>>,
    /// Background refresh task handle
    refresh_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl RepositoryScanner {
    /// Create a new high-performance repository scanner
    pub fn new(config: ScannerConfig) -> Self {
        let semaphore = Arc::new(Semaphore::new(config.max_concurrent_scans));

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
            prefix_cache: Arc::new(DashMap::new()),
            refresh_handle: Arc::new(RwLock::new(None)),
        };

        // Start background refresh if enabled
        if config.enable_background_refresh && config.cache_refresh_interval > 0 {
            scanner.start_background_refresh();
        }

        scanner
    }

    /// Clear the scan cache
    pub fn clear_cache(&self) {
        self.scan_cache.clear();
        self.prefix_cache.clear();
        // Reset metrics
        self.io_time.store(0, Ordering::Relaxed);
        self.parsing_time.store(0, Ordering::Relaxed);
        self.memory_mapped_files.store(0, Ordering::Relaxed);
        self.cache_hits.store(0, Ordering::Relaxed);
        self.cache_misses.store(0, Ordering::Relaxed);
        self.prefix_hits.store(0, Ordering::Relaxed);
        self.peak_concurrency.store(0, Ordering::Relaxed);
        self.current_concurrency.store(0, Ordering::Relaxed);
    }

    /// Get current cache size
    pub fn cache_size(&self) -> usize {
        self.scan_cache.len()
    }

    /// Get cache statistics
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

    /// Get cached result by path prefix matching
    pub fn get_by_prefix(&self, path: &Path) -> Option<PackageScanResult> {
        if !self.config.enable_prefix_matching {
            return None;
        }

        // Normalize the path
        let normalized_path = self.normalize_path(path);

        // First try exact match
        if let Some(mut entry) = self.scan_cache.get_mut(&normalized_path) {
            if self.is_cache_entry_valid(&entry) {
                // Update access statistics
                entry.access_count += 1;
                entry.last_accessed = SystemTime::now();
                self.cache_hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.result.clone());
            }
        }

        // Try prefix matching
        for mut cached_entry in self.scan_cache.iter_mut() {
            let cached_path = cached_entry.key();
            if normalized_path.starts_with(cached_path) || cached_path.starts_with(&normalized_path)
            {
                if self.is_cache_entry_valid(&cached_entry.value()) {
                    // Update access statistics for prefix match
                    cached_entry.value_mut().access_count += 1;
                    cached_entry.value_mut().last_accessed = SystemTime::now();
                    self.prefix_hits.fetch_add(1, Ordering::Relaxed);
                    return Some(cached_entry.value().result.clone());
                }
            }
        }

        None
    }

    /// Preload common paths into cache
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

                        // Update prefix cache
                        let mut prefix_paths = Vec::new();
                        for package_result in &scan_result.packages {
                            prefix_paths.push(package_result.package_file.clone());
                        }
                        self.prefix_cache.insert(path.clone(), prefix_paths);
                    }
                    Err(e) => {
                        eprintln!("Failed to preload path {}: {}", path.display(), e);
                    }
                }
            }
        }

        Ok(preloaded_count)
    }

    /// Preload default common paths from configuration
    pub async fn preload_default_paths(&self) -> Result<usize, RezCoreError> {
        let paths = self.config.preload_paths.clone();
        self.preload_common_paths(&paths).await
    }

    /// Stop background cache refresh task
    pub async fn stop_background_refresh(&self) {
        if let mut refresh_handle = self.refresh_handle.write().await {
            if let Some(handle) = refresh_handle.take() {
                handle.abort();
            }
        }
    }

    /// Start background cache refresh task
    fn start_background_refresh(&self) {
        let scan_cache = self.scan_cache.clone();
        let prefix_cache = self.prefix_cache.clone();
        let refresh_interval = self.config.cache_refresh_interval;
        let preload_paths = self.config.preload_paths.clone();

        let handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(refresh_interval));

            loop {
                interval.tick().await;

                // Refresh expired cache entries
                let mut expired_keys = Vec::new();
                for entry in scan_cache.iter() {
                    if !Self::is_cache_entry_valid_static(&entry.value()) {
                        expired_keys.push(entry.key().clone());
                    }
                }

                for key in expired_keys {
                    scan_cache.remove(&key);
                }

                // Refresh prefix cache for preload paths
                for path in &preload_paths {
                    if path.exists() && path.is_dir() {
                        // Simple refresh: just update the timestamp
                        if let Some(mut entry) = prefix_cache.get_mut(path) {
                            // This is a simplified refresh - in a real implementation,
                            // you might want to re-scan the directory
                            entry.clear();
                        }
                    }
                }
            }
        });

        // Store the handle for cleanup
        if let Ok(mut refresh_handle) = self.refresh_handle.try_write() {
            *refresh_handle = Some(handle);
        }
    }

    /// Scan a repository directory for packages with advanced optimizations
    pub async fn scan_repository(&self, root_path: &Path) -> Result<ScanResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        if !root_path.exists() {
            return Err(RezCoreError::Repository(format!(
                "Repository path does not exist: {}",
                root_path.display()
            )));
        }

        if !root_path.is_dir() {
            return Err(RezCoreError::Repository(format!(
                "Repository path is not a directory: {}",
                root_path.display()
            )));
        }

        // Reset metrics for this scan
        self.io_time.store(0, Ordering::Relaxed);
        self.parsing_time.store(0, Ordering::Relaxed);
        self.memory_mapped_files.store(0, Ordering::Relaxed);
        self.current_concurrency.store(0, Ordering::Relaxed);

        // Use concurrent-safe collections for better performance
        let packages = Arc::new(DashMap::new());
        let errors = Arc::new(DashMap::new());
        let directories_scanned = Arc::new(AtomicUsize::new(0));
        let files_examined = Arc::new(AtomicUsize::new(0));

        // Collect all directories first for batch processing
        let directories = self.collect_directories_recursive(root_path, 0).await?;

        // Process directories in batches for optimal concurrency
        let batch_size = self.config.directory_batch_size;
        let directory_batches: Vec<_> = directories.chunks(batch_size).collect();

        for batch in directory_batches {
            let batch_futures = batch.iter().map(|dir_path| {
                self.scan_directory_optimized(
                    dir_path,
                    packages.clone(),
                    errors.clone(),
                    directories_scanned.clone(),
                    files_examined.clone(),
                )
            });

            // Process batch concurrently
            let results: Vec<_> = futures::future::join_all(batch_futures).await;

            // Handle any errors from the batch
            for result in results {
                if let Err(e) = result {
                    let error_id = errors.len();
                    errors.insert(
                        error_id,
                        ScanError {
                            path: root_path.to_path_buf(),
                            message: format!("Batch processing error: {}", e),
                            error_type: ScanErrorType::Other,
                        },
                    );
                }
            }
        }

        let total_duration_ms = start_time.elapsed().as_millis() as u64;

        // Collect results
        let packages_vec: Vec<PackageScanResult> =
            packages.iter().map(|entry| entry.value().clone()).collect();

        let errors_vec: Vec<ScanError> = errors.iter().map(|entry| entry.value().clone()).collect();

        // Calculate performance metrics
        let total_files = files_examined.load(Ordering::Relaxed);
        let avg_file_size = if total_files > 0 {
            packages_vec.iter().map(|p| p.file_size).sum::<u64>() / total_files as u64
        } else {
            0
        };

        let performance_metrics = ScanPerformanceMetrics {
            io_time_ms: self.io_time.load(Ordering::Relaxed),
            parsing_time_ms: self.parsing_time.load(Ordering::Relaxed),
            memory_mapped_files: self.memory_mapped_files.load(Ordering::Relaxed),
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            avg_file_size,
            peak_memory_usage: 0, // TODO: Implement memory tracking
            peak_concurrency: self.peak_concurrency.load(Ordering::Relaxed),
        };

        Ok(ScanResult {
            packages: packages_vec,
            total_duration_ms,
            directories_scanned: directories_scanned.load(Ordering::Relaxed),
            files_examined: total_files,
            errors: errors_vec,
            performance_metrics,
        })
    }

    /// Collect all directories recursively for batch processing
    fn collect_directories_recursive<'a>(
        &'a self,
        root_path: &'a Path,
        depth: usize,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<PathBuf>, RezCoreError>> + Send + 'a>,
    > {
        Box::pin(async move {
            let mut directories = Vec::new();

            if depth > self.config.max_depth {
                return Ok(directories);
            }

            if self.should_exclude_path(root_path) {
                return Ok(directories);
            }

            directories.push(root_path.to_path_buf());

            let mut entries = fs::read_dir(root_path).await.map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to read directory {}: {}",
                    root_path.display(),
                    e
                ))
            })?;

            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
            })? {
                let path = entry.path();
                if path.is_dir() && !self.should_exclude_path(&path) {
                    let subdirs = self.collect_directories_recursive(&path, depth + 1).await?;
                    directories.extend(subdirs);
                }
            }

            Ok(directories)
        })
    }

    /// Optimized directory scanning with caching and memory mapping
    async fn scan_directory_optimized(
        &self,
        dir_path: &Path,
        packages: Arc<DashMap<PathBuf, PackageScanResult>>,
        errors: Arc<DashMap<usize, ScanError>>,
        directories_scanned: Arc<AtomicUsize>,
        files_examined: Arc<AtomicUsize>,
    ) -> Result<(), RezCoreError> {
        // Track concurrency
        let current = self.current_concurrency.fetch_add(1, Ordering::Relaxed) + 1;
        let peak = self.peak_concurrency.load(Ordering::Relaxed);
        if current > peak {
            self.peak_concurrency.store(current, Ordering::Relaxed);
        }

        // Increment directories scanned counter
        directories_scanned.fetch_add(1, Ordering::Relaxed);

        // Check if this directory should be excluded
        if self.should_exclude_path(dir_path) {
            self.current_concurrency.fetch_sub(1, Ordering::Relaxed);
            return Ok(());
        }

        // Read directory entries
        let mut entries = match fs::read_dir(dir_path).await {
            Ok(entries) => entries,
            Err(e) => {
                let error_id = errors.len();
                errors.insert(
                    error_id,
                    ScanError {
                        path: dir_path.to_path_buf(),
                        message: format!("Failed to read directory: {}", e),
                        error_type: ScanErrorType::FileSystemError,
                    },
                );
                self.current_concurrency.fetch_sub(1, Ordering::Relaxed);
                return Ok(());
            }
        };

        // Collect package files for batch processing
        let mut package_files = SmallVec::<[PathBuf; 8]>::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_file() && self.is_package_file(&path) {
                package_files.push(path);
            }
        }

        // Process package files concurrently with semaphore control
        let package_futures = package_files.into_iter().map(|package_file| {
            let semaphore = self.semaphore.clone();
            let packages_clone = packages.clone();
            let errors_clone = errors.clone();
            let files_examined_clone = files_examined.clone();
            let scanner = self;

            async move {
                let _permit = semaphore.acquire().await.unwrap();

                // Increment files examined counter
                files_examined_clone.fetch_add(1, Ordering::Relaxed);

                match scanner.scan_package_file_optimized(&package_file).await {
                    Ok(package_result) => {
                        packages_clone.insert(package_file.clone(), package_result);
                    }
                    Err(e) => {
                        let error_id = errors_clone.len();
                        errors_clone.insert(
                            error_id,
                            ScanError {
                                path: package_file,
                                message: format!("Failed to scan package: {}", e),
                                error_type: ScanErrorType::PackageParseError,
                            },
                        );
                    }
                }
            }
        });

        // Wait for all package files in this directory to be processed
        futures::future::join_all(package_futures).await;

        self.current_concurrency.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    /// Legacy recursive scan method (kept for compatibility)
    async fn scan_directory_recursive(
        &self,
        dir_path: &Path,
        depth: usize,
        join_set: &mut JoinSet<()>,
        packages: Arc<RwLock<Vec<PackageScanResult>>>,
        errors: Arc<RwLock<Vec<ScanError>>>,
        directories_scanned: Arc<RwLock<usize>>,
        files_examined: Arc<RwLock<usize>>,
    ) -> Result<(), RezCoreError> {
        // Check depth limit
        if depth > self.config.max_depth {
            return Ok(());
        }

        // Increment directories scanned counter
        {
            let mut count = directories_scanned.write().await;
            *count += 1;
        }

        // Check if this directory should be excluded
        if self.should_exclude_path(dir_path) {
            return Ok(());
        }

        // Read directory entries
        let mut entries = match fs::read_dir(dir_path).await {
            Ok(entries) => entries,
            Err(e) => {
                let mut errors_guard = errors.write().await;
                errors_guard.push(ScanError {
                    path: dir_path.to_path_buf(),
                    message: format!("Failed to read directory: {}", e),
                    error_type: ScanErrorType::FileSystemError,
                });
                return Ok(());
            }
        };

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectory
                Box::pin(self.scan_directory_recursive(
                    &path,
                    depth + 1,
                    join_set,
                    packages.clone(),
                    errors.clone(),
                    directories_scanned.clone(),
                    files_examined.clone(),
                ))
                .await?;
            } else if path.is_file() {
                // Check if this is a package file
                if self.is_package_file(&path) {
                    // Spawn a task to scan this package file
                    let semaphore = self.semaphore.clone();
                    let path_clone = path.clone();
                    let packages_clone = packages.clone();
                    let errors_clone = errors.clone();
                    let files_examined_clone = files_examined.clone();

                    join_set.spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();

                        // Increment files examined counter
                        {
                            let mut count = files_examined_clone.write().await;
                            *count += 1;
                        }

                        match Self::scan_package_file(&path_clone).await {
                            Ok(package_result) => {
                                let mut packages_guard = packages_clone.write().await;
                                packages_guard.push(package_result);
                            }
                            Err(e) => {
                                let mut errors_guard = errors_clone.write().await;
                                errors_guard.push(ScanError {
                                    path: path_clone,
                                    message: format!("Failed to scan package: {}", e),
                                    error_type: ScanErrorType::PackageParseError,
                                });
                            }
                        }
                    });
                }
            }
        }

        Ok(())
    }

    /// Optimized package file scanning with caching and memory mapping
    async fn scan_package_file_optimized(
        &self,
        package_file: &Path,
    ) -> Result<PackageScanResult, RezCoreError> {
        let start_time = std::time::Instant::now();
        let io_start = std::time::Instant::now();

        // Get file metadata
        let metadata = fs::metadata(package_file)
            .await
            .map_err(|e| RezCoreError::Repository(format!("Failed to get file metadata: {}", e)))?;

        let file_size = metadata.len();
        let mtime = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        // Check cache first if enabled
        if self.config.enable_scan_cache {
            if let Some(cached_entry) = self.scan_cache.get(package_file) {
                if cached_entry.mtime == mtime && cached_entry.size == file_size {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(cached_entry.result.clone());
                }
            }
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        // Determine package format using smart detection if enabled
        let _format = if self.config.smart_file_detection {
            self.detect_package_format_smart(package_file, file_size)
                .await?
        } else {
            "yaml".to_string()
        };

        // Read file content using memory mapping for large files
        let content =
            if self.config.use_memory_mapping && file_size > self.config.memory_mapping_threshold {
                self.read_file_memory_mapped(package_file).await?
            } else {
                fs::read_to_string(package_file).await.map_err(|e| {
                    RezCoreError::Repository(format!("Failed to read package file: {}", e))
                })?
            };

        let io_time = io_start.elapsed().as_millis() as u64;
        self.io_time.fetch_add(io_time, Ordering::Relaxed);

        // Parse package
        let parse_start = std::time::Instant::now();
        let package: Package = serde_yaml::from_str(&content).map_err(|e| {
            RezCoreError::Repository(format!("Failed to parse package file: {}", e))
        })?;
        let parse_time = parse_start.elapsed().as_millis() as u64;
        self.parsing_time.fetch_add(parse_time, Ordering::Relaxed);

        let scan_duration_ms = start_time.elapsed().as_millis() as u64;
        let package_dir = package_file.parent().unwrap_or(package_file).to_path_buf();

        let result = PackageScanResult {
            package,
            package_file: package_file.to_path_buf(),
            package_dir,
            file_size,
            scan_duration_ms,
        };

        // Cache the result if enabled
        if self.config.enable_scan_cache {
            let now = SystemTime::now();
            let cache_entry = ScanCacheEntry {
                result: result.clone(),
                mtime,
                size: file_size,
                cached_at: now,
                access_count: 1,
                last_accessed: now,
            };
            self.scan_cache
                .insert(package_file.to_path_buf(), cache_entry);

            // Limit cache size
            if self.scan_cache.len() > self.config.max_cache_size_mb * 1000 {
                // Simple cache eviction: remove oldest entries
                // TODO: Implement LRU eviction
                if self.scan_cache.len() > self.config.max_cache_size_mb * 1200 {
                    self.scan_cache.clear();
                }
            }
        }

        Ok(result)
    }

    /// Smart package format detection
    async fn detect_package_format_smart(
        &self,
        package_file: &Path,
        file_size: u64,
    ) -> Result<String, RezCoreError> {
        // First try extension-based detection
        if let Some(ext) = package_file.extension().and_then(|s| s.to_str()) {
            match ext {
                "yaml" | "yml" => return Ok("yaml".to_string()),
                "json" => return Ok("json".to_string()),
                "py" => return Ok("python".to_string()),
                _ => {}
            }
        }

        // For small files, read a sample to detect format
        if file_size < 1024 {
            let content = fs::read_to_string(package_file).await.map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to read package file for format detection: {}",
                    e
                ))
            })?;

            // Simple heuristic detection
            if content.trim_start().starts_with('{') {
                return Ok("json".to_string());
            } else if content.contains("name:") || content.contains("version:") {
                return Ok("yaml".to_string());
            } else if content.contains("name =") || content.contains("version =") {
                return Ok("python".to_string());
            }
        }

        // Fallback to yaml
        Ok("yaml".to_string())
    }

    /// Read file using memory mapping for better performance
    async fn read_file_memory_mapped(&self, package_file: &Path) -> Result<String, RezCoreError> {
        let file = std::fs::File::open(package_file).map_err(|e| {
            RezCoreError::Repository(format!("Failed to open file for memory mapping: {}", e))
        })?;

        let mmap = unsafe { Mmap::map(&file) }
            .map_err(|e| RezCoreError::Repository(format!("Failed to memory map file: {}", e)))?;

        self.memory_mapped_files.fetch_add(1, Ordering::Relaxed);

        String::from_utf8(mmap.to_vec()).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to convert memory mapped file to string: {}",
                e
            ))
        })
    }

    /// Legacy package file scanning method (kept for compatibility)
    async fn scan_package_file(package_file: &Path) -> Result<PackageScanResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        // Get file metadata
        let metadata = fs::metadata(package_file)
            .await
            .map_err(|e| RezCoreError::Repository(format!("Failed to get file metadata: {}", e)))?;

        let file_size = metadata.len();

        // Read and parse package file
        let content = fs::read_to_string(package_file)
            .await
            .map_err(|e| RezCoreError::Repository(format!("Failed to read package file: {}", e)))?;

        // Simple YAML parsing for now
        let package: Package = serde_yaml::from_str(&content).map_err(|e| {
            RezCoreError::Repository(format!("Failed to parse package file: {}", e))
        })?;

        let scan_duration_ms = start_time.elapsed().as_millis() as u64;
        let package_dir = package_file.parent().unwrap_or(package_file).to_path_buf();

        Ok(PackageScanResult {
            package,
            package_file: package_file.to_path_buf(),
            package_dir,
            file_size,
            scan_duration_ms,
        })
    }

    /// Check if a file is a package definition file
    fn is_package_file(&self, path: &Path) -> bool {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            self.config
                .include_patterns
                .iter()
                .any(|pattern| self.matches_pattern(filename, pattern))
        } else {
            false
        }
    }

    /// Check if a path should be excluded from scanning
    fn should_exclude_path(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();

        self.config
            .exclude_patterns
            .iter()
            .any(|pattern| self.matches_pattern(&path_str, pattern))
    }

    /// Normalize path for consistent cache key generation
    fn normalize_path(&self, path: &Path) -> PathBuf {
        // Convert to absolute path and normalize
        match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                // Fallback to simple normalization if canonicalize fails
                let mut normalized = PathBuf::new();
                for component in path.components() {
                    match component {
                        std::path::Component::ParentDir => {
                            normalized.pop();
                        }
                        std::path::Component::CurDir => {
                            // Skip current directory references
                        }
                        _ => {
                            normalized.push(component);
                        }
                    }
                }
                normalized
            }
        }
    }

    /// Check if a cache entry is still valid
    fn is_cache_entry_valid(&self, entry: &ScanCacheEntry) -> bool {
        Self::is_cache_entry_valid_static(entry)
    }

    /// Static version of cache entry validation
    fn is_cache_entry_valid_static(entry: &ScanCacheEntry) -> bool {
        // Check if the source file still exists and hasn't been modified
        if let Ok(metadata) = std::fs::metadata(&entry.result.package_file) {
            if let Ok(mtime) = metadata.modified() {
                return mtime == entry.mtime && metadata.len() == entry.size;
            }
        }
        false
    }

    /// Simple pattern matching (supports * and ? wildcards)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Convert glob pattern to regex
        let regex_pattern = pattern
            .replace("**", ".*") // ** matches any number of directories
            .replace("*", "[^/]*") // * matches anything except directory separator
            .replace("?", "."); // ? matches any single character

        if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(text)
        } else {
            // Fallback to exact match
            text == pattern
        }
    }
}

impl Default for RepositoryScanner {
    fn default() -> Self {
        Self::new(ScannerConfig::default())
    }
}
