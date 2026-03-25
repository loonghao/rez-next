//! Batch package operations for high-performance bulk processing

use crate::{
    cache::PackageCacheManager, Package, PackageInstallOptions, PackageManager,
    PackageOperationResult, PackageValidationOptions, PackageValidationResult, PackageValidator,
};
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use rayon::prelude::*;
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime};

/// Batch operation configuration
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of parallel workers
    pub max_workers: usize,
    /// Batch size for processing chunks
    pub batch_size: usize,
    /// Enable progress reporting
    pub progress_reporting: bool,
    /// Continue on individual failures
    pub continue_on_failure: bool,
    /// Maximum memory usage per worker
    pub max_memory_per_worker: usize,
    /// Timeout per operation in seconds
    pub operation_timeout: u64,
    /// Enable caching
    pub enable_caching: bool,
    /// Cache configuration
    pub cache_config: Option<crate::cache::CacheConfig>,
}

/// Batch operation progress
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProgress {
    /// Total items to process
    pub total_items: usize,
    /// Items completed
    pub completed_items: usize,
    /// Items failed
    pub failed_items: usize,
    /// Items in progress
    pub in_progress_items: usize,
    /// Progress percentage
    pub progress_percentage: f64,
    /// Estimated time remaining in seconds
    pub estimated_time_remaining: u64,
    /// Current operation
    pub current_operation: String,
}

/// Batch operation result
#[derive(Debug, Clone)]
pub struct BatchOperationResult<T> {
    /// Whether the batch operation was successful overall
    pub success: bool,
    /// Individual results
    pub results: Vec<Result<T, RezCoreError>>,
    /// Failed items with their errors
    pub failures: HashMap<String, RezCoreError>,
    /// Operation statistics
    pub statistics: BatchStatistics,
    /// Progress information
    pub progress: BatchProgress,
}

/// Batch operation statistics
#[derive(Debug, Clone, Default)]
pub struct BatchStatistics {
    /// Total processing time in milliseconds
    pub total_time_ms: u64,
    /// Average time per item in milliseconds
    pub avg_time_per_item_ms: u64,
    /// Peak memory usage in bytes
    pub peak_memory_usage: usize,
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    /// Parallel efficiency (actual speedup / theoretical speedup)
    pub parallel_efficiency: f64,
    /// Items processed per second
    pub throughput_items_per_second: f64,
}

/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(&BatchProgress) + Send + Sync>;

/// Batch package processor
#[cfg_attr(feature = "python-bindings", pyclass)]
pub struct BatchPackageProcessor {
    /// Configuration
    config: BatchConfig,
    /// Cache manager
    cache_manager: Option<PackageCacheManager>,
    /// Progress callback
    progress_callback: Option<ProgressCallback>,
    /// Current progress
    progress: Arc<Mutex<BatchProgress>>,
}

/// Batch parsing options
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone)]
pub struct BatchParseOptions {
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Recursive directory scanning
    pub recursive: bool,
    /// Follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum file size to process
    pub max_file_size: usize,
    /// Validate packages after parsing
    pub validate_after_parse: bool,
}

/// Batch validation options
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone)]
pub struct BatchValidationOptions {
    /// Validation options for each package
    pub validation_options: PackageValidationOptions,
    /// Stop on first validation failure
    pub fail_fast: bool,
    /// Generate detailed validation report
    pub detailed_report: bool,
    /// Group validation results by status
    pub group_by_status: bool,
}

/// Batch installation options
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug, Clone)]
pub struct BatchInstallOptions {
    /// Installation options for each package
    pub install_options: PackageInstallOptions,
    /// Target installation directory
    pub target_directory: PathBuf,
    /// Resolve dependencies before installation
    pub resolve_dependencies: bool,
    /// Install in dependency order
    pub dependency_order: bool,
    /// Skip already installed packages
    pub skip_existing: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_workers: num_cpus::get(),
            batch_size: 100,
            progress_reporting: true,
            continue_on_failure: true,
            max_memory_per_worker: 100 * 1024 * 1024, // 100MB
            operation_timeout: 300,                   // 5 minutes
            enable_caching: true,
            cache_config: None,
        }
    }
}

