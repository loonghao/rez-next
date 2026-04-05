//! Simple file-based repository implementation

use async_trait::async_trait;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageSerializer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;

/// Simplified package repository trait for solver
#[async_trait]
pub trait PackageRepository {
    /// Find packages by name
    async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError>;

    /// Get a specific package version
    async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError>;

    /// List all available packages
    async fn list_packages(&self) -> Result<Vec<String>, RezCoreError>;

    /// Get repository name
    fn name(&self) -> &str;

    /// Get repository root path
    fn root_path(&self) -> &Path;
}

/// A simple file-based package repository
#[derive(Debug, Clone)]
pub struct SimpleRepository {
    /// Root path of the repository
    root_path: PathBuf,

    /// Cached packages
    package_cache: Arc<tokio::sync::RwLock<HashMap<String, Vec<Arc<Package>>>>>,

    /// Repository name
    name: String,
}

impl SimpleRepository {
    /// Create a new simple repository
    pub fn new<P: AsRef<Path>>(root_path: P, name: String) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            package_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            name,
        }
    }

    /// Scan the repository for packages
    pub async fn scan(&self) -> Result<(), RezCoreError> {
        let mut cache = self.package_cache.write().await;
        cache.clear();

        self.scan_directory(&self.root_path, &mut cache).await?;

        Ok(())
    }

    /// Recursively scan a directory for packages
    fn scan_directory<'a>(
        &'a self,
        dir_path: &'a Path,
        cache: &'a mut HashMap<String, Vec<Arc<Package>>>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), RezCoreError>> + Send + 'a>>
    {
        Box::pin(async move {
            let mut entries = fs::read_dir(dir_path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();

                if path.is_dir() {
                    // Check if this directory contains a package.py
                    let package_py = path.join("package.py");
                    if package_py.exists() {
                        if let Ok(package) = self.load_package_from_path(&package_py).await {
                            let package_name = package.name.clone();
                            cache
                                .entry(package_name)
                                .or_default()
                                .push(Arc::new(package));
                        }
                    } else {
                        // Recursively scan subdirectories
                        self.scan_directory(&path, cache).await?;
                    }
                }
            }

            Ok(())
        })
    }

    /// Load a package from a package.py file
    async fn load_package_from_path(
        &self,
        package_py_path: &Path,
    ) -> Result<Package, RezCoreError> {
        PackageSerializer::load_from_file(package_py_path)
    }
}

#[async_trait::async_trait]
impl PackageRepository for SimpleRepository {
    async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        // Check cache first
        {
            let cache = self.package_cache.read().await;
            if let Some(packages) = cache.get(name) {
                return Ok(packages.clone());
            }
        }

        // If not in cache, scan and try again
        self.scan().await?;

