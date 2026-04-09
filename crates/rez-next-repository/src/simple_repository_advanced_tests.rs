//! Advanced tests for SimpleRepository and RepositoryManager.
//! Covers: concurrent access, manager cross-repo, multi-format packages.
//! Split from simple_repository_tests.rs (Cycle 146) to keep file size ≤500 lines.

use super::*;
use tempfile::TempDir;
use tokio::fs;

async fn create_package_file(dir: &std::path::Path, name: &str, version: &str) {
    let pkg_dir = dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).await.unwrap();
    let content = format!(
        "name = \"{}\"\nversion = \"{}\"\ndescription = \"Test\"\n",
        name, version
    );
    fs::write(pkg_dir.join("package.py"), content)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_concurrent_find_packages() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "concpkg", "1.0.0").await;
    create_package_file(temp_dir.path(), "concpkg", "2.0.0").await;

    let repo = std::sync::Arc::new(SimpleRepository::new(temp_dir.path(), "repo".to_string()));
    repo.scan().await.unwrap();

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let r = repo.clone();
            tokio::spawn(async move { r.find_packages("concpkg").await.unwrap() })
        })
        .collect();

    for handle in handles {
        let pkgs = handle.await.unwrap();
        assert_eq!(pkgs.len(), 2);
    }
}

#[tokio::test]
async fn test_concurrent_scans_safe() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "scanpkg", "1.0.0").await;

    let repo = std::sync::Arc::new(SimpleRepository::new(temp_dir.path(), "repo".to_string()));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let r = repo.clone();
            tokio::spawn(async move { r.scan().await.unwrap() })
        })
        .collect();

    for h in handles {
        h.await.unwrap();
    }

    let pkgs = repo.find_packages("scanpkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
}

#[tokio::test]
async fn test_manager_get_package_exact_version_across_repos() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    create_package_file(dir1.path(), "crosspkg", "1.0.0").await;
    create_package_file(dir2.path(), "crosspkg", "2.0.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        dir1.path(),
        "r1".to_string(),
    )));
    manager.add_repository(Box::new(SimpleRepository::new(
        dir2.path(),
        "r2".to_string(),
    )));

    let pkg = manager
        .get_package("crosspkg", Some("2.0.0"))
        .await
        .unwrap();
    assert!(pkg.is_some());
    assert_eq!(
        pkg.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("2.0.0")
    );
}

#[tokio::test]
async fn test_manager_get_package_missing_version_returns_none() {
    let dir = TempDir::new().unwrap();
    create_package_file(dir.path(), "mypkg", "1.0.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(dir.path(), "r".to_string())));

    let result = manager.get_package("mypkg", Some("9.9.9")).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_manager_list_packages_deduplicates_names() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    create_package_file(dir1.path(), "shared", "1.0.0").await;
    create_package_file(dir2.path(), "shared", "2.0.0").await;
    create_package_file(dir1.path(), "unique1", "1.0.0").await;
    create_package_file(dir2.path(), "unique2", "1.0.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        dir1.path(),
        "r1".to_string(),
    )));
    manager.add_repository(Box::new(SimpleRepository::new(
        dir2.path(),
        "r2".to_string(),
    )));

    let names = manager.list_packages().await.unwrap();
    let shared_count = names.iter().filter(|n| *n == "shared").count();
    assert_eq!(shared_count, 1);
    assert!(names.contains(&"unique1".to_string()));
    assert!(names.contains(&"unique2".to_string()));
}

#[tokio::test]
async fn test_manager_no_repos_list_packages_empty() {
    let manager = RepositoryManager::new();
    let names = manager.list_packages().await.unwrap();
    assert!(names.is_empty());
}

#[tokio::test]
async fn test_manager_no_repos_get_package_none() {
    let manager = RepositoryManager::new();
    let result = manager.get_package("ghost", None).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_find_packages_lazy_scan_on_cache_miss() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "lazypkg", "1.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let pkgs = repo.find_packages("lazypkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
}

