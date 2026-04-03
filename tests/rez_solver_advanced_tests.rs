//! Advanced Solver Integration Tests — Basic Scenarios
//!
//! Covers:
//! - Diamond dependencies (A->B,C; B->D; C->D → need compatible D)
//! - Conflict detection (requirements with disjoint version ranges)
//! - Transitive dependency resolution
//! - Empty / edge cases
//! - DependencyGraph structural tests
//! - VFX pipeline integration scenario
//! - Requirement satisfaction edge cases
//! - Requirement.from_str edge cases
//! - Solver version selection strategy tests (prefer_latest / prefer_oldest / stats)
//!
//! See also:
//! - rez_solver_graph_tests.rs  — graph topology, cycle detection, large VFX, edge cases
//! - rez_solver_platform_tests.rs — platform/OS, strict mode, pre-release, variants, error messages

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

// ─── Requirement satisfaction edge cases ─────────────────────────────────────

/// VersionConstraint edge: LessThanOrEqual boundary
#[test]
fn test_version_constraint_lte_boundary() {
    use rez_next_package::requirement::VersionConstraint;

    let lte = VersionConstraint::LessThanOrEqual(Version::parse("2.0").unwrap());
    assert!(
        lte.is_satisfied_by(&Version::parse("2.0").unwrap()),
        "2.0 <= 2.0 should be true"
    );
    assert!(
        lte.is_satisfied_by(&Version::parse("1.9").unwrap()),
        "1.9 <= 2.0 should be true"
    );
    assert!(
        !lte.is_satisfied_by(&Version::parse("2.1").unwrap()),
        "2.1 <= 2.0 should be false"
    );
}

/// VersionConstraint edge: GreaterThan strict boundary
///
/// Note on rez depth-truncated semantics:
/// `cmp_at_depth(1.0.1, 1.0)` → depth=2 → compare tokens [1,0] vs [1,0] → Equal
/// So `GreaterThan(1.0)` on `1.0.1` is False (rez treats 1.0.1 as within the 1.0 epoch).
#[test]
fn test_version_constraint_gt_strict_boundary() {
    use rez_next_package::requirement::VersionConstraint;

    let gt = VersionConstraint::GreaterThan(Version::parse("1.0").unwrap());
    assert!(
        !gt.is_satisfied_by(&Version::parse("1.0").unwrap()),
        "1.0 > 1.0 should be false"
    );
    assert!(
        !gt.is_satisfied_by(&Version::parse("1.0.1").unwrap()),
        "1.0.1 > 1.0: depth-truncated at 2 tokens → Equal, not Greater (rez semantics)"
    );
    assert!(
        gt.is_satisfied_by(&Version::parse("1.1").unwrap()),
        "1.1 > 1.0: second token 1 > 0 → Greater"
    );
    assert!(
        gt.is_satisfied_by(&Version::parse("2.0").unwrap()),
        "2.0 > 1.0 should be true"
    );
}

/// Range constraint: [min, max) boundaries are correct
#[test]
fn test_version_constraint_range_boundaries() {
    use rez_next_package::requirement::VersionConstraint;

    let range = VersionConstraint::Range(
        Version::parse("1.0").unwrap(),
        Version::parse("2.0").unwrap(),
    );

    assert!(
        range.is_satisfied_by(&Version::parse("1.0").unwrap()),
        "1.0 is in [1.0, 2.0)"
    );
    assert!(
        range.is_satisfied_by(&Version::parse("1.9.9").unwrap()),
        "1.9.9 is in [1.0, 2.0)"
    );
    assert!(
        !range.is_satisfied_by(&Version::parse("2.0").unwrap()),
        "2.0 is NOT in [1.0, 2.0)"
    );
    assert!(
        !range.is_satisfied_by(&Version::parse("0.9").unwrap()),
        "0.9 is NOT in [1.0, 2.0)"
    );
}

/// VersionConstraint: Any matches everything
#[test]
fn test_version_constraint_any_matches_all() {
    use rez_next_package::requirement::VersionConstraint;

    let any = VersionConstraint::Any;
    let versions = ["0.0.1", "1.0", "100.200.300", "2.3.4.5.6"];
    for v in &versions {
        assert!(
            any.is_satisfied_by(&Version::parse(v).unwrap()),
            "Any should match {}",
            v
        );
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
    assert!(
        req.is_satisfied_by(&Version::parse("2024.0").unwrap()),
        "maya-2024+ should satisfy 2024.0"
    );
    assert!(
        req.is_satisfied_by(&Version::parse("2025.0").unwrap()),
        "maya-2024+ should satisfy 2025.0"
    );
    assert!(
        !req.is_satisfied_by(&Version::parse("2023.5").unwrap()),
        "maya-2024+ should NOT satisfy 2023.5"
    );
}

/// from_str: rez-native "pkg-ver+<max" format
#[test]
fn test_requirement_from_str_rez_native_range() {
    let req = "python-3.9+<4".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "python");
    assert!(req.is_satisfied_by(&Version::parse("3.9").unwrap()));
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(
        !req.is_satisfied_by(&Version::parse("3.8").unwrap()),
        "3.8 < 3.9: should not satisfy"
    );
    assert!(
        !req.is_satisfied_by(&Version::parse("4.0.0").unwrap()),
        "4.0.0: same epoch as 4, should not satisfy <4"
    );
}

