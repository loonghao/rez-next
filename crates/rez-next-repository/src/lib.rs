//! # Rez Core Repository
//!
//! Repository scanning, caching, and management for Rez Core.
//!
//! This crate provides:
//! - Repository scanning and indexing
//! - Package discovery and caching
//! - Repository metadata management
//! - Async repository operations
//! - Resource types for package management

pub mod cache;
pub mod filesystem;
pub mod high_performance_scanner;
pub mod package_repository;
pub mod package_search;
pub mod repository;
pub mod resources;
pub mod scanner;
pub mod scanner_types;
pub mod simple_repository;

pub use cache::*;
pub use filesystem::*;
pub use high_performance_scanner::*;
pub use package_repository::{FilesystemPackageRepository, PackageRepository};
pub use package_search::{ResourceSearchResult, get_plugins, get_reverse_dependency_tree};
pub use repository::{
    PackageSearchCriteria, Repository, RepositoryMetadata, RepositoryStats, RepositoryType,
    deduplicate_packages,
};
pub use resources::{
    PackageFamilyResource, PackageResource, ResourceHandle, ResourcePool, VariantResource,
};
pub use scanner::*;
pub use scanner_types::{
    CacheStatistics, PackageScanResult, REZ_PACKAGE_FILENAMES, ScanError, ScanErrorType,
    ScanPerformanceMetrics, ScanResult, ScannerConfig,
};
pub use simple_repository::*;

/// Get statistics about packages in the given repository paths.
///
/// This is compatible with `rez.solver.package_repo_stats()`.
///
/// # Arguments
/// * `paths` - List of repository paths to scan
///
/// # Returns
/// A `RepositoryStats` object with combined statistics from all repositories.
pub fn package_repo_stats(paths: Vec<String>) -> RepositoryStats {
    let mut combined_stats = RepositoryStats::default();

    for path_str in paths {
        let path = std::path::PathBuf::from(&path_str);
        if !path.exists() {
            continue;
        }

        // Try to scan the repository directory
        if let Ok(entries) = std::fs::read_dir(&path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    let has_package_file = |dir: &std::path::Path| dir.join("package.py").exists();

                    if has_package_file(&entry_path) {
                        combined_stats.package_count += 1;

                        // Count versions (subdirectories)
                        if let Ok(version_entries) = std::fs::read_dir(&entry_path) {
                            let version_count = version_entries
                                .flatten()
                                .filter(|e| e.path().is_dir())
                                .count();
                            combined_stats.version_count += version_count;
                            // For now, assume each version has at least one variant
                            combined_stats.variant_count += version_count;
                        }
                    } else if let Ok(version_entries) = std::fs::read_dir(&entry_path) {
                        let version_count = version_entries
                            .flatten()
                            .filter(|e| e.path().is_dir() && has_package_file(&e.path()))
                            .count();

                        if version_count > 0 {
                            combined_stats.package_count += 1;
                            combined_stats.version_count += version_count;
                            combined_stats.variant_count += version_count;
                        }
                    }
                }

                // Add to size
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        combined_stats.size_bytes += metadata.len();
                    }
                }
            }
        }
    }

    combined_stats
}

#[cfg(test)]
mod lib_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a temporary repository with packages for testing.
    fn create_test_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        // Create python/3.9.0/package.py
        let python_dir = root.join("python");
        fs::create_dir(&python_dir).unwrap();
        let v39 = python_dir.join("3.9.0");
        fs::create_dir(&v39).unwrap();
        fs::write(v39.join("package.py"), "# python 3.9.0").unwrap();

        // Create python/3.10.0/package.py
        let v310 = python_dir.join("3.10.0");
        fs::create_dir(&v310).unwrap();
        fs::write(v310.join("package.py"), "# python 3.10.0").unwrap();

        // Create maya/2024/package.py
        let maya_dir = root.join("maya");
        fs::create_dir(&maya_dir).unwrap();
        let v2024 = maya_dir.join("2024");
        fs::create_dir(&v2024).unwrap();
        fs::write(v2024.join("package.py"), "# maya 2024").unwrap();

        dir
    }

    #[test]
    fn test_package_repo_stats_empty_paths() {
        let stats = package_repo_stats(vec![]);
        assert_eq!(stats.package_count, 0);
        assert_eq!(stats.version_count, 0);
        assert_eq!(stats.variant_count, 0);
    }

    #[test]
    fn test_package_repo_stats_nonexistent_path() {
        let stats = package_repo_stats(vec!["C:\\nonexistent".to_string()]);
        assert_eq!(stats.package_count, 0);
    }

    #[test]
    fn test_package_repo_stats_with_packages() {
        let dir = create_test_repo();
        let path = dir.path().to_string_lossy().to_string();

        let stats = package_repo_stats(vec![path]);

        // Should find 2 packages: python, maya
        assert_eq!(stats.package_count, 2);
        // Should find 3 versions: python-3.9.0, python-3.10.0, maya-2024
        assert!(stats.version_count >= 2);
    }
}
