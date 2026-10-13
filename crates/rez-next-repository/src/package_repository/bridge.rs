//! Bridge from the full, upstream-faithful [`FilesystemPackageRepository`] to
//! the async repository trait used by [`crate::simple_repository::RepositoryManager`].
//!
//! The repository crate currently ships two `PackageRepository` traits with
//! the same name and different shapes:
//!
//! - [`crate::package_repository::PackageRepository`] — the 24-method,
//!   upstream-aligned, **sync** trait implemented by
//!   [`FilesystemPackageRepository`].
//! - [`crate::simple_repository::PackageRepository`] — the 5-method,
//!   **async** trait that `RepositoryManager` and every Rust consumer use.
//!
//! Because the traits disagree on both method names and async-ness, the full
//! implementation cannot be handed to the manager directly. This adapter
//! implements the async trait on top of the full one so that the
//! upstream-faithful repository becomes the source of truth for package data
//! without migrating every call site at once.
//!
//! The filesystem work is blocking, so it runs on [`tokio::task::spawn_blocking`]
//! rather than being polled inline: calling it directly would block the
//! runtime worker for the duration of a directory walk.

use std::sync::Arc;

use rez_next_common::RezCoreError;
use rez_next_package::Package;

use super::{FilesystemPackageRepository, PackageRepository as SyncPackageRepository};
use crate::resources::PackageFamilyResource;
use crate::simple_repository::PackageRepository as AsyncPackageRepository;

/// Exposes a [`FilesystemPackageRepository`] through the async repository trait.
#[derive(Debug, Clone)]
pub struct AsyncRepositoryAdapter {
    inner: Arc<FilesystemPackageRepository>,
}

impl AsyncRepositoryAdapter {
    /// Wrap a filesystem repository so it can be added to a
    /// [`crate::simple_repository::RepositoryManager`].
    pub fn new(repository: FilesystemPackageRepository) -> Self {
        Self {
            inner: Arc::new(repository),
        }
    }

    /// The wrapped upstream-faithful repository.
    pub fn inner(&self) -> &FilesystemPackageRepository {
        &self.inner
    }
}

