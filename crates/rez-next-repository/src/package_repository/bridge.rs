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
//!
//! # Caching
//!
//! `find_packages` memoizes per family, on both the success and the error side.
//! The wrapped repository re-reads and re-parses every `package.py` of a family
//! on each call, and the solver asks for the same families over and over:
//! `AStarSearch::generate_successors` calls `find_packages` once per expanded
//! state. Without memoization that turns a bounded wall-clock budget
//! (`max_search_time`) into resolution failures on repositories that used to
//! solve. Errors are memoized too, so a family whose definition fails to parse
//! keeps failing closed instead of silently reporting an empty family on a
//! later call.
//!
//! The memo uses `std::sync::RwLock` rather than `tokio::sync::RwLock` on
//! purpose. `tokio`'s lock is neither `UnwindSafe` nor `RefUnwindSafe`, so
//! adding it silently dropped those auto traits from this public type — a
//! breaking change that `cargo-semver-checks` catches but that nothing in the
//! test suite would. The memo guards are never held across an `.await`, so the
//! std lock costs nothing here. The assertion below keeps it that way.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use rez_next_common::RezCoreError;
use rez_next_package::Package;

use super::{FilesystemPackageRepository, PackageRepository as SyncPackageRepository};
use crate::resources::PackageFamilyResource;
use crate::simple_repository::PackageRepository as AsyncPackageRepository;

/// Guards the auto traits `AsyncRepositoryAdapter` had before it memoized.
///
/// Swapping the memo to a container with interior mutability that is not
/// unwind-safe would remove these from a public type, which is a breaking
/// change. This turns that into a compile error.
const _: fn() = || {
    fn assert_unwind_safe<T: std::panic::UnwindSafe + std::panic::RefUnwindSafe>() {}
    assert_unwind_safe::<AsyncRepositoryAdapter>();
};

/// Exposes a [`FilesystemPackageRepository`] through the async repository trait.
#[derive(Debug, Clone)]
pub struct AsyncRepositoryAdapter {
    inner: Arc<FilesystemPackageRepository>,
    /// Packages per family, newest first, as returned by [`family_packages`].
    packages: Arc<RwLock<HashMap<String, Vec<Arc<Package>>>>>,
    /// Parse failures per family, mirroring `SimpleRepository::package_errors`.
    errors: Arc<RwLock<HashMap<String, String>>>,
}

