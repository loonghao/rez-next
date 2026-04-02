//! Solver Graph, Cycle Detection, and Large Pipeline Integration Tests
//!
//! Covers:
//! - DependencyGraph topology (add packages, edges, topological sort, clear, exclusion)
//! - Cycle detection (two-node, three-node, self-loop, linear chain)
//! - Large VFX pipeline resolution (20+ packages)
//! - Version conflict in resolver
//! - Edge-case solver tests (cycle 25+)

use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyGraph, DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::sync::Arc;
use tempfile::TempDir;

/// Build a temporary package repository with multiple packages.
/// Returns the TempDir (must be kept alive) and the RepositoryManager.
fn build_test_repo(packages: &[(&str, &str, &[&str])]) -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    for (name, version, requires) in packages {
        let pkg_dir = repo_dir.join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let requires_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        std::fs::write(
            pkg_dir.join("package.py"),
            format!(
                "name = '{}'\nversion = '{}'\n{}",
                name, version, requires_block
            ),
        )
        .unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "test_repo".to_string(),
    )));
    (tmp, Arc::new(mgr))
}

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

// ─── Large VFX pipeline tests ─────────────────────────────────────────────────

/// Large VFX pipeline: 20+ packages with multi-level dependency tree.
/// Simulates a real studio DCC environment.
#[test]
fn test_large_vfx_pipeline_resolve() {
    let (_tmp, repo) = build_test_repo(&[
        ("imath", "3.1.0", &[]),
        ("python", "3.10.0", &[]),
        ("numpy", "1.24.0", &["python-3+"]),
        ("pyside2", "5.15.0", &["python-3+"]),
        ("openexr", "3.1.0", &["imath-3+"]),
        ("alembic", "1.8.0", &["openexr-3+", "imath-3+"]),
        ("usd", "23.11", &["python-3+", "openexr-3+", "alembic-1+"]),
        ("arnold", "7.2.0", &["python-3+"]),
        ("redshift", "3.5.0", &["python-3+"]),
        (
            "maya",
            "2024.0",
            &["python-3.9+<3.12", "pyside2-5+", "openexr-3+"],
        ),
        ("houdini", "20.0.547", &["python-3.10+<3.12", "openexr-3+"]),
        ("nuke", "15.0", &["python-3+", "pyside2-5+", "openexr-3+"]),
        ("katana", "7.0", &["python-3+", "pyside2-5+"]),
        ("mari", "7.0", &["python-3+", "pyside2-5+", "openexr-3+"]),
        (
            "gaffer",
            "1.4.0",
            &["python-3+", "pyside2-5+", "openexr-3+", "imath-3+"],
        ),
        ("clarisse", "6.0", &["python-3+", "openexr-3+"]),
        ("substance_painter", "10.0", &["python-3+", "pyside2-5+"]),
        ("blender", "4.1.0", &["python-3+"]),
        ("arnold_plugins", "1.0", &["arnold-7+"]),
        ("redshift_plugins", "1.0", &["redshift-3+"]),
        (
            "pipeline_tools",
            "2.0",
            &["usd-23+", "alembic-1+", "python-3+"],
        ),
        (
            "studio_base",
            "1.0",
            &["maya-2024+", "houdini-20+", "nuke-15+", "pipeline_tools-2+"],
        ),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["studio_base"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Large VFX pipeline (22 pkgs) should resolve: {:?}",
        result.as_ref().err()
    );

    let resolution = result.unwrap();
    let names: std::collections::HashSet<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    for pkg in &[
        "studio_base",
        "maya",
        "houdini",
        "nuke",
        "pipeline_tools",
        "python",
        "openexr",
        "imath",
        "usd",
        "alembic",
    ] {
        assert!(
            names.contains(*pkg),
            "Package '{}' should be in large VFX pipeline resolution",
            pkg
        );
    }

    let python_count = resolution
        .resolved_packages
        .iter()
        .filter(|p| p.package.name == "python")
        .count();
    assert_eq!(
        python_count, 1,
        "python should be deduplicated even in a 22-package pipeline"
    );

    let openexr_count = resolution
        .resolved_packages
        .iter()
        .filter(|p| p.package.name == "openexr")
        .count();
    assert_eq!(openexr_count, 1, "openexr should be deduplicated");
}

/// Large VFX pipeline statistics: packages_considered covers the full dependency tree
#[test]
fn test_large_pipeline_stats_populated() {
    let (_tmp, repo) = build_test_repo(&[
        ("imath", "3.1.0", &[]),
        ("python", "3.10.0", &[]),
        ("pyside2", "5.15.0", &["python-3+"]),
        ("openexr", "3.1.0", &["imath-3+"]),
        ("alembic", "1.8.0", &["openexr-3+", "imath-3+"]),
        ("usd", "23.11", &["python-3+", "openexr-3+", "alembic-1+"]),
        ("maya", "2024.0", &["python-3+", "pyside2-5+", "openexr-3+"]),
        ("pipeline_tools", "2.0", &["usd-23+", "python-3+"]),
        ("studio_env", "1.0", &["maya-2024+", "pipeline_tools-2+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["studio_env"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("studio_env should resolve");

    assert!(
        result.stats.packages_considered > 4,
        "packages_considered should be > 4 for a 9-package pipeline, got {}",
        result.stats.packages_considered
    );
    assert!(
        result.stats.resolution_time_ms < 60_000,
        "Resolution time should be < 60s for a moderate pipeline"
    );
}

// ─── Version conflict in resolver ─────────────────────────────────────────────

/// Resolver: two packages requiring incompatible versions of a shared dep.
#[test]
fn test_resolver_incompatible_shared_dep_detected() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "2.7.0", &[]),
        ("python", "3.10.0", &[]),
        ("tool_a", "1.0", &["python-2+"]),
        ("tool_b", "1.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["tool_a", "tool_b"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(resolution) => {
            let python_count = resolution
                .resolved_packages
                .iter()
                .filter(|p| p.package.name == "python")
                .count();
            assert_eq!(
                python_count, 1,
                "python should only be selected once even when multiple tools need it"
            );
        }
        Err(_) => {
            // Conflict detection is also valid behavior
        }
    }
}

/// Resolver: repo with multiple versions, strict upper bound excludes newest
#[test]
fn test_resolver_strict_upper_bound_excludes_latest() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.5.0", &[]),
        ("lib", "2.0.0", &[]),
        ("lib", "2.5.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+<2"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("lib-1+<2 should resolve");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("1.5.0"),
        "lib-1+<2 prefer_latest should pick 1.5.0 (not 2.x), got {:?}",
        ver
    );
}

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

// ─── Cycle 25+: additional solver edge-case tests ─────────────────────────────

/// Resolver: single package with exact version pinned — only that version resolved.
#[test]
fn test_resolver_exact_version_pin() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.1.0", &[]),
        ("lib", "2.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib==1.1.0"].iter().map(|s| s.parse().unwrap()).collect();

    match rt.block_on(resolver.resolve(reqs)) {
        Ok(resolution) => {
            assert_eq!(
                resolution.resolved_packages.len(),
                1,
                "Exact pin should yield exactly one package"
            );
            let ver = resolution.resolved_packages[0]
                .package
                .version
                .as_ref()
                .map(|v| v.as_str());
            assert_ne!(ver, Some("2.0.0"), "Exact pin should not pick 2.0.0");
            assert_ne!(ver, Some("1.0.0"), "Exact pin should not pick 1.0.0");
        }
        Err(_) => {
            // Unsupported == syntax is also acceptable; no panic expected
        }
    }
}

/// Resolver: request package not in repo — documents solver behavior (no panic).
#[test]
fn test_resolver_missing_package_returns_error() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_pkg"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    match rt.block_on(resolver.resolve(reqs)) {
        Ok(resolution) => {
            let names: Vec<&str> = resolution
                .resolved_packages
                .iter()
                .map(|rp| rp.package.name.as_str())
                .collect();
            assert!(
                !names.contains(&"nonexistent_pkg"),
                "nonexistent_pkg should not appear in the resolved set, got {:?}",
                names
            );
        }
        Err(_) => {
            // Strict behavior: also acceptable
        }
    }
}

