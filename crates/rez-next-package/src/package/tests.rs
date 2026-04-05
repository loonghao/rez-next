//! Tests for Package and PackageRequirement.

use rez_next_version::Version;

use super::requirement::PackageRequirement;
use super::types::Package;

fn ver(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[test]
fn test_pkg_req_satisfied_no_constraint() {
    let r = PackageRequirement::parse("python").unwrap();
    assert!(r.satisfied_by(&ver("3.9.0")));
}

#[test]
fn test_pkg_req_satisfied_ge() {
    let r = PackageRequirement::with_version("python".into(), ">=3.8.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(r.satisfied_by(&ver("3.8.0")));
    assert!(!r.satisfied_by(&ver("3.7.0")));
}

#[test]
fn test_pkg_req_satisfied_lt() {
    let r = PackageRequirement::with_version("python".into(), "<3.10.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(!r.satisfied_by(&ver("3.10.0")));
}

#[test]
fn test_pkg_req_satisfied_ne() {
    let r = PackageRequirement::with_version("python".into(), "!=3.8.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(!r.satisfied_by(&ver("3.8.0")));
}

#[test]
fn test_pkg_req_satisfied_compatible() {
    let r = PackageRequirement::with_version("mylib".into(), "~=1.4.0".into());
    assert!(r.satisfied_by(&ver("1.4.0")));
    assert!(r.satisfied_by(&ver("1.4.5")));
    assert!(!r.satisfied_by(&ver("1.5.0")));
}

#[test]
fn test_package_new_and_validate() {
    let pkg = Package::new("mylib".to_string());
    assert_eq!(pkg.name, "mylib");
    assert!(pkg.version.is_none());
    assert!(pkg.validate().is_ok());
}

#[test]
fn test_package_empty_name_invalid() {
    assert!(Package::new("".to_string()).validate().is_err());
}

#[test]
fn test_conflict_requirement_parse() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert_eq!(req.name, "python");
    assert!(req.conflict, "!python must be a conflict requirement");
    assert!(!req.weak);
    assert!(req.version_spec.is_none());
}

#[test]
fn test_conflict_requirement_with_version() {
    let req = PackageRequirement::parse("!python-3.9").unwrap();
    assert_eq!(req.name, "python");
    assert!(req.conflict);
    assert_eq!(req.version_spec.as_deref(), Some("3.9"));
}

#[test]
fn test_conflict_requirement_to_string() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert_eq!(req.to_string(), "!python");
}

#[test]
fn test_conflict_requirement_with_version_to_string() {
    let req = PackageRequirement::parse("!python-3.9").unwrap();
    assert_eq!(req.to_string(), "!python-3.9");
}

#[test]
fn test_weak_requirement_to_string() {
    let req = PackageRequirement::parse("~numpy").unwrap();
    assert_eq!(req.name, "numpy");
    assert!(req.weak);
    assert!(!req.conflict);
    assert_eq!(req.to_string(), "~numpy");
}

#[test]
fn test_normal_requirement_not_conflict_not_weak() {
    let req = PackageRequirement::parse("maya-2024").unwrap();
    assert!(!req.conflict);
    assert!(!req.weak);
    assert_eq!(req.name, "maya");
    assert_eq!(req.version_spec.as_deref(), Some("2024"));
}

#[test]
fn test_conflict_takes_priority_over_weak() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert!(req.conflict);
    assert!(!req.weak);
}
