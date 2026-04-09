//! FileSystemRepository tests — variants, package.py loading, and concurrent access.
//! Split from filesystem_tests.rs (Cycle 145) to keep file size ≤500 lines.

use super::*;
use crate::{PackageSearchCriteria, Repository};
use tempfile::TempDir;
use tokio::fs;

// ── helpers (mirrored from filesystem_tests.rs) ───────────────────────────────

async fn make_yaml_pkg(root: &std::path::Path, name: &str, version: &str) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let content = format!(
        "name: \"{}\"\nversion: \"{}\"\ndescription: \"Test\"\n",
        name, version
    );
    fs::write(dir.join("package.yaml"), content).await.unwrap();
}

async fn make_yaml_pkg_with_variants(
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

async fn make_py_pkg(root: &std::path::Path, name: &str, version: &str) {
    let dir = root.join(name).join(version);
    fs::create_dir_all(&dir).await.unwrap();
    let content = format!(
        "name = \"{}\"\nversion = \"{}\"\ndescription = \"Test package\"\n",
        name, version
    );
    fs::write(dir.join("package.py"), content).await.unwrap();
}

async fn make_py_pkg_with_variants(
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

// ── get_package_variants ──────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_package_variants_returns_empty_when_none() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "simplepkg", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let variants = repo.get_package_variants("simplepkg", None).await.unwrap();
    assert!(
        variants.is_empty(),
        "Package without variants should return empty list"
    );
}

// ── get_stats ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_get_stats_reflects_scanned_packages() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "pkg_a", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "pkg_b", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.package_count, 2, "Stats should reflect 2 packages");
    assert!(
        stats.last_scan_time.is_some(),
        "last_scan_time should be set"
    );
    assert!(
        stats.last_scan_duration_ms.is_some(),
        "scan duration should be recorded"
    );
}

// ── matches_pattern (internal logic via find_packages) ───────────────────────

#[tokio::test]
async fn test_matches_pattern_question_mark_wildcard() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "lib_a", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "lib_b", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "libxx", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("lib_?".to_string()),
        ..Default::default()
    };
    let results = repo.find_packages(&criteria).await.unwrap();

    assert_eq!(results.len(), 2, "? should match exactly one char");
}

// ── variants field (YAML) ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_variants_in_yaml_are_discovered() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg_with_variants(
        tmp.path(),
        "mypkg",
        "1.0.0",
        &[&["python-3.9"], &["python-3.10"]],
    )
    .await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v = rez_next_version::Version::parse("1.0.0").unwrap();
    let variants = repo.get_package_variants("mypkg", Some(&v)).await.unwrap();
    assert_eq!(variants.len(), 2, "Should discover 2 variants");
    assert!(variants.contains(&"variant_0".to_string()));
    assert!(variants.contains(&"variant_1".to_string()));
}

#[tokio::test]
async fn test_variants_stats_count_is_updated() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg_with_variants(
        tmp.path(),
        "multi",
        "2.0.0",
        &[&["python-3.9"], &["python-3.10"], &["python-3.11"]],
    )
    .await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.variant_count, 3, "Stats should count 3 variants");
}

#[tokio::test]
async fn test_variants_key_without_version_returns_empty() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg_with_variants(tmp.path(), "pkg", "1.0.0", &[&["python-3.9"]]).await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let variants = repo.get_package_variants("pkg", None).await.unwrap();
    assert!(
        variants.is_empty(),
        "No-version key should not match versioned entry"
    );
}

// ── package.py format loading ─────────────────────────────────────────────────

