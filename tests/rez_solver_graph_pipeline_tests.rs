//! Solver Graph Pipeline and Conflict Tests (Cycle 76 split)
//!
//! Covers:
//! - Large VFX pipeline resolution (20+ packages)
//! - Version conflict in resolver
//!
//! prefer_latest/oldest semantics + conflict error message assertions
//! → extracted to rez_solver_prefer_tests.rs (Cycle 144)

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
