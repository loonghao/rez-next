//! Explicit package support for rez-next.
//!
//! Explicit packages are packages that are explicitly defined (e.g., in a suite or
//! as a variant) rather than discovered from a package repository.

use rez_next_common::{RezCoreError, RezCoreResult};
use rez_next_package::Package;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// An explicit package definition.
///
/// Explicit packages are used in suites and variants to explicitly define
/// which packages are available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitPackage {
    /// The package name.
    pub name: String,
    /// The package version.
    pub version: Option<String>,
    /// The package path (if installed).
    pub path: Option<PathBuf>,
    /// Additional metadata.
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

impl ExplicitPackage {
    /// Create a new explicit package.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            version: None,
            path: None,
            metadata: None,
        }
    }

    /// Set the version.
    pub fn with_version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set the path.
    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    /// Convert to a full Package if possible.
    pub fn to_package(&self) -> Result<Package, RezCoreError> {
        // TODO: Implement conversion to full Package
        Err(RezCoreError::PackageParse(
            "ExplicitPackage::to_package not yet implemented".to_string(),
        ))
    }
}

/// A collection of explicit packages (e.g., a suite).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitPackages {
    /// The packages in this collection.
    pub packages: Vec<ExplicitPackage>,
    /// The name of this collection (e.g., suite name).
    pub name: Option<String>,
}

impl ExplicitPackages {
    /// Create a new empty collection.
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            name: None,
        }
    }

    /// Add a package to the collection.
    pub fn add_package(&mut self, package: ExplicitPackage) {
        self.packages.push(package);
    }

    /// Load from a JSON file.
    pub fn from_path<P: Into<PathBuf>>(path: P) -> RezCoreResult<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to read {}: {}", path.display(), e)))?;
        serde_json::from_str(&content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse {}: {}", path.display(), e)))
    }

    /// Save to a JSON file.
    pub fn to_path<P: Into<PathBuf>>(&self, path: P) -> RezCoreResult<()> {
        let path = path.into();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to serialize: {}", e)))?;
        std::fs::write(&path, content)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to write {}: {}", path.display(), e)))?;
        Ok(())
    }
}

impl Default for ExplicitPackages {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_explicit_package_creation() {
        let pkg = ExplicitPackage::new("python")
            .with_version("3.9.0")
            .with_path(PathBuf::from("/packages/python-3.9.0"));

        assert_eq!(pkg.name, "python");
        assert_eq!(pkg.version, Some("3.9.0".to_string()));
        assert_eq!(pkg.path, Some(PathBuf::from("/packages/python-3.9.0")));
    }

    #[test]
    fn test_explicit_packages_collection() {
        let mut collection = ExplicitPackages::new();
        collection.name = Some("my-suite".to_string());

        collection.add_package(ExplicitPackage::new("python").with_version("3.9.0"));
        collection.add_package(ExplicitPackage::new("maya").with_version("2024"));

        assert_eq!(collection.packages.len(), 2);
        assert_eq!(collection.name, Some("my-suite".to_string()));
    }

    #[test]
    fn test_explicit_packages_serialization() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("explicit.json");

        let mut collection = ExplicitPackages::new();
        collection.name = Some("test-suite".to_string());
        collection.add_package(ExplicitPackage::new("python").with_version("3.9.0"));

        // Save to file
        collection.to_path(&file_path).unwrap();

        // Load from file
        let loaded = ExplicitPackages::from_path(&file_path).unwrap();

        assert_eq!(loaded.name, Some("test-suite".to_string()));
        assert_eq!(loaded.packages.len(), 1);
        assert_eq!(loaded.packages[0].name, "python");
    }
}
