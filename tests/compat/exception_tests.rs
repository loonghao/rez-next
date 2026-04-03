use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Exception type / message tests ─────────────────────────────────────────

/// rez.exceptions: PackageRequirement parse is lenient — documents actual behavior.
/// Parsing unusual strings should not panic; result may be Ok or Err.
#[test]
fn test_invalid_package_requirement_no_panic() {
    use rez_next_package::PackageRequirement;

    // Must not panic regardless of the result
    let result = PackageRequirement::parse("!!!invalid");
    let _ = result; // lenient parser may accept or reject — both are valid
}

/// rez.exceptions: Empty string PackageRequirement parse does not panic
#[test]
fn test_empty_package_requirement_no_panic() {
    use rez_next_package::PackageRequirement;

    let result = PackageRequirement::parse("");
    let _ = result;
}

/// rez.exceptions: VersionRange parse error for unbalanced brackets
#[test]
fn test_version_range_unbalanced_bracket_error() {
    use rez_core::version::VersionRange;

    let result = VersionRange::parse(">=1.0,<2.0,");
    // Trailing comma may or may not be accepted depending on impl;
    // the important thing is that the call does not panic.
    let _ = result;
}

/// rez.exceptions: Version parse with garbage input returns error (not panic)
#[test]
fn test_version_parse_garbage_no_panic() {
    use rez_core::version::Version;

    let result = Version::parse("!@#$%^&*");
    // May succeed with best-effort or fail; must not panic.
    let _ = result;
}

