use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── VersionRange advanced tests (Phase 93) ───────────────────────────────

/// rez version range: negation "!=" (exclude single version)
#[test]
fn test_version_range_exclude_single() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("!=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be excluded"
    );
    assert!(
        r.contains(&Version::parse("1.9").unwrap()),
        "1.9 should be included"
    );
    assert!(
        r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should be included"
    );
}

/// rez version range: upper-inclusive "<=2.0"
#[test]
fn test_version_range_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("<=2.0").unwrap();
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be included in <=2.0"
    );
    assert!(
        r.contains(&Version::parse("1.5").unwrap()),
        "1.5 should be included in <=2.0"
    );
    assert!(
        !r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should not be in <=2.0"
    );
}

/// rez version range: ">1.0" (strict lower bound, exclusive)
#[test]
fn test_version_range_gt_exclusive() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 should be excluded from >1.0"
    );
    assert!(
        r.contains(&Version::parse("1.1").unwrap()),
        "1.1 should be included in >1.0"
    );
}

/// rez version range: combined ">1.0,<=2.0"
#[test]
fn test_version_range_combined_gt_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0,<=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 excluded (strict >)"
    );
    assert!(r.contains(&Version::parse("1.5").unwrap()), "1.5 included");
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 included (<=)"
    );
    assert!(!r.contains(&Version::parse("2.1").unwrap()), "2.1 excluded");
}

/// rez version range: is_superset_of semantics
#[test]
fn test_version_range_is_superset() {
    use rez_core::version::VersionRange;

    let broad = VersionRange::parse(">=1.0").unwrap();
    let narrow = VersionRange::parse(">=1.5,<2.0").unwrap();
    assert!(
        broad.is_superset_of(&narrow),
        ">=1.0 should be superset of >=1.5,<2.0"
    );
    assert!(
        !narrow.is_superset_of(&broad),
        ">=1.5,<2.0 should NOT be superset of >=1.0"
    );
}

