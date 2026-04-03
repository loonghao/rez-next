use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Circular dependency detection tests ────────────────────────────────────

/// rez: topological sort detects direct circular dependency (A → B → A)
#[test]
fn test_circular_dependency_direct() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    let mut pkg_a = Package::new("pkgA".to_string());
    pkg_a.version = Some(Version::parse("1.0").unwrap());
    pkg_a.requires = vec!["pkgB-1.0".to_string()];

    let mut pkg_b = Package::new("pkgB".to_string());
    pkg_b.version = Some(Version::parse("1.0").unwrap());
    pkg_b.requires = vec!["pkgA-1.0".to_string()]; // Circular!

    graph.add_package(pkg_a).unwrap();
    graph.add_package(pkg_b).unwrap();
    graph.add_dependency_edge("pkgA-1.0", "pkgB-1.0").unwrap();
    graph.add_dependency_edge("pkgB-1.0", "pkgA-1.0").unwrap(); // creates cycle

    // get_resolved_packages uses topological sort which detects cycles
    let result = graph.get_resolved_packages();
    assert!(
        result.is_err(),
        "Circular dependency A->B->A should be detected as an error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("ircular") || err_msg.contains("cycle") || err_msg.contains("Circular"),
        "Error should mention circular dependency, got: {}",
        err_msg
    );
}

/// rez: three-package cycle (A → B → C → A)
#[test]
fn test_circular_dependency_three_way() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for (name, dep) in &[
        ("pkgX", "pkgY-1.0"),
        ("pkgY", "pkgZ-1.0"),
        ("pkgZ", "pkgX-1.0"),
    ] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse("1.0").unwrap());
        pkg.requires = vec![dep.to_string()];
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("pkgX-1.0", "pkgY-1.0").unwrap();
    graph.add_dependency_edge("pkgY-1.0", "pkgZ-1.0").unwrap();
    graph.add_dependency_edge("pkgZ-1.0", "pkgX-1.0").unwrap(); // closes cycle

    let result = graph.get_resolved_packages();
    assert!(
        result.is_err(),
        "Three-way cycle X->Y->Z->X must be detected"
    );
}

/// rez: no cycle in linear chain (A → B → C) should succeed
#[test]
fn test_no_circular_dependency_linear() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for (name, dep) in &[
        ("libA", Some("libB-1.0")),
        ("libB", Some("libC-1.0")),
        ("libC", None),
    ] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse("1.0").unwrap());
        if let Some(d) = dep {
            pkg.requires = vec![d.to_string()];
        }
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("libA-1.0", "libB-1.0").unwrap();
    graph.add_dependency_edge("libB-1.0", "libC-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Linear chain A->B->C should resolve without cycle error"
    );
    let packages = result.unwrap();
    assert_eq!(packages.len(), 3, "Should resolve 3 packages");
}

/// rez: diamond dependency (A→B, A→C, B→D, C→D) is not a cycle
#[test]
fn test_diamond_dependency_not_cycle() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    let packages = [
        ("pkgA", vec!["pkgB-1.0", "pkgC-1.0"]),
        ("pkgB", vec!["pkgD-1.0"]),
        ("pkgC", vec!["pkgD-1.0"]),
        ("pkgD", vec![]),
    ];

    for (name, deps) in &packages {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse("1.0").unwrap());
        pkg.requires = deps.iter().map(|s| s.to_string()).collect();
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("pkgA-1.0", "pkgB-1.0").unwrap();
    graph.add_dependency_edge("pkgA-1.0", "pkgC-1.0").unwrap();
    graph.add_dependency_edge("pkgB-1.0", "pkgD-1.0").unwrap();
    graph.add_dependency_edge("pkgC-1.0", "pkgD-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Diamond dependency A->B->D, A->C->D is a DAG, not a cycle: {:?}",
        result
    );
}

/// rez: self-referencing package (A → A) is a cycle
#[test]
fn test_self_referencing_package_is_cycle() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    let mut pkg = Package::new("selfref".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());
    pkg.requires = vec!["selfref-1.0".to_string()];
    graph.add_package(pkg).unwrap();
    graph
        .add_dependency_edge("selfref-1.0", "selfref-1.0")
        .unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_err(),
        "Self-referencing package selfref->selfref must be detected as cycle"
    );
}

