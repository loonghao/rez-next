//! Simple file-based repository implementation

use crate::REZ_PACKAGE_FILENAMES;
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
                    // Check if this directory contains any supported package descriptor.
                    // Priority follows the canonical `REZ_PACKAGE_FILENAMES` order.
                    let pkg_file = REZ_PACKAGE_FILENAMES
                        .iter()
                        .map(|name| path.join(name))
                        .find(|p| p.exists());

                    if let Some(pkg_path) = pkg_file {
                        if let Ok(package) = self.load_package_from_path(&pkg_path).await {
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

    /// Load a package from a supported package descriptor file.
    async fn load_package_from_path(
        &self,
        descriptor_path: &Path,
    ) -> Result<Package, RezCoreError> {
        PackageSerializer::load_from_file(descriptor_path)
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
#[path = "simple_repository_tests.rs"]
mod tests;
