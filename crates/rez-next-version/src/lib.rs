//! # Rez Core Version
//!
//! Version parsing, comparison, and range handling for Rez Core.
//!
//! This crate provides:
//! - Version parsing and validation
//! - Version comparison and ordering
//! - Version range operations
//! - Token-based version representation

use rez_next_common::RezCoreError;

pub mod parser;
pub mod range; // Always available for benchmarks and core functionality
pub mod version;

// Re-export main types
pub use parser::{StateMachineParser, VersionParser};
// Always export VersionRange as it's needed by benchmarks and other core functionality
pub use range::VersionRange;
pub use version::Version;

// Define a custom error type for version parsing
#[derive(Debug)]
pub struct VersionParseError(pub String);

impl std::fmt::Display for VersionParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Version parse error: {}", self.0)
    }
}

impl std::error::Error for VersionParseError {}

impl From<RezCoreError> for VersionParseError {
    fn from(err: RezCoreError) -> Self {
        VersionParseError(err.to_string())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod range_tests;
