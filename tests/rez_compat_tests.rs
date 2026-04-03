//! Rez Compatibility Integration Tests
//!
//! These tests verify that rez-next implements the same behavior as the original
//! rez package manager. Test cases are derived from rez's official test suite
//! and documentation examples.
//!
//! Originally a monolithic 6925-line file, now split into `compat/` sub-modules.
//! Each sub-module is under 500 lines (rule: max 1000 lines per file).

#[path = "compat/mod.rs"]
mod compat;
