//! Solver prefer_latest/oldest Semantics and Conflict Error Message Tests
//!
//! Extracted from rez_solver_graph_pipeline_tests.rs (Cycle 144).

use rez_next_package::Requirement;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

// ─── prefer_latest=true semantic tests ───────────────────────────────────────

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

// ─── prefer_latest=false semantic tests ──────────────────────────────────────

/// prefer_latest=false: always selects the oldest (lowest) satisfying version.
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
#[test]
fn test_solver_conflict_error_message_names_package() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.9.0", &[]), ("python", "3.11.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["python-99+"].iter().map(|s| s.parse().unwrap()).collect();

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
            assert!(
                res.failed_requirements.iter().any(|r| r.name == "python"),
                "python-99+ should appear in failed_requirements; got: {:?}",
                res.failed_requirements
                    .iter()
                    .map(|r| &r.name)
                    .collect::<Vec<_>>()
            );
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
#[test]
fn test_solver_conflict_error_message_format_stable() {
    let (_tmp, repo) = build_test_repo(&[]);

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
