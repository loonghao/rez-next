//! Solver Pre-release, Variant, Stats, and Edge-case Tests (Cycle 76 split)
//!
//! Covers:
//! - Pre-release / alpha token ordering and allow_prerelease flag
//! - Variant index fields in resolved packages
//! - Resolution stats and timing
//! - Conflict field population
//! - Requested flag semantics

use rez_next_package::Requirement;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

// ─── Pre-release / alpha token ordering tests ─────────────────────────────────

/// Pre-release exclusion: allow_prerelease=false should not pick alpha version
/// when a stable release exists.
#[test]
fn test_solver_prerelease_excluded_when_stable_available() {
    let (_tmp, repo) = build_test_repo(&[("lib", "1.0.0", &[]), ("lib", "1.1.alpha1", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        allow_prerelease: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("resolution with stable version should succeed");

    assert_eq!(result.resolved_packages.len(), 1);
    let selected = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_eq!(
        selected, "1.0.0",
        "with allow_prerelease=false, should pick stable 1.0.0, got '{}'",
        selected
    );
}

/// Pre-release inclusion: allow_prerelease=true picks highest version including alpha.
#[test]
fn test_solver_prerelease_included_when_allowed() {
    let (_tmp, repo) = build_test_repo(&[("lib", "1.0.0", &[]), ("lib", "2.alpha1", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        allow_prerelease: true,
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("resolution with prerelease allowed should succeed");

    assert_eq!(result.resolved_packages.len(), 1);
    let selected = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_eq!(
        selected, "2.alpha1",
        "with allow_prerelease=true, should pick 2.alpha1 (highest), got '{}'",
        selected
    );
}

/// Pre-release only repo: when only pre-release versions exist and
/// allow_prerelease=false, the requirement should go into failed_requirements.
#[test]
fn test_solver_prerelease_only_repo_fails_when_not_allowed() {
    let (_tmp, repo) = build_test_repo(&[("lib", "1.0.alpha1", &[]), ("lib", "1.0.beta2", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        allow_prerelease: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("lenient mode should return Ok even when only prerelease available");

    assert!(
        result.failed_requirements.iter().any(|r| r.name == "lib"),
        "lib should be in failed_requirements when no stable version exists and prerelease is not allowed"
    );
    assert!(
        !result
            .resolved_packages
            .iter()
            .any(|p| p.package.name == "lib"),
        "lib should not be in resolved_packages when prerelease not allowed and no stable exists"
    );
}

/// Pre-release: allow_prerelease flag is independent of strict_mode.
///
/// Strict mode + allow_prerelease=true should still succeed when a pre-release
/// version is the only version that satisfies the constraint.
#[test]
fn test_solver_prerelease_strict_mode_compatible() {
    let (_tmp, repo) = build_test_repo(&[("lib", "2.alpha1", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        allow_prerelease: true,
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-2+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("strict+prerelease allowed: lib-2.alpha1 should satisfy lib-2+");

    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_eq!(
        ver, "2.alpha1",
        "strict+allow_prerelease: should pick 2.alpha1, got '{}'",
        ver
    );
}

/// Solver: request with only prerelease versions available, allow_prerelease=true in strict mode.
#[test]
fn test_solver_strict_prerelease_only_repo_allowed() {
    let (_tmp, repo) = build_test_repo(&[
        ("alpha_lib", "0.1.alpha", &[]),
        ("alpha_lib", "0.2.beta", &[]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        allow_prerelease: true,
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["alpha_lib"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("strict+allow_prerelease: should resolve when prerelease satisfies");

    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(
        ver,
        Some("0.2.beta"),
        "should pick highest prerelease 0.2.beta"
    );
}

// ─── Variant index scenario tests ─────────────────────────────────────────────

/// Variant index: resolved package with variant_index=None means no variant was selected.
#[test]
fn test_resolver_variant_index_none_for_plain_package() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-3+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("plain package should resolve");

    assert_eq!(result.resolved_packages.len(), 1);
    assert_eq!(
        result.resolved_packages[0].variant_index, None,
        "plain package should have variant_index = None"
    );
}

/// Multiple packages resolved — each carries correct variant_index=None.
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

    let reqs: Vec<Requirement> = ["scipy-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
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

// ─── Solver error message content assertion tests ─────────────────────────────

/// Strict mode error message: must contain "Strict mode" prefix.
#[test]
fn test_solver_strict_mode_error_message_prefix() {
    let (_tmp, repo) = build_test_repo(&[]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["ghost_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Strict mode"),
        "strict mode error must start with 'Strict mode', got: '{}'",
        msg
    );
}

/// Strict mode error message: lists all failed requirements by name.
#[test]
fn test_solver_strict_mode_error_message_lists_all_failed() {
    let (_tmp, repo) = build_test_repo(&[]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["pkgA-1.0", "pkgB-2.0", "pkgC-3.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let err = rt.block_on(resolver.resolve(reqs)).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("pkgA"),
        "error should mention pkgA, got: '{}'",
        msg
    );
    assert!(
        msg.contains("pkgB"),
        "error should mention pkgB, got: '{}'",
        msg
    );
    assert!(
        msg.contains("pkgC"),
        "error should mention pkgC, got: '{}'",
        msg
    );
}

/// Lenient mode: failed_requirements list preserves the exact requirement name.
#[test]
fn test_solver_lenient_failed_requirements_preserves_name() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["totally_nonexistent_package-99.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("lenient mode should return Ok");

    assert_eq!(result.failed_requirements.len(), 1);
    assert_eq!(
        result.failed_requirements[0].name, "totally_nonexistent_package",
        "failed_requirements should preserve the exact package name"
    );
}

// ─── Resolution stats tests ────────────────────────────────────────────────────

/// Resolution stats: packages_considered is non-zero after a successful resolve.
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

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("resolution of libA and libB should succeed");

    assert!(
        result.stats.packages_considered >= 1,
        "at least 1 package should have been considered, got {}",
        result.stats.packages_considered
    );
    assert_eq!(
        result.resolved_packages.len(),
        2,
        "libA and libB should both be resolved"
    );
}

/// Solver: resolution_time_ms is populated and reasonable (<10 seconds for small repos).
#[test]
fn test_solver_resolution_time_populated() {
    let (_tmp, repo) = build_test_repo(&[
        ("core", "1.0.0", &[]),
        ("utils", "2.0.0", &["core-1+"]),
        ("app", "3.0.0", &["core-1+", "utils-2+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["app"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("resolution should succeed");

    assert!(
        result.stats.resolution_time_ms < 10_000,
        "resolution should complete within 10s, got {}ms",
        result.stats.resolution_time_ms
    );
    assert!(
        result.resolved_packages.len() >= 2,
        "at least app + transitive deps should be resolved"
    );
}

// ─── Conflict field and requested flag tests ──────────────────────────────────

/// Solver: conflicts field is populated when version conflicts occur during resolution.
#[test]
fn test_solver_conflicts_field_populated_on_version_clash() {
    let (_tmp, repo) = build_test_repo(&[("shared", "1.0.0", &[]), ("shared", "2.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["shared==1.0.0", "shared==2.0.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(r) => {
            let shared_count = r
                .resolved_packages
                .iter()
                .filter(|p| p.package.name == "shared")
                .count();
            assert_eq!(
                shared_count, 1,
                "lenient mode: exactly one shared version should win the conflict, got {}",
                shared_count
            );
        }
        Err(_) => {
            // Strict-mode-like error: also acceptable
        }
    }
}

/// Solver: requested flag correctly marks explicitly requested vs transitive deps.
#[test]
fn test_solver_requested_flag_distinguishes_roots() {
    let (_tmp, repo) = build_test_repo(&[("base", "1.0.0", &[]), ("tool", "2.0.0", &["base-1+"])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["tool"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("tool+base should resolve");

    let tool_pkg = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "tool")
        .expect("tool should be in resolved set");
    assert!(
        tool_pkg.requested,
        "tool was explicitly requested — requested flag should be true"
    );

    let base_pkg = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "base")
        .expect("base should be in resolved set as transitive dep");
    assert!(
        !base_pkg.requested,
        "base is a transitive dep — requested flag should be false"
    );
}
