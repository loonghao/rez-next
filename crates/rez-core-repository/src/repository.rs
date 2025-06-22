//! Repository trait and base implementations

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use rez_core_common::RezCoreError;
use rez_core_package::{Package, PackageRequirement};
use rez_core_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Repository metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetadata {
    /// Repository name
    pub name: String,
    /// Repository path
    pub path: PathBuf,
    /// Repository type
    pub repository_type: RepositoryType,
    /// Repository priority (higher = more preferred)
    pub priority: i32,
    /// Whether this repository is read-only
    pub read_only: bool,
    /// Repository description
    pub description: Option<String>,
    /// Repository configuration
    pub config: HashMap<String, String>,
}

/// Repository type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RepositoryType {
    /// Local filesystem repository
    FileSystem,
    /// Memory-based repository (for testing)
    Memory,
    /// Remote repository (future)
    Remote,
}

/// Package search criteria
#[derive(Debug, Clone)]
pub struct PackageSearchCriteria {
    /// Package name pattern (supports wildcards)
    pub name_pattern: Option<String>,
    /// Version requirement (simplified)
    pub version_requirement: Option<String>,
    /// Additional requirements
    pub requirements: Vec<PackageRequirement>,
    /// Maximum number of results
    pub limit: Option<usize>,
    /// Include pre-release versions
    pub include_prerelease: bool,
}

impl Default for PackageSearchCriteria {
    fn default() -> Self {
        Self {
            name_pattern: None,
            version_requirement: None,
            requirements: Vec::new(),
            limit: None,
            include_prerelease: false,
        }
    }
}

/// Repository trait for package discovery and management
#[async_trait::async_trait]
pub trait Repository: Send + Sync {
    /// Get repository metadata
    fn metadata(&self) -> &RepositoryMetadata;

    /// Initialize the repository (scan packages, build cache, etc.)
    async fn initialize(&mut self) -> Result<(), RezCoreError>;

    /// Check if the repository is initialized
    fn is_initialized(&self) -> bool;

    /// Refresh the repository (rescan packages)
    async fn refresh(&mut self) -> Result<(), RezCoreError>;

    /// Find packages matching the given criteria
    async fn find_packages(
        &self,
        criteria: &PackageSearchCriteria,
    ) -> Result<Vec<Package>, RezCoreError>;

