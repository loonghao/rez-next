//! FilesystemPackageRepository implementation
//!
//! This module implements the PackageRepository trait for filesystem-based repositories.
//! It corresponds to the FilesystemPackageRepository class in rez's package_repository.py.

use super::PackageRepository;
use crate::resources::{PackageFamilyResource, PackageResource, VariantResource};
use rez_next_common::RezCoreError;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Filesystem repository type name
pub const FILESYSTEM_REPO_TYPE: &str = "filesystem";

// ── FilesystemPackageRepository ──────────────────────────────────────────────

/// Filesystem-based package repository
///
/// This corresponds to the FilesystemPackageRepository class in rez's
/// package_repository.py. It reads packages from a filesystem path.
#[derive(Debug, Clone)]
pub struct FilesystemPackageRepository {
    /// Repository location (filesystem path)
    location: PathBuf,
    /// Repository name (defaults to directory name)
    name: String,
    /// Repository priority (higher = more preferred)
    priority: i32,
    /// Whether this repository is read-only
    read_only: bool,
    /// Repository description
    description: Option<String>,
    /// Whether the repository has been initialized
    initialized: bool,
}

impl FilesystemPackageRepository {
    /// Create a new filesystem package repository
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let location = path.as_ref().to_path_buf();
        let name = location
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Self {
            location,
            name,
            priority: 0,
            read_only: false,
            description: None,
            initialized: false,
        }
    }

    /// Create with explicit name
    pub fn with_name<P: AsRef<Path>>(path: P, name: String) -> Self {
        let mut repo = Self::new(path);
        repo.name = name;
        repo
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set read-only flag
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Set description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Get the repository location
    pub fn location(&self) -> &Path {
        &self.location
    }

    /// Get the repository name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    fn validate_component<'a>(value: &'a str, label: &str) -> Result<&'a str, RezCoreError> {
        let mut components = Path::new(value).components();
        let is_single_normal = matches!(components.next(), Some(std::path::Component::Normal(_)))
            && components.next().is_none();
        if value.is_empty() || !is_single_normal {
            return Err(RezCoreError::Repository(format!(
                "Invalid {label} path component: {value:?}"
            )));
        }
        Ok(value)
    }

    fn require_version(version: Option<&str>) -> Result<&str, RezCoreError> {
        let version = version.ok_or_else(|| {
            RezCoreError::Repository(
                "Filesystem repository lifecycle operations require a package version".to_string(),
            )
        })?;
        Self::validate_component(version, "package version")
    }

    fn ensure_writable(&self, operation: &str) -> Result<(), RezCoreError> {
        if self.read_only {
            return Err(RezCoreError::Repository(format!(
                "Cannot {operation}: repository '{}' is read-only",
                self.location.display()
            )));
        }
        Ok(())
    }

    fn family_path(&self, name: &str) -> Result<PathBuf, RezCoreError> {
        Ok(self
            .location
            .join(Self::validate_component(name, "package name")?))
    }

    fn version_path(&self, name: &str, version: &str) -> Result<PathBuf, RezCoreError> {
        Ok(self
            .family_path(name)?
            .join(Self::validate_component(version, "package version")?))
    }

    fn ignore_path(&self, name: &str, version: &str) -> Result<PathBuf, RezCoreError> {
        Ok(self.family_path(name)?.join(format!(".ignore{version}")))
    }
}