#[cfg_attr(feature = "python-bindings", pymethods)]
impl BatchConfig {
    /// Create new batch configuration
    #[cfg_attr(feature = "python-bindings", new)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create configuration optimized for development
    #[cfg_attr(feature = "python-bindings", staticmethod)]
    pub fn development() -> Self {
        Self {
            max_workers: 2,
            batch_size: 50,
            progress_reporting: true,
            continue_on_failure: true,
            max_memory_per_worker: 50 * 1024 * 1024, // 50MB
            operation_timeout: 120,                  // 2 minutes
            enable_caching: true,
            cache_config: Some(crate::cache::CacheConfig::development()),
        }
    }

    /// Create configuration optimized for production
    pub fn production() -> Self {
        Self {
            max_workers: num_cpus::get() * 2,
            batch_size: 500,
            progress_reporting: false,
            continue_on_failure: true,
            max_memory_per_worker: 200 * 1024 * 1024, // 200MB
            operation_timeout: 600,                   // 10 minutes
            enable_caching: true,
            cache_config: Some(crate::cache::CacheConfig::production()),
        }
    }

    /// Set maximum workers
    pub fn with_max_workers(mut self, max_workers: usize) -> Self {
        self.max_workers = max_workers;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Enable/disable progress reporting
    pub fn with_progress_reporting(mut self, enabled: bool) -> Self {
        self.progress_reporting = enabled;
        self
    }

    /// Set operation timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.operation_timeout = timeout_seconds;
        self
    }
}

impl Default for BatchParseOptions {
    fn default() -> Self {
        Self {
            include_patterns: vec![
                "*.py".to_string(),
                "*.yaml".to_string(),
                "*.json".to_string(),
            ],
            exclude_patterns: vec![".*".to_string(), "__pycache__".to_string()],
            recursive: true,
            follow_symlinks: false,
            max_file_size: 10 * 1024 * 1024, // 10MB
            validate_after_parse: false,
        }
    }
}

impl Default for BatchValidationOptions {
    fn default() -> Self {
        Self {
            validation_options: PackageValidationOptions::new(),
            fail_fast: false,
            detailed_report: true,
            group_by_status: true,
        }
    }
}

impl BatchProgress {
    /// Create new progress tracker
    pub fn new(total_items: usize) -> Self {
        Self {
            total_items,
            completed_items: 0,
            failed_items: 0,
            in_progress_items: 0,
            progress_percentage: 0.0,
            estimated_time_remaining: 0,
            current_operation: "Starting...".to_string(),
        }
    }

    /// Update progress
    pub fn update(
        &mut self,
        completed: usize,
        failed: usize,
        in_progress: usize,
        operation: String,
    ) {
        self.completed_items = completed;
        self.failed_items = failed;
        self.in_progress_items = in_progress;
        self.current_operation = operation;

        let total_processed = completed + failed;
        if self.total_items > 0 {
            self.progress_percentage = (total_processed as f64 / self.total_items as f64) * 100.0;
        }
    }

    /// Check if operation is complete
    pub fn is_complete(&self) -> bool {
        self.completed_items + self.failed_items >= self.total_items
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total_processed = self.completed_items + self.failed_items;
        if total_processed > 0 {
            self.completed_items as f64 / total_processed as f64
        } else {
            0.0
        }
    }
}

impl<T> BatchOperationResult<T> {
    /// Create new batch result
    pub fn new(total_items: usize) -> Self {
        Self {
            success: true,
            results: Vec::with_capacity(total_items),
            failures: HashMap::new(),
            statistics: BatchStatistics::default(),
            progress: BatchProgress::new(total_items),
        }
    }

    /// Add successful result
    pub fn add_success(&mut self, result: T) {
        self.results.push(Ok(result));
    }

