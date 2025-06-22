//! Filesystem-based repository implementation

use crate::{Repository, RepositoryMetadata, RepositoryType, RepositoryStats, PackageSearchCriteria};
use rez_core_common::RezCoreError;
use rez_core_package::Package;
use rez_core_version::Version;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Filesystem repository implementation
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct FileSystemRepository {
    /// Repository metadata
    metadata: RepositoryMetadata,
    /// Package cache (name -> versions -> package)
    package_cache: Arc<RwLock<HashMap<String, HashMap<String, Package>>>>,
    /// Variant cache (package_key -> variant names)
    variant_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Repository statistics
    stats: Arc<RwLock<RepositoryStats>>,
    /// Initialization status
    initialized: Arc<RwLock<bool>>,
}

#[cfg_attr(feature = "python-bindings", pymethods)]
impl FileSystemRepository {
    #[cfg_attr(feature = "python-bindings", new)]
    pub fn new(path: PathBuf, name: Option<String>) -> Self {
        let repo_name = name.unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("filesystem_repo")
                .to_string()
        });

        let metadata = RepositoryMetadata {
            name: repo_name,
            path,
            repository_type: RepositoryType::FileSystem,
            priority: 0,
            read_only: false,
            description: Some("Filesystem-based package repository".to_string()),
            config: HashMap::new(),
        };

        Self {
            metadata,
            package_cache: Arc::new(RwLock::new(HashMap::new())),
            variant_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RepositoryStats::default())),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Get the repository path
    #[cfg_attr(feature = "python-bindings", getter)]
    pub fn path(&self) -> String {
        self.metadata.path.to_string_lossy().to_string()
    }

    /// Get the repository name
    #[cfg_attr(feature = "python-bindings", getter)]
    pub fn name(&self) -> String {
        self.metadata.name.clone()
    }

    /// Check if the repository is read-only
    #[cfg_attr(feature = "python-bindings", getter)]
    pub fn read_only(&self) -> bool {
        self.metadata.read_only
    }

    /// Set the repository priority
    pub fn set_priority(&mut self, priority: i32) {
        self.metadata.priority = priority;
    }

    /// Set the repository as read-only
    pub fn set_read_only(&mut self, read_only: bool) {
        self.metadata.read_only = read_only;
    }
}

#[async_trait::async_trait]
impl Repository for FileSystemRepository {
    fn metadata(&self) -> &RepositoryMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> Result<(), RezCoreError> {
        let start_time = std::time::Instant::now();
        
        // Check if repository path exists
        if !self.metadata.path.exists() {
            return Err(RezCoreError::Repository(
                format!("Repository path does not exist: {}", self.metadata.path.display())
            ));
        }

        // Clear existing cache
        {
            let mut package_cache = self.package_cache.write().await;
            let mut variant_cache = self.variant_cache.write().await;
            package_cache.clear();
            variant_cache.clear();
        }

        // Scan for packages
        let scan_result = self.scan_packages().await?;
        
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.package_count = scan_result.package_count;
            stats.version_count = scan_result.version_count;
            stats.variant_count = scan_result.variant_count;
            stats.size_bytes = scan_result.size_bytes;
            stats.last_scan_time = Some(chrono::Utc::now().timestamp());
            stats.last_scan_duration_ms = Some(start_time.elapsed().as_millis() as u64);
        }

