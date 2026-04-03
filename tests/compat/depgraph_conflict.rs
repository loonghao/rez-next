use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── DependencyGraph conflict detection extended tests ──────────────────────

/// rez: conflict detection reports incompatible python version ranges
#[test]
fn test_dependency_graph_conflict_python_versions() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    // pkgA requires python-3.9, pkgB requires python-3.11 — incompatible exact specs
    let mut pkg_a = Package::new("pkgA".to_string());
    pkg_a.version = Some(Version::parse("1.0").unwrap());
    pkg_a.requires = vec!["python-3.9".to_string()];

    let mut pkg_b = Package::new("pkgB".to_string());
    pkg_b.version = Some(Version::parse("1.0").unwrap());
    pkg_b.requires = vec!["python-3.11".to_string()];

    graph.add_package(pkg_a).unwrap();
    graph.add_package(pkg_b).unwrap();

    // Add conflicting requirements
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.11".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    // There should be at least one conflict for python
    assert!(
        !conflicts.is_empty(),
        "Incompatible python version requirements should produce at least one conflict"
    );
    assert_eq!(conflicts[0].package_name, "python");
}

/// rez: no conflict when single requirement for each package
#[test]
fn test_dependency_graph_no_conflict_single_requirements() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    let mut pkg = Package::new("myapp".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());
    pkg.requires = vec!["python-3.9".to_string()];
    graph.add_package(pkg).unwrap();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "3.9".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Single requirement per package should produce no conflicts"
    );
}

/// rez: graph stats reflects correct node/edge counts
#[test]
fn test_dependency_graph_stats_counts() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for name in &["a", "b", "c"] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse("1.0").unwrap());
        graph.add_package(pkg).unwrap();
    }
    graph.add_dependency_edge("a-1.0", "b-1.0").unwrap();
    graph.add_dependency_edge("b-1.0", "c-1.0").unwrap();

    let stats = graph.get_stats();
    assert_eq!(stats.node_count, 3, "Graph should have 3 nodes");
    assert_eq!(stats.edge_count, 2, "Graph should have 2 edges");
}

