use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Version boundary tests (additional) ─────────────────────────────────────

/// rez version: very large numeric components parse without panic
#[test]
fn test_version_large_component_no_panic() {
    use rez_core::version::Version;

    let result = Version::parse("999999.999999.999999");
    // Should not panic; result may be Ok or Err depending on limits
    let _ = result;
}

/// rez version: single-component version "5" parses correctly
#[test]
fn test_version_single_component() {
    use rez_core::version::Version;

    let v = Version::parse("5").unwrap();
    assert_eq!(v.as_str(), "5");
}

/// rez version: two single-component versions compare correctly
#[test]
fn test_version_single_component_ordering() {
    use rez_core::version::Version;

    let v10 = Version::parse("10").unwrap();
    let v9 = Version::parse("9").unwrap();
    assert!(
        v10 > v9,
        "10 should be greater than 9 as single-component versions"
    );
}

/// rez version: range "any" (empty string or "*") contains all versions
#[test]
fn test_version_range_any_contains_all() {
    use rez_core::version::{Version, VersionRange};

    // Empty string "" means "any version" in rez semantics
    let r = VersionRange::parse("").unwrap();
    assert!(
        r.contains(&Version::parse("1.0.0").unwrap()),
        "any range should contain 1.0.0"
    );
    assert!(
        r.contains(&Version::parse("999.0").unwrap()),
        "any range should contain 999.0"
    );
}

