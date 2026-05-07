//! PackageResource implementation
//!
//! Represents a specific version of a package in a repository.

use super::ResourceHandle;
use rez_next_package::Package;
use serde::{Deserialize, Serialize};

/// Package resource
///
/// This corresponds to the PackageResource class in rez's package_repository.py.
/// It represents a specific version of a package (e.g., "python-3.9.0").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageResource {
    /// The package data
    #[serde(flatten)]
    pub package: Package,
    /// Repository type that owns this resource
    pub repository_type: String,
    /// Repository location
    pub repository_location: String,
    /// Resource handle for unique identification
    pub handle: Option<ResourceHandle>,
}

impl PackageResource {
    /// Create a new package resource
    pub fn new(package: Package, repository_type: String, repository_location: String) -> Self {
        Self {
            package,
            repository_type,
            repository_location,
            handle: None,
        }
    }

    /// Set the resource handle
    pub fn with_handle(mut self, handle: ResourceHandle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Get the unique identifier for this resource
    pub fn get_handle(&self) -> ResourceHandle {
        match &self.handle {
            Some(h) => h.clone(),
            None => {
                let mut variables = std::collections::HashMap::new();
                variables.insert("name".to_string(), self.package.name.clone());
                if let Some(version) = &self.package.version {
                    variables.insert("version".to_string(), version.as_str().to_string());
                }
                ResourceHandle::new(
                    self.repository_type.clone(),
                    self.repository_location.clone(),
                    variables,
                )
            }
        }
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.package.name
    }

    /// Get the package version
    pub fn version(&self) -> Option<&rez_next_version::Version> {
        self.package.version.as_ref()
    }

}

/// Display trait implementation for PackageResource
///
/// Format: `{repository_type}@{location}/{name}-{version}`
impl std::fmt::Display for PackageResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version_str = self
            .package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");
        write!(
            f,
            "{}@{}/{}-{}",
            self.repository_type, self.repository_location, self.package.name, version_str
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use rez_next_version::Version;

    fn make_package(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg
    }

    #[test]
    fn test_package_resource_create() {
        let pkg = make_package("python", "3.9.0");
        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(resource.name(), "python");
        assert!(resource.version().is_some());
        assert_eq!(resource.version().unwrap().as_str(), "3.9.0");
        assert!(resource.handle.is_none());
    }

    #[test]
    fn test_package_resource_to_string() {
        let pkg = make_package("python", "3.9.0");
        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(
            resource.to_string(),
            "filesystem@/packages/python-3.9.0"
        );
    }

    #[test]
    fn test_package_resource_get_handle() {
        let pkg = make_package("python", "3.9.0");
        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        let handle = resource.get_handle();
        assert_eq!(handle.repository_type, "filesystem");
        assert_eq!(handle.repository_location, "/packages");
        assert_eq!(handle.variables.get("name"), Some(&"python".to_string()));
        assert_eq!(
            handle.variables.get("version"),
            Some(&"3.9.0".to_string())
        );
    }

    #[test]
    fn test_package_resource_with_handle() {
        let pkg = make_package("python", "3.9.0");
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "python".to_string());
        variables.insert("version".to_string(), "3.9.0".to_string());

        let handle = ResourceHandle::new(
            "filesystem".to_string(),
            "/packages".to_string(),
            variables,
        );

        let resource = PackageResource::new(
            pkg,
            "filesystem".to_string(),
            "/packages".to_string(),
        )
        .with_handle(handle.clone());

        assert!(resource.handle.is_some());
        let retrieved_handle = resource.get_handle();
        assert_eq!(retrieved_handle.repository_type, "filesystem");
    }
}
