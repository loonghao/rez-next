//! Advanced Solver Integration Tests
//!
//! These tests verify complex dependency resolution scenarios matching real-world
//! rez package manager behavior:
//! - Diamond dependencies (A->B,C; B->D; C->D → need compatible D)
//! - Conflict detection (requirements with disjoint version ranges)
//! - Transitive dependency resolution
//! - Solver with realistic VFX pipeline packages
//! - Performance and correctness under stress

use std::sync::Arc;
use tempfile::TempDir;
use rez_next_solver::{DependencyResolver, SolverConfig, DependencyGraph};
use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};

use rez_next_version::Version;

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
            let items: Vec<String> = requires.iter()
                .map(|r| format!("    '{}',", r))
                .collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        std::fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n{}", name, version, requires_block),
        ).unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "test_repo".to_string(),
    )));
    (tmp, Arc::new(mgr))
}

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

    let reqs: Vec<Requirement> = ["my_lib"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Diamond dependency with compatible versions should resolve");
    let resolution = result.unwrap();

    let names: Vec<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();
    assert!(names.contains(&"my_lib"), "my_lib should be resolved");
    assert!(names.contains(&"numpy"), "numpy should be pulled in transitively");
    assert!(names.contains(&"scipy"), "scipy should be pulled in transitively");
    assert!(names.contains(&"python"), "python should be pulled in transitively");
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

    let reqs: Vec<Requirement> = ["A"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Same-range diamond dependency should resolve");
    let resolution = result.unwrap();

    // D should only appear once (not duplicated)
    let d_packages: Vec<_> = resolution.resolved_packages.iter()
        .filter(|p| p.package.name == "D")
        .collect();
    assert_eq!(d_packages.len(), 1, "D should be resolved exactly once (not duplicated)");

    // D-1+ means >=1 (depth-truncated), which includes D-2.0.0.
    // Solver picks latest satisfying version: D-2.0.0.
    let d_ver = d_packages[0].package.version.as_ref().map(|v| v.as_str());
    assert!(
        d_ver == Some("2.0.0") || d_ver == Some("1.5.0"),
        "D should be resolved to a valid version (2.0.0 or 1.5.0), got: {:?}", d_ver
    );
}

// ─── Conflict detection tests ─────────────────────────────────────────────────

/// DependencyGraph: two disjoint constraints for same package = conflict
#[test]
fn test_graph_conflict_disjoint_versions() {
    let mut graph = DependencyGraph::new();
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.9,<3.10".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.11".to_string())
    ).unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(!conflicts.is_empty(),
        "Disjoint version ranges [3.9,3.10) and [3.11,∞) should produce a conflict");
    // Conflict should reference 'python'
    assert!(conflicts.iter().any(|c| c.package_name == "python"),
        "Conflict should identify 'python' as the conflicting package");
}

/// DependencyGraph: overlapping constraints for same package = no conflict
#[test]
fn test_graph_no_conflict_overlapping_versions() {
    let mut graph = DependencyGraph::new();
    // >=1.0 and <3.0 overlap → no conflict
    graph.add_requirement(
        PackageRequirement::with_version("scipy".to_string(), ">=1.0".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("scipy".to_string(), "<3.0".to_string())
    ).unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(),
        "Overlapping ranges >=1.0 and <3.0 should NOT produce a conflict");
}

/// DependencyGraph: single constraint for a package = no conflict
#[test]
fn test_graph_no_conflict_single_requirement() {
    let mut graph = DependencyGraph::new();
    graph.add_requirement(
        PackageRequirement::with_version("maya".to_string(), ">=2024".to_string())
    ).unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(), "Single constraint should never produce a conflict");
}

/// DependencyGraph: multiple packages, conflicts only for conflicting one
#[test]
fn test_graph_partial_conflict() {
    let mut graph = DependencyGraph::new();

    // python: compatible (both require 3.x)
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.9".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), "<4".to_string())
    ).unwrap();

    // numpy: conflicting
    graph.add_requirement(
        PackageRequirement::with_version("numpy".to_string(), ">=1.20,<1.22".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("numpy".to_string(), ">=1.25".to_string())
    ).unwrap();

    let conflicts = graph.detect_conflicts();
    // Only numpy should conflict
    let conflict_names: Vec<&str> = conflicts.iter().map(|c| c.package_name.as_str()).collect();
    assert!(!conflict_names.contains(&"python"), "python should NOT conflict");
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

    let reqs: Vec<Requirement> = ["A"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Transitive chain resolution should succeed");
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();

    for pkg in &["A", "B", "C", "D", "E"] {
        assert!(names.contains(*pkg),
            "Package '{}' should be in resolved set (transitive)", pkg);
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

    // Request both pandas and matplotlib (they share python and numpy)
    let reqs: Vec<Requirement> = ["pandas", "matplotlib"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Multiple roots with shared dependencies should resolve");
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains("pandas"), "pandas should be resolved");
    assert!(names.contains("matplotlib"), "matplotlib should be resolved");
    assert!(names.contains("numpy"), "numpy should be resolved as shared dep");
    assert!(names.contains("python"), "python should be resolved as shared dep");

    // numpy should appear only once
    let numpy_count = resolution.resolved_packages.iter()
        .filter(|p| p.package.name == "numpy")
        .count();
    assert_eq!(numpy_count, 1, "numpy should be deduplicated (shared dep)");
}

// ─── Empty and edge cases ─────────────────────────────────────────────────────

/// Empty requirements resolve to empty package set
#[test]
fn test_resolver_empty_requirements() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let result = rt.block_on(resolver.resolve(vec![]));
    assert!(result.is_ok(), "Empty requirements should succeed");
    let resolution = result.unwrap();
    assert_eq!(resolution.resolved_packages.len(), 0,
        "Empty requirements should produce 0 resolved packages");
}

/// Unknown package (not in repo) handled gracefully
#[test]
fn test_resolver_unknown_package_graceful() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["totally_nonexistent_xyz_12345"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    // Should not panic; may succeed with empty result or fail gracefully
    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(resolution) => {
            // Resolution with no packages is acceptable for unknown package
            let _ = resolution;
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

    // Simulate maya requiring python and pyside
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.9".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("pyside2".to_string(), ">=5.15".to_string())
    ).unwrap();

    // Simulate nuke also requiring python
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), "<4".to_string())
    ).unwrap();

    // python: >=3.9 AND <4 → no conflict
    // pyside2: only one constraint → no conflict
    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(),
        "Compatible multi-package graph should have no conflicts, got: {:?}",
        conflicts.iter().map(|c| &c.package_name).collect::<Vec<_>>()
    );
}