/// Collect the packages of one family, newest first.
fn family_packages(
    repository: &FilesystemPackageRepository,
    family: PackageFamilyResource,
) -> Result<Vec<Arc<Package>>, RezCoreError> {
    let mut packages: Vec<Arc<Package>> = repository
        .iter_packages(&family)?
        .into_iter()
        .map(|resource| Arc::new(resource.package))
        .collect();

    // Mirror RepositoryManager's ordering so a mixed manager stays sorted.
    packages.sort_by(|a, b| match (&a.version, &b.version) {
        (Some(v1), Some(v2)) => v2.cmp(v1),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    Ok(packages)
}

#[async_trait::async_trait]
impl AsyncPackageRepository for AsyncRepositoryAdapter {
    async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        let repository = Arc::clone(&self.inner);
        let name = name.to_string();
        tokio::task::spawn_blocking(move || {
            let family = PackageFamilyResource::new(
                name,
                repository.repository_type().to_string(),
                repository.location().display().to_string(),
            );
            family_packages(&repository, family)
        })
        .await
        .map_err(|e| RezCoreError::Repository(format!("Repository task panicked: {e}")))?
    }

    async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError> {
        let packages = self.find_packages(name).await?;

        match version {
            Some(version_str) => {
                let target = rez_next_version::Version::parse(version_str)?;
                Ok(packages
                    .into_iter()
                    .find(|package| package.version.as_ref() == Some(&target)))
            }
            // `find_packages` already orders newest first.
            None => Ok(packages.into_iter().next()),
        }
    }

    async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
        let repository = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || {
            let mut names: Vec<String> = repository
                .iter_package_families()?
                .into_iter()
                .map(|family| family.name)
                .collect();
            names.sort();
            names.dedup();
            Ok(names)
        })
        .await
        .map_err(|e| RezCoreError::Repository(format!("Repository task panicked: {e}")))?
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn root_path(&self) -> &std::path::Path {
        self.inner.location()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build `<root>/<family>/<version>` directories, the layout the
    /// filesystem repository scans for.
    fn make_repository(versions: &[(&str, &[&str])]) -> TempDir {
        let root = TempDir::new().unwrap();
        for (family, family_versions) in versions {
            for version in *family_versions {
                fs::create_dir_all(root.path().join(family).join(version)).unwrap();
            }
        }
        root
    }

    #[tokio::test]
    async fn find_packages_returns_family_versions_newest_first() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0", "3.10.2"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let packages = adapter.find_packages("python").await.unwrap();

        let versions: Vec<String> = packages
            .iter()
            .filter_map(|p| p.version.as_ref().map(|v| v.as_str().to_string()))
            .collect();
        // Newest first, matching RepositoryManager's ordering.
        assert_eq!(versions, vec!["3.11.0", "3.10.2", "3.9.1"]);
    }

    #[tokio::test]
    async fn find_packages_of_unknown_family_is_empty() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        assert!(adapter.find_packages("maya").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_package_resolves_an_exact_version() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let package = adapter
            .get_package("python", Some("3.9.1"))
            .await
            .unwrap()
            .expect("3.9.1 must resolve");

        assert_eq!(package.name, "python");
        assert_eq!(package.version.as_ref().map(|v| v.as_str()), Some("3.9.1"));
    }

    #[tokio::test]
    async fn get_package_without_version_returns_the_latest() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let package = adapter
            .get_package("python", None)
            .await
            .unwrap()
            .expect("latest version must resolve");

        assert_eq!(package.version.as_ref().map(|v| v.as_str()), Some("3.11.0"));
    }

    #[tokio::test]
    async fn get_package_returns_none_for_a_missing_version() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        assert!(
            adapter
                .get_package("python", Some("9.9.9"))
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn list_packages_reports_every_family() {
        let root = make_repository(&[("python", &["3.9.1"]), ("maya", &["2024.1"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let mut names = adapter.list_packages().await.unwrap();
        names.sort();

        assert_eq!(names, vec!["maya".to_string(), "python".to_string()]);
    }

    /// The adapter only earns its place if the manager actually routes through
    /// it. Registering it with a `RepositoryManager` and querying through the
    /// manager covers the wiring that per-method tests cannot: the manager's
    /// own dispatch, and the fact that the adapter is what answers.
    #[tokio::test]
    async fn manager_routes_queries_through_the_adapter() {
        use crate::simple_repository::RepositoryManager;

        let root = make_repository(&[("python", &["3.9.1", "3.11.0"]), ("maya", &["2024.1"])]);
        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(AsyncRepositoryAdapter::new(
            FilesystemPackageRepository::with_name(root.path(), "fs".to_string()),
        )));

        let packages = manager.find_packages("python").await.unwrap();
        let versions: Vec<String> = packages
            .iter()
            .filter_map(|p| p.version.as_ref().map(|v| v.as_str().to_string()))
            .collect();
        assert_eq!(versions, vec!["3.11.0", "3.9.1"]);

        let latest = manager
            .get_package("python", None)
            .await
            .unwrap()
            .expect("latest python must resolve through the manager");
        assert_eq!(latest.version.as_ref().map(|v| v.as_str()), Some("3.11.0"));

        let mut families = manager.list_packages().await.unwrap();
        families.sort();
        assert_eq!(families, vec!["maya".to_string(), "python".to_string()]);
    }

    /// A repository that answers every query with nothing.
    ///
    /// This is registered with the manager directly, never through
    /// `AsyncRepositoryAdapter`, so it says nothing about the adapter itself.
    /// It pins down the manager contract: an empty backend must surface as
    /// empty, not as an error or a defaulted value.
    struct EmptyRepository;

    #[async_trait::async_trait]
    impl AsyncPackageRepository for EmptyRepository {
        async fn find_packages(&self, _name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
            Ok(Vec::new())
        }

        async fn get_package(
            &self,
            _name: &str,
            _version: Option<&str>,
        ) -> Result<Option<Arc<Package>>, RezCoreError> {
            Ok(None)
        }

        async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
            Ok(Vec::new())
        }

        fn name(&self) -> &str {
            "empty"
        }

        fn root_path(&self) -> &std::path::Path {
            std::path::Path::new("")
        }
    }

    /// Confirms an empty backend surfaces as empty through the manager.
    ///
    /// This documents the manager's contract for a backend that returns
    /// nothing; it does not exercise the adapter, and so cannot detect the
    /// adapter being bypassed. The adapter's discriminating power is covered
    /// by `manager_routes_queries_through_the_adapter` failing when the
    /// adapter stops returning data.
    #[tokio::test]
    async fn manager_level_assertions_reject_an_empty_repository() {
        use crate::simple_repository::RepositoryManager;

        let mut manager = RepositoryManager::new();
        manager.add_repository(Box::new(EmptyRepository));

        assert!(manager.find_packages("python").await.unwrap().is_empty());
        assert!(manager.get_package("python", None).await.unwrap().is_none());
        assert!(manager.list_packages().await.unwrap().is_empty());
    }

    #[test]
    fn name_and_root_path_come_from_the_wrapped_repository() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let inner = FilesystemPackageRepository::with_name(root.path(), "int".to_string());
        let adapter = AsyncRepositoryAdapter::new(inner);

        assert_eq!(adapter.name(), "int");
        assert_eq!(adapter.root_path(), root.path());
        assert_eq!(adapter.inner().repository_type(), "filesystem");
    }
}