impl AsyncRepositoryAdapter {
    /// Wrap a filesystem repository so it can be added to a
    /// [`crate::simple_repository::RepositoryManager`].
    pub fn new(repository: FilesystemPackageRepository) -> Self {
        Self {
            inner: Arc::new(repository),
            packages: Arc::new(RwLock::new(HashMap::new())),
            errors: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// The wrapped upstream-faithful repository.
    pub fn inner(&self) -> &FilesystemPackageRepository {
        &self.inner
    }

    /// Resolve a family, memoizing the result on both paths.
    ///
    /// A family that is absent from the repository is memoized as an empty
    /// family, so repeated look-ups of a missing package stay cheap rather than
    /// re-walking the directory each time.
    ///
    /// A poisoned lock is recovered rather than skipped. Treating
    /// `PoisonError` as "no entry" would fall through to the disk read on every
    /// call, which silently reverts to the pre-memoization behaviour: same
    /// results, no error, no test failure — just the solver's hot path slowing
    /// back down. The memo holds already-published immutable snapshots, so
    /// recovering the guard is safe.
    async fn packages_for_family(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        if let Some(packages) = read_memo(&self.packages, name) {
            return Ok(packages);
        }
        if let Some(error) = read_memo(&self.errors, name) {
            return Err(RezCoreError::Repository(error));
        }

        let repository = Arc::clone(&self.inner);
        let owned_name = name.to_string();
        let lookup_name = owned_name.clone();
        let result = tokio::task::spawn_blocking(move || {
            let family = PackageFamilyResource::new(
                lookup_name,
                repository.repository_type().to_string(),
                repository.location().display().to_string(),
            );
            family_packages(&repository, family)
        })
        .await
        .map_err(|e| RezCoreError::Repository(format!("Repository task panicked: {e}")))?;

        match result {
            Ok(packages) => {
                insert_memo(&self.packages, owned_name, packages.clone());
                Ok(packages)
            }
            Err(error) => {
                let message = error.to_string();
                insert_memo(&self.errors, owned_name, message.clone());
                Err(RezCoreError::Repository(message))
            }
        }
    }
}

/// Read a memo entry, recovering from a poisoned lock.
fn read_memo<T: Clone>(memo: &RwLock<HashMap<String, T>>, key: &str) -> Option<T> {
    memo.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(key)
        .cloned()
}

/// Write a memo entry, recovering from a poisoned lock.
fn insert_memo<T>(memo: &RwLock<HashMap<String, T>>, key: String, value: T) {
    memo.write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, value);
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
        self.packages_for_family(name).await
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

    /// One version to write: family, version, and the requirements its
    /// definition declares.
    struct Definition<'a> {
        family: &'a str,
        version: &'a str,
        requires: &'a [&'a str],
    }

    /// Writes package definitions carrying dependency metadata, so a test can
    /// tell "the definition was read" apart from "a directory name was used".
    fn make_repository_with_definitions(definitions: &[Definition<'_>]) -> TempDir {
        let root = TempDir::new().unwrap();
        for definition in definitions {
            let dir = root.path().join(definition.family).join(definition.version);
            fs::create_dir_all(&dir).unwrap();
            let body = if definition.requires.is_empty() {
                format!(
                    "name = '{}'\nversion = '{}'\n",
                    definition.family, definition.version
                )
            } else {
                let listed = definition
                    .requires
                    .iter()
                    .map(|r| format!("'{r}'"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "name = '{}'\nversion = '{}'\nrequires = [{listed}]\n",
                    definition.family, definition.version
                )
            };
            fs::write(dir.join("package.py"), body).unwrap();
        }
        root
    }

    /// The adapter must surface dependency metadata, not just directory names.
    ///
    /// `iter_packages` builds each package from the version directory *and* its
    /// definition file. A regression back to names-only would still report the
    /// right versions, so this asserts on `requires` — the field that only a
    /// parsed definition can supply.
    #[tokio::test]
    async fn find_packages_reports_dependencies_from_the_definition() {
        let root = make_repository_with_definitions(&[
            Definition {
                family: "maya",
                version: "2024.1",
                requires: &["python-3"],
            },
            Definition {
                family: "maya",
                version: "2023.1",
                requires: &[],
            },
        ]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let packages = adapter.find_packages("maya").await.unwrap();
        let with_requires: Vec<_> = packages.iter().filter(|p| !p.requires.is_empty()).collect();

        assert_eq!(
            with_requires.len(),
            1,
            "only the 2024.1 definition has deps"
        );
        assert_eq!(with_requires[0].requires.len(), 1);
        assert!(
            with_requires[0].requires[0].to_string().contains("python"),
            "requirement should name python, got {:?}",
            with_requires[0].requires[0].to_string()
        );
    }

    /// Upstream rez reads packages from a flat two-level layout and does not
    /// recurse into grouping directories.
    ///
    /// This is the contract that separates the upstream-faithful repository from
    /// the recursive scan it replaced: under `<root>/studio/nuke/<version>`,
    /// `studio` is the family and `nuke` is a *version* of it, so `find_packages("nuke")`
    /// is empty. A recursive implementation would instead report `nuke` as a
    /// family, which upstream rez does not do -- see
    /// `_get_family_dirs` in upstream's
    /// `src/rezplugins/package_repository/filesystem.py`, a single `os.listdir`.
    #[tokio::test]
    async fn nested_layout_is_not_recursed_into() {
        let root = TempDir::new().unwrap();
        let nested = root.path().join("studio").join("nuke").join("15.0");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            nested.join("package.py"),
            "name = 'nuke'\nversion = '15.0'\n",
        )
        .unwrap();

        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        assert!(
            adapter.find_packages("nuke").await.unwrap().is_empty(),
            "a nested package must not be reported as a family"
        );
        // The grouping directory is the family, and the package name is its version.
        let families = adapter.list_packages().await.unwrap();
        assert_eq!(families, vec!["studio".to_string()]);
    }

    /// An ignored version is invisible to every Rust consumer.
    ///
    /// Ignore markers are the feature the issue calls out as behaving
    /// differently between Python and the CLI. Now that both sides route through
    /// this one implementation, the marker must hide the version from the async
    /// trait too, not just from the sync one.
    #[tokio::test]
    async fn ignored_versions_are_invisible_through_the_adapter() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let marker = root.path().join("python").join(".ignore3.11.0");
        fs::write(&marker, "").unwrap();

        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let versions: Vec<String> = adapter
            .find_packages("python")
            .await
            .unwrap()
            .iter()
            .filter_map(|p| p.version.as_ref().map(|v| v.as_str().to_string()))
            .collect();
        assert_eq!(versions, vec!["3.9.1".to_string()]);

        // The ignored version is not the latest either.
        let latest = adapter
            .get_package("python", None)
            .await
            .unwrap()
            .expect("3.9.1 must resolve");
        assert_eq!(latest.version.as_ref().map(|v| v.as_str()), Some("3.9.1"));
    }

    /// Payload paths reach Rust consumers through the wrapped repository.
    ///
    /// The issue lists payload paths as CLI-divergent. `inner()` is the seam that
    /// exposes the full trait's methods, which the 5-method async trait cannot
    /// carry.
    #[test]
    fn payload_path_is_reachable_from_rust() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let payload = adapter
            .inner()
            .get_package_payload_path("python", Some("3.9.1"))
            .unwrap();

        assert_eq!(payload, Some(root.path().join("python").join("3.9.1")));
    }

    /// Packages carry the path of the definition they were loaded from.
    ///
    /// `Package::root()` is derived from `filepath`, and variant
    /// materialisation fails without it. The serializer keeps the path on its
    /// container rather than on the package, so the repository has to transfer
    /// it -- without this, `rez test` on a variant package reports "Cannot
    /// materialize variant ... without a definition filepath".
    #[tokio::test]
    async fn packages_carry_the_definition_filepath() {
        let root = make_repository_with_definitions(&[Definition {
            family: "python",
            version: "3.9.1",
            requires: &[],
        }]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let package = adapter
            .get_package("python", Some("3.9.1"))
            .await
            .unwrap()
            .expect("python 3.9.1 must resolve");

        let expected = root
            .path()
            .join("python")
            .join("3.9.1")
            .join("package.py")
            .to_string_lossy()
            .to_string();
        assert_eq!(package.filepath.as_deref(), Some(expected.as_str()));

        // `root()` is what variant materialisation actually reads.
        assert_eq!(
            package.root().as_deref(),
            Some(
                root.path()
                    .join("python")
                    .join("3.9.1")
                    .to_string_lossy()
                    .as_ref()
            )
        );
    }

    /// A family is read from disk once, then served from the adapter's memo.
    ///
    /// This is the regression that matters for the solver:
    /// `AStarSearch::generate_successors` calls `find_packages` once per
    /// expanded state, over the same families. Re-reading and re-parsing every
    /// `package.py` per call made that ~1700x slower than the cached
    /// `SimpleRepository`, and because `AStarSearch` has a `max_search_time`
    /// wall-clock budget, slowness turned into resolutions that fail to
    /// complete rather than merely being slow.
    ///
    /// The observable consequence of memoization is that mutations made after
    /// the first read are no longer visible: the second call below returns the
    /// memoized two versions, not the three now on disk.
    #[tokio::test]
    async fn family_is_read_once_then_memoized() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        let first = adapter.find_packages("python").await.unwrap();
        assert_eq!(first.len(), 2, "fixture has two versions");

        // Add a version after the first read.
        let added = root.path().join("python").join("3.12.0");
        fs::create_dir_all(&added).unwrap();
        fs::write(
            added.join("package.py"),
            "name = 'python'\nversion = '3.12.0'\n",
        )
        .unwrap();

        let second = adapter.find_packages("python").await.unwrap();
        assert_eq!(
            second.len(),
            2,
            "a memoized family must not be re-read from disk"
        );
    }

    /// A poisoned memo lock still serves cached entries.
    ///
    /// If `PoisonError` were treated as "no entry", every look-up would fall
    /// through to a disk read: correct results, no error, no failing test —
    /// just the solver's hot path quietly reverting to pre-memoization cost.
    /// This pins the recovery.
    #[tokio::test]
    async fn memoization_survives_a_poisoned_lock() {
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        // Populate the memo.
        assert_eq!(adapter.find_packages("python").await.unwrap().len(), 2);

        // Poison the memo by panicking while the write guard is held.
        let poison = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = adapter.packages.write().unwrap();
            panic!("poison the memo lock");
        }));
        assert!(poison.is_err(), "the panic must have poisoned the lock");
        assert!(adapter.packages.is_poisoned());

        // A version added after poisoning: the cached entry must still win,
        // which is only observable if the poisoned lock was recovered rather
        // than skipped.
        let added = root.path().join("python").join("3.12.0");
        fs::create_dir_all(&added).unwrap();
        fs::write(
            added.join("package.py"),
            "name = 'python'\nversion = '3.12.0'\n",
        )
        .unwrap();

        let packages = adapter.find_packages("python").await.unwrap();
        assert_eq!(
            packages.len(),
            2,
            "a poisoned memo must still serve its cached entry, not re-read disk"
        );
    }

    /// A family whose definition fails to parse keeps failing.
    ///
    /// Errors are memoized alongside successes so that a broken definition
    /// fails closed on every call, rather than reporting an empty family after
    /// the first failure.
    #[tokio::test]
    async fn a_broken_family_fails_closed_on_every_call() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join("broken").join("1.0.0");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("package.py"), "this is not valid python!! (((").unwrap();

        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        assert!(
            adapter.find_packages("broken").await.is_err(),
            "a broken definition must be an error, not an empty family"
        );
        assert!(
            adapter.find_packages("broken").await.is_err(),
            "the error must be memoized, not downgraded to empty on repeat calls"
        );
    }

    /// A family that is absent is memoized as empty, so repeated look-ups of a
    /// missing package do not re-walk the directory.
    #[tokio::test]
    async fn a_missing_family_stays_empty_and_is_memoized() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let adapter = AsyncRepositoryAdapter::new(FilesystemPackageRepository::new(root.path()));

        assert!(adapter.find_packages("absent").await.unwrap().is_empty());
        assert!(adapter.find_packages("absent").await.unwrap().is_empty());
    }
}
