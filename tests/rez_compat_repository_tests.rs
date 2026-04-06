//! Rez Compat — Repository API Tests (Cycle 34)
//!
//! Covers:
//! - SimpleRepository.get_package by name + exact version
//! - SimpleRepository.get_package returns None for missing name/version
//! - SimpleRepository.get_package with None version returns latest
//! - SimpleRepository.find_packages returns all versions
//! - RepositoryManager.get_package delegates across multiple repos
//! - RepositoryManager.list_packages deduplication
//! - RepositoryManager.find_packages sorting (latest first)

use rez_next_repository::simple_repository::{
    PackageRepository, RepositoryManager, SimpleRepository,
};
use std::fs;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Create a temporary repository with packages of the form (name, version, requires[]).
fn make_repo(packages: &[(&str, &str, &[&str])]) -> (TempDir, SimpleRepository) {
    let tmp = TempDir::new().unwrap();
    for (name, version, requires) in packages {
        let pkg_dir = tmp.path().join(name).join(version);
        fs::create_dir_all(&pkg_dir).unwrap();
        let req_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n{}", name, version, req_block),
        )
        .unwrap();
    }
    let repo = SimpleRepository::new(tmp.path(), "test_repo".to_string());
    (tmp, repo)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// ─── Cycle 34: SimpleRepository get_package tests ────────────────────────────

/// rez repo: get_package by exact version returns the correct package.
#[test]
fn test_simple_repo_get_package_exact_version() {
    let (_tmp, repo) = make_repo(&[
        ("python", "3.9.0", &[]),
        ("python", "3.10.0", &[]),
        ("python", "3.11.0", &[]),
    ]);
    let result = rt()
        .block_on(repo.get_package("python", Some("3.10.0")))
        .unwrap();
    assert!(result.is_some(), "should find python-3.10.0");
    let pkg = result.unwrap();
    assert_eq!(pkg.name, "python");
    assert_eq!(
        pkg.version.as_ref().map(|v| v.as_str()),
        Some("3.10.0"),
        "returned package version should be 3.10.0"
    );
}

/// rez repo: get_package with version=None returns the latest version.
#[test]
fn test_simple_repo_get_package_latest_when_no_version() {
    let (_tmp, repo) = make_repo(&[
        ("numpy", "1.20.0", &[]),
        ("numpy", "1.25.0", &[]),
        ("numpy", "1.24.0", &[]),
    ]);
    let result = rt().block_on(repo.get_package("numpy", None)).unwrap();
    assert!(result.is_some(), "should return a numpy package");
    let pkg = result.unwrap();
    assert_eq!(
        pkg.version.as_ref().map(|v| v.as_str()),
        Some("1.25.0"),
        "should return latest (1.25.0)"
    );
}

/// rez repo: get_package returns None for non-existent package name.
#[test]
fn test_simple_repo_get_package_missing_name() {
    let (_tmp, repo) = make_repo(&[("python", "3.11.0", &[])]);
    let result = rt()
        .block_on(repo.get_package("nonexistent", Some("1.0.0")))
        .unwrap();
    assert!(
        result.is_none(),
        "should return None for nonexistent package"
    );
}

/// rez repo: get_package returns None when version does not exist.
#[test]
fn test_simple_repo_get_package_missing_version() {
    let (_tmp, repo) = make_repo(&[("python", "3.9.0", &[]), ("python", "3.11.0", &[])]);
    let result = rt()
        .block_on(repo.get_package("python", Some("3.10.0")))
        .unwrap();
    assert!(
        result.is_none(),
        "should return None for python-3.10.0 (not in repo)"
    );
}

