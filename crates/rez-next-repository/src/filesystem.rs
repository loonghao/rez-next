//! Filesystem-based repository implementation

use crate::{
    PackageSearchCriteria, Repository, RepositoryMetadata, RepositoryStats, RepositoryType,
};
use rez_next_common::RezCoreError;
use rez_next_package::Package;
use rez_next_version::Version;
use tracing::warn;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Filesystem repository implementation
#[derive(Debug)]
pub struct FileSystemRepository {
    /// Repository metadata
    metadata: RepositoryMetadata,
    /// Package cache (name -> versions -> package)
    package_cache: Arc<RwLock<HashMap<String, HashMap<String, Package>>>>,
    /// Variant cache (package_key -> variant names)
    variant_cache: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Repository statistics
    stats: Arc<RwLock<RepositoryStats>>,
    /// Initialization status (atomic for sync reads)
    initialized: Arc<std::sync::atomic::AtomicBool>,
}

impl FileSystemRepository {
    pub fn new(path: PathBuf, name: Option<String>) -> Self {
        let repo_name = name.unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("filesystem_repo")
                .to_string()
        });

        let metadata = RepositoryMetadata {
            name: repo_name,
            path,
            repository_type: RepositoryType::FileSystem,
            priority: 0,
            read_only: false,
            description: Some("Filesystem-based package repository".to_string()),
            config: HashMap::new(),
        };

        Self {
            metadata,
            package_cache: Arc::new(RwLock::new(HashMap::new())),
            variant_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RepositoryStats::default())),
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Get the repository path
    pub fn path(&self) -> String {
        self.metadata.path.to_string_lossy().to_string()
    }

    /// Get the repository name
    pub fn name(&self) -> String {
        self.metadata.name.clone()
    }

    /// Check if the repository is read-only
    pub fn read_only(&self) -> bool {
        self.metadata.read_only
    }

    /// Set the repository priority
    pub fn set_priority(&mut self, priority: i32) {
        self.metadata.priority = priority;
    }

    /// Set the repository as read-only
    pub fn set_read_only(&mut self, read_only: bool) {
        self.metadata.read_only = read_only;
    }
}

#[async_trait::async_trait]
impl Repository for FileSystemRepository {
    fn metadata(&self) -> &RepositoryMetadata {
        &self.metadata
    }

    async fn initialize(&mut self) -> Result<(), RezCoreError> {
        let start_time = std::time::Instant::now();

        // Check if repository path exists
        if !self.metadata.path.exists() {
            return Err(RezCoreError::Repository(format!(
                "Repository path does not exist: {}",
                self.metadata.path.display()
            )));
        }

        // Clear existing cache
        {
            let mut package_cache = self.package_cache.write().await;
            let mut variant_cache = self.variant_cache.write().await;
            package_cache.clear();
            variant_cache.clear();
        }

        // Scan for packages
        let scan_result = self.scan_packages().await?;

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.package_count = scan_result.package_count;
            stats.version_count = scan_result.version_count;
            stats.variant_count = scan_result.variant_count;
            stats.size_bytes = scan_result.size_bytes;
            stats.last_scan_time = Some(chrono::Utc::now().timestamp());
            stats.last_scan_duration_ms = Some(start_time.elapsed().as_millis() as u64);
        }

        // Mark as initialized
        self.initialized
            .store(true, std::sync::atomic::Ordering::Release);

