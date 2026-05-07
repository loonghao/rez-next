//! VariantResource implementation
//!
//! Represents a specific variant of a package in a repository.
//! A variant is a specific build/configuration of a package version.

use crate::resources::ResourceHandle;
use rez_next_package::{Package, PackageRequirement};
use serde::{Deserialize, Serialize};

/// Variant resource
///
/// This corresponds to the VariantResource class in rez's package_repository.py.
/// It represents a specific variant (build) of a package version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantResource {
    /// Package name
    pub name: String,
    /// Package version (as string for serialization)
    pub version: Option<String>,
    /// Variant index (0-based)
    pub index: usize,
    /// Variant requirements (environment-specific dependencies)
    pub requirements: Vec<PackageRequirement>,
    /// Variant metadata
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Repository type that owns this resource
    pub repository_type: String,
    /// Repository location
    pub repository_location: String,
    /// Resource handle for unique identification
    pub handle: Option<ResourceHandle>,
    /// Root path for this variant (where it's installed)
    pub root: Option<String>,
}

impl VariantResource {
    /// Create a new variant resource
    pub fn new(
        name: String,
        version: Option<String>,
        index: usize,
        repository_type: String,
        repository_location: String,
    ) -> Self {
        Self {
            name,
            version,
            index,
            requirements: Vec::new(),
            metadata: std::collections::HashMap::new(),
            repository_type,
            repository_location,
            handle: None,
            root: None,
        }
    }

    /// Create from a Package's variant
    ///
    /// Note: In rez_next_package, variants are stored as Vec<Vec<String>>
    /// where each inner Vec<String> represents the requirements for that variant.
    pub fn from_package(
        package: &Package,
        variant_index: usize,
        repository_type: String,
        repository_location: String,
    ) -> Option<Self> {
        // Check if variant_index is valid
        if variant_index >= package.variants.len() {
            return None;
        }

        let variant_reqs = &package.variants[variant_index];

        let version_str = package.version.as_ref().map(|v| v.as_str().to_string());

        let mut requirements = Vec::new();
        for req_str in variant_reqs {
            // PackageRequirement::parse returns Result<PackageRequirement, ?>
            if let Ok(req) = PackageRequirement::parse(req_str) {
                requirements.push(req);
            }
        }

        Some(Self {
            name: package.name.clone(),
            version: version_str,
            index: variant_index,
            requirements,
            metadata: std::collections::HashMap::new(),
            repository_type,
            repository_location,
            handle: None,
            root: None,
        })
    }

    /// Set the resource handle
    pub fn with_handle(mut self, handle: ResourceHandle) -> Self {
        self.handle = Some(handle);
        self
    }

    /// Set the root path
    pub fn with_root(mut self, root: String) -> Self {
        self.root = Some(root);
        self
    }

    /// Get the unique identifier for this resource
    pub fn get_handle(&self) -> ResourceHandle {
        match &self.handle {
            Some(h) => h.clone(),
            None => {
                let mut variables = std::collections::HashMap::new();
                variables.insert("name".to_string(), self.name.clone());
                if let Some(ref version) = self.version {
                    variables.insert("version".to_string(), version.clone());
                }
                variables.insert("index".to_string(), self.index.to_string());
                ResourceHandle::new(
                    self.repository_type.clone(),
                    self.repository_location.clone(),
                    variables,
                )
            }
        }
    }

    /// Get the variant index
    pub fn index(&self) -> usize {
        self.index
    }

}

/// Display trait implementation for VariantResource
///
/// Format: `{repository_type}@{location}/{name}-{version}(variant {index})`
impl std::fmt::Display for VariantResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version_str = self.version.as_deref().unwrap_or("unknown");
        write!(
            f,
            "{}@{}/{}-{}(variant {})",
            self.repository_type,
            self.repository_location,
            self.name,
            version_str,
            self.index
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use rez_next_version::Version;

    fn make_package_with_variant(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());

        // Add a simple variant (Vec<String> for requirements)
        pkg.variants = vec![vec!["maya".to_string(), "houdini".to_string()]];
        pkg
    }

    #[test]
    fn test_variant_resource_create() {
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(variant.name, "python");
        assert_eq!(variant.version, Some("3.9.0".to_string()));
        assert_eq!(variant.index(), 0);
        assert!(variant.handle.is_none());
    }

    #[test]
    fn test_variant_resource_from_package() {
        let pkg = make_package_with_variant("python", "3.9.0");
        let variant = VariantResource::from_package(
            &pkg,
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert!(variant.is_some());
        let variant = variant.unwrap();
        assert_eq!(variant.name, "python");
        assert_eq!(variant.version, Some("3.9.0".to_string()));
    }

    #[test]
    fn test_variant_resource_to_string() {
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        assert_eq!(
            variant.to_string(),
            "filesystem@/packages/python-3.9.0(variant 0)"
        );
    }

    #[test]
    fn test_variant_resource_get_handle() {
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        );

        let handle = variant.get_handle();
        assert_eq!(handle.repository_type, "filesystem");
        assert_eq!(handle.repository_location, "/packages");
        assert_eq!(handle.variables.get("name"), Some(&"python".to_string()));
        assert_eq!(handle.variables.get("index"), Some(&"0".to_string()));
    }

    #[test]
    fn test_variant_resource_with_handle() {
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "python".to_string());
        variables.insert("index".to_string(), "0".to_string());

        let handle = ResourceHandle::new(
            "filesystem".to_string(),
            "/packages".to_string(),
            variables,
        );

        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        )
        .with_handle(handle.clone());

        assert!(variant.handle.is_some());
        let retrieved_handle = variant.get_handle();
        assert_eq!(retrieved_handle.repository_type, "filesystem");
    }

    #[test]
    fn test_variant_resource_with_root() {
        let variant = VariantResource::new(
            "python".to_string(),
            Some("3.9.0".to_string()),
            0,
            "filesystem".to_string(),
            "/packages".to_string(),
        )
        .with_root("/packages/python/3.9.0/platform-windows".to_string());

        assert_eq!(
            variant.root,
            Some("/packages/python/3.9.0/platform-windows".to_string())
        );
    }
}
