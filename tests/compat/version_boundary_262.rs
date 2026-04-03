use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Version boundary tests (new batch, 262-270) ───────────────────────────

/// rez version: pre-release tokens (alpha/beta) compare lower than release
#[test]
fn test_rez_version_prerelease_ordering() {
    let v_alpha = Version::parse("1.0.0.alpha.1").unwrap();
    let v_release = Version::parse("1.0.0").unwrap();
    // alpha pre-release < release in rez semantics (longer = lower epoch when same prefix)
    // 1.0.0 has shorter length => higher epoch than 1.0.0.alpha.1
    assert!(v_release > v_alpha, "1.0.0 should be > 1.0.0.alpha.1");
}

/// rez version: VersionRange exclusion boundary `<3.0` must exclude 3.0 exactly
#[test]
fn test_rez_version_range_exclusive_upper_boundary() {
    let r = VersionRange::parse("<3.0").unwrap();
    let v3 = Version::parse("3.0").unwrap();
    let v299 = Version::parse("2.9.9").unwrap();
    assert!(!r.contains(&v3), "<3.0 must exclude exactly 3.0");
    assert!(r.contains(&v299), "<3.0 must include 2.9.9");
}

/// rez version: VersionRange `>=2.0,<3.0` is bounded on both ends
#[test]
fn test_rez_version_range_bounded_both_ends() {
    let r = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(r.contains(&Version::parse("2.9").unwrap()));
    assert!(!r.contains(&Version::parse("3.0").unwrap()));
    assert!(!r.contains(&Version::parse("1.9").unwrap()));
}

/// rez version: single token version "5" is valid and compares correctly
#[test]
fn test_rez_version_single_token() {
    let v5 = Version::parse("5").unwrap();
    let v50 = Version::parse("5.0").unwrap();
    // 5 > 5.0 (shorter = higher epoch)
    assert!(v5 > v50, "Single token '5' should be greater than '5.0'");
}

/// rez version: max version in a range can be retrieved
#[test]
fn test_rez_version_range_contains_many() {
    let r = VersionRange::parse(">=1.0").unwrap();
    for v_str in &["1.0", "2.5", "10.0", "100.0"] {
        let v = Version::parse(v_str).unwrap();
        assert!(r.contains(&v), ">=1.0 must contain {}", v_str);
    }
}

