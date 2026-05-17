//! PackageRepository trait and base implementations
//!
//! This module defines the PackageRepository trait, which corresponds to the
//! PackageRepository class in rez's package_repository.py.
//!
//! All custom package repository plugins must implement this trait.

// ── Submodules ──────────────────────────────────────────────────────────────────
pub mod filesystem;

// ── Re-exports ──────────────────────────────────────────────────────────────────
pub use filesystem::FilesystemPackageRepository;
pub use filesystem::FILESYSTEM_REPO_TYPE;

// ── Imports ─────────────────────────────────────────────────────────────────────
use crate::resources::{PackageFamilyResource, PackageResource, ResourceHandle, VariantResource};
use rez_next_common::RezCoreError;
use std::collections::HashMap;
use std::path::PathBuf;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Special value for `install_variant` overrides parameter to indicate removal
pub const REMOVE: &str = "__remove__";

// ── PackageRepository trait ─────────────────────────────────────────────────────

/// PackageRepository trait
///
/// This trait defines the interface that all package repository implementations
/// must implement. It corresponds to the PackageRepository class in rez's
/// package_repository.py.
///
/// # Required methods
///
/// These methods must be implemented by all implementations:
/// - `name()` - Return the repository type name
/// - `get_package_family()` - Get a package family by name
/// - `iter_package_families()` - Iterate over all package families
/// - `iter_packages()` - Iterate over packages in a family
/// - `iter_variants()` - Iterate over variants in a package
/// - `ignore_package()` - Ignore a package
/// - `unignore_package()` - Unignore a package
/// - `remove_package()` - Remove a package
/// - `remove_package_family()` - Remove a package family
/// - `remove_ignored_since()` - Remove ignored packages older than N days
/// - `install_variant()` - Install a variant into this repository
/// - `get_parent_package_family()` - Get the parent family of a package
/// - `get_parent_package()` - Get the parent package of a variant
/// - `get_package_payload_path()` - Get the payload path for a package
///
/// # Provided methods (with default implementations)
///
/// These methods have default implementations that can be overridden:
/// - `uid()` - Get unique identifier for this repository
/// - `is_empty()` - Check if the repository is empty
/// - `get_package()` - Get a package by name and version
/// - `get_equivalent_variant()` - Get an equivalent variant
/// - `make_resource_handle()` - Create a resource handle
/// - `get_resource()` - Get a resource by handle
/// - `get_variant_state_handle()` - Get variant state handle for caching
/// - `get_last_release_time()` - Get last release time for caching
/// - `pre_variant_install()` - Pre-install hook
/// - `on_variant_install_cancelled()` - Install cancelled hook
pub trait PackageRepository: Send + Sync {
    // ── Required methods (must be implemented by subclasses) ───────────────────

    /// Return the repository type name
    fn name() -> &'static str
    where
        Self: Sized;

    /// Get a package family by name
    fn get_package_family(&self, name: &str)
        -> Result<Option<PackageFamilyResource>, RezCoreError>;

    /// Iterate over all package families in this repository
    fn iter_package_families(&self) -> Result<Vec<PackageFamilyResource>, RezCoreError>;

    /// Iterate over packages in a package family
    fn iter_packages(
        &self,
        package_family: &PackageFamilyResource,
    ) -> Result<Vec<PackageResource>, RezCoreError>;

    /// Iterate over variants in a package
    fn iter_variants(
        &self,
        package: &PackageResource,
    ) -> Result<Vec<VariantResource>, RezCoreError>;