#[tokio::test]
async fn test_scan_nonexistent_root_returns_error() {
    let repo = SimpleRepository::new("/this/path/does/not/exist", "repo".to_string());
    let result = repo.scan().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_scan_package_with_description() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("descpkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    fs::write(
        pkg_dir.join("package.py"),
        "name = 'descpkg'\nversion = '1.0.0'\ndescription = 'A helpful package'\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();
    let pkgs = repo.find_packages("descpkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].description.as_deref(), Some("A helpful package"));
}

#[tokio::test]
async fn test_scan_package_with_tools() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("toolpkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    fs::write(
        pkg_dir.join("package.py"),
        "name = 'toolpkg'\nversion = '1.0.0'\ntools = ['toolpkg_exe', 'toolpkg_cli']\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();
    let pkgs = repo.find_packages("toolpkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
    assert!(!pkgs[0].tools.is_empty());
}

// ── Multi-format descriptor discovery tests ──────────────────────────────────

/// `SimpleRepository` must discover packages in all four supported formats.
#[tokio::test]
async fn test_multi_format_json_discovered() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("jsonpkg").join("1.0.0");
    fs::create_dir_all(&dir).await.unwrap();
    fs::write(
        dir.join("package.json"),
        r#"{"name": "jsonpkg", "version": "1.0.0"}"#,
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let pkgs = repo.find_packages("jsonpkg").await.unwrap();
    assert_eq!(pkgs.len(), 1, "package.json should be discovered");
    assert_eq!(pkgs[0].name, "jsonpkg");
}

/// When multiple formats exist in the same directory, only one package should
/// be created — `package.py` takes priority over `package.yaml`.
#[tokio::test]
async fn test_multi_format_priority_py_beats_yaml_no_duplicate() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("dualfmt").join("1.0.0");
    fs::create_dir_all(&dir).await.unwrap();
    fs::write(
        dir.join("package.py"),
        "name = 'dualfmt'\nversion = '1.0.0'\ndescription = 'from python'\n",
    )
    .await
    .unwrap();
    fs::write(
        dir.join("package.yaml"),
        "name: dualfmt\nversion: 1.0.0\ndescription: from yaml\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let pkgs = repo.find_packages("dualfmt").await.unwrap();
    assert_eq!(
        pkgs.len(),
        1,
        "dual-format dir should yield exactly one package"
    );
    assert_eq!(pkgs[0].description.as_deref(), Some("from python"));
}

/// A mixed-format repository contains packages in different formats; all should
/// be discoverable via `list_packages` and `find_packages`.
#[tokio::test]
async fn test_multi_format_mixed_repository_list_all() {
    let temp_dir = TempDir::new().unwrap();

    // package.py format
    let py_dir = temp_dir.path().join("pypkg").join("1.0.0");
    fs::create_dir_all(&py_dir).await.unwrap();
    fs::write(
        py_dir.join("package.py"),
        "name = 'pypkg'\nversion = '1.0.0'\n",
    )
    .await
    .unwrap();

    // package.yaml format
    let yaml_dir = temp_dir.path().join("yamlpkg2").join("2.0.0");
    fs::create_dir_all(&yaml_dir).await.unwrap();
    fs::write(
        yaml_dir.join("package.yaml"),
        "name: yamlpkg2\nversion: 2.0.0\n",
    )
    .await
    .unwrap();

    // package.yml format
    let yml_dir = temp_dir.path().join("ymlpkg").join("3.0.0");
    fs::create_dir_all(&yml_dir).await.unwrap();
    fs::write(
        yml_dir.join("package.yml"),
        "name: ymlpkg\nversion: 3.0.0\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let names = repo.list_packages().await.unwrap();

    assert!(
        names.contains(&"pypkg".to_string()),
        "package.py pkg missing"
    );
    assert!(
        names.contains(&"yamlpkg2".to_string()),
        "package.yaml pkg missing"
    );
    assert!(
        names.contains(&"ymlpkg".to_string()),
        "package.yml pkg missing"
    );
}
