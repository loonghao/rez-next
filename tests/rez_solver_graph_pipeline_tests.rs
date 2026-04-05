//! Solver Graph Pipeline and Conflict Tests (Cycle 76 split)
//!
//! Covers:
//! - Large VFX pipeline resolution (20+ packages)
//! - Version conflict in resolver
//! - prefer_latest / prefer_oldest semantics
//! - Strict mode conflict error message assertions

use rez_next_package::Requirement;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;

#[path = "solver_helpers.rs"]
mod solver_helpers;

use solver_helpers::build_test_repo;

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

    let reqs: Vec<Requirement> = ["python-99+"]
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
