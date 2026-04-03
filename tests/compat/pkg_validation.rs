use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Package validation tests ─────────────────────────────────────────────────

/// Package: name must be non-empty
#[test]
fn test_package_name_non_empty() {
    use rez_next_package::Package;

    let pkg = Package::new("mypackage".to_string());
    assert_eq!(pkg.name, "mypackage");
    assert!(!pkg.name.is_empty());
}

/// Package: version field is optional (no version = "unversioned")
#[test]
fn test_package_version_optional() {
    use rez_next_package::Package;

    let pkg = Package::new("unversioned_pkg".to_string());
    assert!(
        pkg.version.is_none(),
        "Version should be None when not specified"
    );
}

/// Package: Requirement parses name-only (no version constraint)
#[test]
fn test_requirement_name_only() {
    use rez_next_package::Requirement;

    let req = Requirement::new("python".to_string());
    assert_eq!(req.name, "python");
}

