//! # Rez Core Common
//!
//! Common utilities and types shared across Rez Core components.
//!
//! This crate provides:
//! - Error handling types
//! - Configuration management
//! - Utility functions
//! - Shared data structures

pub mod config;
pub mod error;
pub mod utils;

// Re-export commonly used types
pub use config::RezCoreConfig;
pub use error::RezCoreError;
pub use error::RezCoreResult;
