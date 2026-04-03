use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Version advanced operations ─────────────────────────────────────────────

/// rez: version range union — merge two separate ranges
#[test]
fn test_version_range_union_disjoint() {
    let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r2 = VersionRange::parse(">=3.0,<4.0").unwrap();
    let union = r1.union(&r2);
    // Union of two disjoint ranges should contain elements from both
    assert!(
        union.contains(&Version::parse("1.5").unwrap()),
        "union should contain 1.5"
    );
    assert!(
        union.contains(&Version::parse("3.5").unwrap()),
        "union should contain 3.5"
    );
    assert!(
        !union.contains(&Version::parse("2.5").unwrap()),
        "union should not contain 2.5"
    );
}

/// rez: version range with pre-release label sorting
#[test]
fn test_version_prerelease_ordering() {
    // alpha < beta < rc < release in standard semver-like ordering
    let v_alpha = Version::parse("1.0.0.alpha").unwrap();
    let v_beta = Version::parse("1.0.0.beta").unwrap();
    let v_rc = Version::parse("1.0.0.rc.1").unwrap();
    let v_release = Version::parse("1.0.0").unwrap();
    // In rez: shorter version = higher epoch, so 1.0.0 > 1.0.0.alpha
    assert!(
        v_release > v_alpha,
        "1.0.0 should be greater than 1.0.0.alpha in rez semantics"
    );
    assert!(
        v_release > v_beta,
        "1.0.0 should be greater than 1.0.0.beta"
    );
    assert!(v_release > v_rc, "1.0.0 should be greater than 1.0.0.rc.1");
}

/// rez: version range exclusive upper bound (rez semantics: shorter = higher epoch)
/// In rez: 3.0 > 3.0.1 > 3.0.0, so <3.0 excludes 3.0 but includes 3.0.1 (shorter < longer = smaller)
#[test]
fn test_version_range_exclusive_upper() {
    let r = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(r.contains(&Version::parse("2.9.9").unwrap()));
    assert!(
        !r.contains(&Version::parse("3.0").unwrap()),
        "3.0 should be excluded (upper bound)"
    );
    // In rez semantics: 3.0.1 < 3.0 (shorter version = higher epoch), so 3.0.1 IS within <3.0
    assert!(
        r.contains(&Version::parse("3.0.1").unwrap()),
        "3.0.1 is less than 3.0 in rez semantics (shorter = higher epoch), so should be included"
    );
}

/// rez: version range with version == bound edge
#[test]
fn test_version_range_inclusive_lower_edge() {
    let r = VersionRange::parse(">=1.0").unwrap();
    assert!(
        r.contains(&Version::parse("1.0").unwrap()),
        "lower bound 1.0 should be included"
    );
    assert!(
        !r.contains(&Version::parse("0.9.9").unwrap()),
        "0.9.9 should be excluded"
    );
}

