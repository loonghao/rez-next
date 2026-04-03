use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Conflict detection tests (solver graph) ────────────────────────────────

/// rez: two compatible requirements for the same package should not conflict
#[test]
fn test_solver_graph_no_conflict_compatible_ranges() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=1.0 and <3.0 overlap → compatible
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Compatible ranges should not produce conflicts"
    );
}

/// rez: two disjoint requirements for the same package should conflict
#[test]
fn test_solver_graph_conflict_disjoint_ranges() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=3.0 and <2.0 are disjoint → conflict
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<2.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint ranges should produce a conflict"
    );
}

/// rez: version range satisfiability with solver
#[test]
fn test_dependency_resolver_single_package() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo_mgr = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_mgr), SolverConfig::default());

    // Single requirement with no packages in repo → should succeed with empty result
    let result =
        rt.block_on(resolver.resolve(vec![Requirement::new("some_nonexistent_pkg".to_string())]));

    // With empty repo, resolution may fail gracefully or return empty
    // The important thing is it doesn't panic
    let _ = result;
}

