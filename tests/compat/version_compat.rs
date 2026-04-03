//! Rez Compatibility Integration Tests
//!
//! These tests verify that rez-next implements the same behavior as the original
//! rez package manager. Test cases are derived from rez's official test suite
//! and documentation examples.

use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Version compatibility tests ───────────────────────────────────────────

/// rez version parsing: numeric, alphanumeric, epoch-based
#[test]
fn test_rez_version_numeric() {
    let versions = ["1", "1.2", "1.2.3", "1.2.3.4"];
    for v in &versions {
        let parsed = Version::parse(v).unwrap_or_else(|_| panic!("Failed to parse version: {}", v));
        assert_eq!(parsed.as_str(), *v, "Version roundtrip failed for {}", v);
    }
}

#[test]
fn test_rez_version_ordering() {
    // Rez ordering: 1.0 > 1.0.0 (shorter is "greater epoch")
    let v1 = Version::parse("1.0").unwrap();
    let v2 = Version::parse("1.0.0").unwrap();
    // In rez semantics: 1.0 > 1.0.0
    assert!(v1 > v2, "1.0 should be greater than 1.0.0 in rez semantics");
}

#[test]
fn test_rez_version_compare_major_minor() {
    let cases = [
        ("2.0.0", "1.9.9", true),  // 2.0.0 > 1.9.9
        ("1.10.0", "1.9.0", true), // 1.10 > 1.9
        ("1.0.0", "1.0.0", false), // equal
        ("1.0.0", "2.0.0", false), // 1.0.0 < 2.0.0
    ];
    for (a, b, expected_gt) in &cases {
        let va = Version::parse(a).unwrap();
        let vb = Version::parse(b).unwrap();
        assert_eq!(
            va > vb,
            *expected_gt,
            "{} > {} should be {}",
            a,
            b,
            expected_gt
        );
    }
}

#[test]
fn test_rez_version_range_any() {
    // Empty range means "any version" in rez
    let r = VersionRange::parse("").unwrap();
    assert!(r.is_any(), "Empty range should be 'any'");
    assert!(r.contains(&Version::parse("1.0.0").unwrap()));
    assert!(r.contains(&Version::parse("999.999.999").unwrap()));
}

#[test]
fn test_rez_version_range_exact() {
    // Exact version: "==1.2.3" or just "1.2.3" (point range)
    let r = VersionRange::parse("1.2.3").unwrap();
    assert!(
        r.contains(&Version::parse("1.2.3").unwrap()),
        "Range should contain exact version"
    );
}

#[test]
fn test_rez_version_range_ge() {
    let r = VersionRange::parse(">=1.0").unwrap();
    assert!(r.contains(&Version::parse("1.0").unwrap()));
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(!r.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_ge_lt() {
    let r = VersionRange::parse(">=1.0,<2.0").unwrap();
    assert!(r.contains(&Version::parse("1.5").unwrap()));
    assert!(!r.contains(&Version::parse("2.0").unwrap()));
    assert!(!r.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_intersection() {
    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<2.0").unwrap();
    let intersection = r1.intersect(&r2).expect("Intersection should exist");
    assert!(intersection.contains(&Version::parse("1.5").unwrap()));
    assert!(!intersection.contains(&Version::parse("2.0").unwrap()));
    assert!(!intersection.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_rez_version_range_union() {
    let r1 = VersionRange::parse(">=1.0,<1.5").unwrap();
    let r2 = VersionRange::parse(">=2.0").unwrap();
    let union = r1.union(&r2);
    assert!(union.contains(&Version::parse("1.2").unwrap()));
    assert!(union.contains(&Version::parse("2.5").unwrap()));
    assert!(!union.contains(&Version::parse("1.7").unwrap()));
}

#[test]
fn test_rez_version_range_subset_superset() {
    let r_narrow = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r_wide = VersionRange::parse(">=1.0").unwrap();
    assert!(
        r_narrow.is_subset_of(&r_wide),
        "narrow should be subset of wide"
    );
    assert!(
        r_wide.is_superset_of(&r_narrow),
        "wide should be superset of narrow"
    );
}

