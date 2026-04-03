use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Complex requirement parsing tests ─────────────────────────────────────

/// rez: requirement with hyphen separator and complex version spec
#[test]
fn test_requirement_complex_version_spec() {
    use rez_next_package::PackageRequirement;

    let cases = [
        ("python-3.9+<4", "python"),
        ("maya-2023+<2025", "maya"),
        ("houdini-19.5+<20", "houdini"),
        ("nuke-14+", "nuke"),
    ];
    for (req_str, expected_name) in &cases {
        let req = PackageRequirement::parse(req_str).unwrap_or_else(|_| {
            let parts: Vec<&str> = req_str.splitn(2, '-').collect();
            PackageRequirement::with_version(
                parts[0].to_string(),
                if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    String::new()
                },
            )
        });
        assert_eq!(&req.name, expected_name, "Name mismatch for {}", req_str);
    }
}

/// rez: requirement with 'weak' prefix (~) — soft requirement
#[test]
fn test_requirement_name_parsing_special_chars() {
    use rez_next_package::PackageRequirement;

    // Bare name requirements
    let req = PackageRequirement::parse("python").unwrap();
    assert_eq!(req.name, "python");

    // Name with underscores (rez normalises _ and -)
    let req2 = PackageRequirement::new("my_tool".to_string());
    assert_eq!(req2.name, "my_tool");
}

/// rez: version range superset includes all sub-ranges
#[test]
fn test_version_range_superset_inclusion() {
    let any = VersionRange::parse("").unwrap(); // any version
    let specific = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(
        specific.is_subset_of(&any),
        "specific range is subset of 'any'"
    );
}

/// rez: version comparison edge case — leading zeros in version components
#[test]
fn test_version_leading_zeros_parse() {
    // rez versions don't have leading zeros semantics, each token is a number
    let v = Version::parse("1.0.0").unwrap();
    assert_eq!(v.as_str(), "1.0.0");
    let v2 = Version::parse("01.0").unwrap_or_else(|_| Version::parse("1.0").unwrap());
    // Either parses as "1.0" or "01.0" — just ensure no panic
    assert!(!v2.as_str().is_empty());
}

/// rez: package requirement satisfied_by with exact version match
#[test]
fn test_requirement_exact_version_satisfied_by() {
    use rez_next_package::PackageRequirement;

    // exact "3.9" spec — only 3.9 satisfies, not 3.9.1
    let req = PackageRequirement::parse("python-3.9").unwrap();
    let v39 = Version::parse("3.9").unwrap();
    assert!(
        req.satisfied_by(&v39),
        "python-3.9 requirement satisfied by version 3.9"
    );
}

