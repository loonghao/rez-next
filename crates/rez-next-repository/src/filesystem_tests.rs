//! FileSystemRepository unit tests — extracted from filesystem.rs (Cycle 63).
//!
//! Covers:
//!  - construction, getters, setters
//!  - initialize / refresh
//!  - get_package / get_package_versions
//!  - package_exists
//!  - get_package_names
//!  - find_packages (PackageSearchCriteria)
//!
//! variants / package.py / concurrent tests → filesystem_variants_py_concurrent_tests.rs

use super::*;
use crate::{PackageSearchCriteria, Repository};
use tempfile::TempDir;
use tokio::fs;

// ── helpers ───────────────────────────────────────────────────────────────────

/// Create a minimal package.yaml under `root/name/version/package.yaml`.
pub(super) async fn make_yaml_pkg(root: &std::path::Path, name: &str, version: &str) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let content = format!(
        "name: \"{}\"\nversion: \"{}\"\ndescription: \"Test\"\n",
        name, version
    );
    fs::write(dir.join("package.yaml"), content).await.unwrap();
}

/// Create a package.yaml with a `variants` field.
#[allow(dead_code)]
pub(super) async fn make_yaml_pkg_with_variants(
    root: &std::path::Path,
    name: &str,
    version: &str,
    variants: &[&[&str]],
) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let mut variant_yaml = String::new();
    if !variants.is_empty() {
        variant_yaml.push_str("variants:\n");
        for group in variants {
            let reqs: Vec<String> = group.iter().map(|r| format!("    - \"{}\"", r)).collect();
            variant_yaml.push_str("  -\n");
            variant_yaml.push_str(&reqs.join("\n"));
            variant_yaml.push('\n');
        }
    }
    let content = format!(
        "name: \"{}\"\nversion: \"{}\"\ndescription: \"Test\"\n{}",
        name, version, variant_yaml
    );
    fs::write(dir.join("package.yaml"), content).await.unwrap();
}

/// Create a minimal package.py under `root/name/version/package.py`.
#[allow(dead_code)]
pub(super) async fn make_py_pkg(root: &std::path::Path, name: &str, version: &str) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let content = format!(
        "name = \"{}\"\nversion = \"{}\"\ndescription = \"Test package\"\n",
        name, version
    );
    fs::write(dir.join("package.py"), content).await.unwrap();
}

/// Create a package.py with `variants` field.
#[allow(dead_code)]
pub(super) async fn make_py_pkg_with_variants(
    root: &std::path::Path,
    name: &str,
    version: &str,
    variants: &[&[&str]],
) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let mut variant_lines = String::from("variants = [\n");
    for group in variants {
        let reqs: Vec<String> = group.iter().map(|r| format!("    \"{}\"", r)).collect();
        variant_lines.push_str("    [");
        variant_lines.push_str(&reqs.join(", "));
        variant_lines.push_str("],\n");
    }
    variant_lines.push_str("]\n");
    let content = format!(
        "name = \"{}\"\nversion = \"{}\"\ndescription = \"Test\"\n{}",
        name, version, variant_lines
    );
    fs::write(dir.join("package.py"), content).await.unwrap();
}

// ── construction / getters / setters ─────────────────────────────────────────

#[test]
fn test_new_uses_dir_name_as_default_name() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().to_path_buf();
    let dir_name = path.file_name().unwrap().to_string_lossy().to_string();
    let repo = FileSystemRepository::new(path, None);
    assert_eq!(repo.name(), dir_name);
}

#[test]
fn test_new_with_explicit_name() {
    let tmp = TempDir::new().unwrap();
    let repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("my_repo".to_string()));
    assert_eq!(repo.name(), "my_repo");
}

#[test]
fn test_path_returns_repo_path() {
    let tmp = TempDir::new().unwrap();
    let expected = tmp.path().to_string_lossy().to_string();
    let repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    assert_eq!(repo.path(), expected);
}

#[test]
fn test_read_only_defaults_to_false() {
    let tmp = TempDir::new().unwrap();
    let repo = FileSystemRepository::new(tmp.path().to_path_buf(), None);
    assert!(!repo.read_only());
}

#[test]
fn test_set_read_only_true() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), None);
    repo.set_read_only(true);
    assert!(repo.read_only());
}

#[test]
fn test_set_priority_reflected_in_metadata() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), None);
    repo.set_priority(42);
    assert_eq!(repo.metadata().priority, 42);
}

#[test]
fn test_is_initialized_defaults_false() {
    let tmp = TempDir::new().unwrap();
    let repo = FileSystemRepository::new(tmp.path().to_path_buf(), None);
    assert!(!repo.is_initialized());
}

// ── initialize ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_initialize_nonexistent_path_returns_error() {
    let mut repo = FileSystemRepository::new(
        PathBuf::from("/nonexistent/path/xyz123"),
        Some("bad".to_string()),
    );
    let result = repo.initialize().await;
    assert!(result.is_err(), "Should fail on nonexistent path");
}

#[tokio::test]
async fn test_initialize_empty_dir_succeeds_and_sets_flag() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();
    assert!(repo.is_initialized());
}

