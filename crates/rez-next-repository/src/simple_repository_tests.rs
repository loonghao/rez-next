//! Tests for SimpleRepository and RepositoryManager.
//! Concurrent access + manager advanced + multi-format tests → simple_repository_advanced_tests.rs (Cycle 146)

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
async fn test_simple_repository_scan_and_find() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "test_package", "1.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    repo.scan().await.unwrap();

    let packages = repo.find_packages("test_package").await.unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "test_package");
}

#[tokio::test]
async fn test_simple_repository_find_missing_package() {
    let temp_dir = TempDir::new().unwrap();
    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    let packages = repo.find_packages("nonexistent").await.unwrap();
    assert!(packages.is_empty());
}

#[tokio::test]
async fn test_simple_repository_multiple_versions() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "mylib", "1.0.0").await;
    create_package_file(temp_dir.path(), "mylib", "2.0.0").await;
    create_package_file(temp_dir.path(), "mylib", "3.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    repo.scan().await.unwrap();

    let packages = repo.find_packages("mylib").await.unwrap();
    assert_eq!(packages.len(), 3);
}

#[tokio::test]
async fn test_simple_repository_get_specific_version() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "mylib", "1.0.0").await;
    create_package_file(temp_dir.path(), "mylib", "2.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    let pkg = repo.get_package("mylib", Some("1.0.0")).await.unwrap();
    assert!(pkg.is_some());
    let p = pkg.unwrap();
    assert_eq!(p.version.as_ref().map(|v| v.as_str()), Some("1.0.0"));
}

#[tokio::test]
async fn test_simple_repository_get_latest_version() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "mylib", "1.0.0").await;
    create_package_file(temp_dir.path(), "mylib", "2.5.0").await;
    create_package_file(temp_dir.path(), "mylib", "1.9.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    let pkg = repo.get_package("mylib", None).await.unwrap();
    assert!(pkg.is_some());
    assert_eq!(
        pkg.unwrap().version.as_ref().map(|v| v.as_str()),
        Some("2.5.0")
    );
}

#[tokio::test]
async fn test_simple_repository_list_packages() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "python", "3.9.0").await;
    create_package_file(temp_dir.path(), "maya", "2023.0").await;
    create_package_file(temp_dir.path(), "houdini", "19.5.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    let names = repo.list_packages().await.unwrap();
    assert!(names.contains(&"python".to_string()));
    assert!(names.contains(&"maya".to_string()));
    assert!(names.contains(&"houdini".to_string()));
}

#[tokio::test]
async fn test_simple_repository_name_and_path() {
    let temp_dir = TempDir::new().unwrap();
    let repo = SimpleRepository::new(temp_dir.path(), "my_repo".to_string());
    assert_eq!(repo.name(), "my_repo");
    assert_eq!(repo.root_path(), temp_dir.path());
}

#[tokio::test]
async fn test_repository_manager_empty() {
    let manager = RepositoryManager::new();
    assert_eq!(manager.repository_count(), 0);
    let packages = manager.find_packages("anything").await.unwrap();
    assert!(packages.is_empty());
}

#[tokio::test]
async fn test_repository_manager_add_and_find() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "test_package", "1.0.0").await;

    let mut manager = RepositoryManager::new();
    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    manager.add_repository(Box::new(repo));
    assert_eq!(manager.repository_count(), 1);

    let packages = manager.find_packages("test_package").await.unwrap();
    assert_eq!(packages.len(), 1);
    assert_eq!(packages[0].name, "test_package");
}

#[tokio::test]
async fn test_repository_manager_multiple_repos() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    create_package_file(dir1.path(), "python", "3.9.0").await;
    create_package_file(dir2.path(), "maya", "2023.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        dir1.path(),
        "repo1".to_string(),
    )));
    manager.add_repository(Box::new(SimpleRepository::new(
        dir2.path(),
        "repo2".to_string(),
    )));
    assert_eq!(manager.repository_count(), 2);

    let py_pkgs = manager.find_packages("python").await.unwrap();
    assert_eq!(py_pkgs.len(), 1);

    let maya_pkgs = manager.find_packages("maya").await.unwrap();
    assert_eq!(maya_pkgs.len(), 1);
}

#[tokio::test]
async fn test_repository_manager_list_packages() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "python", "3.9.0").await;
    create_package_file(temp_dir.path(), "maya", "2023.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        temp_dir.path(),
        "r".to_string(),
    )));

    let names = manager.list_packages().await.unwrap();
    assert!(names.contains(&"python".to_string()));
    assert!(names.contains(&"maya".to_string()));
}

