//! Built-in release hook implementations.
//!
//! This module provides sample hooks that can be used for testing
//! or as references for custom hook implementations.

use std::path::PathBuf;
use crate::{ReleaseHook, Result};

/// A no-op release hook that does nothing.
///
/// This hook is useful for testing or as a placeholder.
/// All hook methods return `Ok(())` without doing anything.
pub struct NoopHook;

impl ReleaseHook for NoopHook {
    fn name() -> String {
        "noop".to_string()
    }

    fn new(_source_path: PathBuf) -> Result<Self> {
        Ok(Self)
    }
}

/// A logging release hook that logs messages when hooks are called.
///
/// This hook is useful for debugging the release process.
/// It logs the event name and package path at INFO level.
pub struct LoggingHook;

impl ReleaseHook for LoggingHook {
    fn name() -> String {
        "logging".to_string()
    }

    fn new(_source_path: PathBuf) -> Result<Self> {
        Ok(Self)
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
    ) -> Result<()> {
        tracing::info!("ReleaseHook: pre_build called");
        Ok(())
    }

    fn pre_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: Option<&[usize]>,
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!("ReleaseHook: pre_release called");
        Ok(())
    }

    fn post_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: &[String],
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> Result<()> {
        tracing::info!("ReleaseHook: post_release called");
        Ok(())
    }
}

/// Register all built-in hooks with the global registry.
///
/// This function should be called during initialization.
pub fn register_builtin_hooks() {
    let registry = crate::get_registry();
    
    registry.register("noop", |_path| {
        Ok(Box::new(NoopHook) as Box<dyn ReleaseHook>)
    });
    
    registry.register("logging", |path| {
        Ok(Box::new(LoggingHook::new(path)?) as Box<dyn ReleaseHook>)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_noop_hook_creation() {
        let hook = NoopHook::new(PathBuf::from("/tmp"));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_noop_hook_name() {
        assert_eq!(NoopHook::name(), "noop");
    }

    #[test]
    fn test_noop_hook_pre_build() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_build(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_hook_pre_release() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_release(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_noop_hook_post_release() {
        let hook = NoopHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.post_release(
            "test_user",
            Path::new("/tmp/install"),
            &[],
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_logging_hook_creation() {
        let hook = LoggingHook::new(PathBuf::from("/tmp"));
        assert!(hook.is_ok());
    }

    #[test]
    fn test_logging_hook_name() {
        assert_eq!(LoggingHook::name(), "logging");
    }

    #[test]
    fn test_logging_hook_pre_build() {
        let hook = LoggingHook::new(PathBuf::from("/tmp")).unwrap();
        let result = hook.pre_build(
            "test_user",
            Path::new("/tmp/install"),
            None,
            None,
            None,
            None,
            None,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_register_builtin_hooks() {
        // Note: This test verifies the function runs without panic
        // The global registry is shared, so we can't easily assert registration
        register_builtin_hooks();
        
        let types = crate::get_release_hook_types();
        // After registration, "noop" and "logging" should be available
        assert!(types.contains(&"noop".to_string()));
        assert!(types.contains(&"logging".to_string()));
    }
}
