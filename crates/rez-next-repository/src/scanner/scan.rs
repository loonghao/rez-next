//! Core scanning logic for the repository scanner.
//!
//! This module contains:
//! - `scan_repository` — entry-point for scanning a full repository tree.
//! - `collect_directories_recursive` — async recursive directory collector.
//! - `scan_directory_optimized` — per-directory concurrent file scanner.
//! - `scan_package_file_optimized` — single-file scan with caching + mmap.
//! - `detect_package_format_smart` — extension + content heuristic detection.
//! - `read_file_memory_mapped` — mmap-based file reader.

use super::RepositoryScanner;
use crate::scanner_types::{
    PackageScanResult, ScanCacheEntry, ScanError, ScanErrorType, ScanPerformanceMetrics, ScanResult,
};
use dashmap::DashMap;
use memmap2::Mmap;
use rez_next_common::RezCoreError;
use rez_next_package::Package;
use smallvec::SmallVec;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::fs;

impl RepositoryScanner {
    /// Scan an entire repository tree for package definition files.
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

        // Reset per-scan metrics
        self.io_time.store(0, Ordering::Relaxed);
        self.parsing_time.store(0, Ordering::Relaxed);
        self.memory_mapped_files.store(0, Ordering::Relaxed);
        self.current_concurrency.store(0, Ordering::Relaxed);

        let packages: Arc<DashMap<PathBuf, PackageScanResult>> = Arc::new(DashMap::new());
        let errors: Arc<DashMap<usize, ScanError>> = Arc::new(DashMap::new());
        let directories_scanned = Arc::new(AtomicUsize::new(0));
        let files_examined = Arc::new(AtomicUsize::new(0));

        let directories = self.collect_directories_recursive(root_path, 0).await?;

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

            let results: Vec<_> = futures::future::join_all(batch_futures).await;

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
        let packages_vec: Vec<PackageScanResult> =
            packages.iter().map(|entry| entry.value().clone()).collect();
        let errors_vec: Vec<ScanError> = errors.iter().map(|entry| entry.value().clone()).collect();

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
            peak_memory_usage: self.peak_memory_bytes.load(Ordering::Relaxed),
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

    /// Recursively collect all directories under `root_path` up to `config.max_depth`.
    pub(super) fn collect_directories_recursive<'a>(
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

    /// Scan a single directory for package files, respecting concurrency limits.
    pub(super) async fn scan_directory_optimized(
        &self,
        dir_path: &Path,
        packages: Arc<DashMap<PathBuf, PackageScanResult>>,
        errors: Arc<DashMap<usize, ScanError>>,
        directories_scanned: Arc<AtomicUsize>,
        files_examined: Arc<AtomicUsize>,
    ) -> Result<(), RezCoreError> {
        let current = self.current_concurrency.fetch_add(1, Ordering::Relaxed) + 1;
        let peak = self.peak_concurrency.load(Ordering::Relaxed);
        if current > peak {
            self.peak_concurrency.store(current, Ordering::Relaxed);
        }

        directories_scanned.fetch_add(1, Ordering::Relaxed);

        if self.should_exclude_path(dir_path) {
            self.current_concurrency.fetch_sub(1, Ordering::Relaxed);
            return Ok(());
        }

        let mut entries = match fs::read_dir(dir_path).await {
            Ok(e) => e,
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

        let mut package_files = SmallVec::<[PathBuf; 8]>::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_file() && self.is_package_file(&path) {
                package_files.push(path);
            }
        }

        let package_futures = package_files.into_iter().map(|package_file| {
            let semaphore = self.semaphore.clone();
            let packages_clone = packages.clone();
            let errors_clone = errors.clone();
            let files_examined_clone = files_examined.clone();
            let scanner = self;

            async move {
                let _permit = semaphore.acquire().await.unwrap();
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

        futures::future::join_all(package_futures).await;

        self.current_concurrency.fetch_sub(1, Ordering::Relaxed);
        Ok(())
    }

    /// Scan a single package file with caching and optional memory-mapping.
    pub(super) async fn scan_package_file_optimized(
        &self,
        package_file: &Path,
    ) -> Result<PackageScanResult, RezCoreError> {
        let start_time = std::time::Instant::now();
        let io_start = std::time::Instant::now();

        let metadata = fs::metadata(package_file)
            .await
            .map_err(|e| RezCoreError::Repository(format!("Failed to get file metadata: {}", e)))?;

        let file_size = metadata.len();
        let mtime = metadata
            .modified()
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        // Cache hit check
        if self.config.enable_scan_cache {
            if let Some(cached_entry) = self.scan_cache.get(package_file) {
                if cached_entry.mtime == mtime && cached_entry.size == file_size {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(cached_entry.result.clone());
                }
            }
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }

        let _format = if self.config.smart_file_detection {
            self.detect_package_format_smart(package_file, file_size)
                .await?
        } else {
            "yaml".to_string()
        };

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

        let content_bytes = content.len() as u64;
        let prev = self.peak_memory_bytes.load(Ordering::Relaxed);
        if content_bytes > prev {
            self.peak_memory_bytes
                .store(content_bytes, Ordering::Relaxed);
        }

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

        // Store in cache with LRU eviction
        if self.config.enable_scan_cache {
            let now = SystemTime::now();
            let cache_entry = ScanCacheEntry {
                result: result.clone(),
                mtime,
                size: file_size,
                access_count: 1,
                last_accessed: now,
            };
            self.scan_cache
                .insert(package_file.to_path_buf(), cache_entry);

            let max_entries = self.config.max_cache_size_mb * 1000;
            if self.scan_cache.len() > max_entries {
                let mut entries: Vec<(PathBuf, SystemTime)> = self
                    .scan_cache
                    .iter()
                    .map(|r| (r.key().clone(), r.value().last_accessed))
                    .collect();
                entries.sort_by_key(|(_, ts)| *ts);
                let target = (max_entries as f64 * 0.8) as usize;
                let to_remove = entries.len().saturating_sub(target);
                for (path, _) in entries.into_iter().take(to_remove) {
                    self.scan_cache.remove(&path);
                }
            }
        }

        Ok(result)
    }

    /// Detect the package format using file extension then content heuristics.
    pub(super) async fn detect_package_format_smart(
        &self,
        package_file: &Path,
        file_size: u64,
    ) -> Result<String, RezCoreError> {
        if let Some(ext) = package_file.extension().and_then(|s| s.to_str()) {
            match ext {
                "yaml" | "yml" => return Ok("yaml".to_string()),
                "json" => return Ok("json".to_string()),
                "py" => return Ok("python".to_string()),
                _ => {}
            }
        }

        if file_size < 1024 {
            let content = fs::read_to_string(package_file).await.map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to read package file for format detection: {}",
                    e
                ))
            })?;

            if content.trim_start().starts_with('{') {
                return Ok("json".to_string());
            } else if content.contains("name:") || content.contains("version:") {
                return Ok("yaml".to_string());
            } else if content.contains("name =") || content.contains("version =") {
                return Ok("python".to_string());
            }
        }

        Ok("yaml".to_string())
    }

    /// Read a file using memory-mapping for large-file performance.
    pub(super) async fn read_file_memory_mapped(
        &self,
        package_file: &Path,
    ) -> Result<String, RezCoreError> {
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
}
