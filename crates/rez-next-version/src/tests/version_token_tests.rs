//! Tests for VersionToken compatibility with rez
//!
//! Note: VersionToken tests were removed because the `version_token` module
//! and all related types (SubToken, AlphanumericVersionToken, NumericToken)
//! were gated behind `#[cfg(feature = "python-bindings")]` which is never
//! defined in any Cargo.toml. These tests could never compile or run.
//! See CLEANUP_TODO.md item #1 for the full python-bindings audit.