        Ok(())
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::Acquire)
    }

    async fn refresh(&mut self) -> Result<(), RezCoreError> {
        self.initialize().await
    }

    async fn find_packages(
        &self,
        criteria: &PackageSearchCriteria,
    ) -> Result<Vec<Package>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        let mut results = Vec::new();

        for (package_name, versions) in package_cache.iter() {
            // Check name pattern
            if let Some(ref pattern) = criteria.name_pattern {
                if !self.matches_pattern(package_name, pattern) {
                    continue;
                }
            }

            for (_version_str, package) in versions.iter() {
                // Check version range
                if let Some(ref requirement) = criteria.version_requirement {
                    if let Some(ref version) = package.version {
                        // Simple string matching for now
                        if !requirement.is_empty() && version.as_str() != requirement {
                            continue;
                        }
                    }
                }

                // Check prerelease filter
                if !criteria.include_prerelease {
                    if let Some(ref version) = package.version {
                        if version.is_prerelease() {
                            continue;
                        }
                    }
                }

                // Check requirements (simplified)
                let mut satisfies_requirements = true;
                for req in &criteria.requirements {
                    if req.name == package.name {
                        if let Some(ref version) = package.version {
                            if !req.satisfied_by(version) {
                                satisfies_requirements = false;
                                break;
                            }
                        }
                    }
                }

                if satisfies_requirements {
                    results.push(package.clone());
                }

                // Check limit
                if let Some(limit) = criteria.limit {
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
            }
        }

        Ok(results)
    }

    async fn get_package(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Option<Package>, RezCoreError> {
        let package_cache = self.package_cache.read().await;

        if let Some(versions) = package_cache.get(name) {
            match version {
                Some(v) => Ok(versions.get(v.as_str()).cloned()),
                None => {
                    // Return the latest version
                    let mut version_packages: Vec<_> = versions.values().collect();
                    version_packages.sort_by(|a, b| {
                        match (&a.version, &b.version) {
                            (Some(v1), Some(v2)) => v2.cmp(v1), // Descending order
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => std::cmp::Ordering::Equal,
                        }
                    });
                    Ok(version_packages.first().map(|p| (*p).clone()))
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn get_package_versions(&self, name: &str) -> Result<Vec<Version>, RezCoreError> {
        let package_cache = self.package_cache.read().await;

        if let Some(versions) = package_cache.get(name) {
            let mut version_list: Vec<Version> = versions
                .values()
                .filter_map(|p| p.version.clone())
                .collect();

            version_list.sort_by(|a, b| b.cmp(a)); // Descending order
            Ok(version_list)
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_package_variants(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<Vec<String>, RezCoreError> {
        let variant_cache = self.variant_cache.read().await;

        let key = match version {
            Some(v) => format!("{}-{}", name, v.as_str()),
            None => name.to_string(),
        };

        Ok(variant_cache.get(&key).cloned().unwrap_or_default())
    }

    async fn package_exists(
        &self,
        name: &str,
        version: Option<&Version>,
    ) -> Result<bool, RezCoreError> {
        let package_cache = self.package_cache.read().await;

        if let Some(versions) = package_cache.get(name) {
            match version {
                Some(v) => Ok(versions.contains_key(v.as_str())),
                None => Ok(!versions.is_empty()),
            }
        } else {
            Ok(false)
        }
    }

    async fn get_package_names(&self) -> Result<Vec<String>, RezCoreError> {
        let package_cache = self.package_cache.read().await;
        Ok(package_cache.keys().cloned().collect())
    }

    async fn get_stats(&self) -> Result<RepositoryStats, RezCoreError> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }
}

/// Scan result structure
#[derive(Debug)]
struct ScanResult {
    package_count: usize,
    version_count: usize,
    variant_count: usize,
    size_bytes: u64,
}

impl FileSystemRepository {
    /// Scan the repository for packages
    async fn scan_packages(&self) -> Result<ScanResult, RezCoreError> {
        let mut package_count = 0;
        let mut version_count = 0;
        let mut variant_count = 0;
        let mut size_bytes = 0;

        let mut package_cache = self.package_cache.write().await;
        let mut variant_cache = self.variant_cache.write().await;

        // Walk through the repository directory.
        // Expected layout: root/{family}/{version}/package.yaml
        // We iterate two levels: family dirs first, then version dirs inside each family.
        let mut family_entries = fs::read_dir(&self.metadata.path).await.map_err(|e| {
            RezCoreError::Repository(format!("Failed to read repository directory: {}", e))
        })?;

        while let Some(family_entry) = family_entries.next_entry().await.map_err(|e| {
            RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
        })? {
            let family_path = family_entry.path();
            if !family_path.is_dir() {
                continue;
            }

            // First check if this directory itself contains a package file (flat layout)
            let has_direct_pkg = ["package.yaml", "package.yml", "package.json"]
                .iter()
                .any(|f| family_path.join(f).exists());

            if has_direct_pkg {
                if let Ok(scan_result) = self
                    .scan_package_directory(&family_path, &mut package_cache, &mut variant_cache)
                    .await
                {
                    package_count += scan_result.packages_found;
                    version_count += scan_result.versions_found;
                    variant_count += scan_result.variants_found;
                    size_bytes += scan_result.size_bytes;
                }
                continue;
            }

            // Otherwise treat subdirectories as versioned package dirs
            if let Ok(mut version_entries) = fs::read_dir(&family_path).await {
                while let Ok(Some(version_entry)) = version_entries.next_entry().await {
                    let version_path = version_entry.path();
                    if version_path.is_dir() {
                        if let Ok(scan_result) = self
                            .scan_package_directory(
                                &version_path,
                                &mut package_cache,
                                &mut variant_cache,
                            )
                            .await
                        {
                            package_count += scan_result.packages_found;
                            version_count += scan_result.versions_found;
                            variant_count += scan_result.variants_found;
                            size_bytes += scan_result.size_bytes;
                        }
                    }
                }
            }
        }

        Ok(ScanResult {
            package_count,
            version_count,
            variant_count,
            size_bytes,
        })
    }

    /// Scan a single package directory
    async fn scan_package_directory(
        &self,
        path: &Path,
        package_cache: &mut HashMap<String, HashMap<String, Package>>,
        variant_cache: &mut HashMap<String, Vec<String>>,
    ) -> Result<PackageScanResult, RezCoreError> {
        let mut packages_found = 0;
        let mut versions_found = 0;
        let mut variants_found = 0;
        let mut size_bytes = 0;

        // Look for package definition files
        let package_files = [
            path.join("package.py"),
            path.join("package.yaml"),
            path.join("package.yml"),
            path.join("package.json"),
        ];

        for package_file in &package_files {
            if package_file.exists() {
                match self.load_package_from_file(package_file).await {
                    Ok(package) => {
                        let package_name = package.name.clone();
                        let version_str = package
                            .version
                            .as_ref()
                            .map(|v| v.as_str())
                            .unwrap_or("latest")
                            .to_string();

                        // Add to package cache
                        package_cache
                            .entry(package_name.clone())
                            .or_default()
                            .insert(version_str.clone(), package.clone());

                        packages_found += 1;
                        versions_found += 1;

                        // Create variants if the package has variant definitions
                        if !package.variants.is_empty() {
                            let mut variant_names = Vec::new();
                            for (index, _variant_reqs) in package.variants.iter().enumerate() {
                                let variant_name = format!("variant_{}", index);
                                variant_names.push(variant_name);
                                variants_found += 1;
                            }

                            let variant_key = format!("{}-{}", package_name, version_str);
                            variant_cache.insert(variant_key, variant_names);
                        }

                        // Calculate file size
                        if let Ok(metadata) = fs::metadata(package_file).await {
                            size_bytes += metadata.len();
                        }

                        break; // Found a package file, no need to check others
                    }
                    Err(e) => {
                        warn!(
                            "Failed to load package from {}: {}",
                            package_file.display(),
                            e
                        );
                    }
                }
            }
        }

        Ok(PackageScanResult {
            packages_found,
            versions_found,
            variants_found,
            size_bytes,
        })
    }

    /// Load a package from a file
    async fn load_package_from_file(&self, path: &Path) -> Result<Package, RezCoreError> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to read package file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Simple YAML parsing for now
        if path.extension().and_then(|s| s.to_str()) == Some("yaml")
            || path.extension().and_then(|s| s.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content).map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to parse YAML package file {}: {}",
                    path.display(),
                    e
                ))
            })
        } else {
            Err(RezCoreError::Repository(format!(
                "Unsupported package file format: {}",
                path.display()
            )))
        }
    }

    /// Check if a string matches a pattern (supports basic wildcards)
    fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple wildcard matching (supports * and ?)
        let regex_pattern = pattern.replace("*", ".*").replace("?", ".");

        if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(text)
        } else {
            // Fallback to exact match
            text == pattern
        }
    }
}

