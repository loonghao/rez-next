//! Integration tests for rez-core

use rez_core::common::RezCoreConfig;
use rez_core::version::{Version, VersionRange};

#[test]
fn test_version_creation() {
    let version = Version::parse("1.2.3").expect("Should create version");
    assert_eq!(version.as_str(), "1.2.3");
}

#[test]
fn test_version_range_creation() {
    let range = VersionRange::parse("1.0.0..2.0.0").expect("Should create version range");
    assert_eq!(range.as_str(), "1.0.0..2.0.0");
}

#[test]
fn test_version_token_creation() {
    let numeric_token = VersionToken::from_str("123");
    assert_eq!(numeric_token, VersionToken::Numeric(123));

    let alpha_token = VersionToken::from_str("alpha");
    assert_eq!(alpha_token, VersionToken::Alphanumeric("alpha".to_string()));
}

#[test]
fn test_version_token_comparison() {
    let num1 = VersionToken::Numeric(1);
    let num2 = VersionToken::Numeric(2);
    let alpha = VersionToken::Alphanumeric("alpha".to_string());

    assert!(num1 < num2);
    assert!(num1 < alpha);
}

#[test]
fn test_config_defaults() {
    let config = RezCoreConfig::default();
    assert!(config.use_rust_version);
    assert!(config.use_rust_solver);
    assert!(config.use_rust_repository);
    assert!(config.rust_fallback);
}

#[test]
fn test_module_structure() {
    // Test that all modules can be imported and basic functionality works
    let _version = Version::parse("1.0.0").expect("Version creation should work");
    let _range = VersionRange::parse("1.0.0+").expect("Range creation should work");
    let _config = RezCoreConfig::default();

    // This test ensures the basic module structure is working
    assert!(true);
}