/// rez repo: find_packages returns all versions of a package sorted latest first.
#[test]
fn test_simple_repo_find_packages_all_versions() {
    let (_tmp, repo) = make_repo(&[
        ("scipy", "1.10.0", &[]),
        ("scipy", "1.11.0", &[]),
        ("scipy", "1.9.0", &[]),
    ]);
    let packages = rt().block_on(repo.find_packages("scipy")).unwrap();
    assert_eq!(packages.len(), 3, "should find 3 scipy versions");
    let versions: Vec<&str> = packages
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| v.as_str()))
        .collect();
    assert!(versions.contains(&"1.10.0"), "1.10.0 should be in results");
    assert!(versions.contains(&"1.11.0"), "1.11.0 should be in results");
    assert!(versions.contains(&"1.9.0"), "1.9.0 should be in results");
}

/// rez repo: find_packages returns empty vec for unknown package.
#[test]
fn test_simple_repo_find_packages_empty_for_unknown() {
    let (_tmp, repo) = make_repo(&[("python", "3.11.0", &[])]);
    let packages = rt().block_on(repo.find_packages("unknown_pkg")).unwrap();
    assert!(
        packages.is_empty(),
        "find_packages should return empty for unknown package"
    );
}

// ─── Cycle 34: RepositoryManager get_package tests ───────────────────────────

/// rez repo_manager: get_package finds package across multiple repos.
#[test]
fn test_repo_manager_get_package_across_repos() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    // repo1 has python 3.9
    let pkg_dir = tmp1.path().join("python").join("3.9.0");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(
        pkg_dir.join("package.py"),
        "name = 'python'\nversion = '3.9.0'\n",
    )
    .unwrap();

    // repo2 has python 3.11
    let pkg_dir2 = tmp2.path().join("python").join("3.11.0");
    fs::create_dir_all(&pkg_dir2).unwrap();
    fs::write(
        pkg_dir2.join("package.py"),
        "name = 'python'\nversion = '3.11.0'\n",
    )
    .unwrap();

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp1.path(),
        "repo1".to_string(),
    )));
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp2.path(),
        "repo2".to_string(),
    )));

    // Ask for 3.9.0 — lives in repo1
    let result = rt()
        .block_on(mgr.get_package("python", Some("3.9.0")))
        .unwrap();
    assert!(result.is_some(), "repo manager should find python-3.9.0");
    assert_eq!(
        result.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("3.9.0")
    );
}

/// rez repo_manager: list_packages deduplicates names across repos.
#[test]
fn test_repo_manager_list_packages_dedup() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    for (root, name, ver) in [
        (tmp1.path(), "python", "3.9.0"),
        (tmp1.path(), "numpy", "1.20.0"),
        (tmp2.path(), "python", "3.11.0"),
        (tmp2.path(), "scipy", "1.10.0"),
    ] {
        let d = root.join(name).join(ver);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n", name, ver),
        )
        .unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp1.path(),
        "r1".to_string(),
    )));
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp2.path(),
        "r2".to_string(),
    )));

    let names = rt().block_on(mgr.list_packages()).unwrap();
    // Deduplicated: python, numpy, scipy
    assert_eq!(
        names.len(),
        3,
        "should have 3 unique package names, got {:?}",
        names
    );
    assert!(names.contains(&"python".to_string()));
    assert!(names.contains(&"numpy".to_string()));
    assert!(names.contains(&"scipy".to_string()));
}

/// rez repo_manager: find_packages returns all versions sorted latest-first.
#[test]
fn test_repo_manager_find_packages_sorted_latest_first() {
    let tmp = TempDir::new().unwrap();
    for ver in ["1.0.0", "3.0.0", "2.0.0"] {
        let d = tmp.path().join("mylib").join(ver);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("package.py"),
            format!("name = 'mylib'\nversion = '{}'\n", ver),
        )
        .unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(tmp.path(), "r".to_string())));

    let packages = rt().block_on(mgr.find_packages("mylib")).unwrap();
    assert_eq!(packages.len(), 3);
    assert_eq!(
        packages[0].version.as_ref().map(|v| v.as_str()),
        Some("3.0.0"),
        "latest version should be first"
    );
    assert_eq!(
        packages[2].version.as_ref().map(|v| v.as_str()),
        Some("1.0.0"),
        "oldest version should be last"
    );
}
