use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver boundary tests ────────────────────────────────────────────────────

/// rez solver: single package with no dependencies resolves immediately
#[test]
fn test_solver_single_package_no_deps() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("standalone".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages();
    assert!(result.is_ok(), "Single package with no deps should resolve");
    assert_eq!(result.unwrap().len(), 1);
}

/// rez solver: version range intersection for multi-constraint requirement
#[test]
fn test_solver_multi_constraint_version_range() {
    use rez_core::version::VersionRange;

    let r_ge = VersionRange::parse(">=3.9").unwrap();
    let r_lt = VersionRange::parse("<4.0").unwrap();
    let intersection = r_ge
        .intersect(&r_lt)
        .expect(">=3.9 and <4.0 should intersect");

    assert!(intersection.contains(&rez_core::version::Version::parse("3.9").unwrap()));
    assert!(intersection.contains(&rez_core::version::Version::parse("3.11").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("4.0").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("3.8").unwrap()));
}

/// rez solver: two packages with exclusive version ranges → conflict
#[test]
fn test_solver_exclusive_ranges_detect_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Exclusive ranges >=1.0,<2.0 and >=2.0 should conflict for lib"
    );
}

/// rez solver: compatible ranges do not produce a conflict
#[test]
fn test_solver_compatible_ranges_no_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=3.8 and <4.0 are compatible
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.8".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<4.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(), ">=3.8 and <4.0 should not conflict");
}

/// rez solver: weak requirement (~pkg) is parsed correctly
#[test]
fn test_solver_weak_requirement_parse() {
    use rez_next_package::Requirement;

    let req = "~python>=3.9".parse::<Requirement>().unwrap();
    assert!(req.weak, "~ prefix should set weak=true");
    assert_eq!(req.name, "python");
}

/// rez solver: topological sort on a chain A → B → C
#[test]
fn test_solver_topological_sort_chain() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for (name, ver) in &[("pkgA", "1.0"), ("pkgB", "1.0"), ("pkgC", "1.0")] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("pkgA-1.0", "pkgB-1.0").unwrap();
    graph.add_dependency_edge("pkgB-1.0", "pkgC-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Linear chain A->B->C should resolve (no cycles)"
    );
    assert_eq!(
        result.unwrap().len(),
        3,
        "All 3 packages should be in resolved order"
    );
}

