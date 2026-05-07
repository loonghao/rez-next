//! Release hook registry.
//!
//! Manages registration and creation of release hooks.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use crate::{ReleaseHook, ReleaseHookError, Result};

type HookFactory = Box<dyn Fn(PathBuf) -> Result<Box<dyn ReleaseHook>> + Send + Sync>;

/// Registry for release hooks.
///
/// This manages the registration and creation of release hook instances.
#[derive(Default)]
pub struct ReleaseHookRegistry {
    hooks: Mutex<HashMap<String, HookFactory>>,
}

impl ReleaseHookRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            hooks: Mutex::new(HashMap::new()),
        }
    }

    /// Register a hook type.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the hook type (e.g., "email", "webhook")
    /// * `factory` - Factory function that creates hook instances
    pub fn register<F>(&self, name: &str, factory: F)
    where
        F: Fn(PathBuf) -> Result<Box<dyn ReleaseHook>> + Send + Sync + 'static,
    {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.insert(name.to_string(), Box::new(factory));
    }

    /// Get the names of all registered hook types.
    pub fn get_hook_types(&self) -> Vec<String> {
        let hooks = self.hooks.lock().unwrap();
        hooks.keys().cloned().collect()
    }

    /// Create a new hook instance by name.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the hook type to create
    /// * `source_path` - Path containing source that was released
    ///
    /// # Returns
    ///
    /// The created hook instance, or an error if the hook type is not found.
    pub fn create_hook(&self, name: &str, source_path: PathBuf) -> Result<Box<dyn ReleaseHook>> {
        let hooks = self.hooks.lock().unwrap();
        match hooks.get(name) {
            Some(factory) => factory(source_path),
            None => Err(ReleaseHookError::Error(format!(
                "Release hook '{}' is not available",
                name
            ))),
        }
    }

    /// Create multiple hook instances.
    ///
    /// Returns a list of successfully created hooks.
    /// Hooks that cannot be created are logged as warnings.
    pub fn create_hooks(&self, names: &[&str], source_path: PathBuf) -> Vec<Box<dyn ReleaseHook>> {
        let mut hooks = Vec::new();

        for &name in names {
            match self.create_hook(name, source_path.clone()) {
                Ok(hook) => hooks.push(hook),
                Err(e) => {
                    tracing::warn!("Release hook '{}' is not available: {}", name, e);
                }
            }
        }

        hooks
    }
}

/// Global registry instance.
static GLOBAL_REGISTRY: std::sync::OnceLock<ReleaseHookRegistry> = std::sync::OnceLock::new();

/// Get the global release hook registry.
pub fn get_registry() -> &'static ReleaseHookRegistry {
    GLOBAL_REGISTRY.get_or_init(|| {
        ReleaseHookRegistry::new()
    })
}

/// Get a mutable reference to the global registry.
/// Note: This is only possible if we have exclusive access.
/// In practice, users should use `get_registry()` and the registry
/// should use internal mutability (e.g., Mutex) if needed.
pub fn get_registry_mut() -> &'static ReleaseHookRegistry {
    // OnceLock doesn't allow mutation after initialization.
    // For mutation, users should use interior mutability.
    get_registry()
}

/// Helper function to get available hook types (matches Python API).
pub fn get_release_hook_types() -> Vec<String> {
    get_registry().get_hook_types()
}

/// Helper function to create a hook by name (matches Python API).
pub fn create_release_hook(name: &str, source_path: PathBuf) -> Result<Box<dyn ReleaseHook>> {
    get_registry().create_hook(name, source_path)
}

/// Helper function to create multiple hooks (matches Python API).
pub fn create_release_hooks(names: &[&str], source_path: PathBuf) -> Vec<Box<dyn ReleaseHook>> {
    get_registry().create_hooks(names, source_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NoopHook;

    /// Mock hook for testing.
    #[derive(Debug)]
    struct MockHook {
        #[allow(dead_code)]
        source_path: std::path::PathBuf,
    }

    impl ReleaseHook for MockHook {
        fn name() -> String {
            "mock_hook".to_string()
        }

        fn new(source_path: std::path::PathBuf) -> std::result::Result<Self, ReleaseHookError> {
            Ok(Self { source_path })
        }

        fn pre_build(
            &self,
            _user: &str,
            _install_path: &std::path::Path,
            _variants: Option<&[usize]>,
            _release_message: Option<&str>,
            _changelog: Option<&[String]>,
            _previous_version: Option<&str>,
            _previous_revision: Option<&str>,
        ) -> std::result::Result<(), ReleaseHookError> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = ReleaseHookRegistry::new();
        assert!(registry.get_hook_types().is_empty());
    }

    #[test]
    fn test_registry_register_and_create() {
        let registry = ReleaseHookRegistry::new();

        registry.register("mock_hook", |path| {
            Ok(Box::new(MockHook { source_path: path }))
        });

        let types = registry.get_hook_types();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0], "mock_hook");

        let hook = registry.create_hook("mock_hook", std::path::PathBuf::from("/tmp"));
        assert!(hook.is_ok());

        let hook = registry.create_hook("non_existent", std::path::PathBuf::from("/tmp"));
        assert!(hook.is_err());
    }

    #[test]
    fn test_registry_create_hooks() {
        let registry = ReleaseHookRegistry::new();

        registry.register("hook1", |path| {
            Ok(Box::new(MockHook { source_path: path }))
        });
        registry.register("hook2", |path| {
            Ok(Box::new(MockHook { source_path: path }))
        });

        let hooks = registry.create_hooks(
            &["hook1", "hook2", "non_existent"],
            std::path::PathBuf::from("/tmp"),
        );

        assert_eq!(hooks.len(), 2);
    }

    #[test]
    fn test_get_release_hook_types() {
        // Test that get_release_hook_types() returns a list
        // Note: global registry may not be empty (other tests may register hooks)
        let types = get_release_hook_types();
        // Just verify it returns a Vec<String>
        let _: Vec<String> = types;
        
        // Test that we can register a hook and it appears in the list
        let registry = GLOBAL_REGISTRY.get_or_init(ReleaseHookRegistry::new);
        registry.register("test_hook_type_for_test", |path| {
            NoopHook::new(path).map(|h| Box::new(h) as Box<dyn ReleaseHook>)
        });
        
        let types_after = get_release_hook_types();
        assert!(types_after.contains(&"test_hook_type_for_test".to_string()));
    }
}