    /// Get a specific package by name and version
    async fn get_package(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>, RezCoreError>;

    /// Get all versions of a package
    async fn get_package_versions(&self, name: &str) -> Result<Vec<Version>, RezCoreError>;

    /// Get package variants for a specific package (simplified - returns variant names)
    async fn get_package_variants(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Vec<String>, RezCoreError>;

    /// Check if a package exists
    async fn package_exists(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<bool, RezCoreError>;

    /// Get all package names in the repository
    async fn get_package_names(&self) -> Result<Vec<String>, RezCoreError>;

    /// Get repository statistics
    async fn get_stats(&self) -> Result<RepositoryStats, RezCoreError>;
}

/// Repository statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    /// Total number of packages
    pub package_count: usize,
    /// Total number of package versions
    pub version_count: usize,
    /// Total number of package variants
    pub variant_count: usize,
    /// Repository size in bytes
    pub size_bytes: u64,
    /// Last scan time (Unix timestamp)
    pub last_scan_time: Option<i64>,
    /// Scan duration in milliseconds
    pub last_scan_duration_ms: Option<u64>,
}

impl Default for RepositoryStats {
    fn default() -> Self {
        Self {
            package_count: 0,
            version_count: 0,
            variant_count: 0,
            size_bytes: 0,
            last_scan_time: None,
            last_scan_duration_ms: None,
        }
    }
}

/// Repository manager for handling multiple repositories
#[cfg_attr(feature = "python-bindings", pyclass)]
pub struct RepositoryManager {
    /// List of repositories in priority order
    repositories: Arc<RwLock<Vec<Arc<RwLock<dyn Repository>>>>>,
    /// Repository cache
    cache: Arc<RwLock<HashMap<String, Arc<RwLock<dyn Repository>>>>>,
}

#[cfg_attr(feature = "python-bindings", pymethods)]
impl RepositoryManager {
    #[cfg_attr(feature = "python-bindings", new)]
    pub fn new() -> Self {
        Self {
            repositories: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the number of repositories
    #[cfg_attr(feature = "python-bindings", getter)]
    pub fn repository_count(&self) -> usize {
        // This is a simplified sync version for Python binding
        // In async context, use the async methods
        0 // TODO: Implement sync version
    }
}

impl RepositoryManager {
    /// Add a repository to the manager
    pub async fn add_repository(
        &self,
        repository: Arc<RwLock<dyn Repository>>,
    ) -> Result<(), RezCoreError> {
        let mut repos = self.repositories.write().await;
        let mut cache = self.cache.write().await;

        let metadata = {
            let repo = repository.read().await;
            repo.metadata().clone()
        };

        // Insert in priority order (higher priority first)
        let insert_pos = repos
            .iter()
            .position(|r| {
                // This is a simplified comparison - in practice you'd need async access
                false // TODO: Implement proper priority comparison
            })
            .unwrap_or(repos.len());

        repos.insert(insert_pos, repository.clone());
        cache.insert(metadata.name.clone(), repository);

        Ok(())
    }

    /// Remove a repository by name
    pub async fn remove_repository(&self, name: &str) -> Result<bool, RezCoreError> {
        let mut repos = self.repositories.write().await;
        let mut cache = self.cache.write().await;

        cache.remove(name);

        // Find and remove from repositories list
        if let Some(pos) = repos.iter().position(|r| {
            // TODO: Implement proper name comparison
            false
        }) {
            repos.remove(pos);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get a repository by name
    pub async fn get_repository(&self, name: &str) -> Option<Arc<RwLock<dyn Repository>>> {
        let cache = self.cache.read().await;
        cache.get(name).cloned()
    }

    /// Find packages across all repositories
    pub async fn find_packages(
        &self,
        criteria: &PackageSearchCriteria,
    ) -> Result<Vec<Package>, RezCoreError> {
        let repos = self.repositories.read().await;
        let mut all_packages = Vec::new();

        for repo in repos.iter() {
            let repo_guard = repo.read().await;
            if repo_guard.is_initialized() {
                match repo_guard.find_packages(criteria).await {
                    Ok(mut packages) => all_packages.append(&mut packages),
                    Err(e) => {
                        // Log error but continue with other repositories
                        eprintln!(
                            "Error searching repository {}: {}",
                            repo_guard.metadata().name,
                            e
                        );
                    }
                }
            }
        }

        // Remove duplicates and sort by priority/version
        self.deduplicate_packages(all_packages)
    }

    /// Get a specific package from the first repository that has it
    pub async fn get_package(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>, RezCoreError> {
        let repos = self.repositories.read().await;

        for repo in repos.iter() {
            let repo_guard = repo.read().await;
            if repo_guard.is_initialized() {
                if let Ok(Some(package)) = repo_guard.get_package(name, version).await {
                    return Ok(Some(package));
                }
            }
        }

        Ok(None)
    }

    /// Initialize all repositories
    pub async fn initialize_all(&self) -> Result<(), RezCoreError> {
        let repos = self.repositories.read().await;

        for repo in repos.iter() {
            let mut repo_guard = repo.write().await;
            if let Err(e) = repo_guard.initialize().await {
                eprintln!(
                    "Failed to initialize repository {}: {}",
                    repo_guard.metadata().name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Refresh all repositories
    pub async fn refresh_all(&self) -> Result<(), RezCoreError> {
        let repos = self.repositories.read().await;

        for repo in repos.iter() {
            let mut repo_guard = repo.write().await;
            if let Err(e) = repo_guard.refresh().await {
                eprintln!(
                    "Failed to refresh repository {}: {}",
                    repo_guard.metadata().name,
                    e
                );
            }
        }

        Ok(())
    }

    /// Remove duplicate packages, keeping the highest priority/version
    fn deduplicate_packages(
        &self,
        mut packages: Vec<Package>,
    ) -> Result<Vec<Package>, RezCoreError> {
        // Sort by name and version (descending)
        packages.sort_by(|a, b| {
            match a.name.cmp(&b.name) {
                std::cmp::Ordering::Equal => {
                    match (&a.version, &b.version) {
                        (Some(v1), Some(v2)) => v2.cmp(v1), // Descending version order
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                other => other,
            }
        });

        // Remove duplicates (keep first occurrence, which is highest priority/version)
        let mut unique_packages = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for package in packages {
            let key = match &package.version {
                Some(version) => format!("{}-{}", package.name, version.as_str()),
                None => package.name.clone(),
            };

            if seen.insert(key) {
                unique_packages.push(package);
            }
        }

        Ok(unique_packages)
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}
