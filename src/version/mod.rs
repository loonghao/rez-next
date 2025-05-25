//! Version system implementation
//! 
//! This module provides high-performance version parsing, comparison, and range operations.

pub mod version;
pub mod range;
pub mod token;
pub mod parser;

pub use version::Version;
pub use range::VersionRange;
pub use token::VersionToken;