#[tokio::test]
async fn test_initialize_discovers_yaml_packages() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "boost", "1.78.0").await;
    make_yaml_pkg(tmp.path(), "python", "3.10.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names = repo.get_package_names().await.unwrap();
    assert!(names.contains(&"boost".to_string()));
    assert!(names.contains(&"python".to_string()));
}

#[tokio::test]
async fn test_refresh_rescans_and_picks_up_new_packages() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "alpha", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names_before = repo.get_package_names().await.unwrap();
    assert_eq!(names_before.len(), 1);

    make_yaml_pkg(tmp.path(), "beta", "2.0.0").await;
    repo.refresh().await.unwrap();

    let names_after = repo.get_package_names().await.unwrap();
    assert_eq!(names_after.len(), 2);
    assert!(names_after.contains(&"beta".to_string()));
}

// ── get_package / get_package_versions ───────────────────────────────────────

#[tokio::test]
async fn test_get_package_by_exact_version() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "mylib", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "mylib", "2.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v1 = Version::parse("1.0.0").unwrap();
    let pkg = repo.get_package("mylib", Some(&v1)).await.unwrap();
    assert!(pkg.is_some());
    assert_eq!(
        pkg.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("1.0.0")
    );
}

#[tokio::test]
async fn test_get_package_latest_returns_highest_version() {
    let tmp = TempDir::new().unwrap();
    for v in &["1.0.0", "3.0.0", "2.0.0"] {
        make_yaml_pkg(tmp.path(), "mylib", v).await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let pkg = repo.get_package("mylib", None).await.unwrap();
    assert!(pkg.is_some());
    assert_eq!(
        pkg.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("3.0.0"),
        "Latest should be 3.0.0"
    );
}

#[tokio::test]
async fn test_get_package_nonexistent_name_returns_none() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let result = repo.get_package("ghost", None).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_package_nonexistent_version_returns_none() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "mylib", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v99 = Version::parse("9.9.9").unwrap();
    let result = repo.get_package("mylib", Some(&v99)).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_package_versions_returns_sorted_descending() {
    let tmp = TempDir::new().unwrap();
    for v in &["1.0.0", "3.0.0", "2.0.0"] {
        make_yaml_pkg(tmp.path(), "sortpkg", v).await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let versions = repo.get_package_versions("sortpkg").await.unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].as_str(), "3.0.0", "First should be latest");
    assert_eq!(versions[2].as_str(), "1.0.0", "Last should be oldest");
}

#[tokio::test]
async fn test_get_package_versions_empty_for_unknown() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();
    let versions = repo.get_package_versions("ghost").await.unwrap();
    assert!(versions.is_empty());
}

// ── package_exists ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_package_exists_by_name() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "existing", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    assert!(repo.package_exists("existing", None).await.unwrap());
    assert!(!repo.package_exists("ghost", None).await.unwrap());
}

#[tokio::test]
async fn test_package_exists_by_name_and_version() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "mypkg", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v1 = Version::parse("1.0.0").unwrap();
    let v2 = Version::parse("2.0.0").unwrap();

    assert!(repo.package_exists("mypkg", Some(&v1)).await.unwrap());
    assert!(!repo.package_exists("mypkg", Some(&v2)).await.unwrap());
}

// ── get_package_names ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_package_names_empty_repo() {
    let tmp = TempDir::new().unwrap();
    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();
    let names = repo.get_package_names().await.unwrap();
    assert!(names.is_empty());
}

#[tokio::test]
async fn test_get_package_names_multiple_packages() {
    let tmp = TempDir::new().unwrap();
    for name in &["alpha", "beta", "gamma"] {
        make_yaml_pkg(tmp.path(), name, "1.0.0").await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names = repo.get_package_names().await.unwrap();
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"alpha".to_string()));
    assert!(names.contains(&"beta".to_string()));
    assert!(names.contains(&"gamma".to_string()));
}

// ── find_packages (with PackageSearchCriteria) ────────────────────────────────

#[tokio::test]
async fn test_find_packages_no_criteria_returns_all() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "aaa", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "bbb", "2.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria::default();
    let results = repo.find_packages(&criteria).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_find_packages_with_exact_name_pattern() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "python", "3.9.0").await;
    make_yaml_pkg(tmp.path(), "pyside2", "5.15.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("python".to_string()),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "python");
}

#[tokio::test]
async fn test_find_packages_with_wildcard_name_pattern() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "py_core", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "py_utils", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "boost", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("py_*".to_string()),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();
    assert_eq!(
        results.len(),
        2,
        "Wildcard should match py_core and py_utils"
    );
}

#[tokio::test]
async fn test_find_packages_with_limit() {
    let tmp = TempDir::new().unwrap();
    for i in 0..5 {
        make_yaml_pkg(tmp.path(), &format!("pkg{}", i), "1.0.0").await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        limit: Some(3),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();

    assert_eq!(
        results.len(),
        3,
        "Limit should stop after exactly 3 matches"
    );
}

#[tokio::test]
async fn test_find_packages_star_pattern_matches_all() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "aaa", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "bbb", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("*".to_string()),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();
    assert_eq!(results.len(), 2, "* should match all packages");
}

#[tokio::test]
async fn test_find_packages_no_match_returns_empty() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "python", "3.9.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("nonexistent_pkg".to_string()),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();
    assert!(results.is_empty());
}