    /// Add failure result
    pub fn add_failure(&mut self, item_id: String, error: RezCoreError) {
        self.results.push(Err(error.clone()));
        self.failures.insert(item_id, error);
        self.success = false;
    }

    /// Get successful results
    pub fn get_successes(&self) -> Vec<&T> {
        self.results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .collect()
    }

    /// Get failure count
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    /// Get success count
    pub fn success_count(&self) -> usize {
        self.results.len() - self.failure_count()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.results.is_empty() {
            0.0
        } else {
            self.success_count() as f64 / self.results.len() as f64
        }
    }
}

#[cfg_attr(feature = "python-bindings", pymethods)]
impl BatchPackageProcessor {
    /// Create new batch processor
    #[cfg_attr(feature = "python-bindings", new)]
    pub fn new(config: BatchConfig) -> Self {
        let cache_manager = if config.enable_caching {
            Some(PackageCacheManager::new(
                config.cache_config.clone().unwrap_or_default(),
            ))
        } else {
            None
        };

        Self {
            config,
            cache_manager,
            progress_callback: None,
            progress: Arc::new(Mutex::new(BatchProgress::new(0))),
        }
    }

    /// Set progress callback
    pub fn set_progress_callback(&mut self, callback: ProgressCallback) {
        self.progress_callback = Some(callback);
    }

    /// Batch parse packages from files
    pub fn batch_parse_files(
        &self,
        file_paths: Vec<PathBuf>,
        options: BatchParseOptions,
    ) -> BatchOperationResult<Package> {
        let start_time = Instant::now();
        let mut result = BatchOperationResult::new(file_paths.len());

        // Update progress
        {
            let mut progress = self.progress.lock().unwrap();
            *progress = BatchProgress::new(file_paths.len());
            progress.current_operation = "Starting batch parse...".to_string();
        }

        // Configure thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.max_workers)
            .build()
            .unwrap();

        // Process files in parallel
        let results: Vec<_> = pool.install(|| {
            file_paths
                .par_iter()
                .enumerate()
                .map(|(index, path)| self.parse_single_file(path, &options, index))
                .collect()
        });

        // Collect results
        let mut completed = 0;
        let mut failed = 0;

        for (index, parse_result) in results.into_iter().enumerate() {
            match parse_result {
                Ok(package) => {
                    result.add_success(package);
                    completed += 1;
                }
                Err(error) => {
                    let file_path = file_paths[index].to_string_lossy().to_string();
                    result.add_failure(file_path, error);
                    failed += 1;
                }
            }

            // Update progress
            if self.config.progress_reporting {
                let mut progress = self.progress.lock().unwrap();
                progress.update(
                    completed,
                    failed,
                    0,
                    format!("Parsed {}/{} files", completed + failed, file_paths.len()),
                );

                if let Some(ref callback) = self.progress_callback {
                    callback(&progress);
                }
            }
        }

        // Update statistics
        result.statistics.total_time_ms = start_time.elapsed().as_millis() as u64;
        if !file_paths.is_empty() {
            result.statistics.avg_time_per_item_ms =
                result.statistics.total_time_ms / file_paths.len() as u64;
            result.statistics.throughput_items_per_second =
                file_paths.len() as f64 / (result.statistics.total_time_ms as f64 / 1000.0);
        }

        // Update cache statistics if available
        if let Some(ref cache_manager) = self.cache_manager {
            result.statistics.cache_hit_ratio = cache_manager.get_overall_hit_ratio();
        }

        result
    }

    /// Batch validate packages
    pub fn batch_validate_packages(
        &self,
        packages: Vec<Package>,
        options: BatchValidationOptions,
    ) -> BatchOperationResult<PackageValidationResult> {
        let start_time = Instant::now();
        let mut result = BatchOperationResult::new(packages.len());

        // Update progress
        {
            let mut progress = self.progress.lock().unwrap();
            *progress = BatchProgress::new(packages.len());
            progress.current_operation = "Starting batch validation...".to_string();
        }

        // Create validator
        let validator = PackageValidator::new(Some(options.validation_options.clone()));

        // Configure thread pool
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.config.max_workers)
            .build()
            .unwrap();

