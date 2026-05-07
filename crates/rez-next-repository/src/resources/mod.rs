//! Resource types for package repository
//!
//! This module defines the resource types that represent packages in a repository.
//! These types are compatible with the original rez package_repository.py implementation.

// ── Submodules ──────────────────────────────────────────────────────────────────
pub mod family;
pub mod package;
pub mod variant;

// ── Re-exports ──────────────────────────────────────────────────────────────────
pub use family::PackageFamilyResource;
pub use package::PackageResource;
pub use variant::VariantResource;

// ── Common types ─────────────────────────────────────────────────────────────────

/// Resource handle for unique identification
///
/// A resource handle uniquely identifies a resource in a repository.
/// It consists of the repository type, location, and resource-specific variables.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResourceHandle {
    /// Repository type (e.g., "filesystem", "memory")
    pub repository_type: String,
    /// Repository location (e.g., "/path/to/packages")
    pub repository_location: String,
    /// Resource-specific variables for unique identification
    pub variables: std::collections::HashMap<String, String>,
}

// Manual implementation of Hash to handle HashMap
// We hash the sorted key-value pairs to ensure deterministic hash
impl std::hash::Hash for ResourceHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.repository_type.hash(state);
        self.repository_location.hash(state);

        // Sort variables by key for deterministic hashing
        let mut sorted_vars: Vec<_> = self.variables.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_vars {
            key.hash(state);
            value.hash(state);
        }
    }
}

impl ResourceHandle {
    /// Create a new resource handle
    pub fn new(
        repository_type: String,
        repository_location: String,
        variables: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            repository_type,
            repository_location,
            variables,
        }
    }

}

/// Display trait implementation for ResourceHandle
///
/// Format: `{repository_type}@{location}#{resource_path}`
impl std::fmt::Display for ResourceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let resource_path = self
            .variables
            .get("name")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        write!(
            f,
            "{}@{}/{}",
            self.repository_type, self.repository_location, resource_path
        )
    }
}

/// Resource pool for managing resource instances
///
/// This is a simplified version of rez's ResourcePool.
/// In a full implementation, this would manage caching and deduplication of resources.
#[derive(Debug, Default)]
pub struct ResourcePool {
    // In the full implementation, this would contain caches
}

impl ResourcePool {
    /// Create a new resource pool
    pub fn new() -> Self {
        Self {}
    }

    /// Clear all cached resources
    pub fn clear_caches(&mut self) {
        // In the full implementation, this would clear caches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_handle_create() {
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "python".to_string());
        variables.insert("version".to_string(), "3.9.0".to_string());

        let handle = ResourceHandle::new(
            "filesystem".to_string(),
            "/packages".to_string(),
            variables,
        );

        assert_eq!(handle.repository_type, "filesystem");
        assert_eq!(handle.repository_location, "/packages");
        assert_eq!(
            handle.variables.get("name"),
            Some(&"python".to_string())
        );
    }

    #[test]
    fn test_resource_handle_to_string() {
        let mut variables = std::collections::HashMap::new();
        variables.insert("name".to_string(), "python".to_string());

        let handle = ResourceHandle::new(
            "filesystem".to_string(),
            "/packages".to_string(),
            variables,
        );

        assert_eq!(handle.to_string(), "filesystem@/packages/python");
    }

    #[test]
    fn test_resource_pool_create() {
        let pool = ResourcePool::new();
        // Just verify it creates without error
        let _ = pool;
    }

    #[test]
    fn test_resource_pool_clear_caches() {
        let mut pool = ResourcePool::new();
        pool.clear_caches();
        // Just verify it doesn't panic
    }
}
