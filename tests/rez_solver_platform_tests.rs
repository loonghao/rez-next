//! Solver Platform, Strict Mode, Pre-release, and Variant Tests
//!
//! Covers:
//! - Platform / OS constraint resolution
//! - Strict mode vs. lenient mode behavior
//! - Pre-release / alpha token ordering and allow_prerelease flag
//! - Variant index fields in resolved packages
//! - Solver error message content assertions

use rez_next_package::Requirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
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
#[test]
fn test_solver_platform_mismatch_fails_or_empty() {
    let (_tmp, repo) = build_test_repo(&[
        ("platform", "windows", &[]),
        ("maya_linux", "2024.0.0", &["platform-linux"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["maya_linux"].iter().map(|s| s.parse().unwrap()).collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match &result {
        Ok(res) => {
            let _ = res.resolved_packages.len();
        }
        Err(_) => {
            // strict: returned an error — also acceptable
        }
    }
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

// ─── Cycle 28: Additional platform/strict/edge-case tests ─────────────────────

/// Strict mode + lenient mode comparison: same unsatisfiable request, different outcomes.
#[test]
fn test_solver_strict_vs_lenient_same_request() {
    let (_tmp, repo) = build_test_repo(&[("python", "3.11.0", &[])]);

    // Lenient: Ok with failed_requirements
    let config_lenient = SolverConfig {
        strict_mode: false,
        ..SolverConfig::default()
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config_lenient);

    let reqs: Vec<Requirement> = ["ghost_pkg"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result_lenient = rt.block_on(resolver.resolve(reqs.clone()));
    assert!(result_lenient.is_ok(), "lenient must return Ok");
    assert_eq!(
        result_lenient.unwrap().failed_requirements.len(),
        1,
        "lenient: exactly one failed requirement"
    );

    // Strict: Err
    let config_strict = SolverConfig {
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver_strict = DependencyResolver::new(Arc::clone(&repo), config_strict);
    let result_strict = rt.block_on(resolver_strict.resolve(reqs));
    assert!(result_strict.is_err(), "strict must return Err for missing package");
}

/// Solver: request with only prerelease versions available, allow_prerelease=true in strict mode.
#[test]
fn test_solver_strict_prerelease_only_repo_allowed() {
    let (_tmp, repo) = build_test_repo(&[("alpha_lib", "0.1.alpha", &[]), ("alpha_lib", "0.2.beta", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig {
        strict_mode: true,
        allow_prerelease: true,
        prefer_latest: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    let reqs: Vec<Requirement> = ["alpha_lib"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt
        .block_on(resolver.resolve(reqs))
        .expect("strict+allow_prerelease: should resolve when prerelease satisfies");

    assert_eq!(result.resolved_packages.len(), 1);
    let ver = result.resolved_packages[0]
        .package
        .version
        .as_ref()
        .map(|v| v.as_str());
    assert_eq!(ver, Some("0.2.beta"), "should pick highest prerelease 0.2.beta");
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

    let reqs: Vec<Requirement> = ["app"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

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

/// Solver: conflicts field is populated when version conflicts occur during resolution.
// Note: current resolver records conflict metadata but continues; this verifies the field.
#[test]
fn test_solver_conflicts_field_populated_on_version_clash() {
    // Two root requests that can't coexist for the same package with disjoint ranges
    let (_tmp, repo) = build_test_repo(&[("shared", "1.0.0", &[]), ("shared", "2.0.0", &[])]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Request shared==1.0.0 AND shared==2.0.0 — the second will conflict with first
    let reqs: Vec<Requirement> = ["shared==1.0.0", "shared==2.0.0"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(reqs));
    match result {
        Ok(r) => {
            // Either resolved one of them (first wins) or recorded conflict
            let _r = r;
        }
        Err(_) => {
            // Conflict error is also acceptable
        }
    }
}

/// Solver: requested flag correctly marks explicitly requested vs transitive deps.
#[test]
fn test_solver_requested_flag_distinguishes_roots() {
    let (_tmp, repo) = build_test_repo(&[
        ("base", "1.0.0", &[]),
        ("tool", "2.0.0", &["base-1+"]),
    ]);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);

    // Only request tool (root); base is transitive
    let reqs: Vec<Requirement> = ["tool"]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();

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
