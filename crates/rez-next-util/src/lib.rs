//! Utility functions for rez-next
//!
//! This crate provides common utility functions used across the rez-next project,
//! including file system utilities, string utilities, time utilities, command
//! execution, and platform detection.

use std::env;

use rez_next_common::RezCoreError;

// ── Module declarations ──────────────────────────────────────────
mod command;
mod filesystem;
mod platform;
mod string;
mod time;
pub mod which;

pub use command::*;
pub use filesystem::*;
pub use platform::*;
pub use string::*;
pub use time::*;
pub use which::*;

// ── Re-exports ─────────────────────────────────────────────────
// Common types used by other crates
pub use rez_next_common::RezCoreResult;
pub type RezResult<T> = RezCoreResult<T>;

/// Get the current rez-next version
#[inline]
pub fn get_rez_next_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get the name of the current executable
pub fn get_executable_name() -> RezResult<String> {
    env::current_exe()
        .map(|p| {
            p.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .map_err(RezCoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rez_next_version() {
        let version = get_rez_next_version();
        assert!(!version.is_empty());
        // Version should be in format X.Y.Z
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn test_get_executable_name() {
        let result = get_executable_name();
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(!name.is_empty());
    }
}
