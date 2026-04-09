//! Solver Tests — Requirement Satisfaction, from_str edge cases, Version Selection Strategy
//!
//! Extracted from rez_solver_advanced_tests.rs (Cycle 142).

use rez_next_package::{PackageRequirement, Requirement};
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

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
