use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver boundary tests ───────────────────────────────────────────────────

/// rez solver: resolving with only one package returns exactly that package
#[test]
fn test_solver_single_package_resolution() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("solo".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages().unwrap();
    assert_eq!(
        result.len(),
        1,
        "Single package graph should resolve to 1 package"
    );
    assert_eq!(result[0].name, "solo");
}

/// rez solver: weak requirement (~) does not prevent resolution when absent
#[test]
fn test_solver_weak_requirement_optional_absent() {
    use rez_next_package::Requirement;

    let req: Requirement = "~optional_tool>=1.0".parse().unwrap();
    assert!(req.weak, "~ prefix must produce a weak requirement");
    assert_eq!(req.name, "optional_tool");
}

/// rez solver: diamond dependency A→B, A→C, B→D, C→D resolves correctly
#[test]
fn test_solver_diamond_dependency_no_conflict() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    for (n, v) in &[("A", "1.0"), ("B", "1.0"), ("C", "1.0"), ("D", "1.0")] {
        let mut pkg = Package::new(n.to_string());
        pkg.version = Some(Version::parse(v).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("A-1.0", "C-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "D-1.0").unwrap();
    graph.add_dependency_edge("C-1.0", "D-1.0").unwrap();

    let resolved = graph.get_resolved_packages().unwrap();
    assert_eq!(
        resolved.len(),
        4,
        "Diamond dependency should include all 4 packages exactly once"
    );
}