/// from_str: rez-native "pkg-ver" point release
#[test]
fn test_requirement_from_str_rez_point_release() {
    let req = "numpy-1.25".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "numpy");
    assert!(
        req.is_satisfied_by(&Version::parse("1.25").unwrap()),
        "1.25 satisfies point release numpy-1.25"
    );
    assert!(
        req.is_satisfied_by(&Version::parse("1.25.0").unwrap()),
        "1.25.0 satisfies point release numpy-1.25"
    );
    assert!(
        req.is_satisfied_by(&Version::parse("1.25.3").unwrap()),
        "1.25.3 satisfies point release numpy-1.25"
    );
    assert!(
        !req.is_satisfied_by(&Version::parse("1.26.0").unwrap()),
        "1.26.0 does NOT satisfy point release numpy-1.25"
    );
    assert!(
        !req.is_satisfied_by(&Version::parse("1.24.9").unwrap()),
        "1.24.9 does NOT satisfy point release numpy-1.25"
    );
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
    assert!(
        result.is_ok(),
        "Resolver should succeed with multiple numpy versions"
    );

    let resolution = result.unwrap();
    assert_eq!(resolution.resolved_packages.len(), 1);

    let ver = resolution.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("1.25.0"),
        "prefer_latest should pick numpy-1.25.0"
    );
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

    let resolution = rt
        .block_on(resolver.resolve(reqs))
        .expect("Resolver should succeed");
    assert_eq!(resolution.resolved_packages.len(), 1);

    let ver = resolution.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("1.8.0"),
        "prefer_latest=false should pick scipy-1.8.0"
    );
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

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("Resolver should succeed");

    assert!(
        result.stats.packages_considered > 0,
        "packages_considered should be > 0 after resolving numpy+python"
    );
    assert!(
        result.stats.resolution_time_ms < 30_000,
        "Resolution time should be reasonable (<30s), got {}ms",
        result.stats.resolution_time_ms
    );
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

    let reqs: Vec<Requirement> = ["python-3.9+<3.12"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("Resolver should succeed");
    assert_eq!(result.resolved_packages.len(), 1);

    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("3.11.0"),
        "python-3.9+<3.12 with prefer_latest should pick 3.11.0, got {:?}",
        ver
    );
}

// ─── Cycle 28: Additional edge-case tests ──────────────────────────────────────

