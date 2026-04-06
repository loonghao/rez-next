//! Rez Compat — Solver, Conflict Detection, Dependency Resolution Tests
//!
//! Extracted from rez_compat_tests.rs (Cycle 32).
//! Refactored in Cycle 73: package.py commands tests → rez_compat_package_commands_tests.rs
//!                          requirement/constraint tests → rez_compat_requirement_tests.rs
//!
//! Covers: solver graph conflict detection, dependency resolver, DCC pipeline scenario,
//! diamond dependency patterns.
//!
//! See also: rez_compat_tests.rs (version, package, rex, suite, config, e2e)
//!           rez_compat_package_commands_tests.rs (package.py commands parsing)
//!           rez_compat_requirement_tests.rs (requirement format, constraints)

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

    // Empty repo: lenient mode returns Ok with the requirement in failed_requirements.
    let res = result.expect("empty-repo lenient resolve should return Ok, not panic");
    assert!(
        res.resolved_packages.is_empty(),
        "empty repo: resolved_packages should be empty, got {:?}",
        res.resolved_packages
            .iter()
            .map(|p| &p.package.name)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        res.failed_requirements.len(),
        1,
        "empty repo: exactly one failed requirement expected, got {}",
        res.failed_requirements.len()
    );
}

/// rez: solver with real temp repo - common DCC pipeline scenario
#[test]
fn test_solver_dcc_pipeline_scenario() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Build a realistic DCC pipeline package graph
    macro_rules! pkg {
        ($dir:expr, $name:expr, $ver:expr, $requires:expr) => {{
            let pkg_dir = $dir.join($name).join($ver);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            let requires_block = if $requires.is_empty() {
                String::new()
            } else {
                let items: Vec<String> = $requires
                    .iter()
                    .map(|r: &&str| format!("    '{}',", r))
                    .collect();
                format!("requires = [\n{}\n]\n", items.join("\n"))
            };
            std::fs::write(
                pkg_dir.join("package.py"),
                format!(
                    "name = '{}'\nversion = '{}'\n{}",
                    $name, $ver, requires_block
                ),
            )
            .unwrap();
        }};
    }

    // Packages
    pkg!(repo_dir, "python", "3.11.0", &[] as &[&str]);
    pkg!(repo_dir, "pyside2", "5.15.0", &["python-3+<4"]);
    pkg!(repo_dir, "pyside6", "6.5.0", &["python-3+<4"]);
    pkg!(
        repo_dir,
        "maya",
        "2024.0",
        &["python-3.9+<3.12", "pyside2-5+"]
    );
    pkg!(repo_dir, "houdini", "20.0.547", &["python-3.10+<3.12"]);
    pkg!(
        repo_dir,
        "nuke",
        "15.0.0",
        &["python-3.9+<3.12", "pyside2-5+"]
    );

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "dcc_repo".to_string(),
    )));
    let repo = Arc::new(mgr);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Resolve maya environment
    let maya_reqs: Vec<Requirement> = ["maya"].iter().map(|s| s.parse().unwrap()).collect();

    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let result = rt.block_on(resolver.resolve(maya_reqs)).unwrap();

    let names: Vec<&str> = result
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains(&"maya"), "maya should be in resolved set");
    assert!(
        names.contains(&"python"),
        "python should be pulled in for maya"
    );
    assert!(
        names.contains(&"pyside2"),
        "pyside2 should be pulled in for maya"
    );
}

/// rez: solver handles diamond dependency pattern correctly
/// A -> B and C; B -> D-1.0; C -> D-2.0 (conflict)
#[test]
fn test_solver_diamond_dependency_conflict_detection() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();

    // Package A requires B and C
    // Package B requires D>=1.0,<2.0
    // Package C requires D>=2.0
    // These D requirements are disjoint → conflict
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            "<2.0".to_string(),
        ))
        .unwrap();
    // No conflict yet (>=1.0 AND <2.0 are compatible)
    assert!(
        graph.detect_conflicts().is_empty(),
        ">=1.0 and <2.0 are compatible for D"
    );

    // Now add disjoint constraint
    let mut conflict_graph = DependencyGraph::new();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();
    let conflicts = conflict_graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "D requiring >=1.0,<2.0 AND >=2.0 simultaneously should conflict"
    );
}