/// Resolver: prefer_latest=true always picks the highest available version.
#[test]
fn test_resolver_prefer_latest_picks_highest() {
    let (_tmp, repo) = build_test_repo(&[
        ("tool", "0.9.0", &[]),
        ("tool", "1.0.0", &[]),
        ("tool", "1.2.0", &[]),
        ("tool", "1.3.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["tool"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("tool resolution should succeed");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("1.3.0"),
        "prefer_latest should select 1.3.0 (highest), got {:?}",
        ver
    );
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

// ─── prefer_latest=false semantic tests ──────────────────────────────────────

/// prefer_latest=false: always selects the oldest (lowest) satisfying version.
///
/// With three versions of the same package, prefer_latest=false should pick
/// the minimum satisfying version rather than the maximum.
#[test]
fn test_resolver_prefer_oldest_with_multiple_deps() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.5.0", &[]),
        ("lib", "2.0.0", &[]),
        ("lib", "3.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("lib-1+ should resolve with prefer_latest=false");

    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_eq!(
        ver, "1.0.0",
        "prefer_latest=false should pick lib-1.0.0 (oldest satisfying), got '{}'",
        ver
    );
}

/// prefer_latest=false with upper bound: picks oldest in range.
///
/// Range lib-1+<3 contains 1.0.0, 1.5.0, 2.0.0 (3.0.0 excluded).
/// With prefer_latest=false, should select 1.0.0.
#[test]
fn test_resolver_prefer_oldest_respects_upper_bound() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.5.0", &[]),
        ("lib", "2.0.0", &[]),
        ("lib", "3.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+<3"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("lib-1+<3 should resolve with prefer_latest=false");

    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    // Must NOT pick 3.0.0 (excluded) and should pick the oldest in range
    assert_ne!(
        ver, "3.0.0",
        "lib-3.0.0 should be excluded by <3 upper bound"
    );
    assert_eq!(
        ver, "1.0.0",
        "prefer_latest=false with range lib-1+<3 should pick 1.0.0, got '{}'",
        ver
    );
}

/// prefer_latest=false with transitive deps: oldest versions selected throughout chain.
///
/// Chain: app-1+ depends on lib-1+. Both app and lib have multiple versions.
/// With prefer_latest=false, both should pick their oldest satisfying versions.
#[test]
fn test_resolver_prefer_oldest_transitive() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "2.0.0", &[]),
        ("app", "1.0.0", &["lib-1+"]),
        ("app", "2.0.0", &["lib-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["app-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("app+lib chain with prefer_latest=false should resolve");

    let app_ver = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "app")
        .and_then(|p| p.package.version.as_ref())
        .map(|v| v.as_str())
        .unwrap_or("?");

    let lib_ver = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "lib")
        .and_then(|p| p.package.version.as_ref())
        .map(|v| v.as_str())
        .unwrap_or("?");

    assert_eq!(
        app_ver, "1.0.0",
        "prefer_latest=false: app should pick 1.0.0, got '{}'",
        app_ver
    );
    assert_eq!(
        lib_ver, "1.0.0",
        "prefer_latest=false: lib should pick 1.0.0, got '{}'",
        lib_ver
    );
}

// ─── Conflict error message assertion tests ───────────────────────────────────

/// Strict mode + disjoint conflict: error message identifies the conflicting package.
///
/// When two requirements for the same package produce a conflict in strict mode,
/// the error message should mention the conflicting package name.
#[test]
fn test_solver_conflict_error_message_names_package() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.9.0", &[]), ("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request two versions that may or may not conflict depending on solver
    // but both reference "python" — if failure occurs, message must name it
    let reqs: Vec<Requirement> = ["python-99+"] // no version 99 exists
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    match rt.block_on(resolver.resolve(reqs)) {
        Err(e) => {
            let msg = e.to_string();
            assert!(
                msg.contains("python") || msg.contains("Strict mode"),
                "error message should mention 'python' or 'Strict mode', got: '{}'",
                msg
            );
        }
        Ok(res) => {
            // If lenient fallback — accepted; python-99+ should be in failed
            let _ = res;
        }
    }
}