/// Result of scanning a package directory
#[derive(Debug)]
struct PackageScanResult {
    packages_found: usize,
    versions_found: usize,
    variants_found: usize,
    size_bytes: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Phase 121: FileSystemRepository unit tests
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PackageSearchCriteria, Repository};
    use tempfile::TempDir;
    use tokio::fs;

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Create a minimal package.yaml under `root/name/version/package.yaml`.
    async fn make_yaml_pkg(root: &std::path::Path, name: &str, version: &str) {
        let dir = root.join(name).join(version);
        fs::create_dir_all(&dir).await.unwrap();
        let content = format!("name: \"{}\"\nversion: \"{}\"\ndescription: \"Test\"\n", name, version);
        fs::write(dir.join("package.yaml"), content).await.unwrap();
    }

    /// Create a package.yaml with explicit requires list.
    #[allow(dead_code)]
    async fn make_yaml_pkg_with_requires(
        root: &std::path::Path,
        name: &str,
        version: &str,
        requires: &[&str],
    ) {
        let dir = root.join(name).join(version);
        fs::create_dir_all(&dir).await.unwrap();
        let reqs = requires
            .iter()
            .map(|r| format!("  - \"{}\"", r))
            .collect::<Vec<_>>()
            .join("\n");
        let content = format!(
            "name: \"{}\"\nversion: \"{}\"\ndescription: \"Test\"\nrequires:\n{}\n",
            name, version, reqs
        );
        fs::write(dir.join("package.yaml"), content).await.unwrap();
    }

    // ── construction / getters / setters ─────────────────────────────────────

    #[test]
    fn test_new_uses_dir_name_as_default_name() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();
        let dir_name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
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

    // ── initialize ────────────────────────────────────────────────────────────

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

        // Add a new package and refresh
        make_yaml_pkg(tmp.path(), "beta", "2.0.0").await;
        repo.refresh().await.unwrap();

        let names_after = repo.get_package_names().await.unwrap();
        assert_eq!(names_after.len(), 2);
        assert!(names_after.contains(&"beta".to_string()));
    }

    // ── get_package / get_package_versions ────────────────────────────────────

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

    // ── package_exists ────────────────────────────────────────────────────────

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

    // ── get_package_names ─────────────────────────────────────────────────────

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

    // ── find_packages (with PackageSearchCriteria) ────────────────────────────

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

        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("python".to_string());
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

        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("py_*".to_string());
        let results = repo.find_packages(&criteria).await.unwrap();
        assert_eq!(results.len(), 2, "Wildcard should match py_core and py_utils");
    }

    #[tokio::test]
    async fn test_find_packages_with_limit() {
        let tmp = TempDir::new().unwrap();
        for i in 0..5 {
            make_yaml_pkg(tmp.path(), &format!("pkg{}", i), "1.0.0").await;
        }

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        let mut criteria = PackageSearchCriteria::default();
        criteria.limit = Some(3);
        let results = repo.find_packages(&criteria).await.unwrap();
        assert!(results.len() <= 3, "Result count should respect limit");
    }

    #[tokio::test]
    async fn test_find_packages_star_pattern_matches_all() {
        let tmp = TempDir::new().unwrap();
        make_yaml_pkg(tmp.path(), "aaa", "1.0.0").await;
        make_yaml_pkg(tmp.path(), "bbb", "1.0.0").await;

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("*".to_string());
        let results = repo.find_packages(&criteria).await.unwrap();
        assert_eq!(results.len(), 2, "* should match all packages");
    }

    #[tokio::test]
    async fn test_find_packages_no_match_returns_empty() {
        let tmp = TempDir::new().unwrap();
        make_yaml_pkg(tmp.path(), "python", "3.9.0").await;

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("nonexistent_pkg".to_string());
        let results = repo.find_packages(&criteria).await.unwrap();
        assert!(results.is_empty());
    }

    // ── get_package_variants ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_package_variants_returns_empty_when_none() {
        let tmp = TempDir::new().unwrap();
        make_yaml_pkg(tmp.path(), "simplepkg", "1.0.0").await;

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        let variants = repo.get_package_variants("simplepkg", None).await.unwrap();
        // YAML package without variants field → empty list
        assert!(variants.is_empty(), "Package without variants should return empty list");
    }

    // ── get_stats ─────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_stats_reflects_scanned_packages() {
        let tmp = TempDir::new().unwrap();
        make_yaml_pkg(tmp.path(), "pkg_a", "1.0.0").await;
        make_yaml_pkg(tmp.path(), "pkg_b", "1.0.0").await;

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        let stats = repo.get_stats().await.unwrap();
        assert_eq!(stats.package_count, 2, "Stats should reflect 2 packages");
        assert!(stats.last_scan_time.is_some(), "last_scan_time should be set");
        assert!(
            stats.last_scan_duration_ms.is_some(),
            "scan duration should be recorded"
        );
    }

    // ── matches_pattern (internal logic via find_packages) ────────────────────

    #[tokio::test]
    async fn test_matches_pattern_question_mark_wildcard() {
        let tmp = TempDir::new().unwrap();
        make_yaml_pkg(tmp.path(), "lib_a", "1.0.0").await;
        make_yaml_pkg(tmp.path(), "lib_b", "1.0.0").await;
        make_yaml_pkg(tmp.path(), "libxx", "1.0.0").await;

        let mut repo = FileSystemRepository::new(tmp.path().to_path_buf(), Some("r".to_string()));
        repo.initialize().await.unwrap();

        // `lib_?` should match lib_a and lib_b but not libxx
        let mut criteria = PackageSearchCriteria::default();
        criteria.name_pattern = Some("lib_?".to_string());
        let results = repo.find_packages(&criteria).await.unwrap();
        assert_eq!(results.len(), 2, "? should match exactly one char");
    }
}