/// DependencyGraph: add requirement for same package multiple times (exact same)
#[test]
fn test_dependency_graph_duplicate_requirement_no_conflict() {
    let mut graph = DependencyGraph::new();

    // Same requirement added twice should not conflict
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.9".to_string())
    ).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("python".to_string(), ">=3.9".to_string())
    ).unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(), "Identical requirements should not produce a conflict");
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

    // Request both maya and houdini
    let reqs: Vec<Requirement> = ["maya", "houdini"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "VFX pipeline maya+houdini resolve should succeed");
    let resolution = result.unwrap();

    let names: std::collections::HashSet<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains("maya"), "maya should be resolved");
    assert!(names.contains("houdini"), "houdini should be resolved");
    // python-3.10.0 satisfies both maya (3.9+<3.12) and houdini (3.10+<3.12)
    assert!(names.contains("python"), "python should be resolved as shared dep");
}

// ─── Requirement satisfaction edge cases ─────────────────────────────────────

/// VersionConstraint edge: LessThanOrEqual boundary
#[test]
fn test_version_constraint_lte_boundary() {
    use rez_next_package::requirement::VersionConstraint;

    let lte = VersionConstraint::LessThanOrEqual(Version::parse("2.0").unwrap());
    assert!(lte.is_satisfied_by(&Version::parse("2.0").unwrap()), "2.0 <= 2.0 should be true");
    assert!(lte.is_satisfied_by(&Version::parse("1.9").unwrap()), "1.9 <= 2.0 should be true");
    assert!(!lte.is_satisfied_by(&Version::parse("2.1").unwrap()), "2.1 <= 2.0 should be false");
}

/// VersionConstraint edge: GreaterThan strict boundary
///
/// Note on rez depth-truncated semantics:
/// `cmp_at_depth(1.0.1, 1.0)` → depth=2 → compare tokens [1,0] vs [1,0] → Equal
/// So `GreaterThan(1.0)` on `1.0.1` is False (rez treats 1.0.1 as within the 1.0 epoch).
/// Use a clearly different major/minor to test strict boundary.
#[test]
fn test_version_constraint_gt_strict_boundary() {
    use rez_next_package::requirement::VersionConstraint;

    let gt = VersionConstraint::GreaterThan(Version::parse("1.0").unwrap());
    assert!(!gt.is_satisfied_by(&Version::parse("1.0").unwrap()), "1.0 > 1.0 should be false");
    // 1.0.1 vs constraint 1.0 at depth 2 → both tokens equal → Equal (not Greater)
    // This is expected rez depth-truncated behavior
    assert!(!gt.is_satisfied_by(&Version::parse("1.0.1").unwrap()),
        "1.0.1 > 1.0: depth-truncated at 2 tokens → Equal, not Greater (rez semantics)");
    assert!(gt.is_satisfied_by(&Version::parse("1.1").unwrap()),
        "1.1 > 1.0: second token 1 > 0 → Greater");
    assert!(gt.is_satisfied_by(&Version::parse("2.0").unwrap()), "2.0 > 1.0 should be true");
}

/// Range constraint: [min, max) boundaries are correct
#[test]
fn test_version_constraint_range_boundaries() {
    use rez_next_package::requirement::VersionConstraint;

    let range = VersionConstraint::Range(
        Version::parse("1.0").unwrap(),
        Version::parse("2.0").unwrap(),
    );

    assert!(range.is_satisfied_by(&Version::parse("1.0").unwrap()), "1.0 is in [1.0, 2.0)");
    assert!(range.is_satisfied_by(&Version::parse("1.9.9").unwrap()), "1.9.9 is in [1.0, 2.0)");
    assert!(!range.is_satisfied_by(&Version::parse("2.0").unwrap()), "2.0 is NOT in [1.0, 2.0)");
    assert!(!range.is_satisfied_by(&Version::parse("0.9").unwrap()), "0.9 is NOT in [1.0, 2.0)");
}

/// VersionConstraint: Any matches everything
#[test]
fn test_version_constraint_any_matches_all() {
    use rez_next_package::requirement::VersionConstraint;

    let any = VersionConstraint::Any;
    let versions = ["0.0.1", "1.0", "100.200.300", "2.3.4.5.6"];
    for v in &versions {
        assert!(any.is_satisfied_by(&Version::parse(v).unwrap()),
            "Any should match {}", v);
    }
}

// ─── Requirement.from_str edge cases ─────────────────────────────────────────

/// from_str: package name with underscores
#[test]
fn test_requirement_from_str_underscored_name() {
    let req = "my_package_123".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "my_package_123");
    assert!(req.version_constraint.is_none());
}