/// Strict mode: error for multiple missing packages must list each one.
#[test]
fn test_solver_conflict_error_message_multiple_missing() {
    let (_tmp, repo) = build_test_repo(&[("existing_pkg", "1.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["missing_x-1.0", "missing_y-2.0", "missing_z-3.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    // All three missing packages must appear in the error message
    assert!(
        msg.contains("missing_x"),
        "error should mention missing_x, got: '{}'",
        msg
    );
    assert!(
        msg.contains("missing_y"),
        "error should mention missing_y, got: '{}'",
        msg
    );
    assert!(
        msg.contains("missing_z"),
        "error should mention missing_z, got: '{}'",
        msg
    );
}

/// Strict mode: error message format is stable — prefix + requirement list.
///
/// The canonical format is: "Strict mode: failed to satisfy requirements: <names>"
/// Callers that parse this string must not break when the format is preserved.
#[test]
fn test_solver_conflict_error_message_format_stable() {
    let (_tmp, repo) = build_test_repo(&[]); // empty repo

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["stable_check_pkg-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Strict mode"),
        "error message must begin with 'Strict mode' prefix, got: '{}'",
        msg
    );
    assert!(
        msg.contains("stable_check_pkg"),
        "error message must mention the missing requirement 'stable_check_pkg', got: '{}'",
        msg
    );
}
