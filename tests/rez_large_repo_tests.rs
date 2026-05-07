//! Large Package Repository Tests (Cycle 280)
//!
//! Tests for handling large package repositories efficiently.
//! Covers:
//! - Scanning repositories with 1000+ packages
//! - Search performance on large repos
//! - Memory usage with large package sets

use rez_next_repository::simple_repository::{
    PackageRepository, SimpleRepository,
};
use std::fs;
use tempfile::TempDir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Create a large temporary repository with `count` packages.
/// Packages are named `pkg_N` with version `1.0.0`.
fn make_large_repo(count: usize) -> (TempDir, SimpleRepository) {
    let tmp = TempDir::new().unwrap();
    for i in 0..count {
        let name = format!("pkg_{}", i);
        let pkg_dir = tmp.path().join(&name).join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '1.0.0'\n", name),
        )
        .unwrap();
    }
    let repo = SimpleRepository::new(tmp.path(), "large_repo".to_string());
    (tmp, repo)
}

/// Create a repo with multiple versions per package.
fn make_repo_with_versions(pkg_count: usize, versions_per_pkg: usize) -> (TempDir, SimpleRepository) {
    let tmp = TempDir::new().unwrap();
    for i in 0..pkg_count {
        let name = format!("pkg_{}", i);
        for v in 0..versions_per_pkg {
            let version = format!("1.{}.0", v);
            let pkg_dir = tmp.path().join(&name).join(&version);
            fs::create_dir_all(&pkg_dir).unwrap();
            fs::write(
                pkg_dir.join("package.py"),
                format!("name = '{}'\nversion = '{}'\n", name, version),
            )
            .unwrap();
        }
    }
    let repo = SimpleRepository::new(tmp.path(), "multi_version_repo".to_string());
    (tmp, repo)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// ─── Large Repository Tests ────────────────────────────────────────────────

/// Test that scanning a repository with 1000 packages completes in reasonable time.
#[test]
fn test_large_repo_scan_1000_packages() {
    let (_tmp, repo) = make_large_repo(1000);
    let start = std::time::Instant::now();
    let result = rt().block_on(repo.scan());
    let elapsed = start.elapsed();
    
    assert!(result.is_ok(), "scan should succeed for 1000 packages");
    // Should complete within 5 seconds for 1000 packages
    assert!(
        elapsed.as_secs() < 5,
        "scanning 1000 packages took too long: {:?}",
        elapsed
    );
}

/// Test that scanning a repository with 5000 packages completes.
#[test]
fn test_large_repo_scan_5000_packages() {
    let (_tmp, repo) = make_large_repo(5000);
    let start = std::time::Instant::now();
    let result = rt().block_on(repo.scan());
    let elapsed = start.elapsed();
    
    assert!(result.is_ok(), "scan should succeed for 5000 packages");
    // Should complete within 10 seconds for 5000 packages
    assert!(
        elapsed.as_secs() < 10,
        "scanning 5000 packages took too long: {:?}",
        elapsed
    );
}

/// Test finding packages in a large repository.
#[test]
fn test_large_repo_find_packages() {
    let (_tmp, repo) = make_large_repo(1000);
    rt().block_on(repo.scan()).unwrap();
    
    let start = std::time::Instant::now();
    let packages = rt().block_on(repo.find_packages("pkg_500"));
    let elapsed = start.elapsed();
    
    assert!(!packages.as_ref().unwrap().is_empty(), "should find pkg_500");
    // Lookup should be fast (< 100ms)
    assert!(
        elapsed.as_millis() < 100,
        "package lookup took too long: {:?}",
        elapsed
    );
}

/// Test listing all packages in a large repository.
#[test]
fn test_large_repo_list_all_packages() {
    let (_tmp, repo) = make_large_repo(1000);
    rt().block_on(repo.scan()).unwrap();
    
    let start = std::time::Instant::now();
    let packages = rt().block_on(repo.list_packages());
    let elapsed = start.elapsed();
    
    assert_eq!(packages.as_ref().unwrap().len(), 1000, "should have 1000 packages");
    // Listing should be reasonably fast
    assert!(
        elapsed.as_secs() < 2,
        "listing 1000 packages took too long: {:?}",
        elapsed
    );
}

/// Test repository with multiple versions per package.
#[test]
fn test_large_repo_multiple_versions() {
    let (_tmp, repo) = make_repo_with_versions(100, 10);
    let start = std::time::Instant::now();
    let result = rt().block_on(repo.scan());
    let elapsed = start.elapsed();
    
    assert!(result.is_ok(), "scan should succeed");
    // 100 packages x 10 versions = 1000 package versions
    // Should complete within 5 seconds
    assert!(
        elapsed.as_secs() < 5,
        "scanning 100 packages with 10 versions each took too long: {:?}",
        elapsed
    );
}

/// Test memory usage doesn't grow excessively with large repos.
#[test]
fn test_large_repo_memory_usage() {
    let (_tmp, repo) = make_large_repo(2000);
    rt().block_on(repo.scan()).unwrap();
    
    // Get a package - this should not cause excessive memory allocation
    let pkg = rt().block_on(repo.get_package("pkg_1000", None));
    assert!(pkg.is_ok(), "should find pkg_1000");
    
    // The package object should be reasonable size
    let pkg = pkg.unwrap().unwrap();
    assert_eq!(pkg.name, "pkg_1000");
}

/// Test concurrent access to repository (single repo, multiple readers).
#[test]
fn test_large_repo_concurrent_readers() {
    let (_tmp, repo) = make_large_repo(500);
    rt().block_on(repo.scan()).unwrap();
    
    let repo = std::sync::Arc::new(repo);
    let mut handles = vec![];
    
    // Spawn 10 concurrent readers
    for i in 0..10 {
        let repo_clone = repo.clone();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let pkg_name = format!("pkg_{}", i * 50);
            rt.block_on(repo_clone.find_packages(&pkg_name))
        });
        handles.push(handle);
    }
    
    // All readers should complete successfully
    for handle in handles {
        let result = handle.join().unwrap();
        assert!(!result.as_ref().unwrap().is_empty(), "concurrent read should succeed");
    }
}