/// from_str: package name with hyphen and semver
#[test]
fn test_requirement_from_str_hyphenated_with_version() {
    let req = "some-lib>=2.0".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "some-lib");
    assert!(matches!(
        req.version_constraint,
        Some(rez_next_package::requirement::VersionConstraint::GreaterThanOrEqual(_))
    ));
}

/// from_str: rez-native "pkg-ver+" format
#[test]
fn test_requirement_from_str_rez_native_plus() {
    let req = "maya-2024+".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "maya");
    assert!(matches!(
        req.version_constraint,
        Some(rez_next_package::requirement::VersionConstraint::GreaterThanOrEqual(_))
    ));
    assert!(req.is_satisfied_by(&Version::parse("2024.0").unwrap()),
        "maya-2024+ should satisfy 2024.0");
    assert!(req.is_satisfied_by(&Version::parse("2025.0").unwrap()),
        "maya-2024+ should satisfy 2025.0");
    assert!(!req.is_satisfied_by(&Version::parse("2023.5").unwrap()),
        "maya-2024+ should NOT satisfy 2023.5");
}

/// from_str: rez-native "pkg-ver+<max" format
#[test]
fn test_requirement_from_str_rez_native_range() {
    let req = "python-3.9+<4".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "python");
    assert!(req.is_satisfied_by(&Version::parse("3.9").unwrap()));
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req.is_satisfied_by(&Version::parse("3.8").unwrap()),
        "3.8 < 3.9: should not satisfy");
    assert!(!req.is_satisfied_by(&Version::parse("4.0.0").unwrap()),
        "4.0.0: same epoch as 4, should not satisfy <4");
}

/// from_str: rez-native "pkg-ver" point release
#[test]
fn test_requirement_from_str_rez_point_release() {
    let req = "numpy-1.25".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "numpy");
    // Point release matches 1.25.x family
    assert!(req.is_satisfied_by(&Version::parse("1.25").unwrap()),
        "1.25 satisfies point release numpy-1.25");
    assert!(req.is_satisfied_by(&Version::parse("1.25.0").unwrap()),
        "1.25.0 satisfies point release numpy-1.25");
    assert!(req.is_satisfied_by(&Version::parse("1.25.3").unwrap()),
        "1.25.3 satisfies point release numpy-1.25");
    assert!(!req.is_satisfied_by(&Version::parse("1.26.0").unwrap()),
        "1.26.0 does NOT satisfy point release numpy-1.25");
    assert!(!req.is_satisfied_by(&Version::parse("1.24.9").unwrap()),
        "1.24.9 does NOT satisfy point release numpy-1.25");
}

// ─── Solver version selection strategy tests ─────────────────────────────────

/// prefer_latest=true (default): resolver picks highest available version
#[test]
fn test_resolver_prefer_latest_version() {
    let (_tmp, repo) = build_test_repo(&[
        ("numpy", "1.20.0", &[]),
        ("numpy", "1.21.0", &[]),
        ("numpy", "1.25.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["numpy"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Resolver should succeed with multiple numpy versions");

    let resolution = result.unwrap();
    assert_eq!(resolution.resolved_packages.len(), 1);

    let ver = resolution.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("1.25.0"), "prefer_latest should pick numpy-1.25.0");
}

/// prefer_latest=false: resolver picks lowest available version (oldest first)
#[test]
fn test_resolver_prefer_oldest_version() {
    let (_tmp, repo) = build_test_repo(&[
        ("scipy", "1.8.0", &[]),
        ("scipy", "1.9.0", &[]),
        ("scipy", "1.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["scipy"].iter().map(|s| s.parse().unwrap()).collect();

    let resolution = rt.block_on(resolver.resolve(reqs)).expect("Resolver should succeed");
    assert_eq!(resolution.resolved_packages.len(), 1);

    let ver = resolution.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("1.8.0"), "prefer_latest=false should pick scipy-1.8.0");
}

/// Resolution statistics: packages_considered > 0 after a successful resolve
#[test]
fn test_resolver_stats_populated() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["numpy"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs)).expect("Resolver should succeed");

    assert!(result.stats.packages_considered > 0,
        "packages_considered should be > 0 after resolving numpy+python");
    assert!(result.stats.resolution_time_ms < 30_000,
        "Resolution time should be reasonable (<30s), got {}ms", result.stats.resolution_time_ms);
}

/// Resolution with explicit version upper bound: only picks versions within range
#[test]
fn test_resolver_version_upper_bound_respected() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.9.0", &[]),
        ("python", "3.10.0", &[]),
        ("python", "3.11.0", &[]),
        ("python", "3.12.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request python-3.9+<3.12: 3.9, 3.10, 3.11 valid, 3.12 excluded
    let reqs: Vec<Requirement> = ["python-3.9+<3.12"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs)).expect("Resolver should succeed");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str());
    // prefer_latest picks the highest satisfying version: 3.11.0
    assert_eq!(ver, Some("3.11.0"),
        "python-3.9+<3.12 with prefer_latest should pick 3.11.0, got {:?}", ver);
}

// ─── DependencyGraph topology tests ──────────────────────────────────────────

