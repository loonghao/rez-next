//! Solver Graph Topology and Cycle Detection Tests (Cycle 76 split)
//!
//! Covers:
//! - DependencyGraph topology (add packages, edges, topological sort, clear, exclusion)
//! - Cycle detection (two-node, three-node, self-loop, linear chain)

use rez_next_package::{PackageRequirement, Requirement};
use rez_next_solver::{DependencyGraph, DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

// ─── DependencyGraph topology tests ──────────────────────────────────────────

/// DependencyGraph: add packages and edges, verify node count and stats
#[test]
fn test_dependency_graph_add_packages_and_edges() {
    use rez_next_package::Package;

    let mut pkg_a = Package::new("A".to_string());
    pkg_a.version = Some(Version::parse("1.0.0").unwrap());
    let mut pkg_b = Package::new("B".to_string());
    pkg_b.version = Some(Version::parse("1.0.0").unwrap());

    let mut graph = DependencyGraph::new();
    graph.add_package(pkg_a).unwrap();
    graph.add_package(pkg_b).unwrap();
    graph.add_dependency_edge("A-1.0.0", "B-1.0.0").unwrap();

    let stats = graph.get_stats();
    assert_eq!(stats.node_count, 2, "Graph should have 2 nodes");
    assert_eq!(stats.edge_count, 1, "Graph should have 1 directed edge");
    assert_eq!(stats.conflict_count, 0, "No requirements → no conflicts");
}

/// DependencyGraph: topological sort correctness
#[test]
fn test_dependency_graph_topological_sort() {
    use rez_next_package::Package;

    // Build linear chain: C <- B <- A (A depends on B, B depends on C)
    let make_pkg = |name: &str, ver: &str| {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        pkg
    };

    let mut graph = DependencyGraph::new();
    graph.add_package(make_pkg("A", "1.0")).unwrap();
    graph.add_package(make_pkg("B", "1.0")).unwrap();
    graph.add_package(make_pkg("C", "1.0")).unwrap();
    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "C-1.0").unwrap();

    let resolved = graph.get_resolved_packages().unwrap();
    assert_eq!(
        resolved.len(),
        3,
        "All 3 packages should be in topological sort"
    );

    // In topological order, A comes before B, B before C
    let names: Vec<&str> = resolved.iter().map(|p| p.name.as_str()).collect();
    let pos_a = names.iter().position(|&n| n == "A").unwrap();
    let pos_b = names.iter().position(|&n| n == "B").unwrap();
    let pos_c = names.iter().position(|&n| n == "C").unwrap();
    assert!(pos_a < pos_b, "A should come before B in topological order");
    assert!(pos_b < pos_c, "B should come before C in topological order");
}

/// DependencyGraph: clear() resets to empty state
#[test]
fn test_dependency_graph_clear() {
    use rez_next_package::Package;

    let mut pkg = Package::new("X".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());

    let mut graph = DependencyGraph::new();
    graph.add_package(pkg).unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "Y".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();
    graph
        .add_constraint(PackageRequirement::with_version(
            "Z".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();

    let stats_before = graph.get_stats();
    assert_eq!(stats_before.node_count, 1);
    assert_eq!(stats_before.constraint_count, 1);

    graph.clear();
    let stats_after = graph.get_stats();
    assert_eq!(
        stats_after.node_count, 0,
        "After clear(), node_count should be 0"
    );
    assert_eq!(
        stats_after.edge_count, 0,
        "After clear(), edge_count should be 0"
    );
    assert_eq!(
        stats_after.conflict_count, 0,
        "After clear(), conflict_count should be 0"
    );
    assert_eq!(
        stats_after.constraint_count, 0,
        "After clear(), constraint_count should be 0"
    );
}

/// DependencyGraph: exclusion prevents adding excluded package
#[test]
fn test_dependency_graph_exclusion() {
    use rez_next_package::Package;

    let mut graph = DependencyGraph::new();
    graph.add_exclusion("banned_pkg".to_string()).unwrap();

    let mut banned = Package::new("banned_pkg".to_string());
    banned.version = Some(Version::parse("1.0").unwrap());

    let result = graph.add_package(banned);
    assert!(result.is_err(), "Adding excluded package should return Err");

    let stats = graph.get_stats();
    assert_eq!(stats.exclusion_count, 1, "exclusion_count should be 1");
    assert_eq!(
        stats.node_count, 0,
        "Excluded package should not be in graph"
    );
}

// ─── Cycle detection tests ────────────────────────────────────────────────────

/// DependencyGraph: direct cycle A → B → A triggers topological sort error
#[test]
fn test_graph_cycle_detection_two_nodes() {
    use rez_next_package::Package;

    let make_pkg = |name: &str, ver: &str| {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        pkg
    };

    let mut graph = DependencyGraph::new();
    graph.add_package(make_pkg("A", "1.0")).unwrap();
    graph.add_package(make_pkg("B", "1.0")).unwrap();
    // A depends on B, B depends on A → cycle
    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "A-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_err(),
        "Cyclic graph (A→B→A) should fail topological sort"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.to_lowercase().contains("circular") || err_msg.to_lowercase().contains("cycle"),
        "Error message should mention circular/cycle, got: {}",
        err_msg
    );
}

/// DependencyGraph: three-node cycle A → B → C → A
#[test]
fn test_graph_cycle_detection_three_nodes() {
    use rez_next_package::Package;

    let make_pkg = |name: &str, ver: &str| {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        pkg
    };

    let mut graph = DependencyGraph::new();
    graph.add_package(make_pkg("A", "1.0")).unwrap();
    graph.add_package(make_pkg("B", "1.0")).unwrap();
    graph.add_package(make_pkg("C", "1.0")).unwrap();
    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "C-1.0").unwrap();
    graph.add_dependency_edge("C-1.0", "A-1.0").unwrap(); // closes the cycle

    let result = graph.get_resolved_packages();
    assert!(
        result.is_err(),
        "Three-node cycle (A→B→C→A) should fail topological sort"
    );
}