        // Process packages in parallel
        let results: Vec<_> = pool.install(|| {
            packages
                .par_iter()
                .enumerate()
                .map(|(index, package)| self.validate_single_package(package, &validator, index))
                .collect()
        });

        // Collect results
        let mut completed = 0;
        let mut failed = 0;

        for (index, validation_result) in results.into_iter().enumerate() {
            match validation_result {
                Ok(validation) => {
                    if validation.is_valid || !options.fail_fast {
                        result.add_success(validation);
                        completed += 1;
                    } else {
                        let package_name = packages[index].name.clone();
                        let error = RezCoreError::PackageValidation(format!(
                            "Validation failed for package: {}",
                            package_name
                        ));
                        result.add_failure(package_name, error);
                        failed += 1;

                        if options.fail_fast {
                            break;
                        }
                    }
                }
                Err(error) => {
                    let package_name = packages[index].name.clone();
                    result.add_failure(package_name, error);
                    failed += 1;

                    if options.fail_fast {
                        break;
                    }
                }
            }

            // Update progress
            if self.config.progress_reporting {
                let mut progress = self.progress.lock().unwrap();
                progress.update(
                    completed,
                    failed,
                    0,
                    format!(
                        "Validated {}/{} packages",
                        completed + failed,
                        packages.len()
                    ),
                );

                if let Some(ref callback) = self.progress_callback {
                    callback(&progress);
                }
            }
        }

        // Update statistics
        result.statistics.total_time_ms = start_time.elapsed().as_millis() as u64;
        if !packages.is_empty() {
            result.statistics.avg_time_per_item_ms =
                result.statistics.total_time_ms / packages.len() as u64;
            result.statistics.throughput_items_per_second =
                packages.len() as f64 / (result.statistics.total_time_ms as f64 / 1000.0);
        }