#[tokio::test]
async fn test_package_py_is_discovered() {
    let tmp = TempDir::new().unwrap();
    make_py_pkg(tmp.path(), "pypkg", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names = repo.get_package_names().await.unwrap();
    assert!(
        names.contains(&"pypkg".to_string()),
        "package.py should be discovered; got: {:?}",
        names
    );
}

#[tokio::test]
async fn test_package_py_correct_version_loaded() {
    let tmp = TempDir::new().unwrap();
    make_py_pkg(tmp.path(), "pyver", "2.5.1").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v = rez_next_version::Version::parse("2.5.1").unwrap();
    let pkg = repo.get_package("pyver", Some(&v)).await.unwrap();
    assert!(pkg.is_some(), "Should find pyver 2.5.1");
    assert_eq!(
        pkg.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("2.5.1")
    );
}

#[tokio::test]
async fn test_package_py_and_yaml_coexist() {
    let tmp = TempDir::new().unwrap();
    make_py_pkg(tmp.path(), "alpha", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "beta", "2.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names = repo.get_package_names().await.unwrap();
    assert!(
        names.contains(&"alpha".to_string()),
        "py pkg should be found"
    );
    assert!(
        names.contains(&"beta".to_string()),
        "yaml pkg should be found"
    );
    assert_eq!(names.len(), 2);
}

#[tokio::test]
async fn test_package_py_variants_discovered() {
    let tmp = TempDir::new().unwrap();
    make_py_pkg_with_variants(
        tmp.path(),
        "variantpkg",
        "1.0.0",
        &[
            &["python-3.9", "platform-linux"],
            &["python-3.10", "platform-linux"],
        ],
    )
    .await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let v = rez_next_version::Version::parse("1.0.0").unwrap();
    let variants = repo
        .get_package_variants("variantpkg", Some(&v))
        .await
        .unwrap();
    assert_eq!(variants.len(), 2, "package.py variants should be parsed");
}

#[tokio::test]
async fn test_package_py_multiple_versions() {
    let tmp = TempDir::new().unwrap();
    for v in &["1.0.0", "1.1.0", "2.0.0"] {
        make_py_pkg(tmp.path(), "pylib", v).await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let versions = repo.get_package_versions("pylib").await.unwrap();
    assert_eq!(versions.len(), 3, "All three .py versions should be found");
    assert_eq!(versions[0].as_str(), "2.0.0", "Latest first");
}

#[tokio::test]
async fn test_package_py_stats_package_count() {
    let tmp = TempDir::new().unwrap();
    make_py_pkg(tmp.path(), "p1", "1.0.0").await;
    make_py_pkg(tmp.path(), "p2", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "p3", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let stats = repo.get_stats().await.unwrap();
    assert_eq!(stats.package_count, 3, "Mixed py+yaml packages counted");
}

// ── Cycle 64: concurrent access tests ────────────────────────────────────────

/// Concurrent initialize calls on the same repo must not corrupt state.
#[tokio::test]
async fn test_concurrent_initialize_is_safe() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "alpha", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "beta", "2.0.0").await;

    use std::sync::Arc;
    use tokio::sync::Mutex;

    let repo = Arc::new(Mutex::new(FileSystemRepository::new(
        tmp.path().to_path_buf(),
        Some("concurrent_repo".to_string()),
    )));

    let mut handles = Vec::new();
    for _ in 0..4 {
        let repo_clone = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let mut r = repo_clone.lock().await;
            r.initialize().await
        }));
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    let repo_guard = repo.lock().await;
    let criteria_alpha = PackageSearchCriteria {
        name_pattern: Some("alpha".to_string()),
        ..Default::default()
    };
    let criteria_beta = PackageSearchCriteria {
        name_pattern: Some("beta".to_string()),
        ..Default::default()
    };
    let pkgs_alpha = repo_guard.find_packages(&criteria_alpha).await.unwrap();
    let pkgs_beta = repo_guard.find_packages(&criteria_beta).await.unwrap();
    assert_eq!(
        pkgs_alpha.len(),
        1,
        "alpha should be found after concurrent init"
    );
    assert_eq!(
        pkgs_beta.len(),
        1,
        "beta should be found after concurrent init"
    );
}

/// Concurrent find_packages calls return consistent results.
#[tokio::test]
async fn test_concurrent_find_packages_consistent() {
    let tmp = TempDir::new().unwrap();
    for i in 0..5 {
        make_yaml_pkg(tmp.path(), "pkg", &format!("1.{}.0", i)).await;
    }

    use std::sync::Arc;

    let repo = Arc::new(tokio::sync::RwLock::new(FileSystemRepository::new(
        tmp.path().to_path_buf(),
        Some("concurrent_read_repo".to_string()),
    )));

    repo.write().await.initialize().await.unwrap();

    let mut handles = Vec::new();
    for _ in 0..8 {
        let repo_clone = Arc::clone(&repo);
        handles.push(tokio::spawn(async move {
            let r = repo_clone.write().await;
            let criteria = PackageSearchCriteria {
                name_pattern: Some("pkg".to_string()),
                ..Default::default()
            };
            r.find_packages(&criteria).await
        }));
    }

    for handle in handles {
        let result = handle.await.unwrap().unwrap();
        assert_eq!(
            result.len(),
            5,
            "All 5 versions should be found in every concurrent call"
        );
    }
}

/// get_package_names returns names in sorted order.
#[tokio::test]
async fn test_get_package_names_sorted() {
    let tmp = TempDir::new().unwrap();
    for name in &["zzz", "aaa", "mmm", "bbb"] {
        make_yaml_pkg(tmp.path(), name, "1.0.0").await;
    }

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let names = repo.get_package_names().await.unwrap();
    let mut expected = names.clone();
    expected.sort();
    assert_eq!(
        names, expected,
        "get_package_names should return sorted names"
    );
}

/// find_packages with exact version pattern returns only matching version.
#[tokio::test]
async fn test_find_packages_version_filter() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "mypkg", "1.0.0").await;
    make_yaml_pkg(tmp.path(), "mypkg", "2.0.0").await;
    make_yaml_pkg(tmp.path(), "mypkg", "3.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("mypkg".to_string()),
        version_requirement: Some("2.0.0".to_string()),
        ..Default::default()
    };
    let pkgs = repo.find_packages(&criteria).await.unwrap();
    assert_eq!(
        pkgs.len(),
        1,
        "Version filter should return exactly one package"
    );
    assert_eq!(
        pkgs[0].version.as_ref().map(|v| v.as_str()).unwrap_or(""),
        "2.0.0"
    );
}

/// Package not in repo returns empty find_packages result.
#[tokio::test]
async fn test_find_packages_unknown_name_empty() {
    let tmp = TempDir::new().unwrap();
    make_yaml_pkg(tmp.path(), "known_pkg", "1.0.0").await;

    let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
    repo.initialize().await.unwrap();

    let criteria = PackageSearchCriteria {
        name_pattern: Some("unknown_pkg".to_string()),
        ..Default::default()
    };
    let pkgs = repo.find_packages(&criteria).await.unwrap();
    assert!(
        pkgs.is_empty(),
        "Unknown package name should return empty vec"
    );
}