        // Mark as initialized
        {
            let mut initialized = self.initialized.write().await;
            *initialized = true;
        }

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        // This is a sync method, so we can't use async read
        // In practice, you might want to use a sync primitive or return a future
        false // TODO: Implement proper sync check
    }

    async fn refresh(&mut self) -> Result<(), RezCoreError> {
        self.initialize().await
    }

    async fn find_packages(&self, criteria: &PackageSearchCriteria) -> Result<Vec<Package>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        let mut results = Vec::new();

        for (package_name, versions) in package_cache.iter() {
            // Check name pattern
            if let Some(ref pattern) = criteria.name_pattern {
                if !self.matches_pattern(package_name, pattern) {
                    continue;
                }
            }

            for (version_str, package) in versions.iter() {
                // Check version range
                if let Some(ref requirement) = criteria.version_requirement {
                    if let Some(ref version) = package.version {
                        // Simple string matching for now
                        if !requirement.is_empty() && version.as_str() != requirement {
                            continue;
                        }
                    }
                }

                // Check prerelease filter
                if !criteria.include_prerelease {
                    if let Some(ref version) = package.version {
                        if version.is_prerelease() {
                            continue;
                        }
                    }
                }

                // Check requirements (simplified)
                let mut satisfies_requirements = true;
                for req in &criteria.requirements {
                    if req.name == package.name {
                        if let Some(ref version) = package.version {
                            if !req.satisfied_by(version) {
                                satisfies_requirements = false;
                                break;
                            }
                        }
                    }
                }

                if satisfies_requirements {
                    results.push(package.clone());
                }

                // Check limit
                if let Some(limit) = criteria.limit {
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_package(&self, name: &str, version: Option<&Version>) -> Result<Option<Package>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        
        if let Some(versions) = package_cache.get(name) {
            match version {
                Some(v) => Ok(versions.get(v.as_str()).cloned()),
                None => {
                    // Return the latest version
                    let mut version_packages: Vec<_> = versions.values().collect();
                    version_packages.sort_by(|a, b| {
                        match (&a.version, &b.version) {
                            (Some(v1), Some(v2)) => v2.cmp(v1), // Descending order
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => std::cmp::Ordering::Equal,
                        }
                    });
                    Ok(version_packages.first().map(|p| (*p).clone()))
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn get_package_versions(&self, name: &str) -> Result<Vec<Version>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        
        if let Some(versions) = package_cache.get(name) {
            let mut version_list: Vec<Version> = versions.values()
                .filter_map(|p| p.version.clone())
                .collect();
            
            version_list.sort_by(|a, b| b.cmp(a)); // Descending order
            Ok(version_list)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_package_variants(&self, name: &str, version: Option<&Version>) -> Result<Vec<String>, RezCoreError> {
        let variant_cache = self.variant_cache.read().await;
        
        let key = match version {
            Some(v) => format!("{}-{}", name, v.as_str()),
            None => name.to_string(),
        };
        
        Ok(variant_cache.get(&key).cloned().unwrap_or_default())
    }

    async fn package_exists(&self, name: &str, version: Option<&Version>) -> Result<bool, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        
        if let Some(versions) = package_cache.get(name) {
            match version {
                Some(v) => Ok(versions.contains_key(v.as_str())),
                None => Ok(!versions.is_empty()),
            }
        } else {
            Ok(false)
        }
    }

    async fn get_package_names(&self) -> Result<Vec<String>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        Ok(package_cache.keys().cloned().collect())
    }

    async fn get_stats(&self) -> Result<RepositoryStats, RezCoreError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

/// Scan result structure
#[derive(Debug)]
struct ScanResult {
    package_count: usize,
    version_count: usize,
    variant_count: usize,
    size_bytes: u64,
}

impl FileSystemRepository {
    /// Scan the repository for packages
    async fn scan_packages(&self) -> Result<ScanResult, RezCoreError> {
        let mut package_count = 0;
        let mut version_count = 0;
        let mut variant_count = 0;
        let mut size_bytes = 0;

        let mut package_cache = self.package_cache.write().await;
        let mut variant_cache = self.variant_cache.write().await;

        // Walk through the repository directory
        let mut entries = fs::read_dir(&self.metadata.path).await
            .map_err(|e| RezCoreError::Repository(
                format!("Failed to read repository directory: {}", e)
            ))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| RezCoreError::Repository(
                format!("Failed to read directory entry: {}", e)
            ))? {
            
            let path = entry.path();
            if path.is_dir() {
                // This might be a package directory
                if let Ok(scan_result) = self.scan_package_directory(&path, &mut package_cache, &mut variant_cache).await {
                    package_count += scan_result.packages_found;
                    version_count += scan_result.versions_found;
                    variant_count += scan_result.variants_found;
                    size_bytes += scan_result.size_bytes;
                }
            }
        }

        Ok(ScanResult {
            package_count,
            version_count,
            variant_count,
            size_bytes,
        })
    }

    /// Scan a single package directory
    async fn scan_package_directory(
        &self,
        path: &Path,
        package_cache: &mut HashMap<String, HashMap<String, Package>>,
        variant_cache: &mut HashMap<String, Vec<String>>,
    ) -> Result<PackageScanResult, RezCoreError> {
        let mut packages_found = 0;
        let mut versions_found = 0;
        let mut variants_found = 0;
        let mut size_bytes = 0;

        // Look for package definition files
        let package_files = [
            path.join("package.py"),
            path.join("package.yaml"),
            path.join("package.yml"),
            path.join("package.json"),
        ];

        for package_file in &package_files {
            if package_file.exists() {
                match self.load_package_from_file(package_file).await {
                    Ok(package) => {
                        let package_name = package.name.clone();
                        let version_str = package.version.as_ref()
                            .map(|v| v.as_str())
                            .unwrap_or("latest")
                            .to_string();

                        // Add to package cache
                        package_cache.entry(package_name.clone())
                            .or_insert_with(HashMap::new)
                            .insert(version_str.clone(), package.clone());

                        packages_found += 1;
                        versions_found += 1;

                        // Create variants if the package has variant definitions
                        if !package.variants.is_empty() {
                            let mut variant_names = Vec::new();
                            for (index, _variant_reqs) in package.variants.iter().enumerate() {
                                let variant_name = format!("variant_{}", index);
                                variant_names.push(variant_name);
                                variants_found += 1;
                            }

                            let variant_key = format!("{}-{}", package_name, version_str);
                            variant_cache.insert(variant_key, variant_names);
                        }

                        // Calculate file size
                        if let Ok(metadata) = fs::metadata(package_file).await {
                            size_bytes += metadata.len();
                        }

                        break; // Found a package file, no need to check others
                    }
                    Err(e) => {
                        eprintln!("Failed to load package from {}: {}", package_file.display(), e);
                    }
                }
            }
        }

        Ok(PackageScanResult {
            packages_found,
            versions_found,
            variants_found,
            size_bytes,
        })
    }

    /// Load a package from a file
    async fn load_package_from_file(&self, path: &Path) -> Result<Package, RezCoreError> {
        let content = fs::read_to_string(path).await
            .map_err(|e| RezCoreError::Repository(
                format!("Failed to read package file {}: {}", path.display(), e)
            ))?;

        // Simple YAML parsing for now
        if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
           path.extension().and_then(|s| s.to_str()) == Some("yml") {
            serde_yaml::from_str(&content)
                .map_err(|e| RezCoreError::Repository(
                    format!("Failed to parse YAML package file {}: {}", path.display(), e)
                ))
        } else {
            Err(RezCoreError::Repository(
                format!("Unsupported package file format: {}", path.display())
            ))
        }
    }

    /// Check if a string matches a pattern (supports basic wildcards)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple wildcard matching (supports * and ?)
        let regex_pattern = pattern
            .replace("*", ".*")
            .replace("?", ".");
        
        if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(text)
        } else {
            // Fallback to exact match
            text == pattern
        }
    }
}

/// Result of scanning a package directory
#[derive(Debug)]
struct PackageScanResult {
    packages_found: usize,
    versions_found: usize,
    variants_found: usize,
    size_bytes: u64,
}