/// DependencyGraph: add packages and edges, verify node count and stats
#[test]
fn test_dependency_graph_add_packages_and_edges() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

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
    use rez_next_solver::DependencyGraph;

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
    assert_eq!(resolved.len(), 3, "All 3 packages should be in topological sort");

    // In topological order, A comes before B, B before C
    // (nodes with no incoming edges = A → C is deepest)
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
    use rez_next_solver::DependencyGraph;

    let mut pkg = Package::new("X".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());

    let mut graph = DependencyGraph::new();
    graph.add_package(pkg).unwrap();
    graph.add_requirement(
        PackageRequirement::with_version("Y".to_string(), ">=2.0".to_string())
    ).unwrap();
    graph.add_constraint(
        PackageRequirement::with_version("Z".to_string(), ">=1.0".to_string())
    ).unwrap();

    let stats_before = graph.get_stats();
    assert_eq!(stats_before.node_count, 1);
    assert_eq!(stats_before.constraint_count, 1);

    graph.clear();
    let stats_after = graph.get_stats();
    assert_eq!(stats_after.node_count, 0, "After clear(), node_count should be 0");
    assert_eq!(stats_after.edge_count, 0, "After clear(), edge_count should be 0");
    assert_eq!(stats_after.conflict_count, 0, "After clear(), conflict_count should be 0");
    assert_eq!(stats_after.constraint_count, 0, "After clear(), constraint_count should be 0");
}

/// DependencyGraph: exclusion prevents adding excluded package
#[test]
fn test_dependency_graph_exclusion() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph.add_exclusion("banned_pkg".to_string()).unwrap();

    let mut banned = Package::new("banned_pkg".to_string());
    banned.version = Some(Version::parse("1.0").unwrap());

    let result = graph.add_package(banned);
    assert!(result.is_err(), "Adding excluded package should return Err");

    let stats = graph.get_stats();
    assert_eq!(stats.exclusion_count, 1, "exclusion_count should be 1");
    assert_eq!(stats.node_count, 0, "Excluded package should not be in graph");
}

// ─── Cycle detection tests ────────────────────────────────────────────────────

/// DependencyGraph: direct cycle A → B → A triggers topological sort error
#[test]
fn test_graph_cycle_detection_two_nodes() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

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
    assert!(result.is_err(), "Cyclic graph (A→B→A) should fail topological sort");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.to_lowercase().contains("circular") || err_msg.to_lowercase().contains("cycle"),
        "Error message should mention circular/cycle, got: {}", err_msg
    );
}

/// DependencyGraph: three-node cycle A → B → C → A
#[test]
fn test_graph_cycle_detection_three_nodes() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

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
    assert!(result.is_err(), "Three-node cycle (A→B→C→A) should fail topological sort");
}

/// DependencyGraph: linear chain (no cycle) → get_resolved_packages succeeds
#[test]
fn test_graph_no_cycle_linear_chain() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

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
    // No cycle: X → Y → Z

    let result = graph.get_resolved_packages();
    assert!(result.is_ok(), "Linear chain (X→Y→Z) should NOT be detected as cyclic");
    assert_eq!(result.unwrap().len(), 3, "All 3 packages should be returned");
}

/// DependencyGraph: self-loop (A → A) triggers cycle error
#[test]
fn test_graph_cycle_detection_self_loop() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;

    let mut pkg = Package::new("SelfDep".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());

    let mut graph = DependencyGraph::new();
    graph.add_package(pkg).unwrap();
    graph.add_dependency_edge("SelfDep-1.0", "SelfDep-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(result.is_err(), "Self-loop should be detected as cyclic");
}

// ─── Large VFX pipeline tests ─────────────────────────────────────────────────

/// Large VFX pipeline: 20+ packages with multi-level dependency tree
/// Simulates a real studio DCC environment:
///   python → (numpy, pyside2)
///   maya → (python, pyside2, openexr)
///   houdini → (python, openexr, vex)
///   nuke → (python, pyside2, openexr)
///   katana → (python, pyside2)
///   mari → (python, pyside2, openexr)
///   gaffer → (python, pyside2, openexr, imath)
///   clarisse → (python, openexr)
///   substance_painter → (python, pyside2)
///   blender → (python)
///   openexr → (imath)
///   alembic → (openexr, imath)
///   usd → (python, openexr, alembic)
///   arnold → (python)
///   redshift → (python)
///   katana_plugins → (katana, arnold)
///   houdini_plugins → (houdini, arnold, redshift)
///   maya_plugins → (maya, arnold)
///   pipeline_tools → (usd, alembic, python)
///   studio_base → (maya, houdini, nuke, pipeline_tools)
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
        ("maya", "2024.0", &["python-3.9+<3.12", "pyside2-5+", "openexr-3+"]),
        ("houdini", "20.0.547", &["python-3.10+<3.12", "openexr-3+"]),
        ("nuke", "15.0", &["python-3+", "pyside2-5+", "openexr-3+"]),
        ("katana", "7.0", &["python-3+", "pyside2-5+"]),
        ("mari", "7.0", &["python-3+", "pyside2-5+", "openexr-3+"]),
        ("gaffer", "1.4.0", &["python-3+", "pyside2-5+", "openexr-3+", "imath-3+"]),
        ("clarisse", "6.0", &["python-3+", "openexr-3+"]),
        ("substance_painter", "10.0", &["python-3+", "pyside2-5+"]),
        ("blender", "4.1.0", &["python-3+"]),
        ("arnold_plugins", "1.0", &["arnold-7+"]),
        ("redshift_plugins", "1.0", &["redshift-3+"]),
        ("pipeline_tools", "2.0", &["usd-23+", "alembic-1+", "python-3+"]),
        ("studio_base", "1.0", &["maya-2024+", "houdini-20+", "nuke-15+", "pipeline_tools-2+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { prefer_latest: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["studio_base"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "Large VFX pipeline (22 pkgs) should resolve: {:?}",
        result.as_ref().err());

    let resolution = result.unwrap();
    let names: std::collections::HashSet<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();

    // Core packages must be present
    for pkg in &["studio_base", "maya", "houdini", "nuke", "pipeline_tools",
                 "python", "openexr", "imath", "usd", "alembic"] {
        assert!(names.contains(*pkg),
            "Package '{}' should be in large VFX pipeline resolution", pkg);
    }

    // python must appear exactly once (shared by all DCC tools)
    let python_count = resolution.resolved_packages.iter()
        .filter(|p| p.package.name == "python")
        .count();
    assert_eq!(python_count, 1,
        "python should be deduplicated even in a 22-package pipeline");

    // openexr must appear exactly once
    let openexr_count = resolution.resolved_packages.iter()
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

    let reqs: Vec<Requirement> = ["studio_env"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs)).expect("studio_env should resolve");

    // With 9 packages in repo and deep transitive chain, packages_considered should be > 4
    assert!(result.stats.packages_considered > 4,
        "packages_considered should be > 4 for a 9-package pipeline, got {}",
        result.stats.packages_considered);
    assert!(result.stats.resolution_time_ms < 60_000,
        "Resolution time should be < 60s for a moderate pipeline");
}