#[tokio::test]
async fn test_scan_nested_packages() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "top_pkg", "1.0.0").await;
    create_package_file(temp_dir.path(), "nested_pkg", "2.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "nested_repo".to_string());
    repo.scan().await.unwrap();

    let top = repo.find_packages("top_pkg").await.unwrap();
    let nested = repo.find_packages("nested_pkg").await.unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(nested.len(), 1);
}

#[tokio::test]
async fn test_scan_rescan_picks_up_new_packages() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "alpha", "1.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "rescan_repo".to_string());
    repo.scan().await.unwrap();

    assert_eq!(repo.find_packages("alpha").await.unwrap().len(), 1);
    assert!(repo.find_packages("beta").await.unwrap().is_empty());

    create_package_file(temp_dir.path(), "beta", "1.0.0").await;
    repo.scan().await.unwrap();

    assert_eq!(repo.find_packages("beta").await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_scan_package_with_requires() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("mypkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    let content = "name = 'mypkg'\nversion = '1.0.0'\nrequires = ['python-3', 'boost-1']\n";
    fs::write(pkg_dir.join("package.py"), content)
        .await
        .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "test_repo".to_string());
    let pkgs = repo.find_packages("mypkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
    assert!(!pkgs[0].requires.is_empty());
}

#[tokio::test]
async fn test_list_packages_sorted() {
    let temp_dir = TempDir::new().unwrap();
    for name in &["zzz_pkg", "aaa_pkg", "mmm_pkg"] {
        create_package_file(temp_dir.path(), name, "1.0.0").await;
    }

    let repo = SimpleRepository::new(temp_dir.path(), "sorted_repo".to_string());
    let names = repo.list_packages().await.unwrap();
    assert_eq!(names, vec!["aaa_pkg", "mmm_pkg", "zzz_pkg"]);
}

#[tokio::test]
async fn test_manager_find_packages_sorted_latest_first() {
    let temp_dir = TempDir::new().unwrap();
    for v in &["1.0.0", "3.0.0", "2.0.0"] {
        create_package_file(temp_dir.path(), "sortpkg", v).await;
    }

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        temp_dir.path(),
        "r".to_string(),
    )));
    let pkgs = manager.find_packages("sortpkg").await.unwrap();

    assert_eq!(pkgs.len(), 3);
    let first_ver = pkgs[0].version.as_ref().map(|v| v.as_str()).unwrap_or("");
    assert_eq!(first_ver, "3.0.0");
}

#[tokio::test]
async fn test_get_package_nonexistent_version() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "mypkg", "1.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let result = repo.get_package("mypkg", Some("9.9.9")).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_empty_repository_list_packages() {
    let temp_dir = TempDir::new().unwrap();
    let repo = SimpleRepository::new(temp_dir.path(), "empty_repo".to_string());
    let names = repo.list_packages().await.unwrap();
    assert!(names.is_empty());
}

#[tokio::test]
async fn test_scan_depth3_standard_layout() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "deep_pkg", "1.0.0").await;
    create_package_file(temp_dir.path(), "deep_pkg", "2.0.0").await;
    create_package_file(temp_dir.path(), "another_pkg", "3.5.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "depth3".to_string());
    repo.scan().await.unwrap();

    let deep_pkgs = repo.find_packages("deep_pkg").await.unwrap();
    assert_eq!(deep_pkgs.len(), 2);

    let another = repo.find_packages("another_pkg").await.unwrap();
    assert_eq!(another.len(), 1);
}

