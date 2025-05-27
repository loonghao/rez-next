//! # Rez Core Repository
//!
//! Repository scanning, caching, and management for Rez Core.
//!
//! This crate provides:
//! - Repository scanning and indexing
//! - Package discovery and caching
//! - Repository metadata management
//! - Async repository operations

// Re-export from mod.rs for now
pub use mod_repository::*;

// Rename the module to avoid conflicts
#[path = "mod.rs"]
mod mod_repository;
