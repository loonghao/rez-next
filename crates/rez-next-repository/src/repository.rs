//! Repository trait and base implementations

use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement};
use rez_next_version::Version;
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
pub struct RepositoryManager {
    /// List of repositories in priority order
    repositories: Arc<RwLock<Vec<Arc<RwLock<dyn Repository>>>>>,
    /// Repository cache
    cache: Arc<RwLock<HashMap<String, Arc<RwLock<dyn Repository>>>>>,
    /// Sync-accessible count (kept in sync with repositories)
    count: Arc<std::sync::atomic::AtomicUsize>,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
            count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    /// Get the number of repositories (sync-safe via atomic counter)
    pub fn repository_count(&self) -> usize {
        self.count.load(std::sync::atomic::Ordering::Acquire)
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

        let new_priority = metadata.priority;

        // Insert in priority-descending order (higher priority first)
        // Collect priorities without holding read locks simultaneously
        let mut priorities = Vec::with_capacity(repos.len());
        for r in repos.iter() {
            let p = r.read().await.metadata().priority;
            priorities.push(p);
        }
        let insert_pos = priorities
            .iter()
            .position(|&p| p < new_priority)
            .unwrap_or(priorities.len());

        repos.insert(insert_pos, repository.clone());
        cache.insert(metadata.name.clone(), repository);
        self.count.fetch_add(1, std::sync::atomic::Ordering::Release);

        Ok(())
    }

    /// Remove a repository by name
    pub async fn remove_repository(&self, name: &str) -> Result<bool, RezCoreError> {
        let mut repos = self.repositories.write().await;
        let mut cache = self.cache.write().await;

        let removed_from_cache = cache.remove(name).is_some();

        // Find by name and remove from ordered list
        let mut found_pos: Option<usize> = None;
        for (i, r) in repos.iter().enumerate() {
            if r.read().await.metadata().name == name {
                found_pos = Some(i);
                break;
            }
        }

        if let Some(pos) = found_pos {
            repos.remove(pos);
            self.count.fetch_sub(1, std::sync::atomic::Ordering::Release);
            Ok(true)
        } else {
            // Still return true if we removed from cache
            Ok(removed_from_cache)
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

#[cfg(test)]
mod repository_tests {
    use super::*;
    use rez_next_package::Package;
    use rez_next_version::Version;

    fn make_pkg(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg
    }

    fn make_pkg_no_ver(name: &str) -> Package {
        Package::new(name.to_string())
    }

    // ── RepositoryStats defaults ─────────────────────────────────────

    #[test]
    fn test_repository_stats_default() {
        let stats = RepositoryStats::default();
        assert_eq!(stats.package_count, 0);
        assert_eq!(stats.version_count, 0);
        assert_eq!(stats.variant_count, 0);
        assert_eq!(stats.size_bytes, 0);
        assert!(stats.last_scan_time.is_none());
        assert!(stats.last_scan_duration_ms.is_none());
    }

    // ── RepositoryManager deduplicate_packages ───────────────────────

    #[test]
    fn test_deduplicate_removes_exact_duplicates() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.9.0"), // duplicate
        ];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "python");
    }

    #[test]
    fn test_deduplicate_preserves_different_versions() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.11.0"),
            make_pkg("python", "3.10.0"),
        ];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 3, "all 3 different versions must be kept");
    }

    #[test]
    fn test_deduplicate_sorts_versions_descending() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.11.0"),
            make_pkg("python", "3.10.0"),
        ];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        // All under same name, should be sorted descending: 3.11 > 3.10 > 3.9
        let versions: Vec<&str> = result
            .iter()
            .map(|p| p.version.as_ref().unwrap().as_str())
            .collect();
        assert_eq!(versions[0], "3.11.0", "highest version should come first");
        assert_eq!(versions[2], "3.9.0", "lowest version should come last");
    }

    #[test]
    fn test_deduplicate_multiple_packages() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![
            make_pkg("maya", "2024.1"),
            make_pkg("python", "3.11.0"),
            make_pkg("maya", "2023.0"),
            make_pkg("python", "3.9.0"),
            make_pkg("houdini", "20.0"),
        ];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 5, "5 distinct name+version combos");
        // Packages should be ordered by name alphabetically then version descending
        let houdini: Vec<_> = result.iter().filter(|p| p.name == "houdini").collect();
        assert_eq!(houdini.len(), 1);
        let maya: Vec<_> = result.iter().filter(|p| p.name == "maya").collect();
        assert_eq!(maya.len(), 2);
        let python: Vec<_> = result.iter().filter(|p| p.name == "python").collect();
        assert_eq!(python.len(), 2);
    }

    #[test]
    fn test_deduplicate_empty_input() {
        let mgr = RepositoryManager::new();
        let result = mgr.deduplicate_packages(vec![]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplicate_no_version_packages() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![
            make_pkg_no_ver("unnamed"),
            make_pkg_no_ver("unnamed"), // duplicate with no version
        ];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 1, "no-version duplicates should be deduplicated");
    }

    // ── RepositoryMetadata creation ──────────────────────────────────

    #[test]
    fn test_repository_metadata_fields() {
        let meta = RepositoryMetadata {
            name: "test-repo".to_string(),
            path: std::path::PathBuf::from("/tmp/packages"),
            repository_type: RepositoryType::Memory,
            priority: 10,
            read_only: false,
            description: Some("Test repository".to_string()),
            config: HashMap::new(),
        };
        assert_eq!(meta.name, "test-repo");
        assert_eq!(meta.priority, 10);
        assert!(!meta.read_only);
        assert_eq!(meta.repository_type, RepositoryType::Memory);
    }

    #[test]
    fn test_package_search_criteria_defaults() {
        let criteria = PackageSearchCriteria::default();
        assert!(criteria.name_pattern.is_none());
        assert!(criteria.version_requirement.is_none());
        assert!(criteria.requirements.is_empty());
        assert!(criteria.limit.is_none());
        assert!(!criteria.include_prerelease);
    }

    #[test]
    fn test_package_search_criteria_with_pattern() {
        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("python*".to_string());
        criteria.limit = Some(10);
        assert_eq!(criteria.name_pattern, Some("python*".to_string()));
        assert_eq!(criteria.limit, Some(10));
    }

    // ── RepositoryManager repository_count (atomic) ──────────────────

    #[test]
    fn test_repository_manager_initial_count_is_zero() {
        let mgr = RepositoryManager::new();
        assert_eq!(mgr.repository_count(), 0);
    }

    // ── RepositoryManager deduplicate is exhaustive ──────────────────

    #[test]
    fn test_deduplicate_single_package() {
        let mgr = RepositoryManager::new();
        let pkgs = vec![make_pkg("houdini", "19.5.0")];
        let result = mgr.deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "houdini");
    }
}