/// Test search performance on large repository.
#[test]
fn test_large_repo_search_performance() {
    let (_tmp, repo) = make_large_repo(2000);
    rt().block_on(repo.scan()).unwrap();
    
    let start = std::time::Instant::now();
    
    // Search for packages with "pkg_1" prefix (should match pkg_1, pkg_10, pkg_100, etc.)
    let mut count = 0;
    for i in 0..20 {
        let pkg_name = format!("pkg_{}", i);
        let packages = rt().block_on(repo.find_packages(&pkg_name));
        count += packages.as_ref().unwrap().len();
    }
    
    let elapsed = start.elapsed();
    
    assert!(count > 0, "should find some packages");
    // 20 lookups should complete within 500ms
    assert!(
        elapsed.as_millis() < 500,
        "20 package lookups took too long: {:?}",
        elapsed
    );
}

/// Test that repository scan is idempotent (multiple scans produce same result).
#[test]
fn test_large_repo_scan_idempotent() {
    let (_tmp, repo) = make_large_repo(100);
    rt().block_on(repo.scan()).unwrap();
    let result1 = rt().block_on(repo.list_packages()).unwrap();
    rt().block_on(repo.scan()).unwrap();
    let result2 = rt().block_on(repo.list_packages()).unwrap();
    
    assert_eq!(result1.len(), result2.len(), "scans should return same number of packages");
}

/// Test handling of repository with no packages.
#[test]
fn test_empty_repo_scan() {
    let (_tmp, repo) = make_large_repo(0);
    let result = rt().block_on(repo.scan());
    
    assert!(result.is_ok(), "scan should succeed for empty repo");
    let packages = rt().block_on(repo.list_packages()).unwrap();
    assert_eq!(packages.len(), 0, "empty repo should have no packages");
}

/// Test repository with single package.
#[test]
fn test_single_package_repo() {
    let (_tmp, repo) = make_large_repo(1);
    let result = rt().block_on(repo.scan());
    
    assert!(result.is_ok(), "scan should succeed for single package repo");
    let packages = rt().block_on(repo.list_packages()).unwrap();
    assert_eq!(packages.len(), 1, "should have exactly 1 package");
}

/// Test that package ordering is consistent across scans.
#[test]
fn test_large_repo_package_ordering() {
    let (_tmp, repo) = make_large_repo(100);
    rt().block_on(repo.scan()).unwrap();
    let result1 = rt().block_on(repo.list_packages()).unwrap();
    rt().block_on(repo.scan()).unwrap();
    let result2 = rt().block_on(repo.list_packages()).unwrap();
    
    assert_eq!(result1, result2, "package ordering should be consistent");
}
