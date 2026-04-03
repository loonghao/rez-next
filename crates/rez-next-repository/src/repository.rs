//! Repository trait and base implementations

use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement};
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
#[derive(Debug, Clone, Default)]
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Remove duplicate packages from a list, keeping unique name+version combinations.
///
/// Packages are sorted by name (ascending) and version (descending), so the
/// highest version for each package appears first. Exact `name-version` duplicates
/// are deduplicated (only the first occurrence is kept).
pub fn deduplicate_packages(mut packages: Vec<Package>) -> Result<Vec<Package>, RezCoreError> {
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

    // ── deduplicate_packages ───────────────────────────────────────────────

    #[test]
    fn test_deduplicate_removes_exact_duplicates() {
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.9.0"), // duplicate
        ];
        let result = deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "python");
    }

    #[test]
    fn test_deduplicate_preserves_different_versions() {
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.11.0"),
            make_pkg("python", "3.10.0"),
        ];
        let result = deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 3, "all 3 different versions must be kept");
    }

    #[test]
    fn test_deduplicate_sorts_versions_descending() {
        let pkgs = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("python", "3.11.0"),
            make_pkg("python", "3.10.0"),
        ];
        let result = deduplicate_packages(pkgs).unwrap();
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
        let pkgs = vec![
            make_pkg("maya", "2024.1"),
            make_pkg("python", "3.11.0"),
            make_pkg("maya", "2023.0"),
            make_pkg("python", "3.9.0"),
            make_pkg("houdini", "20.0"),
        ];
        let result = deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 5, "5 distinct name+version combos");
        let houdini: Vec<_> = result.iter().filter(|p| p.name == "houdini").collect();
        assert_eq!(houdini.len(), 1);
        let maya: Vec<_> = result.iter().filter(|p| p.name == "maya").collect();
        assert_eq!(maya.len(), 2);
        let python: Vec<_> = result.iter().filter(|p| p.name == "python").collect();
        assert_eq!(python.len(), 2);
    }

    #[test]
    fn test_deduplicate_empty_input() {
        let result = deduplicate_packages(vec![]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_deduplicate_no_version_packages() {
        let pkgs = vec![
            make_pkg_no_ver("unnamed"),
            make_pkg_no_ver("unnamed"), // duplicate with no version
        ];
        let result = deduplicate_packages(pkgs).unwrap();
        assert_eq!(
            result.len(),
            1,
            "no-version duplicates should be deduplicated"
        );
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
        let criteria = PackageSearchCriteria {
            name_pattern: Some("python*".to_string()),
            limit: Some(10),
            ..Default::default()
        };
        assert_eq!(criteria.name_pattern, Some("python*".to_string()));
        assert_eq!(criteria.limit, Some(10));
    }

    // ── deduplicate_packages: single and exhaustive ──────────────────

    #[test]
    fn test_deduplicate_single_package() {
        let pkgs = vec![make_pkg("houdini", "19.5.0")];
        let result = deduplicate_packages(pkgs).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "houdini");
    }
}
