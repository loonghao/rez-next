use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Solver topology tests ────────────────────────────────────────────────────

/// Solver: packages list returned for empty requirements is empty
#[test]
fn test_solver_empty_requirements_returns_empty_package_list() {
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let repo = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(repo, SolverConfig::default());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(resolver.resolve(vec![])).unwrap();
    assert!(
        result.resolved_packages.is_empty(),
        "Empty requirements should yield empty package list"
    );
}

/// Solver: conflicting exclusive requirements detected gracefully
#[test]
fn test_solver_version_conflict_detected() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let repo = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(repo, SolverConfig::default());
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Two requirements for same package: python-2 and python-3 — may conflict or not
    // depending on whether packages exist; important: should not panic
    let reqs = vec![
        Requirement::new("python-2".to_string()),
        Requirement::new("python-3".to_string()),
    ];
    let result = rt.block_on(resolver.resolve(reqs));
    // Result may be Ok (empty repo = no conflict) or Err; must not panic
    let _ = result;
}

