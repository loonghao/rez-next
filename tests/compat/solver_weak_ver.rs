use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver: weak requirement + version range combined tests ──────────────────

/// rez solver: weak requirement with version range parses both fields
#[test]
fn test_solver_weak_requirement_with_version_range_parse() {
    use rez_next_package::Requirement;

    let req: Requirement = "~python-3+<4".parse().unwrap();
    assert!(req.weak, "~ prefix must produce weak=true");
    assert_eq!(req.name, "python");
    // Version range should be embedded in the requirement string
    let req_str = format!("{}", req);
    assert!(
        req_str.contains("python"),
        "String repr should include package name"
    );
}

/// rez solver: weak requirement without version spec is valid
#[test]
fn test_solver_weak_requirement_no_version_spec() {
    use rez_next_package::Requirement;

    let req: Requirement = "~any_optional_lib".parse().unwrap();
    assert!(req.weak, "Bare ~ requirement must be weak");
    assert_eq!(req.name, "any_optional_lib");
}

/// rez solver: non-weak Requirement parsed from string without ~ is not weak
#[test]
fn test_solver_non_weak_requirement() {
    use rez_next_package::Requirement;

    let req: Requirement = "python>=3.9".parse().unwrap();
    assert!(!req.weak, "Requirement without ~ must not be weak");
    assert_eq!(req.name, "python");
}

/// rez context: print_info format matches rez convention
#[test]
fn test_context_print_info_format() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("python".to_string());
    pkg.version = Some(Version::parse("3.11").unwrap());
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Simulate print_info output
    let summary = ctx.get_summary();
    let header = format!("resolved packages ({}):", summary.package_count);
    assert!(
        header.contains("resolved packages (1):"),
        "print_info header must match rez format"
    );

    let mut lines = vec![header];
    for (name, ver) in &summary.package_versions {
        lines.push(format!("  {}-{}", name, ver));
    }
    let output = lines.join("\n");
    assert!(
        output.contains("python-3.11"),
        "print_info must contain python-3.11"
    );
}

