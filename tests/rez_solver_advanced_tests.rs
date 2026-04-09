//! Advanced Solver Integration Tests — Basic Scenarios
//!
//! Covers:
//! - Diamond dependencies (A->B,C; B->D; C->D → need compatible D)
//! - Conflict detection (requirements with disjoint version ranges)
//! - Transitive dependency resolution
//! - Empty / edge cases
//! - DependencyGraph structural tests
//! - VFX pipeline integration scenario
//!
//! Cycle 142: extracted Requirement satisfaction edge cases + from_str edge cases
//!            + version selection strategy into rez_solver_strategy_tests.rs.
//!
//! See also:
//! - rez_solver_graph_tests.rs  — graph topology, cycle detection, large VFX, edge cases
//! - rez_solver_platform_tests.rs — platform/OS, strict mode, pre-release, variants, error messages

use rez_next_package::{PackageRequirement, Requirement};
use rez_next_solver::{DependencyGraph, DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

// ─── Diamond dependency tests ────────────────────────────────────────────────

/// Diamond dependency: A->B,C; B->D-1.x; C->D-1.x; D-1.5.0 satisfies both.
/// Expected: resolution succeeds with D-1.5.0 selected.
#[test]
fn test_diamond_dependency_compatible() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
        ("scipy", "1.11.0", &["python-3+", "numpy-1.20+"]),
        ("my_lib", "1.0.0", &["numpy-1.20+", "scipy-1.10+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["my_lib"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Diamond dependency with compatible versions should resolve"
    );
    let resolution = result.unwrap();

    let names: Vec<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();
    assert!(names.contains(&"my_lib"), "my_lib should be resolved");
    assert!(
        names.contains(&"numpy"),
        "numpy should be pulled in transitively"
    );
    assert!(
        names.contains(&"scipy"),
        "scipy should be pulled in transitively"
    );
    assert!(
        names.contains(&"python"),
        "python should be pulled in transitively"
    );
}

/// Diamond dependency resolution: B requires D-1+, C requires D-1+
/// (same range, should unify to a single D selection)
/// Note: D-1+ means >=1, which includes D-2.0.0. Solver picks latest → D-2.0.0.
#[test]
fn test_diamond_dependency_same_range_unifies() {
    let (_tmp, repo) = build_test_repo(&[
        ("D", "1.5.0", &[]),
        ("D", "2.0.0", &[]),
        ("B", "1.0.0", &["D-1+"]),
        ("C", "1.0.0", &["D-1+"]),
        ("A", "1.0.0", &["B-1+", "C-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["A"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Same-range diamond dependency should resolve"
    );
    let resolution = result.unwrap();

    let d_packages: Vec<_> = resolution
        .resolved_packages
        .iter()
        .filter(|p| p.package.name == "D")
        .collect();
    assert_eq!(
        d_packages.len(),
        1,
        "D should be resolved exactly once (not duplicated)"
    );

    let d_ver = d_packages[0].package.version.as_ref().map(|v| v.as_str());
    assert!(
        d_ver == Some("2.0.0") || d_ver == Some("1.5.0"),
        "D should be resolved to a valid version (2.0.0 or 1.5.0), got: {:?}",
        d_ver
    );
}

// ─── Conflict detection tests ─────────────────────────────────────────────────

/// DependencyGraph: two disjoint constraints for same package = conflict
#[test]
fn test_graph_conflict_disjoint_versions() {
    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9,<3.10".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.11".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint version ranges [3.9,3.10) and [3.11,∞) should produce a conflict"
    );
    assert!(
        conflicts.iter().any(|c| c.package_name == "python"),
        "Conflict should identify 'python' as the conflicting package"
    );
}

/// DependencyGraph: overlapping constraints for same package = no conflict
#[test]
fn test_graph_no_conflict_overlapping_versions() {
    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "scipy".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "scipy".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Overlapping ranges >=1.0 and <3.0 should NOT produce a conflict"
    );
}

/// DependencyGraph: single constraint for a package = no conflict
#[test]
fn test_graph_no_conflict_single_requirement() {
    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            ">=2024".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Single constraint should never produce a conflict"
    );
}

/// DependencyGraph: multiple packages, conflicts only for conflicting one
#[test]
fn test_graph_partial_conflict() {
    let mut graph = DependencyGraph::new();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<4".to_string(),
        ))
        .unwrap();

    graph
        .add_requirement(PackageRequirement::with_version(
            "numpy".to_string(),
            ">=1.20,<1.22".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "numpy".to_string(),
            ">=1.25".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    let conflict_names: Vec<&str> = conflicts.iter().map(|c| c.package_name.as_str()).collect();
    assert!(
        !conflict_names.contains(&"python"),
        "python should NOT conflict"
    );
    assert!(conflict_names.contains(&"numpy"), "numpy should conflict");
}

// ─── Transitive dependency resolution ────────────────────────────────────────

/// Deep transitive chain: A->B->C->D→E, all resolve correctly
#[test]
fn test_transitive_chain_resolution() {
    let (_tmp, repo) = build_test_repo(&[
        ("E", "1.0.0", &[]),
        ("D", "1.0.0", &["E-1+"]),
        ("C", "1.0.0", &["D-1+"]),
        ("B", "1.0.0", &["C-1+"]),
        ("A", "1.0.0", &["B-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["A"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Transitive chain resolution should succeed");
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    for pkg in &["A", "B", "C", "D", "E"] {
        assert!(
            names.contains(*pkg),
            "Package '{}' should be in resolved set (transitive)",
            pkg
        );
    }
}

/// Multiple roots: resolving [A, B] together with shared dependency C
#[test]
fn test_multiple_root_requirements_shared_dep() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
        ("pandas", "2.0.0", &["python-3+", "numpy-1.20+"]),
        ("matplotlib", "3.7.0", &["python-3+", "numpy-1.20+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["pandas", "matplotlib"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Multiple roots with shared dependencies should resolve"
    );
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains("pandas"), "pandas should be resolved");
    assert!(
        names.contains("matplotlib"),
        "matplotlib should be resolved"
    );
    assert!(
        names.contains("numpy"),
        "numpy should be resolved as shared dep"
    );
    assert!(
        names.contains("python"),
        "python should be resolved as shared dep"
    );

    let numpy_count = resolution
        .resolved_packages
        .iter()
        .filter(|p| p.package.name == "numpy")
        .count();
    assert_eq!(numpy_count, 1, "numpy should be deduplicated (shared dep)");
}

// ─── Empty and edge cases ─────────────────────────────────────────────────────

/// Empty requirements resolve to empty package set
#[test]
fn test_resolver_empty_requirements() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let result = rt.block_on(resolver.resolve(vec![]));
    assert!(result.is_ok(), "Empty requirements should succeed");
    let resolution = result.unwrap();
    assert_eq!(
        resolution.resolved_packages.len(),
        0,
        "Empty requirements should produce 0 resolved packages"
    );
}

/// Unknown package (not in repo) handled gracefully
#[test]
fn test_resolver_unknown_package_graceful() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["totally_nonexistent_xyz_12345"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(resolution) => {
            assert!(
                resolution.resolved_packages.is_empty()
                    || !resolution.failed_requirements.is_empty(),
                "unknown package should not resolve cleanly"
            );
        }
        Err(_) => {
            // Error is also acceptable
        }
    }
}

// ─── DependencyGraph structural tests ────────────────────────────────────────

/// DependencyGraph: add multiple packages' requirements
#[test]
fn test_dependency_graph_multiple_packages() {
    let mut graph = DependencyGraph::new();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "pyside2".to_string(),
            ">=5.15".to_string(),
        ))
        .unwrap();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<4".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Compatible multi-package graph should have no conflicts, got: {:?}",
        conflicts
            .iter()
            .map(|c| &c.package_name)
            .collect::<Vec<_>>()
    );
}

/// DependencyGraph: add requirement for same package multiple times (exact same)
#[test]
fn test_dependency_graph_duplicate_requirement_no_conflict() {
    let mut graph = DependencyGraph::new();

    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Identical requirements should not produce a conflict"
    );
}

// ─── VFX pipeline integration scenario ───────────────────────────────────────

/// Full VFX pipeline resolve: maya + houdini sharing python
#[test]
fn test_vfx_pipeline_shared_python_resolve() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.10.0", &[]),
        ("pyside2", "5.15.0", &["python-3+"]),
        ("maya", "2024.0", &["python-3.9+<3.12", "pyside2-5+"]),
        ("houdini", "20.0.547", &["python-3.10+<3.12"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["maya", "houdini"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "VFX pipeline maya+houdini resolve should succeed"
    );
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains("maya"), "maya should be resolved");
    assert!(names.contains("houdini"), "houdini should be resolved");
    assert!(
        names.contains("python"),
        "python should be resolved as shared dep"
    );
}

// Tests for Cycle 28 and Cycle 30 edge cases have been moved to:
// tests/rez_solver_edge_case_tests.rs
//
// Tests for Cycle 34 ResolvedPackageInfo.variant_index have been moved to:
// tests/rez_solver_variant_tests.rs
