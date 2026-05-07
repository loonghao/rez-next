//! Release hook system for rez-next.
//!
//! This module provides functionality for custom behaviour during package releases.
//! It is a Rust reimplementation of rez's `release_hook.py` module.
//!
//! # Examples
//!
//! ```rust
//! use rez_next_release_hook::{ReleaseHook, ReleaseHookEvent, ReleaseHookError};
//! use std::path::PathBuf;
//!
//! // Create a custom release hook
//! struct MyHook;
//!
//! impl ReleaseHook for MyHook {
//!     fn name() -> String {
//!         "my_hook".to_string()
//!     }
//!
//!     fn new(_source_path: PathBuf) -> std::result::Result<Self, ReleaseHookError> {
//!         Ok(Self)
//!     }
//! }
//! ```

mod hook;
pub use hook::*;

mod event;
pub use event::*;

mod registry;
pub use registry::*;

mod builtin;
pub use builtin::*;

pub mod prelude {
    pub use super::{ReleaseHook, ReleaseHookEvent, ReleaseHookRegistry};
}

/// Initialize the release hook system.
///
/// This registers all built-in hooks with the global registry.
pub fn init() {
    register_builtin_hooks();
}

#[derive(Debug, thiserror::Error)]
pub enum ReleaseHookError {
    #[error("Release hook error: {0}")]
    Error(String),

    #[error("Release hook cancelled: {0}")]
    Cancelled(String),
}

pub type Result<T> = std::result::Result<T, ReleaseHookError>;
