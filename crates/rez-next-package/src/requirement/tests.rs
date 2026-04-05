//! Tests for requirement parsing and version constraints.

use std::collections::HashMap;

use rez_next_version::Version;

use super::types::{Requirement, VersionConstraint};

#[test]
fn test_basic_requirement_parsing() {
    let req: Requirement = "python".parse().unwrap();
    assert_eq!(req.name, "python");
    assert!(req.version_constraint.is_none());
    assert!(!req.weak);
}

#[test]
fn test_version_constraint_parsing() {
    let req: Requirement = "python>=3.8".parse().unwrap();
    assert_eq!(req.name, "python");
    assert!(matches!(
        req.version_constraint,
        Some(VersionConstraint::GreaterThanOrEqual(_))
    ));
}

#[test]
fn test_weak_requirement() {
    let req: Requirement = "~python>=3.8".parse().unwrap();
    assert_eq!(req.name, "python");
    assert!(req.weak);
}

#[test]
fn test_namespace_requirement() {
    let req: Requirement = "company::python>=3.8".parse().unwrap();
    assert_eq!(req.name, "python");
    assert_eq!(req.namespace, Some("company".to_string()));
    assert_eq!(req.qualified_name(), "company::python");
}

#[test]
fn test_wildcard_version() {
    let req: Requirement = "python==3.8.*".parse().unwrap();
    assert_eq!(req.name, "python");
    assert!(matches!(
        req.version_constraint,
        Some(VersionConstraint::Wildcard(_))
    ));

    let version = Version::parse("3.8.5").unwrap();
    assert!(req.is_satisfied_by(&version));

    let version = Version::parse("3.9.0").unwrap();
    assert!(!req.is_satisfied_by(&version));
}

#[test]
fn test_range_constraint() {
    let req: Requirement = "python>=3.8,<4.0".parse().unwrap();
    assert_eq!(req.name, "python");
    assert!(matches!(
        req.version_constraint,
        Some(VersionConstraint::Multiple(_))
    ));

    let version = Version::parse("3.9.0").unwrap();
    assert!(req.is_satisfied_by(&version));

    let version = Version::parse("4.0.1").unwrap();
    assert!(!req.is_satisfied_by(&version));
}

#[test]
fn test_platform_condition() {
    let mut req = Requirement::new("python".to_string());
    req.add_platform_condition("linux".to_string(), None, false);

    assert!(req.is_platform_satisfied("linux", None));
    assert!(!req.is_platform_satisfied("windows", None));
}

#[test]
fn test_env_condition() {
    let mut req = Requirement::new("python".to_string());
    req.add_env_condition("PYTHON_VERSION".to_string(), Some("3.8".to_string()), false);

    let mut env_vars = HashMap::new();
    env_vars.insert("PYTHON_VERSION".to_string(), "3.8".to_string());
    assert!(req.is_env_satisfied(&env_vars));

    env_vars.insert("PYTHON_VERSION".to_string(), "3.9".to_string());
    assert!(!req.is_env_satisfied(&env_vars));
}

#[test]
fn test_version_constraint_and_or() {
    let constraint1 = VersionConstraint::GreaterThanOrEqual(Version::parse("1.0").unwrap());
    let constraint2 = VersionConstraint::LessThan(Version::parse("2.0").unwrap());

    let combined = constraint1.and(constraint2);
    assert!(matches!(combined, VersionConstraint::Multiple(_)));

    let version = Version::parse("1.5").unwrap();
    assert!(combined.is_satisfied_by(&version));

    let version = Version::parse("2.5").unwrap();
    assert!(!combined.is_satisfied_by(&version));
}

#[test]
fn test_compatible_version() {
    // ~=1.4 (2 segments): prefix=["1"] locked, minor >= 4
    let constraint = VersionConstraint::Compatible(Version::parse("1.4").unwrap());

    assert!(constraint.is_satisfied_by(&Version::parse("1.4.2").unwrap()));
    assert!(constraint.is_satisfied_by(&Version::parse("1.5.0").unwrap()));
    assert!(constraint.is_satisfied_by(&Version::parse("1.4").unwrap()));
    assert!(!constraint.is_satisfied_by(&Version::parse("2.0.0").unwrap()));
    assert!(!constraint.is_satisfied_by(&Version::parse("1.3").unwrap()));

    // ~=1.4.0 (3 segments): prefix=["1","4"] locked, patch >= 0
    let constraint3 = VersionConstraint::Compatible(Version::parse("1.4.0").unwrap());

    assert!(constraint3.is_satisfied_by(&Version::parse("1.4.0").unwrap()));
    assert!(constraint3.is_satisfied_by(&Version::parse("1.4.5").unwrap()));
    assert!(!constraint3.is_satisfied_by(&Version::parse("1.5.0").unwrap()));
    assert!(!constraint3.is_satisfied_by(&Version::parse("1.3.9").unwrap()));
}

#[test]
fn test_requirement_display() {
    let req: Requirement = "~company::python>=3.8".parse().unwrap();
    let display_str = req.to_string();
    assert!(display_str.contains('~'));
    assert!(display_str.contains("company::"));
    assert!(display_str.contains("python"));
    assert!(display_str.contains(">=3.8"));
}