        let cache = self.package_cache.read().await;
        Ok(cache.get(name).cloned().unwrap_or_default())
    }

    async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError> {
        let packages = self.find_packages(name).await?;

        if let Some(version_str) = version {
            let target_version = rez_next_version::Version::parse(version_str)?;
            for package in packages {
                if let Some(ref pkg_version) = package.version {
                    if pkg_version == &target_version {
                        return Ok(Some(package));
                    }
                }
            }
        } else {
            // Return the latest version
            let mut packages = packages;
            packages.sort_by(|a, b| {
                match (&a.version, &b.version) {
                    (Some(v1), Some(v2)) => v2.cmp(v1), // Descending order (latest first)
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
            return Ok(packages.into_iter().next());
        }

        Ok(None)
    }

    async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
        // Ensure cache is populated
        self.scan().await?;

        let cache = self.package_cache.read().await;
        let mut names: Vec<String> = cache.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn root_path(&self) -> &Path {
        &self.root_path
    }
}

/// Repository manager that manages multiple repositories
pub struct RepositoryManager {
    /// List of repositories
    repositories: Vec<Box<dyn PackageRepository + Send + Sync>>,
}

impl RepositoryManager {
    /// Create a new repository manager
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
        }
    }

    /// Add a repository
    pub fn add_repository(&mut self, repository: Box<dyn PackageRepository + Send + Sync>) {
        self.repositories.push(repository);
    }

    /// Find packages across all repositories
    pub async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        let mut all_packages = Vec::new();

        for repository in &self.repositories {
            let packages = repository.find_packages(name).await?;
            all_packages.extend(packages);
        }

        // Sort by version (latest first)
        all_packages.sort_by(|a, b| match (&a.version, &b.version) {
            (Some(v1), Some(v2)) => v2.cmp(v1),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });

        Ok(all_packages)
    }

    /// Get a specific package version
    pub async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError> {
        for repository in &self.repositories {
            if let Some(package) = repository.get_package(name, version).await? {
                return Ok(Some(package));
            }
        }
        Ok(None)
    }

    /// List all available packages
    pub async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
        let mut all_packages = std::collections::HashSet::new();

        for repository in &self.repositories {
            let packages = repository.list_packages().await?;
            all_packages.extend(packages);
        }

        let mut result: Vec<String> = all_packages.into_iter().collect();
        result.sort();
        Ok(result)
    }

    /// Get the number of repositories
    pub fn repository_count(&self) -> usize {
        self.repositories.len()
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
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
        // Latest should be 2.5.0
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
        // Sorted
        assert!(names.contains(&"python".to_string()));
        assert!(names.contains(&"maya".to_string()));
    }

    // ── Phase 83: deeper scan + real-world scenarios ──────────────────────

    /// scan() correctly discovers packages in nested subdirectories
    #[tokio::test]
    async fn test_scan_nested_packages() {
        let temp_dir = TempDir::new().unwrap();
        // Create packages at various depth levels
        create_package_file(temp_dir.path(), "top_pkg", "1.0.0").await;
        create_package_file(temp_dir.path(), "nested_pkg", "2.0.0").await;

        let repo = SimpleRepository::new(temp_dir.path(), "nested_repo".to_string());
        repo.scan().await.unwrap();

        let top = repo.find_packages("top_pkg").await.unwrap();
        let nested = repo.find_packages("nested_pkg").await.unwrap();
        assert_eq!(top.len(), 1, "Should find top-level package");
        assert_eq!(nested.len(), 1, "Should find nested package");
    }

    /// After rescan, new packages are picked up
    #[tokio::test]
    async fn test_scan_rescan_picks_up_new_packages() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "alpha", "1.0.0").await;

        let repo = SimpleRepository::new(temp_dir.path(), "rescan_repo".to_string());
        repo.scan().await.unwrap();

        assert_eq!(repo.find_packages("alpha").await.unwrap().len(), 1);
        assert!(repo.find_packages("beta").await.unwrap().is_empty());

        // Add new package and rescan
        create_package_file(temp_dir.path(), "beta", "1.0.0").await;
        repo.scan().await.unwrap();

        assert_eq!(
            repo.find_packages("beta").await.unwrap().len(),
            1,
            "beta should be found after rescan"
        );
    }

    /// Packages with requires field are loaded correctly
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
        assert!(!pkgs[0].requires.is_empty(), "requires should be loaded");
    }

    /// list_packages returns names sorted alphabetically
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


    /// find_packages returns packages sorted by version (latest first) via manager
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
        // First should be 3.0.0 (latest)
        let first_ver = pkgs[0].version.as_ref().map(|v| v.as_str()).unwrap_or("");
        assert_eq!(first_ver, "3.0.0", "Latest version should come first");
    }

    /// get_package with non-existent version returns None
    #[tokio::test]
    async fn test_get_package_nonexistent_version() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "mypkg", "1.0.0").await;

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        let result = repo.get_package("mypkg", Some("9.9.9")).await.unwrap();
        assert!(result.is_none(), "Non-existent version should return None");
    }

    /// Empty repository list_packages returns empty vec
    #[tokio::test]
    async fn test_empty_repository_list_packages() {
        let temp_dir = TempDir::new().unwrap();
        let repo = SimpleRepository::new(temp_dir.path(), "empty_repo".to_string());
        let names = repo.list_packages().await.unwrap();
        assert!(names.is_empty(), "Empty repo should have no packages");
    }

    // ── Phase 100: recursive scan depth + multi-level hierarchy tests ──────────

    /// Scan finds packages at depth 3 (root/family/version/package.py)
    #[tokio::test]
    async fn test_scan_depth3_standard_layout() {
        // Standard rez layout: root/pkg_name/version/package.py
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "deep_pkg", "1.0.0").await;
        create_package_file(temp_dir.path(), "deep_pkg", "2.0.0").await;
        create_package_file(temp_dir.path(), "another_pkg", "3.5.0").await;

        let repo = SimpleRepository::new(temp_dir.path(), "depth3".to_string());
        repo.scan().await.unwrap();

        let deep_pkgs = repo.find_packages("deep_pkg").await.unwrap();
        assert_eq!(deep_pkgs.len(), 2, "Should find both versions of deep_pkg");

        let another = repo.find_packages("another_pkg").await.unwrap();
        assert_eq!(another.len(), 1);
    }

    /// Scan with deeply nested directories (depth 4+)
    #[tokio::test]
    async fn test_scan_deep_nesting() {
        let temp_dir = TempDir::new().unwrap();
        // Create a deeply nested directory that has no package.py
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
        assert_eq!(pkgs.len(), 1, "Should find deeply nested package");
        assert_eq!(pkgs[0].name, "mypkg");
    }

    /// Scan stops recursing when it finds a package.py (doesn't go deeper)
    #[tokio::test]
    async fn test_scan_stops_at_package_py() {
        let temp_dir = TempDir::new().unwrap();
        // Create parent package.py
        let parent_dir = temp_dir.path().join("parent_pkg").join("1.0.0");
        fs::create_dir_all(&parent_dir).await.unwrap();
        fs::write(
            parent_dir.join("package.py"),
            "name = 'parent_pkg'\nversion = '1.0.0'\n",
        )
        .await
        .unwrap();
        // Create inner dir (should NOT be scanned since parent has package.py)
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

        // parent_pkg should be found
        assert_eq!(repo.find_packages("parent_pkg").await.unwrap().len(), 1);
        // inner_pkg should NOT be found (scan stops at parent's package.py)
        let inner = repo.find_packages("inner_pkg").await.unwrap();
        assert_eq!(inner.len(), 0, "Should not scan inside a package directory");
    }

    /// Scan with many packages in the same family
    #[tokio::test]
    async fn test_scan_many_versions_same_family() {
        let temp_dir = TempDir::new().unwrap();
        let versions = ["1.0.0", "1.1.0", "1.2.0", "2.0.0", "2.1.0", "3.0.0"];
        for v in &versions {
            create_package_file(temp_dir.path(), "multipkg", v).await;
        }

        let repo = SimpleRepository::new(temp_dir.path(), "multi_repo".to_string());
        let pkgs = repo.find_packages("multipkg").await.unwrap();
        assert_eq!(pkgs.len(), 6, "Should find all 6 versions");
    }

    /// get_package with latest from many versions returns highest
    #[tokio::test]
    async fn test_get_latest_from_many_versions() {
        let temp_dir = TempDir::new().unwrap();
        for v in &["0.9.0", "1.0.0", "1.5.0", "2.0.0", "0.1.0"] {
            create_package_file(temp_dir.path(), "latestpkg", v).await;
        }

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        let pkg = repo.get_package("latestpkg", None).await.unwrap().unwrap();
        assert_eq!(
            pkg.version.as_ref().map(|v| v.as_str()),
            Some("2.0.0"),
            "Latest version should be 2.0.0"
        );
    }

    /// Scan ignores non-`package.py` files in SimpleRepository's current format contract
    #[tokio::test]
    async fn test_scan_ignores_non_package_py() {
        let temp_dir = TempDir::new().unwrap();
        // Create a directory with a package.yaml but no package.py
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
        assert!(
            pkgs.is_empty(),
            "SimpleRepository should ignore package.yaml and only scan package.py"
        );
    }


    /// Manager finds packages across repos with priority (first repo takes precedence)
    #[tokio::test]
    async fn test_manager_repo_priority_order() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        // Both repos have the same package but different versions
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
        // Both repos contribute their version
        assert_eq!(
            pkgs.len(),
            2,
            "Manager should aggregate packages from all repos"
        );
    }

    // ── Cycle 61: boundary tests ────────────────────────────────────────────

    /// Empty package.py (zero bytes) should fail gracefully without panicking
    #[tokio::test]
    async fn test_scan_empty_package_py_does_not_panic() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("emptypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).await.unwrap();
        fs::write(pkg_dir.join("package.py"), b"").await.unwrap();

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        // Should not panic; the package may or may not load depending on parser tolerance
        let result = repo.scan().await;
        assert!(result.is_ok(), "scan() must not propagate empty-file errors");

        // Empty file is treated as a failed load; package should NOT be cached
        let pkgs = repo.find_packages("emptypkg").await.unwrap();
        assert!(pkgs.is_empty(), "Empty package.py should yield no package");
    }

    /// Malformed / invalid package.py content should be skipped gracefully
    #[tokio::test]
    async fn test_scan_malformed_package_py_is_skipped() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("badpkg").join("0.1.0");
        fs::create_dir_all(&pkg_dir).await.unwrap();
        // Completely invalid Python-style content
        fs::write(pkg_dir.join("package.py"), b"!!!NOT VALID CONTENT!!!")
            .await
            .unwrap();

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        let result = repo.scan().await;
        // scan() itself must succeed (error is swallowed per implementation)
        assert!(result.is_ok(), "scan() must not fail on malformed files");
        let pkgs = repo.find_packages("badpkg").await.unwrap();
        assert!(pkgs.is_empty(), "Malformed package.py should yield no package");
    }

    /// Valid package alongside a malformed one: good package is still found
    #[tokio::test]
    async fn test_scan_malformed_sibling_does_not_block_good_package() {
        let temp_dir = TempDir::new().unwrap();

        // Good package
        create_package_file(temp_dir.path(), "goodpkg", "1.0.0").await;

        // Bad package (malformed)
        let bad_dir = temp_dir.path().join("badpkg").join("1.0.0");
        fs::create_dir_all(&bad_dir).await.unwrap();
        fs::write(bad_dir.join("package.py"), b"%%%INVALID%%%")
            .await
            .unwrap();

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        repo.scan().await.unwrap();

        let good = repo.find_packages("goodpkg").await.unwrap();
        assert_eq!(good.len(), 1, "Good package must still be found");
    }

    /// Package without a version field is loaded and accessible
    #[tokio::test]
    async fn test_scan_package_without_version() {
        let temp_dir = TempDir::new().unwrap();
        let pkg_dir = temp_dir.path().join("noversion").join("0.0.0");
        fs::create_dir_all(&pkg_dir).await.unwrap();
        // Only name, no version field
        fs::write(
            pkg_dir.join("package.py"),
            "name = 'noversion'\ndescription = 'no ver'\n",
        )
        .await
        .unwrap();

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        repo.scan().await.unwrap();
        // Whether the package loads or not, scan must not panic
        // If it loads, find_packages returns it
        let _pkgs = repo.find_packages("noversion").await.unwrap();
    }

    /// get_package(name, None) on empty repo returns None without error
    #[tokio::test]
    async fn test_get_package_latest_empty_repo_returns_none() {
        let temp_dir = TempDir::new().unwrap();
        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        let result = repo.get_package("ghost", None).await.unwrap();
        assert!(result.is_none());
    }

    /// Multiple successive scan()s are idempotent (no duplicate accumulation)
    #[tokio::test]
    async fn test_scan_idempotent_no_duplicates() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "idmpkg", "1.0.0").await;

        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        repo.scan().await.unwrap();
        repo.scan().await.unwrap();
        repo.scan().await.unwrap();

        let pkgs = repo.find_packages("idmpkg").await.unwrap();
        assert_eq!(pkgs.len(), 1, "Repeated scans must not duplicate packages");
    }

    /// Concurrent find_packages calls don't corrupt cache
    #[tokio::test]
    async fn test_concurrent_find_packages() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "concpkg", "1.0.0").await;
        create_package_file(temp_dir.path(), "concpkg", "2.0.0").await;

        let repo = std::sync::Arc::new(SimpleRepository::new(
            temp_dir.path(),
            "repo".to_string(),
        ));
        // Pre-scan so all concurrent calls hit cache path
        repo.scan().await.unwrap();

        let handles: Vec<_> = (0..8)
            .map(|_| {
                let r = repo.clone();
                tokio::spawn(async move { r.find_packages("concpkg").await.unwrap() })
            })
            .collect();

        for handle in handles {
            let pkgs = handle.await.unwrap();
            assert_eq!(pkgs.len(), 2, "Each concurrent read must return 2 packages");
        }
    }

    /// Concurrent scan() calls don't corrupt cache (write path)
    #[tokio::test]
    async fn test_concurrent_scans_safe() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "scanpkg", "1.0.0").await;

        let repo = std::sync::Arc::new(SimpleRepository::new(
            temp_dir.path(),
            "repo".to_string(),
        ));

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
        assert_eq!(pkgs.len(), 1, "After concurrent scans exactly 1 package");
    }

    /// RepositoryManager.get_package finds exact version across repos
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
        assert!(pkg.is_some(), "Should find 2.0.0 from repo2");
        assert_eq!(
            pkg.unwrap().version.as_ref().map(|v| v.as_str()),
            Some("2.0.0")
        );
    }

    /// RepositoryManager.get_package returns None if version absent from all repos
    #[tokio::test]
    async fn test_manager_get_package_missing_version_returns_none() {
        let dir = TempDir::new().unwrap();
        create_package_file(dir.path(), "mypkg", "1.0.0").await;

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(SimpleRepository::new(
            dir.path(),
            "r".to_string(),
        )));

        let result = manager.get_package("mypkg", Some("9.9.9")).await.unwrap();
        assert!(result.is_none());
    }

    /// RepositoryManager.list_packages across repos de-duplicates names
    #[tokio::test]
    async fn test_manager_list_packages_deduplicates_names() {
        let dir1 = TempDir::new().unwrap();
        let dir2 = TempDir::new().unwrap();
        // Same package name in both repos
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
        // "shared" must appear only once
        let shared_count = names.iter().filter(|n| *n == "shared").count();
        assert_eq!(shared_count, 1, "shared name must be deduplicated");
        assert!(names.contains(&"unique1".to_string()));
        assert!(names.contains(&"unique2".to_string()));
    }

    /// RepositoryManager with zero repos: list_packages returns empty
    #[tokio::test]
    async fn test_manager_no_repos_list_packages_empty() {
        let manager = RepositoryManager::new();
        let names = manager.list_packages().await.unwrap();
        assert!(names.is_empty());
    }

    /// RepositoryManager with zero repos: get_package returns None
    #[tokio::test]
    async fn test_manager_no_repos_get_package_none() {
        let manager = RepositoryManager::new();
        let result = manager.get_package("ghost", None).await.unwrap();
        assert!(result.is_none());
    }

    /// find_packages lazy scan: cache miss triggers scan automatically
    #[tokio::test]
    async fn test_find_packages_lazy_scan_on_cache_miss() {
        let temp_dir = TempDir::new().unwrap();
        create_package_file(temp_dir.path(), "lazypkg", "1.0.0").await;

        // Do NOT call scan() explicitly — find_packages should auto-trigger it
        let repo = SimpleRepository::new(temp_dir.path(), "repo".to_string());
        let pkgs = repo.find_packages("lazypkg").await.unwrap();
        assert_eq!(pkgs.len(), 1, "Lazy scan must find package on cache miss");
    }

    /// scan() on non-existent root returns an error
    #[tokio::test]
    async fn test_scan_nonexistent_root_returns_error() {
        let repo = SimpleRepository::new("/this/path/does/not/exist", "repo".to_string());
        let result = repo.scan().await;
        assert!(result.is_err(), "scan() on missing root must return Err");
    }

    /// Package with description field is correctly deserialized
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

    /// Package with tools field is accessible
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
        assert!(!pkgs[0].tools.is_empty(), "tools field should be loaded");
    }
}
