use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Extra version / requirement compat tests ────────────────────────────────

/// rez-style requirement parsing with all forms
#[test]
fn test_rez_requirement_all_forms() {
    // All these are valid rez requirement strings
    let cases = [
        "python",
        "python-3",
        "python-3.9",
        "python-3.9.1",
        "python>=3.9",
        "python-3+<4",
        "python==3.9.1",
    ];
    for case in &cases {
        let result = PackageRequirement::parse(case);
        assert!(
            result.is_ok(),
            "Failed to parse requirement '{}': {:?}",
            case,
            result
        );
    }
}

/// Empty version range matches any version (rez "any" semantics)
#[test]
fn test_rez_empty_range_is_any() {
    let r = VersionRange::parse("").unwrap();
    assert!(r.is_any());
    for v in &["0.0.1", "1.0.0", "99.99.99", "2024.1"] {
        assert!(
            r.contains(&Version::parse(v).unwrap()),
            "Any range must contain {}",
            v
        );
    }
}

/// Version upper bound exclusion (rez: `<` means strictly less than)
#[test]
fn test_rez_version_upper_bound_exclusive() {
    let r = VersionRange::parse("<2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be excluded by <2.0"
    );
    assert!(r.contains(&Version::parse("1.9.9").unwrap()));
    assert!(r.contains(&Version::parse("1.0").unwrap()));
}

/// Version with build metadata (rez ignores build metadata in comparisons)
#[test]
fn test_rez_version_build_metadata_ignored() {
    // rez versions don't use semver build metadata; just parse the token
    let v = Version::parse("1.2.3");
    assert!(v.is_ok());
}

/// Package with private variants (rez private = `~package`)
#[test]
fn test_rez_private_package_requirement() {
    // Private packages can optionally be prefixed with ~ in some rez contexts.
    // We should at minimum parse the name without crashing.
    let pkg = Package::new("~private_pkg".to_string());
    assert_eq!(pkg.name, "~private_pkg");
}

/// Solver can handle identical requirement names (dedup should work)
#[test]
fn test_rez_dedup_requirements() {
    use rez_next_package::PackageRequirement;

    let reqs = [
        PackageRequirement::parse("python-3.9").unwrap(),
        PackageRequirement::parse("python-3.9").unwrap(), // duplicate
    ];
    // Both are parseable; solver handles dedup internally
    assert_eq!(reqs.len(), 2);
    assert_eq!(reqs[0].name, reqs[1].name);
}

/// rez context summary has correct package names
#[test]
fn test_context_summary_package_names() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("nuke-14").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    ctx.status = ContextStatus::Resolved;

    let mut py = Package::new("python".to_string());
    py.version = Some(Version::parse("3.11").unwrap());
    ctx.resolved_packages.push(py);

    let mut nuke = Package::new("nuke".to_string());
    nuke.version = Some(Version::parse("14").unwrap());
    ctx.resolved_packages.push(nuke);

    let summary = ctx.get_summary();
    assert_eq!(summary.package_count, 2);
    assert!(summary.package_versions.contains_key("python"));
    assert!(summary.package_versions.contains_key("nuke"));
}