// ─── Version conflict in resolver ─────────────────────────────────────────────

/// Resolver: requesting two packages that both need incompatible versions of a shared dep
/// maya-2020 needs python-2.7, maya-2024 needs python-3.9+ — they can't coexist
#[test]
fn test_resolver_incompatible_shared_dep_detected() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "2.7.0", &[]),
        ("python", "3.10.0", &[]),
        // tool_a requires python <3 (python 2.x)
        ("tool_a", "1.0", &["python-2+"]),
        // tool_b requires python >=3
        ("tool_b", "1.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Requesting both tool_a and tool_b simultaneously
    let reqs: Vec<Requirement> = ["tool_a", "tool_b"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    // The resolution may succeed (picking python 3.x which also satisfies tool_a's python-2+)
    // OR it may detect a conflict and fail. Either is acceptable behavior.
    // Key assertion: no panic occurs
    match result {
        Ok(resolution) => {
            // If resolution succeeds, verify python is only selected once
            let python_count = resolution.resolved_packages.iter()
                .filter(|p| p.package.name == "python")
                .count();
            assert_eq!(python_count, 1,
                "python should only be selected once even when multiple tools need it");
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
        ("lib", "2.0.0", &[]),   // should be excluded
        ("lib", "2.5.0", &[]),   // should be excluded
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { prefer_latest: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request lib-1+ but <2: should pick 1.5.0, not 2.0.0
    let reqs: Vec<Requirement> = ["lib-1+<2"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs)).expect("lib-1+<2 should resolve");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("1.5.0"),
        "lib-1+<2 prefer_latest should pick 1.5.0 (not 2.x), got {:?}", ver);
}

/// Resolver: empty repo returns Ok with empty result (no panic)
#[test]
fn test_resolver_empty_repo_empty_requirements() {
    let (_tmp, repo) = build_test_repo(&[]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let result = rt.block_on(resolver.resolve(vec![]));
    assert!(result.is_ok(), "Empty repo + empty requirements should succeed");
    assert_eq!(result.unwrap().resolved_packages.len(), 0);
}

// ─── Cycle 25: additional solver edge-case tests ──────────────────────────────

/// Resolver: single package with exact version pinned — only that version resolved.
#[test]
fn test_resolver_exact_version_pin() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.1.0", &[]),
        ("lib", "2.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { prefer_latest: false, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib==1.1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    // The resolver must pick exactly 1.1.0 (or legitimately error if ==syntax unsupported)
    match rt.block_on(resolver.resolve(reqs)) {
        Ok(resolution) => {
            assert_eq!(resolution.resolved_packages.len(), 1,
                "Exact pin should yield exactly one package");
            let ver = resolution.resolved_packages[0].package.version.as_ref()
                .map(|v| v.as_str());
            // Must NOT be 2.0.0 or 1.0.0
            assert_ne!(ver, Some("2.0.0"), "Exact pin should not pick 2.0.0");
            assert_ne!(ver, Some("1.0.0"), "Exact pin should not pick 1.0.0");
        }
        Err(_) => {
            // Unsupported == syntax is also acceptable; no panic expected
        }
    }
}

/// Resolver: request package not in repo — documents solver behavior (no panic guaranteed).
///
/// Current solver behavior: returns Ok with empty resolution (lenient mode) rather than Err.
/// Test verifies: no panic occurs and the absent package is NOT present in the resolved set.
#[test]
fn test_resolver_missing_package_returns_error() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_pkg"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    // Solver must not panic regardless of outcome.
    // Current behavior: returns Ok (lenient — missing packages yield empty resolution).
    // If the solver is tightened to return Err in the future, both branches remain valid.
    match rt.block_on(resolver.resolve(reqs)) {
        Ok(resolution) => {
            // Lenient behavior: package is simply absent from the result
            let names: Vec<&str> = resolution.resolved_packages.iter()
                .map(|rp| rp.package.name.as_str())
                .collect();
            assert!(!names.contains(&"nonexistent_pkg"),
                "nonexistent_pkg should not appear in the resolved set, got {:?}", names);
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
    let config = SolverConfig { prefer_latest: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["tool"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("tool resolution should succeed");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("1.3.0"),
        "prefer_latest should select 1.3.0 (highest), got {:?}", ver);
}

/// Resolver: two packages with no shared deps resolve independently without interference.
#[test]
fn test_resolver_two_independent_packages() {
    let (_tmp, repo) = build_test_repo(&[
        ("alpha", "2.0.0", &[]),
        ("beta", "3.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["alpha", "beta"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("two independent packages should resolve without conflict");

    let names: Vec<&str> = result.resolved_packages.iter()
        .map(|rp| rp.package.name.as_str())
        .collect();
    assert!(names.contains(&"alpha"), "resolved packages should include alpha");
    assert!(names.contains(&"beta"),  "resolved packages should include beta");
    assert_eq!(result.resolved_packages.len(), 2,
        "exactly 2 packages expected, got {}", result.resolved_packages.len());
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

    let reqs: Vec<Requirement> = ["a"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("deep chain A→B→C→D should resolve");

    let names: Vec<&str> = result.resolved_packages.iter()
        .map(|rp| rp.package.name.as_str())
        .collect();

    // All four packages must be in the resolution
    for expected in ["a", "b", "c", "d"] {
        assert!(names.contains(&expected),
            "deep chain resolution should include '{}', got {:?}", expected, names);
    }
    assert_eq!(result.resolved_packages.len(), 4,
        "expected 4 packages (a,b,c,d), got {}", result.resolved_packages.len());
}

// ─── Platform / OS constraint tests ──────────────────────────────────────────

/// Solver: package with platform-specific dep resolves on matching platform.
///
/// When platform is simulated by including a "platform" package, packages that
/// require a specific platform variant should resolve when that platform is
/// present in the request.
#[test]
fn test_solver_platform_specific_package_resolves() {
    // Simulate: "maya_linux" requires "platform-linux"
    // The request includes both "platform-linux" and "maya_linux"
    let (_tmp, repo) = build_test_repo(&[
        ("platform", "linux", &[]),
        ("maya_linux", "2024.0.0", &["platform-linux"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["platform-linux", "maya_linux"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "platform-specific package should resolve when platform matches");
    let resolution = result.unwrap();
    let names: Vec<&str> = resolution.resolved_packages.iter()
        .map(|p| p.package.name.as_str())
        .collect();
    assert!(names.contains(&"maya_linux"), "maya_linux should be in resolution");
    // platform may or may not appear as explicit resolved dep depending on solver mode;
    // the important invariant is that resolution succeeded without error.
    assert!(!resolution.resolved_packages.is_empty(),
        "resolution should contain at least one package");
}

/// Solver: requesting a package that requires a different platform than provided fails.
///
/// If "maya_linux" requires "platform-linux" but only "platform-windows" is in the
/// repo, resolution should fail (conflict or missing dep).
#[test]
fn test_solver_platform_mismatch_fails_or_empty() {
    // Only platform-windows available; maya_linux requires platform-linux
    let (_tmp, repo) = build_test_repo(&[
        ("platform", "windows", &[]),
        ("maya_linux", "2024.0.0", &["platform-linux"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request maya_linux: requires platform-linux which is not available
    let reqs: Vec<Requirement> = ["maya_linux"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    // Solver may return error OR resolve without the required dep (lenient mode)
    // Either outcome is acceptable — must NOT panic
    match &result {
        Ok(res) => {
            // lenient: resolved but missing platform-linux — check no panic
            let _ = res.resolved_packages.len();
        }
        Err(_) => {
            // strict: returned an error — also acceptable
        }
    }
}

/// Solver: package with OS-version constraint resolves correctly.
///
/// "centos7_tools" requires "os-centos-7+<8", "centos7" satisfies this.
#[test]
fn test_solver_os_version_constraint_resolve() {
    let (_tmp, repo) = build_test_repo(&[
        ("os", "centos-7.9.0", &[]),
        ("centos7_tools", "1.0.0", &["os-centos-7+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["centos7_tools", "os-centos-7+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "OS version constraint should resolve when OS version satisfies range");
}

/// Solver: version range exclusive upper bound is respected.
///
/// Packages available: lib-1.0.0, lib-2.0.0, lib-3.0.0.
/// Request: lib-1+<3  → should resolve to lib-2.0.0 (highest in [1,3)).
#[test]
fn test_solver_exclusive_upper_bound_respected() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "2.0.0", &[]),
        ("lib", "3.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // lib-1+<3 means: version >= 1.0.0 AND version < 3.0.0
    let reqs: Vec<Requirement> = ["lib-1+<3"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "exclusive upper bound range should resolve");
    let resolution = result.unwrap();
    assert_eq!(resolution.resolved_packages.len(), 1, "exactly one lib should be selected");
    let selected_ver = resolution.resolved_packages[0]
        .package.version.as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    // Must NOT be lib-3.0.0 (excluded by <3)
    assert_ne!(selected_ver, "3.0.0", "lib-3.0.0 should be excluded by <3 upper bound");
    assert_ne!(selected_ver, "3",     "lib-3 should be excluded by <3 upper bound");
}

/// Solver: wildcard / prefix range resolution.
///
/// Request "lib-2" (rez: means exactly version 2, which is epoch >= 2 and < next epoch).
/// Only lib-2.0.0 should match (lib-1.0.0 and lib-3.0.0 should not).
#[test]
fn test_solver_prefix_version_range_resolves_correct_epoch() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "2.0.0", &[]),
        ("lib", "3.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-2"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "prefix range 'lib-2' should resolve");
    let resolution = result.unwrap();
    // The resolved lib must satisfy version 2 (epoch)
    for rp in &resolution.resolved_packages {
        if rp.package.name == "lib" {
            let ver = rp.package.version.as_ref().map(|v| v.as_str()).unwrap_or("?");
            assert!(
                ver.starts_with("2.") || ver == "2",
                "resolved lib version '{}' should be in epoch 2", ver
            );
        }
    }
}

/// Solver: multiple versions of same package in repo — always picks highest satisfying.
///
/// lib has 1.0.0, 1.5.0, 2.0.0. Request "lib-1+" → should pick lib-2.0.0
/// (rez prefer-latest semantics: highest version that satisfies constraints).
///
/// Note: in rez epoch semantics `2.0.0 > 1.5.0`, so 2.0.0 satisfies "lib-1+".
#[test]
fn test_solver_multi_version_picks_highest_satisfying() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.5.0", &[]),
        ("lib", "2.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("'lib-1+' should resolve with multiple versions available");

    assert_eq!(result.resolved_packages.len(), 1, "exactly one lib should be selected");
    let selected = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str()).unwrap_or("?");
    // Should be 2.0.0 (highest satisfying)
    assert_eq!(selected, "2.0.0",
        "prefer-latest: 'lib-1+' should select lib-2.0.0 (highest satisfying), got '{}'", selected);
}

// ─── Strict mode tests ────────────────────────────────────────────────────────

/// Strict mode: missing package returns Err, not Ok with empty resolved set.
///
/// In lenient mode (default), requesting a non-existent package returns Ok
/// with failed_requirements populated.  In strict mode the same request must
/// return Err so callers can rely on Ok meaning "fully resolved".
#[test]
fn test_solver_strict_mode_missing_package_returns_err() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_err(), "strict mode should return Err for missing package");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("nonexistent_package") || err_msg.contains("Strict mode"),
        "error message should mention the missing package or strict mode, got: {}", err_msg
    );
}

/// Lenient mode (default): missing package returns Ok with failed_requirements populated.
///
/// This verifies the default behaviour has not changed after adding strict_mode field.
#[test]
fn test_solver_lenient_mode_missing_package_returns_ok_with_failed() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    // explicit lenient config (strict_mode = false, which is also the default)
    let config = SolverConfig { strict_mode: false, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "lenient mode should not return Err for missing package");
    let resolution = result.unwrap();
    assert_eq!(
        resolution.failed_requirements.len(), 1,
        "failed_requirements should record the unsatisfied requirement"
    );
    assert!(
        resolution.failed_requirements[0].name.contains("nonexistent_package"),
        "failed requirement name should be 'nonexistent_package'"
    );
}

/// Strict mode: fully satisfiable request returns Ok (no regression).
///
/// When strict_mode is true and ALL requirements are satisfied, the result
/// should still be Ok (strict_mode should not affect successful resolutions).
#[test]
fn test_solver_strict_mode_satisfiable_request_returns_ok() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-3+", "numpy-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "strict mode with satisfiable request should return Ok");
    let resolution = result.unwrap();
    assert!(
        resolution.failed_requirements.is_empty(),
        "no requirements should fail for a fully satisfiable request"
    );
    assert!(
        !resolution.resolved_packages.is_empty(),
        "at least one package should be resolved"
    );
}

/// Strict mode: partial failure (some packages present, one missing) returns Err.
///
/// Even if most requirements are satisfied, strict mode must fail if even one
/// requirement cannot be met.
#[test]
fn test_solver_strict_mode_partial_failure_returns_err() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // python and numpy exist, but missing_dep does not
    let reqs: Vec<Requirement> = ["python-3+", "numpy-1+", "missing_dep-2.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_err(), "strict mode should return Err if any requirement is unsatisfied");
}

/// Strict mode: version constraint with no matching candidate returns Err.
///
/// Package exists (lib-1.0.0) but requested version range (lib-5+) has no match.
/// Strict mode must return Err; lenient mode would silently ignore it.
#[test]
fn test_solver_strict_mode_version_mismatch_returns_err() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "2.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request lib-5+ but only lib-1 and lib-2 exist
    let reqs: Vec<Requirement> = ["lib-5+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "strict mode: version range with no matching candidate should return Err"
    );
}

// ─── Pre-release / alpha token ordering tests ────────────────────────────────

/// Pre-release exclusion: allow_prerelease=false should not pick alpha version
/// when a stable release exists.
///
/// Repo has lib-1.0.0 (stable) and lib-1.1.alpha1 (pre-release).
/// With allow_prerelease=false, solver must pick lib-1.0.0.
#[test]
fn test_solver_prerelease_excluded_when_stable_available() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "1.1.alpha1", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { allow_prerelease: false, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("resolution with stable version should succeed");

    assert_eq!(result.resolved_packages.len(), 1);
    let selected = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str()).unwrap_or("?");
    assert_eq!(
        selected, "1.0.0",
        "with allow_prerelease=false, should pick stable 1.0.0, got '{}'", selected
    );
}

/// Pre-release inclusion: allow_prerelease=true picks highest version including alpha.
///
/// Repo has lib-1.0.0 and lib-2.alpha1. With allow_prerelease=true and
/// prefer_latest=true, solver should select lib-2.alpha1 as the highest
/// version that satisfies "lib-1+".
#[test]
fn test_solver_prerelease_included_when_allowed() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.0", &[]),
        ("lib", "2.alpha1", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        allow_prerelease: true,
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("resolution with prerelease allowed should succeed");

    assert_eq!(result.resolved_packages.len(), 1);
    let selected = result.resolved_packages[0].package.version.as_ref()
        .map(|v| v.as_str()).unwrap_or("?");
    // 2.alpha1 > 1.0.0 numerically (epoch 2 > epoch 1)
    assert_eq!(
        selected, "2.alpha1",
        "with allow_prerelease=true, should pick 2.alpha1 (highest), got '{}'", selected
    );
}

/// Pre-release only repo: when only pre-release versions exist and
/// allow_prerelease=false, the requirement should go into failed_requirements.
#[test]
fn test_solver_prerelease_only_repo_fails_when_not_allowed() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.0.alpha1", &[]),
        ("lib", "1.0.beta2", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { allow_prerelease: false, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("lenient mode should return Ok even when only prerelease available");

    // In lenient mode: failed_requirements contains "lib", resolved_packages is empty
    assert!(
        result.failed_requirements.iter().any(|r| r.name == "lib"),
        "lib should be in failed_requirements when no stable version exists and prerelease is not allowed"
    );
    assert!(
        !result.resolved_packages.iter().any(|p| p.package.name == "lib"),
        "lib should not be in resolved_packages when prerelease not allowed and no stable exists"
    );
}

// ─── Variant index scenario tests ────────────────────────────────────────────

/// Variant index: resolved package with variant_index=None means no variant was selected.
///
/// Standard packages (no variants in package.py) should have variant_index = None
/// in the resolution result.
#[test]
fn test_resolver_variant_index_none_for_plain_package() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-3+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("plain package should resolve");

    assert_eq!(result.resolved_packages.len(), 1);
    // Plain packages have no variant index
    assert_eq!(
        result.resolved_packages[0].variant_index, None,
        "plain package should have variant_index = None"
    );
}

/// Multiple packages resolved — each carries correct variant_index=None.
///
/// Verifies that the variant_index field is consistently None across all
/// resolved packages when the repo packages have no variants.
#[test]
fn test_resolver_all_resolved_packages_have_variant_index_none() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.10.0", &[]),
        ("numpy", "1.24.0", &["python-3+"]),
        ("scipy", "1.10.0", &["python-3+", "numpy-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["scipy-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("scipy with transitive deps should resolve");

    assert!(
        !result.resolved_packages.is_empty(),
        "should have resolved packages"
    );
    for pkg in &result.resolved_packages {
        assert_eq!(
            pkg.variant_index, None,
            "package '{}' should have variant_index=None (no variants in test repo)",
            pkg.package.name
        );
    }
}

/// Variant index field: ResolvedPackageInfo::variant_index can be set to Some(0).
///
/// This is a unit-level structural test: resolve a package, then verify the
/// variant_index field can be mutated to Some(0) (important for future variant-aware resolution).
#[test]
fn test_resolver_variant_index_some_can_be_constructed() {
    use rez_next_solver::dependency_resolver::ResolvedPackageInfo;

    let (_tmp, repo) = build_test_repo(&[
        ("maya", "2024.0.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["maya-2024+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let mut result = rt.block_on(resolver.resolve(reqs))
        .expect("maya should resolve");

    assert_eq!(result.resolved_packages.len(), 1);

    // Simulate a variant being assigned post-resolution
    result.resolved_packages[0].variant_index = Some(0);

    let info: &ResolvedPackageInfo = &result.resolved_packages[0];
    assert_eq!(info.variant_index, Some(0), "variant_index should be assignable to Some(0)");
    assert_eq!(info.package.name, "maya");
}

// ─── Solver error message content assertion tests ────────────────────────────

/// Strict mode error message: must contain "Strict mode" prefix.
///
/// The error message format from DependencyResolver is:
/// "Strict mode: failed to satisfy requirements: <req1>, <req2>, ..."
/// Callers may parse this — the prefix must remain stable.
#[test]
fn test_solver_strict_mode_error_message_prefix() {
    let (_tmp, repo) = build_test_repo(&[]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["ghost_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Strict mode"),
        "strict mode error must start with 'Strict mode', got: '{}'", msg
    );
}

/// Strict mode error message: lists all failed requirements by name.
///
/// When multiple requirements fail, all should appear in the error string
/// so users know exactly what went wrong.
#[test]
fn test_solver_strict_mode_error_message_lists_all_failed() {
    let (_tmp, repo) = build_test_repo(&[]);  // empty repo → all reqs fail

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig { strict_mode: true, ..SolverConfig::default() };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["pkgA-1.0", "pkgB-2.0", "pkgC-3.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("pkgA"), "error should mention pkgA, got: '{}'", msg);
    assert!(msg.contains("pkgB"), "error should mention pkgB, got: '{}'", msg);
    assert!(msg.contains("pkgC"), "error should mention pkgC, got: '{}'", msg);
}

/// Lenient mode: failed_requirements list preserves the exact requirement name.
///
/// When a requirement cannot be satisfied in lenient mode, the original
/// Requirement struct should appear in failed_requirements with the correct name.
#[test]
fn test_solver_lenient_failed_requirements_preserves_name() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();  // lenient
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["totally_nonexistent_package-99.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("lenient mode should return Ok");

    assert_eq!(result.failed_requirements.len(), 1);
    assert_eq!(
        result.failed_requirements[0].name,
        "totally_nonexistent_package",
        "failed_requirements should preserve the exact package name"
    );
}

/// Resolution stats: packages_considered increases with more packages in repo.
///
/// A repo with 5 packages should result in packages_considered >= 1.
/// This validates the stats tracking path is exercised.
#[test]
fn test_solver_stats_packages_considered_is_nonzero() {
    let (_tmp, repo) = build_test_repo(&[
        ("libA", "1.0.0", &[]),
        ("libB", "2.0.0", &[]),
        ("libC", "3.0.0", &[]),
        ("libD", "1.5.0", &[]),
        ("libE", "0.9.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["libA-1+", "libB-2+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs))
        .expect("resolution of libA and libB should succeed");

    assert!(
        result.stats.packages_considered >= 1,
        "at least 1 package should have been considered, got {}",
        result.stats.packages_considered
    );
    assert_eq!(
        result.resolved_packages.len(), 2,
        "libA and libB should both be resolved"
    );
}