        result
    }

    /// Parse a single file
    fn parse_single_file(
        &self,
        path: &Path,
        _options: &BatchParseOptions,
        _index: usize,
    ) -> Result<Package, RezCoreError> {
        // Check cache first
        if let Some(ref cache_manager) = self.cache_manager {
            let cache_key = cache_manager.parse_cache.generate_key(path, None);
            if let Some(cached_package) = cache_manager.parse_cache.get(&cache_key) {
                return Ok(cached_package);
            }
        }

        // Parse package
        let package = crate::PackageSerializer::load_from_file(path)?;

        // Cache result
        if let Some(ref cache_manager) = self.cache_manager {
            let cache_key = cache_manager.parse_cache.generate_key(path, None);
            cache_manager.parse_cache.put(cache_key, package.clone());
        }

        Ok(package)
    }

    /// Validate a single package
    fn validate_single_package(
        &self,
        package: &Package,
        validator: &PackageValidator,
        _index: usize,
    ) -> Result<PackageValidationResult, RezCoreError> {
        // Check cache first
        if let Some(ref cache_manager) = self.cache_manager {
            let package_hash = cache_manager.generate_package_hash(package);
            let options_hash = cache_manager.generate_validation_options_hash("default");
            let cache_key = cache_manager
                .validation_cache
                .generate_key(&package_hash, &options_hash);

            if let Some(cached_result) = cache_manager.validation_cache.get(&cache_key) {
                return Ok(cached_result);
            }
        }

        // Validate package
        let validation_result = validator.validate_package(package)?;

        // Cache result
        if let Some(ref cache_manager) = self.cache_manager {
            let package_hash = cache_manager.generate_package_hash(package);
            let options_hash = cache_manager.generate_validation_options_hash("default");
            let cache_key = cache_manager
                .validation_cache
                .generate_key(&package_hash, &options_hash);
            cache_manager
                .validation_cache
                .put(cache_key, validation_result.clone());
        }

        Ok(validation_result)
    }

    /// Get current progress
    pub fn get_progress(&self) -> BatchProgress {
        self.progress.lock().unwrap().clone()
    }

    /// Clear cache if available
    pub fn clear_cache(&self) {
        if let Some(ref cache_manager) = self.cache_manager {
            cache_manager.clear_all();
        }
    }

    /// Get cache statistics
    pub fn get_cache_statistics(&self) -> Option<HashMap<String, crate::cache::CacheStatistics>> {
        self.cache_manager
            .as_ref()
            .map(|cm| cm.get_combined_statistics())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;

    fn create_test_package(name: &str, version: &str) -> Package {
        let mut package = Package::new(name.to_string());
        package.version = Some(Version::parse(version).unwrap());
        package.description = Some(format!("Test package {}", name));
        package
    }

    #[test]
    fn test_batch_config() {
        let config = BatchConfig::new();
        assert_eq!(config.max_workers, num_cpus::get());
        assert_eq!(config.batch_size, 100);
        assert!(config.progress_reporting);
        assert!(config.continue_on_failure);

        let dev_config = BatchConfig::development();
        assert_eq!(dev_config.max_workers, 2);
        assert_eq!(dev_config.batch_size, 50);

        let prod_config = BatchConfig::production();
        assert_eq!(prod_config.max_workers, num_cpus::get() * 2);
        assert_eq!(prod_config.batch_size, 500);
    }

    #[test]
    fn test_batch_progress() {
        let mut progress = BatchProgress::new(100);
        assert_eq!(progress.total_items, 100);
        assert_eq!(progress.completed_items, 0);
        assert_eq!(progress.progress_percentage, 0.0);
        assert!(!progress.is_complete());

        progress.update(50, 5, 10, "Processing...".to_string());
        assert_eq!(progress.completed_items, 50);
        assert_eq!(progress.failed_items, 5);
        assert_eq!(progress.in_progress_items, 10);
        assert_eq!(progress.progress_percentage, 55.0);
        assert!(!progress.is_complete());

        progress.update(90, 10, 0, "Almost done...".to_string());
        assert!(progress.is_complete());
        assert_eq!(progress.success_rate(), 0.9);
    }

    #[test]
    fn test_batch_operation_result() {
        let mut result: BatchOperationResult<Package> = BatchOperationResult::new(3);
        assert!(result.success);
        assert_eq!(result.results.len(), 0);
        assert_eq!(result.failure_count(), 0);

        let package1 = create_test_package("pkg1", "1.0.0");
        let package2 = create_test_package("pkg2", "2.0.0");

        result.add_success(package1);
        result.add_success(package2);
        result.add_failure(
            "pkg3".to_string(),
            RezCoreError::PackageParse("Test error".to_string()),
        );

        assert!(!result.success);
        assert_eq!(result.success_count(), 2);
        assert_eq!(result.failure_count(), 1);
        assert_eq!(result.success_rate(), 2.0 / 3.0);

        let successes = result.get_successes();
        assert_eq!(successes.len(), 2);
    }

    #[test]
    fn test_batch_processor_creation() {
        let config = BatchConfig::development();
        let processor = BatchPackageProcessor::new(config);

        assert!(processor.cache_manager.is_some());
        assert!(processor.progress_callback.is_none());

        let progress = processor.get_progress();
        assert_eq!(progress.total_items, 0);
    }

    #[test]
    fn test_batch_parse_options() {
        let options = BatchParseOptions::default();
        assert!(options.include_patterns.contains(&"*.py".to_string()));
        assert!(options.exclude_patterns.contains(&".*".to_string()));
        assert!(options.recursive);
        assert!(!options.follow_symlinks);
        assert_eq!(options.max_file_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_batch_validation_options() {
        let options = BatchValidationOptions::default();
        assert!(!options.fail_fast);
        assert!(options.detailed_report);
        assert!(options.group_by_status);
    }
}