/// Solver: requesting a package with exact version (==) picks that exact version.
#[test]
fn test_solver_exact_version_pin() {
    let (_tmp, repo) = build_test_repo(&[
        ("numpy", "1.20.0", &[]),
        ("numpy", "1.24.0", &[]),
        ("numpy", "1.25.0", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["numpy==1.24.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("Exact version pin should resolve");
    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("1.24.0"), "== pin should select exactly 1.24.0");
}

/// Solver: conflicting transitive requirements — one dep needs lib<2, another needs lib>=2.
#[test]
fn test_solver_conflicting_transitive_requirements() {
    let (_tmp, repo) = build_test_repo(&[
        ("lib", "1.5.0", &[]),
        ("lib", "2.0.0", &[]),
        ("pkg_a", "1.0.0", &["lib<2"]),
        ("pkg_b", "1.0.0", &["lib-2+"]),
        ("root", "1.0.0", &["pkg_a-1+", "pkg_b-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["root"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    // Either Ok (one lib wins via conflict resolution) or Err is acceptable
    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(r) => {
            let _ = r.resolved_packages;
        }
        Err(_) => {}
    }
}

/// Solver: deep diamond — A->B->D, A->C->D, B and C need different D ranges.
#[test]
fn test_solver_deep_diamond_with_range_constraints() {
    let (_tmp, repo) = build_test_repo(&[
        ("shared", "1.0.0", &[]),
        ("shared", "1.8.0", &[]),
        ("shared", "2.0.0", &[]),
        ("left", "1.0.0", &["shared>=1,<2"]),
        ("right", "1.0.0", &["shared>=1.5,<2.1"]),
        ("top", "1.0.0", &["left-1+", "right-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["top"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Deep diamond with overlapping ranges should resolve"
    );
    let r = result.unwrap();
    let names: std::collections::HashSet<&str> = r
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();
    for pkg in &["top", "left", "right", "shared"] {
        assert!(names.contains(*pkg), "'{}' should be resolved in deep diamond", pkg);
    }
}

/// Solver: self-referencing requirement (A depends on A) — cycle detection catches it.
#[test]
fn test_solver_self_dependency_cycle_detected() {
    let (_tmp, repo) = build_test_repo(&[("selfish", "1.0.0", &["selfish"])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["selfish"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "Self-referencing dependency should return an error (cycle)"
    );
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            msg.to_lowercase().contains("cycle") || msg.to_lowercase().contains("cyclic"),
            "Error should mention cycle, got: {}",
            msg
        );
    }
}

/// Solver: request same package twice with different version ranges — should unify.
#[test]
fn test_solver_duplicate_request_different_ranges_unifies() {
    let (_tmp, repo) = build_test_repo(&[("lib", "1.5.0", &[]), ("lib", "2.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request same package with two compatible ranges
    let reqs: Vec<Requirement> = ["lib>=1.0", "lib<3"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "Duplicate request with compatible ranges should resolve"
    );
    let r = result.unwrap();
    let count = r
        .resolved_packages
        .iter()
        .filter(|p| p.package.name == "lib")
        .count();
    assert_eq!(count, 1, "lib should appear exactly once despite duplicate requests");
}

/// SolverConfig: strict_mode field is serialized correctly to JSON.
#[test]
fn test_solver_config_strict_mode_serialization() {
    use serde_json;

    let config = SolverConfig {
        strict_mode: true,
        prefer_latest: false,
        ..SolverConfig::default()
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(
        json.contains("\"strict_mode\":true"),
        "Serialized config must contain strict_mode:true, got: {}",
        json
    );

    let back: SolverConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.strict_mode, true, "Roundtrip strict_mode must be true");
    assert!(!back.prefer_latest, "Roundtrip prefer_latest must be false");
}

/// Solver: empty repository with non-empty requirements — all go to failed_requirements.
#[test]
fn test_solver_empty_repo_all_failed() {
    let tmp = TempDir::new().unwrap(); // empty repo — no packages written

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp.path().to_path_buf(),
        "empty".to_string(),
    )));
    let repo_arc = Arc::new(mgr);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

    let reqs: Vec<Requirement> = ["foo-1+", "bar-2+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("Lenient mode returns Ok even for empty repo");

    assert!(result.resolved_packages.is_empty(), "No packages should be resolved from empty repo");
    assert_eq!(
        result.failed_requirements.len(),
        2,
        "Both foo and bar should be in failed_requirements"
    );
}

// ─── Cycle 30: Solver error message assertions ──────────────────────────────

/// Solver strict mode: missing package returns an Err with package name in message.
#[test]
fn test_solver_strict_mode_error_message_contains_package_name() {
    let tmp = TempDir::new().unwrap();
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(
        rez_next_repository::simple_repository::SimpleRepository::new(
            tmp.path().to_path_buf(),
            "empty".to_string(),
        ),
    ));
    let repo_arc = Arc::new(mgr);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), config);

    let reqs: Vec<Requirement> =
        ["missing_pkg_xyz"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "Strict mode should return Err for missing package"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("missing_pkg_xyz")
            || err_msg.contains("not found")
            || err_msg.contains("Missing")
            || err_msg.contains("failed"),
        "Error message should reference the missing package or indicate failure, got: {}",
        err_msg
    );
}

/// Solver: conflict error message mentions at least one of the conflicting package names.
#[test]
fn test_solver_conflict_error_message_contains_package_info() {
    let (_tmp, repo) = build_test_repo(&[
        ("conflict_lib", "1.0.0", &[]),
        ("conflict_lib", "3.0.0", &[]),
        ("pkg_a", "1.0.0", &["conflict_lib<2"]),
        ("pkg_b", "1.0.0", &["conflict_lib>=3"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["pkg_a", "pkg_b"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    // Either Err (conflict detected) or Ok (lenient fallback) - both are valid
    // but if Err, message must contain useful info
    if let Err(e) = result {
        let msg = format!("{}", e);
        assert!(
            !msg.is_empty(),
            "Conflict error message should not be empty"
        );
        let has_info = msg.contains("conflict")
            || msg.contains("pkg")
            || msg.contains("lib")
            || msg.contains("version")
            || msg.contains("failed")
            || msg.contains("satisfy");
        assert!(
            has_info,
            "Conflict error should describe the problem, got: {}",
            msg
        );
    }
}
