//! PackageFamilyResource implementation
//!
//! Represents a package family in a repository.
//! A package family is a named group of package versions.

use super::ResourceHandle;
use serde::{Deserialize, Serialize};

/// Package family resource
///
/// This corresponds to the PackageFamilyResource class in rez's package_repository.py.
/// It represents a named group of package versions (e.g., "python" is a family
/// that contains versions like "3.7", "3.8", "3.9", etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageFamilyResource {
    /// Package family name (e.g., "python", "maya")
    pub name: String,
    /// Repository type that owns this resource
    pub repository_type: String,
    /// Repository location
    pub repository_location: String,
    /// Resource handle for unique identification
    pub handle: Option<ResourceHandle>,
}

impl PackageFamilyResource {
    /// Create a new package family resource
    pub fn new(name: String, repository_type: String, repository_location: String) -> Self {
        Self {
            name,
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
                variables.insert("name".to_string(), self.name.clone());
                ResourceHandle::new(
                    self.repository_type.clone(),
                    self.repository_location.clone(),
                    variables,
                )
            }
        }
    }

}

/// Display trait implementation for PackageFamilyResource
///
/// Format: `{repository_type}@{location}/{name}`
impl std::fmt::Display for PackageFamilyResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}@{}/{}",
            self.repository_type, self.repository_location, self.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_family_resource_create() {
        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(family.name, "python");
        assert_eq!(family.repository_type, "filesystem");
        assert_eq!(family.repository_location, "/packages");
        assert!(family.handle.is_none());
    }

    #[test]
    fn test_package_family_resource_to_string() {
        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(family.to_string(), "filesystem@/packages/python");
    }

    #[test]
    fn test_package_family_resource_get_handle() {
        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        let handle = family.get_handle();
        assert_eq!(handle.repository_type, "filesystem");
        assert_eq!(handle.repository_location, "/packages");
        assert_eq!(handle.variables.get("name"), Some(&"python".to_string()));
    }

    #[test]
    fn test_package_family_resource_with_handle() {
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "python".to_string());

        let handle = ResourceHandle::new(
            "filesystem".to_string(),
            "/packages".to_string(),
            variables,
        );

        let family = PackageFamilyResource::new(
            "python".to_string(),
            "filesystem".to_string(),
            "/packages".to_string(),
        )
        .with_handle(handle.clone());

        assert!(family.handle.is_some());
        assert_eq!(family.get_handle().repository_type, "filesystem");
    }
}
