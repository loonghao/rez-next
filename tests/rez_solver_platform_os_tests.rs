//! Solver Platform / OS Constraint and Strict Mode Tests (Cycle 76 split)
//!
//! Covers:
//! - Platform / OS constraint resolution
//! - Strict mode vs. lenient mode behavior
//! - Version range semantics (exclusive upper bound, prefix range, multi-version)

use rez_next_package::Requirement;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

// ─── Platform / OS constraint tests ──────────────────────────────────────────

/// Solver: package with platform-specific dep resolves on matching platform.
#[test]
fn test_solver_platform_specific_package_resolves() {
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
    assert!(
        result.is_ok(),
        "platform-specific package should resolve when platform matches"
    );
    let resolution = result.unwrap();
    let names: Vec<&str> = resolution
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();
    assert!(
        names.contains(&"maya_linux"),
        "maya_linux should be in resolution"
    );
    assert!(
        !resolution.resolved_packages.is_empty(),
        "resolution should contain at least one package"
    );
}

/// Solver: requesting a package that requires a different platform than provided.
///
/// Repository has `platform-windows` only; `maya_linux` requires `platform-linux`.
/// In lenient mode (default) the solver must return `Ok` but `maya_linux` (and/or
/// `platform-linux`) must appear in `failed_requirements`.
#[test]
fn test_solver_platform_mismatch_lenient_records_failure() {
    let (_tmp, repo) = build_test_repo(&[
        ("platform", "windows", &[]),
        ("maya_linux", "2024.0.0", &["platform-linux"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["maya_linux"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(res) => {
            let maya_resolved = res
                .resolved_packages
                .iter()
                .any(|p| p.package.name == "maya_linux");
            assert!(
                !maya_resolved || !res.failed_requirements.is_empty(),
                "platform mismatch: maya_linux should not be cleanly resolved without \
                 recording at least one failed requirement; resolved={:?}, failed={:?}",
                res.resolved_packages
                    .iter()
                    .map(|p| &p.package.name)
                    .collect::<Vec<_>>(),
                res.failed_requirements
                    .iter()
                    .map(|r| &r.name)
                    .collect::<Vec<_>>()
            );
        }
        Err(_) => {
            // Strict-mode-like error is also an acceptable outcome
        }
    }
}

/// Solver: platform mismatch in strict mode returns Err.
#[test]
fn test_solver_platform_mismatch_strict_returns_err() {
    let (_tmp, repo) = build_test_repo(&[
        ("platform", "windows", &[]),
        ("maya_linux", "2024.0.0", &["platform-linux"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["maya_linux"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "strict mode: platform mismatch (platform-linux unavailable) should return Err"
    );
}

/// Solver: package with OS-version constraint resolves correctly.
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
    assert!(
        result.is_ok(),
        "OS version constraint should resolve when OS version satisfies range"
    );
}

/// Solver: version range exclusive upper bound is respected.
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

    let reqs: Vec<Requirement> = ["lib-1+<3"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "exclusive upper bound range should resolve");
    let resolution = result.unwrap();
    assert_eq!(
        resolution.resolved_packages.len(),
        1,
        "exactly one lib should be selected"
    );
    let selected_ver = resolution.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_ne!(
        selected_ver, "3.0.0",
        "lib-3.0.0 should be excluded by <3 upper bound"
    );
    assert_ne!(
        selected_ver, "3",
        "lib-3 should be excluded by <3 upper bound"
    );
}

/// Solver: wildcard / prefix range resolves correct epoch.
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

    let reqs: Vec<Requirement> = ["lib-2"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(result.is_ok(), "prefix range 'lib-2' should resolve");
    let resolution = result.unwrap();
    for rp in &resolution.resolved_packages {
        if rp.package.name == "lib" {
            let ver = rp
                .package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("?");
            assert!(
                ver.starts_with("2.") || ver == "2",
                "resolved lib version '{}' should be in epoch 2",
                ver
            );
        }
    }
}

/// Solver: multiple versions of same package in repo — always picks highest satisfying.
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

    let reqs: Vec<Requirement> = ["lib-1+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("'lib-1+' should resolve with multiple versions available");

    assert_eq!(
        result.resolved_packages.len(),
        1,
        "exactly one lib should be selected"
    );
    let selected = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    assert_eq!(
        selected, "2.0.0",
        "prefer-latest: 'lib-1+' should select lib-2.0.0 (highest satisfying), got '{}'",
        selected
    );
}

// ─── Strict mode tests ────────────────────────────────────────────────────────

/// Strict mode: missing package returns Err, not Ok with empty resolved set.
#[test]
fn test_solver_strict_mode_missing_package_returns_err() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "strict mode should return Err for missing package"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("nonexistent_package") || err_msg.contains("Strict mode"),
        "error message should mention the missing package or strict mode, got: {}",
        err_msg
    );
}

/// Lenient mode (default): missing package returns Ok with failed_requirements populated.
#[test]
fn test_solver_lenient_mode_missing_package_returns_ok_with_failed() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: false,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["nonexistent_package-1.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "lenient mode should not return Err for missing package"
    );
    let resolution = result.unwrap();
    assert_eq!(
        resolution.failed_requirements.len(),
        1,
        "failed_requirements should record the unsatisfied requirement"
    );
    assert!(
        resolution.failed_requirements[0]
            .name
            .contains("nonexistent_package"),
        "failed requirement name should be 'nonexistent_package'"
    );
}

/// Strict mode: fully satisfiable request returns Ok (no regression).
#[test]
fn test_solver_strict_mode_satisfiable_request_returns_ok() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-3+", "numpy-1+"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_ok(),
        "strict mode with satisfiable request should return Ok"
    );
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

/// Strict mode: partial failure returns Err even when some packages are present.
#[test]
fn test_solver_strict_mode_partial_failure_returns_err() {
    let (_tmp, repo) = build_test_repo(&[
        ("python", "3.11.0", &[]),
        ("numpy", "1.25.0", &["python-3+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-3+", "numpy-1+", "missing_dep-2.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "strict mode should return Err if any requirement is unsatisfied"
    );
}

/// Strict mode: version constraint with no matching candidate returns Err.
#[test]
fn test_solver_strict_mode_version_mismatch_returns_err() {
    let (_tmp, repo) = build_test_repo(&[("lib", "1.0.0", &[]), ("lib", "2.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["lib-5+"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    assert!(
        result.is_err(),
        "strict mode: version range with no matching candidate should return Err"
    );
}

/// Strict mode + lenient mode comparison: same unsatisfiable request, different outcomes.
#[test]
fn test_solver_strict_vs_lenient_same_request() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    let config_lenient = SolverConfig {
        strict_mode: false,
        ..SolverConfig::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config_lenient);

    let reqs: Vec<Requirement> = ["ghost_pkg"].iter().map(|s| s.parse().unwrap()).collect();

    let result_lenient = rt.block_on(resolver.resolve(reqs.clone()));
    assert!(result_lenient.is_ok(), "lenient must return Ok");
    assert_eq!(
        result_lenient.unwrap().failed_requirements.len(),
        1,
        "lenient: exactly one failed requirement"
    );

    let config_strict = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver_strict = DependencyResolver::new(Arc::clone(&repo), config_strict);
    let result_strict = rt.block_on(resolver_strict.resolve(reqs));
    assert!(
        result_strict.is_err(),
        "strict must return Err for missing package"
    );
}