/// DependencyGraph: linear chain (no cycle) → get_resolved_packages succeeds
#[test]
fn test_graph_no_cycle_linear_chain() {
    use rez_next_package::Package;

    let make_pkg = |name: &str, ver: &str| {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        pkg
    };

    let mut graph = DependencyGraph::new();
    graph.add_package(make_pkg("X", "1.0")).unwrap();
    graph.add_package(make_pkg("Y", "1.0")).unwrap();
    graph.add_package(make_pkg("Z", "1.0")).unwrap();
    graph.add_dependency_edge("X-1.0", "Y-1.0").unwrap();
    graph.add_dependency_edge("Y-1.0", "Z-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Linear chain (X→Y→Z) should NOT be detected as cyclic"
    );
    assert_eq!(
        result.unwrap().len(),
        3,
        "All 3 packages should be returned"
    );
}

/// DependencyGraph: self-loop (A → A) triggers cycle error
#[test]
fn test_graph_cycle_detection_self_loop() {
    use rez_next_package::Package;

    let mut pkg = Package::new("SelfDep".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());

    let mut graph = DependencyGraph::new();
    graph.add_package(pkg).unwrap();
    graph
        .add_dependency_edge("SelfDep-1.0", "SelfDep-1.0")
        .unwrap();

    let result = graph.get_resolved_packages();
    assert!(result.is_err(), "Self-loop should be detected as cyclic");
}

// ─── Basic resolver edge-case tests ──────────────────────────────────────────

/// Resolver: empty repo returns Ok with empty result (no panic)
#[test]
fn test_resolver_empty_repo_empty_requirements() {
    let (_tmp, repo) = build_test_repo(&[]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let result = rt.block_on(resolver.resolve(vec![]));
    assert!(
        result.is_ok(),
        "Empty repo + empty requirements should succeed"
    );
    assert_eq!(result.unwrap().resolved_packages.len(), 0);
}

/// Resolver: two packages with no shared deps resolve independently.
#[test]
fn test_resolver_two_independent_packages() {
    let (_tmp, repo) = build_test_repo(&[("alpha", "2.0.0", &[]), ("beta", "3.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["alpha", "beta"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("two independent packages should resolve without conflict");

    let names: Vec<&str> = result
        .resolved_packages
        .iter()
        .map(|rp| rp.package.name.as_str())
        .collect();
    assert!(
        names.contains(&"alpha"),
        "resolved packages should include alpha"
    );
    assert!(
        names.contains(&"beta"),
        "resolved packages should include beta"
    );
    assert_eq!(
        result.resolved_packages.len(),
        2,
        "exactly 2 packages expected, got {}",
        result.resolved_packages.len()
    );
}

/// Resolver: deep transitive chain A→B→C→D resolves correctly.
#[test]
fn test_resolver_deep_transitive_chain() {
    let (_tmp, repo) = build_test_repo(&[
        ("d", "1.0.0", &[]),
        ("c", "1.0.0", &["d-1+"]),
        ("b", "1.0.0", &["c-1+"]),
        ("a", "1.0.0", &["b-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["a"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("deep chain A→B→C→D should resolve");

    let names: Vec<&str> = result
        .resolved_packages
        .iter()
        .map(|rp| rp.package.name.as_str())
        .collect();

    for expected in ["a", "b", "c", "d"] {
        assert!(
            names.contains(&expected),
            "deep chain resolution should include '{}', got {:?}",
            expected,
            names
        );
    }
    assert_eq!(
        result.resolved_packages.len(),
        4,
        "expected 4 packages (a,b,c,d), got {}",
        result.resolved_packages.len()
    );
}