#[tokio::test]
async fn test_scan_deep_nesting() {
    let temp_dir = TempDir::new().unwrap();
    let deep_dir = temp_dir
        .path()
        .join("category")
        .join("subcategory")
        .join("mypkg")
        .join("1.0.0");
    fs::create_dir_all(&deep_dir).await.unwrap();
    fs::write(
        deep_dir.join("package.py"),
        "name = 'mypkg'\nversion = '1.0.0'\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "deep_repo".to_string());
    repo.scan().await.unwrap();

    let pkgs = repo.find_packages("mypkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].name, "mypkg");
}

#[tokio::test]
async fn test_scan_stops_at_package_py() {
    let temp_dir = TempDir::new().unwrap();
    let parent_dir = temp_dir.path().join("parent_pkg").join("1.0.0");
    fs::create_dir_all(&parent_dir).await.unwrap();
    fs::write(
        parent_dir.join("package.py"),
        "name = 'parent_pkg'\nversion = '1.0.0'\n",
    )
    .await
    .unwrap();
    let inner_dir = parent_dir.join("inner_pkg").join("0.1.0");
    fs::create_dir_all(&inner_dir).await.unwrap();
    fs::write(
        inner_dir.join("package.py"),
        "name = 'inner_pkg'\nversion = '0.1.0'\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "stop_repo".to_string());
    repo.scan().await.unwrap();

    assert_eq!(repo.find_packages("parent_pkg").await.unwrap().len(), 1);
    let inner = repo.find_packages("inner_pkg").await.unwrap();
    assert_eq!(inner.len(), 0);
}

#[tokio::test]
async fn test_scan_many_versions_same_family() {
    let temp_dir = TempDir::new().unwrap();
    let versions = ["1.0.0", "1.1.0", "1.2.0", "2.0.0", "2.1.0", "3.0.0"];
    for v in &versions {
        create_package_file(temp_dir.path(), "multipkg", v).await;
    }

    let repo = SimpleRepository::new(temp_dir.path(), "multi_repo".to_string());
    let pkgs = repo.find_packages("multipkg").await.unwrap();
    assert_eq!(pkgs.len(), 6);
}

#[tokio::test]
async fn test_get_latest_from_many_versions() {
    let temp_dir = TempDir::new().unwrap();
    for v in &["0.9.0", "1.0.0", "1.5.0", "2.0.0", "0.1.0"] {
        create_package_file(temp_dir.path(), "latestpkg", v).await;
    }

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let pkg = repo.get_package("latestpkg", None).await.unwrap().unwrap();
    assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("2.0.0"));
}

#[tokio::test]
async fn test_scan_yaml_packages_discovered() {
    let temp_dir = TempDir::new().unwrap();
    let dir = temp_dir.path().join("yamlpkg").join("1.0.0");
    fs::create_dir_all(&dir).await.unwrap();
    fs::write(
        dir.join("package.yaml"),
        "name: yamlpkg\nversion: '1.0.0'\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();
    let pkgs = repo.find_packages("yamlpkg").await.unwrap();
    assert_eq!(
        pkgs.len(),
        1,
        "package.yaml should be discovered by SimpleRepository"
    );
    assert_eq!(pkgs[0].name, "yamlpkg");
}

#[tokio::test]
async fn test_manager_repo_priority_order() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    create_package_file(dir1.path(), "shared_pkg", "1.0.0").await;
    create_package_file(dir2.path(), "shared_pkg", "2.0.0").await;

    let mut manager = RepositoryManager::new();
    manager.add_repository(Box::new(SimpleRepository::new(
        dir1.path(),
        "repo1".to_string(),
    )));
    manager.add_repository(Box::new(SimpleRepository::new(
        dir2.path(),
        "repo2".to_string(),
    )));

    let pkgs = manager.find_packages("shared_pkg").await.unwrap();
    assert_eq!(pkgs.len(), 2);
}

#[tokio::test]
async fn test_scan_empty_package_py_does_not_panic() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("emptypkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    fs::write(pkg_dir.join("package.py"), b"").await.unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let result = repo.scan().await;
    assert!(result.is_ok());

    let pkgs = repo.find_packages("emptypkg").await.unwrap();
    assert!(pkgs.is_empty());
}

#[tokio::test]
async fn test_scan_malformed_package_py_is_skipped() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("badpkg").join("0.1.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    fs::write(pkg_dir.join("package.py"), b"!!!NOT VALID CONTENT!!!")
        .await
        .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let result = repo.scan().await;
    assert!(result.is_ok());
    let pkgs = repo.find_packages("badpkg").await.unwrap();
    assert!(pkgs.is_empty());
}

#[tokio::test]
async fn test_scan_malformed_sibling_does_not_block_good_package() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "goodpkg", "1.0.0").await;

    let bad_dir = temp_dir.path().join("badpkg").join("1.0.0");
    fs::create_dir_all(&bad_dir).await.unwrap();
    fs::write(bad_dir.join("package.py"), b"%%%INVALID%%%")
        .await
        .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();

    let good = repo.find_packages("goodpkg").await.unwrap();
    assert_eq!(good.len(), 1);
}

#[tokio::test]
async fn test_scan_package_without_version() {
    let temp_dir = TempDir::new().unwrap();
    let pkg_dir = temp_dir.path().join("noversion").join("0.0.0");
    fs::create_dir_all(&pkg_dir).await.unwrap();
    fs::write(
        pkg_dir.join("package.py"),
        "name = 'noversion'\ndescription = 'no ver'\n",
    )
    .await
    .unwrap();

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();
    let _pkgs = repo.find_packages("noversion").await.unwrap();
}

#[tokio::test]
async fn test_get_package_latest_empty_repo_returns_none() {
    let temp_dir = TempDir::new().unwrap();
    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    let result = repo.get_package("ghost", None).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_scan_idempotent_no_duplicates() {
    let temp_dir = TempDir::new().unwrap();
    create_package_file(temp_dir.path(), "idmpkg", "1.0.0").await;

    let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
    repo.scan().await.unwrap();
    repo.scan().await.unwrap();
    repo.scan().await.unwrap();

    let pkgs = repo.find_packages("idmpkg").await.unwrap();
    assert_eq!(pkgs.len(), 1);
}
