//! Data types for the repository scanner.

use rez_next_package::Package;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

/// Canonical set of rez package definition filenames.
///
/// Both `RepositoryScanner` and `HighPerformanceScanner` use this list as the
/// single source of truth so the two scanners always stay in sync.
pub const REZ_PACKAGE_FILENAMES: &[&str] = &[
    "package.py",
    "package.yaml",
    "package.yml",
    "package.json",
];

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
            max_concurrent_scans: 20,
            max_depth: 10,
            include_patterns: REZ_PACKAGE_FILENAMES
                .iter()
                .map(|s| s.to_string())
                .collect(),
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
            timeout_seconds: 300,
            use_memory_mapping: true,
            memory_mapping_threshold: 1024 * 1024, // 1 MB
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
            cache_refresh_interval: 300,
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

/// Cache entry for scan results (internal use only)
#[derive(Debug, Clone)]
pub(crate) struct ScanCacheEntry {
    /// Cached package scan result
    pub(crate) result: PackageScanResult,
    /// File modification time when cached
    pub(crate) mtime: SystemTime,
    /// File size when cached
    pub(crate) size: u64,
    /// Access count for LRU tracking
    pub(crate) access_count: u64,
    /// Last access time
    pub(crate) last_accessed: SystemTime,
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