impl PackageRepository for FilesystemPackageRepository {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        FILESYSTEM_REPO_TYPE
    }

    fn get_package_family(
        &self,
        name: &str,
    ) -> Result<Option<PackageFamilyResource>, RezCoreError> {
        // Check if directory <location>/<name> exists
        let family_path = self.location.join(name);

        if !family_path.is_dir() {
            return Ok(None);
        }

        // Create PackageFamilyResource
        let family = PackageFamilyResource::new(
            name.to_string(),
            FILESYSTEM_REPO_TYPE.to_string(),
            self.location.to_string_lossy().to_string(),
        );

        Ok(Some(family))
    }

    fn iter_package_families(&self) -> Result<Vec<PackageFamilyResource>, RezCoreError> {
        let mut families = Vec::new();

        // Read the repository location directory
        let entries = fs::read_dir(&self.location).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to read repository directory '{}': {}",
                self.location.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            // Only include directories (package families are directories)
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip hidden directories and special directories
                if name.starts_with('.') || name.starts_with('_') {
                    continue;
                }

                // Create a PackageFamilyResource
                let family = PackageFamilyResource::new(
                    name,
                    FILESYSTEM_REPO_TYPE.to_string(),
                    self.location.to_string_lossy().to_string(),
                );

                families.push(family);
            }
        }

        Ok(families)
    }

    fn iter_packages(
        &self,
        package_family: &PackageFamilyResource,
    ) -> Result<Vec<PackageResource>, RezCoreError> {
        let mut packages = Vec::new();

        // The family directory is <location>/<family_name>
        let family_path = self.location.join(&package_family.name);

        if !family_path.is_dir() {
            return Ok(packages);
        }

        // Read the family directory to find version directories
        let entries = fs::read_dir(&family_path).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to read family directory '{}': {}",
                family_path.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                RezCoreError::Repository(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            // Only include directories (versions are directories)
            if path.is_dir() {
                let version_str = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip hidden directories and special directories
                if version_str.starts_with('.') || version_str.starts_with('_') {
                    continue;
                }

                // Rez hides a version when a sibling `.ignore{version}` marker exists.
                if family_path.join(format!(".ignore{version_str}")).is_file() {
                    continue;
                }

                // Try to parse version string
                let version = rez_next_version::Version::parse(&version_str).ok();

                // Create a Package object
                let mut pkg = rez_next_package::Package::new(package_family.name.clone());
                pkg.version = version;

                // Create PackageResource
                let resource = PackageResource::new(
                    pkg,
                    FILESYSTEM_REPO_TYPE.to_string(),
                    self.location.to_string_lossy().to_string(),
                );

                packages.push(resource);
            }
        }

        Ok(packages)
    }

    fn iter_variants(
        &self,
        package: &PackageResource,
    ) -> Result<Vec<VariantResource>, RezCoreError> {
        let mut variants = Vec::new();

        // Get version string
        let version_str = match &package.version() {
            Some(v) => v.as_str().to_string(),
            None => {
                // Package without version, no variants
                return Ok(variants);
            }
        };

        // The package directory is <location>/<family>/<version>
        let package_path = self.location.join(package.name()).join(&version_str);

        if !package_path.is_dir() {
            return Ok(variants);
        }

        // For now, create a single variant (index 0)
        // In a full implementation, would parse package.py for variants
        let variant = VariantResource::new(
            package.name().to_string(),
            Some(version_str),
            0, // index
            FILESYSTEM_REPO_TYPE.to_string(),
            self.location.to_string_lossy().to_string(),
        );

        variants.push(variant);

        Ok(variants)
    }

    fn ignore_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
        allow_missing: bool,
    ) -> Result<i32, RezCoreError> {
        self.ensure_writable("ignore package")?;
        let version = Self::require_version(pkg_version)?;
        let package_path = self.version_path(pkg_name, version)?;
        if !allow_missing && !package_path.is_dir() {
            return Ok(-1);
        }

        let family_path = self.family_path(pkg_name)?;
        fs::create_dir_all(&family_path).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to create package family '{}': {e}",
                family_path.display()
            ))
        })?;
        let marker = self.ignore_path(pkg_name, version)?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker)
        {
            Ok(_) => Ok(1),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(0),
            Err(e) => Err(RezCoreError::Repository(format!(
                "Failed to create ignore marker '{}': {e}",
                marker.display()
            ))),
        }
    }

    fn unignore_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<i32, RezCoreError> {
        self.ensure_writable("unignore package")?;
        let version = Self::require_version(pkg_version)?;
        let marker = self.ignore_path(pkg_name, version)?;
        let removed = match fs::remove_file(&marker) {
            Ok(()) => true,
            Err(e) if e.kind() == ErrorKind::NotFound => false,
            Err(e) => {
                return Err(RezCoreError::Repository(format!(
                    "Failed to remove ignore marker '{}': {e}",
                    marker.display()
                )));
            }
        };

        if !self.version_path(pkg_name, version)?.is_dir() {
            return Ok(-1);
        }
        Ok(i32::from(removed))
    }

    fn remove_package(
        &mut self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<bool, RezCoreError> {
        self.ensure_writable("remove package")?;
        let version = Self::require_version(pkg_version)?;
        if self.ignore_package(pkg_name, Some(version), false)? == -1 {
            return Ok(false);
        }

        let package_path = self.version_path(pkg_name, version)?;
        if let Err(e) = fs::remove_dir_all(&package_path) {
            return Err(RezCoreError::Repository(format!(
                "Failed to remove package payload '{}': {e}; the ignore marker was retained",
                package_path.display()
            )));
        }

        let marker = self.ignore_path(pkg_name, version)?;
        match fs::remove_file(&marker) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(true),
            Err(e) => Err(RezCoreError::Repository(format!(
                "Package payload was removed, but ignore marker '{}' could not be removed: {e}",
                marker.display()
            ))),
        }
    }

    fn remove_package_family(&mut self, pkg_name: &str, force: bool) -> Result<bool, RezCoreError> {
        self.ensure_writable("remove package family")?;
        let family_path = self.family_path(pkg_name)?;
        if !family_path.is_dir() {
            return Ok(false);
        }

        if !force {
            let has_packages = fs::read_dir(&family_path)
                .map_err(|e| {
                    RezCoreError::Repository(format!(
                        "Failed to inspect package family '{}': {e}",
                        family_path.display()
                    ))
                })?
                .filter_map(Result::ok)
                .any(|entry| {
                    entry.path().is_dir() && !entry.file_name().to_string_lossy().starts_with('.')
                });
            if has_packages {
                return Err(RezCoreError::Repository(format!(
                    "Cannot remove non-empty package family {pkg_name:?} without force"
                )));
            }
        }

        fs::remove_dir_all(&family_path).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to remove package family '{}': {e}",
                family_path.display()
            ))
        })?;
        Ok(true)
    }

    fn remove_ignored_since(
        &mut self,
        days: i32,
        dry_run: bool,
        verbose: bool,
    ) -> Result<i32, RezCoreError> {
        self.ensure_writable("remove ignored packages")?;
        if days < 0 {
            return Err(RezCoreError::Repository(
                "Ignored-package age must be zero or greater".to_string(),
            ));
        }

        let threshold = Duration::from_secs(days as u64 * 24 * 60 * 60);
        let now = SystemTime::now();
        let mut candidates = Vec::new();
        if !self.location.is_dir() {
            return Ok(0);
        }

        for family in fs::read_dir(&self.location).map_err(|e| {
            RezCoreError::Repository(format!(
                "Failed to inspect repository '{}': {e}",
                self.location.display()
            ))
        })? {
            let family = family.map_err(|e| RezCoreError::Repository(e.to_string()))?;
            if !family.path().is_dir() {
                continue;
            }
            let family_name = family.file_name().to_string_lossy().to_string();
            for entry in fs::read_dir(family.path()).map_err(|e| {
                RezCoreError::Repository(format!(
                    "Failed to inspect package family '{}': {e}",
                    family.path().display()
                ))
            })? {
                let entry = entry.map_err(|e| RezCoreError::Repository(e.to_string()))?;
                let filename = entry.file_name().to_string_lossy().to_string();
                let Some(version) = filename.strip_prefix(".ignore") else {
                    continue;
                };
                if version.is_empty() || !entry.path().is_file() {
                    continue;
                }
                let metadata = entry.metadata().map_err(|e| {
                    RezCoreError::Repository(format!(
                        "Failed to read ignore marker '{}': {e}",
                        entry.path().display()
                    ))
                })?;
                let timestamp = metadata
                    .created()
                    .or_else(|_| metadata.modified())
                    .map_err(|e| {
                        RezCoreError::Repository(format!(
                            "Failed to determine ignore marker age '{}': {e}",
                            entry.path().display()
                        ))
                    })?;
                if now.duration_since(timestamp).unwrap_or_default() >= threshold {
                    candidates.push((family_name.clone(), version.to_string()));
                }
            }
        }

        let mut removed = 0;
        for (name, version) in candidates {
            if verbose {
                tracing::info!(
                    package = %name,
                    version = %version,
                    dry_run,
                    "removing ignored package"
                );
            }
            if dry_run || self.remove_package(&name, Some(&version))? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    fn install_variant(
        &mut self,
        _variant: &VariantResource,
        _dry_run: bool,
        _overrides: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Option<PackageResource>, RezCoreError> {
        Err(RezCoreError::Repository(
            "Filesystem variant installation is not implemented; use the build or release workflow"
                .to_string(),
        ))
    }

    fn get_parent_package_family(
        &self,
        package: &PackageResource,
    ) -> Result<PackageFamilyResource, RezCoreError> {
        Self::validate_component(package.name(), "package name")?;
        Ok(PackageFamilyResource::new(
            package.name().to_string(),
            FILESYSTEM_REPO_TYPE.to_string(),
            self.location.to_string_lossy().to_string(),
        ))
    }

    fn get_parent_package(
        &self,
        variant: &VariantResource,
    ) -> Result<PackageResource, RezCoreError> {
        Self::validate_component(&variant.name, "package name")?;
        let mut package = rez_next_package::Package::new(variant.name.clone());
        package.version = variant
            .version
            .as_deref()
            .map(rez_next_version::Version::parse)
            .transpose()
            .map_err(|e| RezCoreError::Repository(format!("Invalid package version: {e}")))?;
        Ok(PackageResource::new(
            package,
            FILESYSTEM_REPO_TYPE.to_string(),
            self.location.to_string_lossy().to_string(),
        ))
    }

    fn get_package_payload_path(
        &self,
        pkg_name: &str,
        pkg_version: Option<&str>,
    ) -> Result<Option<PathBuf>, RezCoreError> {
        let mut path = self.family_path(pkg_name)?;
        if let Some(version) = pkg_version {
            path.push(Self::validate_component(version, "package version")?);
        }
        Ok(Some(path))
    }

    fn repository_type(&self) -> &str {
        FILESYSTEM_REPO_TYPE
    }

    fn location(&self) -> &str {
        self.location.to_str().unwrap_or_default()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_filesystem_repository_create() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        assert_eq!(
            repo.name(),
            temp_dir.path().file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(repo.repository_type(), "filesystem");
        assert!(!repo.is_initialized());
    }

    #[test]
    fn test_filesystem_repository_with_name() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::with_name(temp_dir.path(), "my_repo".to_string());

        assert_eq!(repo.name(), "my_repo");
    }

    #[test]
    fn test_filesystem_repository_with_priority() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path()).with_priority(10);

        // Need to add a way to get priority, or test via metadata
        assert_eq!(
            repo.name(),
            temp_dir.path().file_name().unwrap().to_str().unwrap()
        );
    }

    #[test]
    fn test_filesystem_repository_with_read_only() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path()).with_read_only(true);

        // Need to add a way to check read_only, or test via metadata
        assert!(!repo.is_initialized());
    }

    #[test]
    fn test_filesystem_repository_location() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        assert_eq!(repo.location(), temp_dir.path());
    }

    #[test]
    fn test_filesystem_repository_get_package_family_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        let result = repo.get_package_family("python");
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_filesystem_repository_iter_package_families_empty() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        let result = repo.iter_package_families();
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_iter_package_families_with_families() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create family directories
        let python_dir = temp_dir.path().join("python");
        let maya_dir = temp_dir.path().join("maya");
        fs::create_dir(&python_dir).unwrap();
        fs::create_dir(&maya_dir).unwrap();

        // Add a file (should not be included)
        let file_path = temp_dir.path().join("not_a_family.txt");
        fs::write(&file_path, "test").unwrap();

        let families = repo.iter_package_families().unwrap();
        assert_eq!(families.len(), 2);

        let names: Vec<&str> = families.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"python"));
        assert!(names.contains(&"maya"));
        assert!(!names.contains(&"not_a_family.txt"));
    }

    #[test]
    fn test_get_package_family_exists() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create family directory
        let python_dir = temp_dir.path().join("python");
        fs::create_dir(&python_dir).unwrap();

        let family = repo.get_package_family("python").unwrap();
        assert!(family.is_some());

        let family = family.unwrap();
        assert_eq!(family.name, "python");
        assert_eq!(family.repository_type, "filesystem");
    }

    #[test]
    fn test_get_package_family_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        let family = repo.get_package_family("nonexistent").unwrap();
        assert!(family.is_none());
    }

    #[test]
    fn test_iter_packages_with_versions() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create family directory
        let python_dir = temp_dir.path().join("python");
        fs::create_dir(&python_dir).unwrap();

        // Create version directories
        let v390 = python_dir.join("3.9.0");
        let v3100 = python_dir.join("3.10.0");
        fs::create_dir(&v390).unwrap();
        fs::create_dir(&v3100).unwrap();

        // Create a PackageFamilyResource
        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        let packages = repo.iter_packages(&family).unwrap();
        assert_eq!(packages.len(), 2);

        let versions: Vec<Option<&str>> = packages
            .iter()
            .map(|p| p.version().map(|v| v.as_str()))
            .collect();
        assert!(versions.contains(&Some("3.9.0")));
        assert!(versions.contains(&Some("3.10.0")));
    }

    #[test]
    fn test_iter_packages_empty() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create empty family directory
        let python_dir = temp_dir.path().join("python");
        fs::create_dir(&python_dir).unwrap();

        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        let packages = repo.iter_packages(&family).unwrap();
        assert!(packages.is_empty());
    }

    #[test]
    fn test_iter_variants() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create package family directory
        let python_dir = temp_dir.path().join("python");
        fs::create_dir(&python_dir).unwrap();

        // Create version directory
        let version_dir = python_dir.join("3.9.0");
        fs::create_dir(&version_dir).unwrap();

        // Create a PackageResource with version
        let mut pkg = Package::new("python".to_string());
        pkg.version = Some(rez_next_version::Version::parse("3.9.0").unwrap());

        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        let variants = repo.iter_variants(&resource).unwrap();
        assert_eq!(variants.len(), 1);

        let variant = &variants[0];
        assert_eq!(variant.name, "python");
        assert_eq!(variant.index, 0);
    }

    #[test]
    fn test_iter_variants_no_version() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());

        // Create a PackageResource without version
        let pkg = rez_next_package::Package::new("python".to_string());
        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        let variants = repo.iter_variants(&resource).unwrap();
        assert!(variants.is_empty());
    }

    #[test]
    fn ignore_and_unignore_package_follow_rez_marker_contract() {
        let temp_dir = TempDir::new().unwrap();
        let family_dir = temp_dir.path().join("python");
        fs::create_dir_all(family_dir.join("3.12.0")).unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());

        assert_eq!(
            repo.ignore_package("python", Some("3.12.0"), false)
                .unwrap(),
            1
        );
        assert!(family_dir.join(".ignore3.12.0").is_file());
        assert!(
            repo.get_package("python", Some("3.12.0"))
                .unwrap()
                .is_none()
        );
        assert_eq!(
            repo.ignore_package("python", Some("3.12.0"), false)
                .unwrap(),
            0
        );

        assert_eq!(repo.unignore_package("python", Some("3.12.0")).unwrap(), 1);
        assert!(!family_dir.join(".ignore3.12.0").exists());
        assert!(
            repo.get_package("python", Some("3.12.0"))
                .unwrap()
                .is_some()
        );
        assert_eq!(repo.unignore_package("python", Some("3.12.0")).unwrap(), 0);
    }

    #[test]
    fn ignore_missing_package_requires_explicit_allow_missing() {
        let temp_dir = TempDir::new().unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());

        assert_eq!(
            repo.ignore_package("python", Some("3.12.0"), false)
                .unwrap(),
            -1
        );
        assert_eq!(
            repo.ignore_package("python", Some("3.12.0"), true).unwrap(),
            1
        );
        assert!(temp_dir.path().join("python/.ignore3.12.0").is_file());
    }

    #[test]
    fn remove_package_deletes_payload_and_marker() {
        let temp_dir = TempDir::new().unwrap();
        let package_dir = temp_dir.path().join("python/3.12.0");
        fs::create_dir_all(&package_dir).unwrap();
        fs::write(package_dir.join("package.py"), "name = 'python'").unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());

        assert!(repo.remove_package("python", Some("3.12.0")).unwrap());
        assert!(!package_dir.exists());
        assert!(!temp_dir.path().join("python/.ignore3.12.0").exists());
        assert!(!repo.remove_package("python", Some("3.12.0")).unwrap());
    }

    #[test]
    fn remove_package_family_requires_force_when_non_empty() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("python/3.12.0")).unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());

        assert!(repo.remove_package_family("python", false).is_err());
        assert!(repo.remove_package_family("python", true).unwrap());
        assert!(!temp_dir.path().join("python").exists());
        assert!(!repo.remove_package_family("python", true).unwrap());
    }

    #[test]
    fn remove_ignored_since_supports_dry_run_and_removal() {
        let temp_dir = TempDir::new().unwrap();
        let family_dir = temp_dir.path().join("python");
        fs::create_dir_all(family_dir.join("3.12.0")).unwrap();
        fs::write(family_dir.join(".ignore3.12.0"), "").unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());

        assert_eq!(repo.remove_ignored_since(0, true, false).unwrap(), 1);
        assert!(family_dir.join("3.12.0").exists());
        assert_eq!(repo.remove_ignored_since(0, false, false).unwrap(), 1);
        assert!(!family_dir.join("3.12.0").exists());
        assert!(!family_dir.join(".ignore3.12.0").exists());
    }

    #[test]
    fn payload_and_parent_resources_are_derived_from_repository_location() {
        let temp_dir = TempDir::new().unwrap();
        let repo = FilesystemPackageRepository::new(temp_dir.path());
        let mut package = Package::new("python".to_string());
        package.version = Some(rez_next_version::Version::parse("3.12.0").unwrap());
        let package = PackageResource::new(
            package,
            FILESYSTEM_REPO_TYPE.to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.12.0".to_string()),
            0,
            FILESYSTEM_REPO_TYPE.to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        assert_eq!(
            repo.get_package_payload_path("python", Some("3.12.0"))
                .unwrap(),
            Some(temp_dir.path().join("python/3.12.0"))
        );
        assert_eq!(
            repo.get_parent_package_family(&package).unwrap().name,
            "python"
        );
        assert_eq!(repo.get_parent_package(&variant).unwrap().name(), "python");
    }

    #[test]
    fn destructive_operations_reject_read_only_and_unsafe_paths() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("python/3.12.0")).unwrap();
        let mut read_only = FilesystemPackageRepository::new(temp_dir.path()).with_read_only(true);
        assert!(read_only.remove_package("python", Some("3.12.0")).is_err());

        let mut repo = FilesystemPackageRepository::new(temp_dir.path());
        assert!(repo.remove_package("../outside", Some("3.12.0")).is_err());
        assert!(repo.remove_package("python", Some("../outside")).is_err());
        assert!(repo.remove_package("python", None).is_err());
    }

    #[test]
    fn install_variant_fails_closed_until_supported() {
        let temp_dir = TempDir::new().unwrap();
        let mut repo = FilesystemPackageRepository::new(temp_dir.path());
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.12.0".to_string()),
            0,
            FILESYSTEM_REPO_TYPE.to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        );

        assert!(repo.install_variant(&variant, false, None).is_err());
    }
}
