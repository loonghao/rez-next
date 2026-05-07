//! Package filtering for rez-next.
//!
//! This module provides functionality to filter packages based on various rules.
//! It is a Rust reimplementation of rez's `package_filter.py` module.

mod rule;
pub use rule::*;

mod filter;
pub use filter::*;

pub mod prelude {
    pub use super::{PackageFilter, Rule, RuleMatch};
    pub use super::rule::{GlobRule, RegexRule, RangeRule, TimestampRule};
}

/// Re-exports for convenience.
pub use rule::{GlobRule, RegexRule, RangeRule, TimestampRule};
