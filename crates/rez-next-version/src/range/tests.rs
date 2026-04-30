//! Tests for VersionRange!

use super::VersionRange;
use crate::Version;

fn ver(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[test]
fn test_range_any() {
    let range = VersionRange::any();
    assert!(range.contains(&ver("1.0.0")));
    assert!(range.contains(&ver("999.999.999")));
    assert!(range.contains(&ver("0.0.0")));
}

#[test]
fn test_range_none() {
    let range = VersionRange::none();
    assert!(!range.contains(&ver("1.0.0")));
    assert!(!range.contains(&ver("0.0.0")));
}

#[test]
fn test_range_parse_empty() {
    let range = VersionRange::parse("").unwrap();
    // Empty string should match any version
    assert!(range.contains(&ver("1.0.0")));
}

#[test]
fn test_range_parse_star() {
    let range = VersionRange::parse("*").unwrap();
    // "*" should match any version
    assert!(range.contains(&ver("1.0.0")));
}

#[test]
fn test_range_parse_exact() {
    let range = VersionRange::parse("==1.2.3").unwrap();
    assert!(range.contains(&ver("1.2.3")));
    assert!(!range.contains(&ver("1.2.4")));
}

#[test]
fn test_range_parse_ge() {
    let range = VersionRange::parse(">=1.0.0").unwrap();
    assert!(range.contains(&ver("1.0.0")));
    assert!(range.contains(&ver("1.0.1")));
    assert!(!range.contains(&ver("0.9.9")));
}

#[test]
fn test_range_parse_gt() {
    let range = VersionRange::parse(">1.0.0").unwrap();
    assert!(!range.contains(&ver("1.0.0")));
    assert!(range.contains(&ver("1.0.1")));
}

#[test]
fn test_range_parse_le() {
    let range = VersionRange::parse("<=2.0.0").unwrap();
    assert!(range.contains(&ver("2.0.0")));
    assert!(range.contains(&ver("1.9.9")));
    assert!(!range.contains(&ver("2.0.1")));
}

#[test]
fn test_range_parse_lt() {
    let range = VersionRange::parse("<2.0.0").unwrap();
    assert!(!range.contains(&ver("2.0.0")));
    assert!(range.contains(&ver("1.9.9")));
}

#[test]
fn test_range_parse_compatible() {
    // ~=1.4.0 means >=1.4.0, <1.5.0
    let range = VersionRange::parse("~=1.4.0").unwrap();
    assert!(range.contains(&ver("1.4.0")));
    assert!(range.contains(&ver("1.4.5")));
    assert!(!range.contains(&ver("1.5.0")));
    assert!(!range.contains(&ver("1.3.0")));
}

#[test]
fn test_range_parse_not_equal() {
    let range = VersionRange::parse("!=1.2.3").unwrap();
    assert!(!range.contains(&ver("1.2.3")));
    assert!(range.contains(&ver("1.2.4")));
}

#[test]
fn test_range_subtract() {
    let range1 = VersionRange::parse("*").unwrap();
    let range2 = VersionRange::parse("==1.0.0").unwrap();
    let subtracted = range1.subtract(&range2).expect("Subtraction should work");
    assert!(!subtracted.contains(&ver("1.0.0")));
    assert!(subtracted.contains(&ver("1.0.1")));
}
