//! Release hook trait and base implementation.
//!
//! Defines the `ReleaseHook` trait that custom hooks must implement.

use crate::ReleaseHookError;
use std::path::PathBuf;

/// Trait that custom release hooks must implement.
///
/// A release hook provides methods that you implement to inject custom
/// behaviour during parts of the release process.
pub trait ReleaseHook: Send + Sync {
    /// Return name of this hook (e.g., "email", "webhook").
    fn name() -> String
    where
        Self: Sized;

    /// Create a new release hook instance.
    fn new(source_path: PathBuf) -> std::result::Result<Self, ReleaseHookError>
    where
        Self: Sized;

    /// Pre-build hook.
    ///
    /// Called before the build process starts.
    ///
    /// # Arguments
    ///
    /// * `user` - Name of person doing the release
    /// * `install_path` - Directory the package will be installed into
    /// * `variants` - List of variant indices being built, or None for all
    /// * `release_message` - User-supplied release message
    /// * `changelog` - List of strings describing changes since last release
    /// * `previous_version` - Previously released version, or None
    /// * `previous_revision` - Revision of previously-released package
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or `Err(ReleaseHookError::Cancelled)` to cancel.
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

    /// Pre-release hook.
    ///
    /// Called before any package variants are released.
    fn pre_release(
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

    /// Post-release hook.
    ///
    /// Called after all package variants have been released.
    fn post_release(
        &self,
        _user: &str,
        _install_path: &std::path::Path,
        _variants: &[String],
        _release_message: Option<&str>,
        _changelog: Option<&[String]>,
        _previous_version: Option<&str>,
        _previous_revision: Option<&str>,
    ) -> std::result::Result<(), ReleaseHookError> {
        Ok(())
    }
}
