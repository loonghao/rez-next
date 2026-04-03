use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver weak dependency (~pkg) tests (301-304) ──────────────────────────

/// rez solver: weak requirement flag defaults to false
#[test]
fn test_solver_weak_requirement_default_false() {
    use rez_next_package::PackageRequirement;

    let normal = PackageRequirement::parse("python").unwrap();
    assert!(
        !normal.weak,
        "Normal requirement 'python' should not be weak"
    );

    let with_ver = PackageRequirement::parse("python-3.9").unwrap();
    assert!(
        !with_ver.weak,
        "Versioned requirement 'python-3.9' should not be weak"
    );
}

/// rez solver: weak requirement preserves package name correctly
#[test]
fn test_solver_weak_requirement_name_preserved() {
    use rez_next_package::PackageRequirement;

    let weak_req = PackageRequirement {
        name: "numpy".to_string(),
        version_spec: None,
        weak: true,
        conflict: false,
    };
    assert_eq!(weak_req.name(), "numpy");
    assert!(
        weak_req.weak,
        "Explicitly set weak=true should be preserved"
    );
}

/// rez solver: non-conflicting requirements yield no conflicts
#[test]
fn test_solver_weak_no_conflict_if_compatible() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "numpy".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Non-conflicting requirements should yield no conflicts"
    );
}

/// rez solver: disjoint version ranges for same package produce conflict
#[test]
fn test_solver_disjoint_ranges_produce_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            ">=4.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint requirements >=4.0 and <3.0 should produce conflict"
    );
}