    /// Ignore a package (make it invisible to resolution)
    fn ignore_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
        allow_missing: bool,
    ) -> Result<i32, RezCoreError>;

    /// Cancel ignoring a package
    fn unignore_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<i32, RezCoreError>;

    /// Remove a package (can remove ignored packages)
    fn remove_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<bool, RezCoreError>;

    /// Remove a package family (force=true to remove even if family has packages)
    fn remove_package_family(&mut self, pkg_name: &str, force: bool) -> Result<bool, RezCoreError>;

    /// Remove ignored packages older than specified days
    fn remove_ignored_since(
        &mut self,
        days: i32,
        dry_run: bool,
        verbose: bool,
    ) -> Result<i32, RezCoreError>;

    /// Install a variant into this repository
    ///
    /// If `dry_run` is true, only return the equivalent variant without installing.
    /// `overrides` can be used to modify package metadata during installation.
    fn install_variant(
        &mut self,
        variant: &VariantResource,
        dry_run: bool,
        overrides: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Option<PackageResource>, RezCoreError>;

    /// Get the parent package family of a package
    fn get_parent_package_family(
        &self,
        package: &PackageResource,
    ) -> Result<PackageFamilyResource, RezCoreError>;

    /// Get the parent package of a variant
    fn get_parent_package(
        &self,
        variant: &VariantResource,
    ) -> Result<PackageResource, RezCoreError>;

    /// Get the payload path for a package
    fn get_package_payload_path(
        &self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<Option<PathBuf>, RezCoreError>;

    // ── Provided methods (with default implementations) ────────────────────────

    /// Get the unique identifier for this repository
    ///
    /// Default: (repository_type, location)
    fn uid(&self) -> (String, String) {
        (
            self.repository_type().to_string(),
            self.location().to_string(),
        )
    }

    /// Check if the repository is empty
    fn is_empty(&self) -> Result<bool, RezCoreError> {
        let families = self.iter_package_families()?;
        Ok(families.is_empty())
    }

    /// Get a package by name and version
    fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<PackageResource>, RezCoreError> {
        let family = match self.get_package_family(name)? {
            Some(f) => f,
            None => return Ok(None),
        };

        let packages = self.iter_packages(&family)?;
        for pkg in packages {
            match version {
                Some(v) => {
                    if let Some(pkg_version) = pkg.version() {
                        if pkg_version.as_str() == v {
                            return Ok(Some(pkg));
                        }
                    }
                }
                None => return Ok(Some(pkg)),
            }
        }

        Ok(None)
    }

    /// Get an equivalent variant in this repository
    fn get_equivalent_variant(
        &mut self,
        variant: &VariantResource,
    ) -> Result<Option<VariantResource>, RezCoreError> {
        // Default implementation: perform a dry-run install to find equivalent variant
        self.install_variant(variant, true, None)
            .map(|opt| opt.map(|_| variant.clone()))
    }

    /// Create a resource handle for unique identification
    fn make_resource_handle(
        &self,
        _resource_key: &str,
        variables: HashMap<String, String>,
    ) -> Result<ResourceHandle, RezCoreError> {
        // Validate repository type and location
        let repo_type = self.repository_type();
        let location = self.location();

        // In a full implementation, would validate variables

        Ok(ResourceHandle::new(
            repo_type.to_string(),
            location.to_string(),
            variables,
        ))
    }

    /// Get variant state handle for caching
    ///
    /// Default: returns None (no caching)
    fn get_variant_state_handle(&self, _variant: &VariantResource) -> Option<String> {
        None
    }

    /// Get last release time for caching
    ///
    /// Default: returns 0 (no caching optimization)
    fn get_last_release_time(&self, _family: &PackageFamilyResource) -> i64 {
        0
    }

    /// Pre-install hook for variants
    fn pre_variant_install(&mut self, _variant: &VariantResource) -> Result<(), RezCoreError> {
        Ok(())
    }

    /// Install cancelled hook for variants
    fn on_variant_install_cancelled(
        &mut self,
        _variant: &VariantResource,
    ) -> Result<(), RezCoreError> {
        Ok(())
    }

    // ── Helper methods (not in original, but useful for Rust) ─────────────────

    /// Get the repository type
    fn repository_type(&self) -> &str;

    /// Get the repository location
    fn location(&self) -> &str;
}

// ── ResourceHandle (re-export from resources module) ───────────────────────────

// ResourceHandle is defined in resources/mod.rs and re-exported in lib.rs

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{PackageFamilyResource, ResourceHandle};

    // Mock implementation for testing
    struct MockRepository {
        repository_type: String,
        location: String,
    }

    impl MockRepository {
        fn new(repository_type: &str, location: &str) -> Self {
            Self {
                repository_type: repository_type.to_string(),
                location: location.to_string(),
            }
        }
    }

    impl PackageRepository for MockRepository {
        fn name() -> &'static str
        where
            Self: Sized,
        {
            "mock"
        }

        fn get_package_family(
            &self,
            _name: &str,
        ) -> Result<Option<PackageFamilyResource>, RezCoreError> {
            Ok(None)
        }

        fn iter_package_families(&self) -> Result<Vec<PackageFamilyResource>, RezCoreError> {
            Ok(Vec::new())
        }

        fn iter_packages(
            &self,
            _package_family: &PackageFamilyResource,
        ) -> Result<Vec<PackageResource>, RezCoreError> {
            Ok(Vec::new())
        }

        fn iter_variants(
            &self,
            _package: &PackageResource,
        ) -> Result<Vec<VariantResource>, RezCoreError> {
            Ok(Vec::new())
        }

        fn ignore_package(
            &mut self,
            _pkg_name: &str,
            _pkg_version: Option<&str>,
            _allow_missing: bool,
        ) -> Result<i32, RezCoreError> {
            Ok(0)
        }

        fn unignore_package(
            &mut self,
            _pkg_name: &str,
            _pkg_version: Option<&str>,
        ) -> Result<i32, RezCoreError> {
            Ok(0)
        }

        fn remove_package(
            &mut self,
            _pkg_name: &str,
            _pkg_version: Option<&str>,
        ) -> Result<bool, RezCoreError> {
            Ok(false)
        }

        fn remove_package_family(
            &mut self,
            _pkg_name: &str,
            _force: bool,
        ) -> Result<bool, RezCoreError> {
            Ok(false)
        }

        fn remove_ignored_since(
            &mut self,
            _days: i32,
            _dry_run: bool,
            _verbose: bool,
        ) -> Result<i32, RezCoreError> {
            Ok(0)
        }

        fn install_variant(
            &mut self,
            _variant: &VariantResource,
            _dry_run: bool,
            _overrides: Option<HashMap<String, serde_json::Value>>,
        ) -> Result<Option<PackageResource>, RezCoreError> {
            Ok(None)
        }

        fn get_parent_package_family(
            &self,
            _package: &PackageResource,
        ) -> Result<PackageFamilyResource, RezCoreError> {
            Err(RezCoreError::Repository(
                "MockRepository::get_parent_package_family not implemented".to_string(),
            ))
        }

        fn get_parent_package(
            &self,
            _variant: &VariantResource,
        ) -> Result<PackageResource, RezCoreError> {
            Err(RezCoreError::Repository(
                "MockRepository::get_parent_package not implemented".to_string(),
            ))
        }

        fn get_package_payload_path(
            &self,
            _pkg_name: &str,
            _pkg_version: Option<&str>,
        ) -> Result<Option<PathBuf>, RezCoreError> {
            Ok(None)
        }

        fn repository_type(&self) -> &str {
            &self.repository_type
        }

        fn location(&self) -> &str {
            &self.location
        }
    }

    #[test]
    fn test_mock_repository_create() {
        let repo = MockRepository::new("mock", "/tmp/packages");
        assert_eq!(repo.repository_type(), "mock");
        assert_eq!(repo.location(), "/tmp/packages");
    }

    #[test]
    fn test_mock_repository_name() {
        assert_eq!(MockRepository::name(), "mock");
    }

    #[test]
    fn test_mock_repository_is_empty() {
        let repo = MockRepository::new("mock", "/tmp/packages");
        let result = repo.is_empty();
        assert!(result.is_ok());
        // Empty because iter_package_families returns empty vec
        assert!(result.unwrap());
    }

    #[test]
    fn test_resource_handle_create() {
        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "python".to_string());

        let handle =
            ResourceHandle::new("mock".to_string(), "/tmp/packages".to_string(), variables);

        assert_eq!(handle.repository_type, "mock");
        assert_eq!(handle.repository_location, "/tmp/packages");
    }

    #[test]
    fn test_make_resource_handle() {
        let repo = MockRepository::new("mock", "/tmp/packages");

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "python".to_string());

        let result = repo.make_resource_handle("test", variables);
        assert!(result.is_ok());

        let handle = result.unwrap();
        assert_eq!(handle.repository_type, "mock");
        assert_eq!(handle.repository_location, "/tmp/packages");
    }
}
