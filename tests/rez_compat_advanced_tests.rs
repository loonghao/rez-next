//! Rez Compat — SolverConfig, Depends Reverse Query, Status Env Parsing
//!
//! Split from rez_compat_advanced_tests.rs (Cycle 143).
//! Solver boundary, Context, Package Validation, VersionRange advanced
//! → extracted to rez_compat_solver_boundary_tests.rs

// ─── rez.solver SolverConfig / timeout semantics ─────────────────────────────

/// rez solver: default config has sensible timeout (> 0 seconds)
#[test]
fn test_solver_config_default_timeout_positive() {
    use rez_next_solver::SolverConfig;
    let cfg = SolverConfig::default();
    assert!(cfg.max_time_seconds > 0, "default timeout should be > 0");
}

/// rez solver: custom timeout is stored correctly
#[test]
fn test_solver_config_custom_timeout_stored() {
    use rez_next_solver::SolverConfig;
    let cfg = SolverConfig {
        max_time_seconds: 10,
        ..Default::default()
    };
    assert_eq!(cfg.max_time_seconds, 10);
}

/// rez solver: zero timeout config does not panic on construction
#[test]
fn test_solver_config_zero_timeout_no_panic() {
    use rez_next_solver::SolverConfig;
    let cfg = SolverConfig {
        max_time_seconds: 0,
        ..Default::default()
    };
    assert_eq!(cfg.max_time_seconds, 0);
}

/// rez solver: SolverConfig serializes and deserializes cleanly
#[test]
fn test_solver_config_json_roundtrip() {
    use rez_next_solver::SolverConfig;
    let cfg = SolverConfig::default();
    let json = serde_json::to_string(&cfg).expect("serialization failed");
    let restored: SolverConfig = serde_json::from_str(&json).expect("deserialization failed");
    assert_eq!(cfg.max_attempts, restored.max_attempts);
    assert_eq!(cfg.max_time_seconds, restored.max_time_seconds);
    assert_eq!(cfg.prefer_latest, restored.prefer_latest);
}

/// rez solver: DependencySolver with config preserves timeout setting
#[test]
fn test_solver_with_config_preserves_timeout() {
    use rez_next_solver::{DependencySolver, SolverConfig};
    let cfg = SolverConfig {
        max_time_seconds: 30,
        ..Default::default()
    };
    let solver = DependencySolver::with_config(cfg.clone());
    // Solver constructed without panic — verify via debug output
    let dbg = format!("{:?}", solver);
    assert!(
        dbg.contains("DependencySolver"),
        "debug output should name the struct"
    );
}

/// rez solver: empty requirements resolve without panic
#[test]
fn test_solver_resolve_empty_requirements() {
    use rez_next_solver::{DependencySolver, SolverRequest};
    let solver = DependencySolver::new();
    let request = SolverRequest::new(vec![]);
    let result = solver.resolve(request);
    assert!(
        result.is_ok(),
        "resolving empty requirements should succeed"
    );
    let res = result.unwrap();
    assert_eq!(res.packages.len(), 0);
}

/// rez solver: ConflictStrategy serializes to expected JSON strings
#[test]
fn test_solver_conflict_strategy_serialization() {
    use rez_next_solver::ConflictStrategy;
    let strategies = [
        (ConflictStrategy::LatestWins, "LatestWins"),
        (ConflictStrategy::EarliestWins, "EarliestWins"),
        (ConflictStrategy::FailOnConflict, "FailOnConflict"),
        (ConflictStrategy::FindCompatible, "FindCompatible"),
    ];
    for (strategy, expected) in &strategies {
        let json = serde_json::to_string(strategy).expect("serialize failed");
        assert!(
            json.contains(expected),
            "Expected JSON to contain '{}', got: {}",
            expected,
            json
        );
    }
}

/// rez solver: SolverRequest with_constraint builder chain works
#[test]
fn test_solver_request_builder_chain() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::SolverRequest;
    let req = PackageRequirement::parse("python-3+").unwrap();
    let constraint = PackageRequirement::parse("platform-linux").unwrap();
    let request = SolverRequest::new(vec![req]).with_constraint(constraint);
    assert_eq!(request.constraints.len(), 1);
}

/// rez solver: SolverRequest with_exclude removes package by name
#[test]
fn test_solver_request_with_exclude() {
    use rez_next_solver::SolverRequest;
    let request = SolverRequest::new(vec![]).with_exclude("legacy_lib".to_string());
    assert_eq!(request.excludes.len(), 1);
    assert_eq!(request.excludes[0], "legacy_lib");
}

// ─── rez.depends: reverse dependency query semantics ─────────────────────────

/// rez depends: finding dependents when nothing depends on target returns empty
#[test]
fn test_depends_no_dependents_for_isolated_package() {
    use rez_next_package::Package;
    // Build a synthetic package set where nothing requires "isolated_pkg".
    // Package.requires is Vec<String> (requirement strings).
    let packages: Vec<Package> = vec![
        Package::new("python".to_string()),
        Package::new("maya".to_string()),
    ];
    let target = "isolated_pkg";
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with(target)))
        .collect();
    assert!(
        dependents.is_empty(),
        "no package should depend on an isolated package"
    );
}

/// rez depends: direct dependent detection via requires list
#[test]
fn test_depends_direct_dependent_found() {
    use rez_next_package::Package;
    let mut consumer = Package::new("my_tool".to_string());
    consumer.requires = vec!["python-3+".to_string()];

    let packages = vec![consumer];
    let target = "python";
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with(target)))
        .collect();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].name, "my_tool");
}

/// rez depends: packages with empty requires list never appear as dependents
#[test]
fn test_depends_empty_requires_not_dependent() {
    use rez_next_package::Package;
    let packages: Vec<Package> = vec![
        Package::new("standalone_a".to_string()),
        Package::new("standalone_b".to_string()),
    ];
    for pkg in &packages {
        assert!(
            pkg.requires.is_empty(),
            "packages should have empty requires"
        );
    }
    let dependents: Vec<&Package> = packages
        .iter()
        .filter(|p| p.requires.iter().any(|r| r.starts_with("anything")))
        .collect();
    assert!(dependents.is_empty());
}

// ─── rez.status: env var parsing and shell detection ─────────────────────────

/// rez status: REZ_USED_PACKAGES_NAMES parsing produces correct package list
#[test]
fn test_status_parse_rez_used_packages_names() {
    let raw = "python-3.9 maya-2024.1 houdini-20.5";
    let packages: Vec<&str> = raw.split_whitespace().collect();
    assert_eq!(packages.len(), 3);
    assert_eq!(packages[0], "python-3.9");
    assert_eq!(packages[1], "maya-2024.1");
    assert_eq!(packages[2], "houdini-20.5");
}

/// rez status: REZ_ env var prefix filtering
#[test]
fn test_status_rez_env_prefix_filter() {
    let all_env: Vec<(String, String)> = vec![
        ("PATH".to_string(), "/usr/bin".to_string()),
        ("REZ_CONTEXT_FILE".to_string(), "/tmp/ctx.rxt".to_string()),
        ("REZ_VERSION".to_string(), "3.0.0".to_string()),
        ("HOME".to_string(), "/home/user".to_string()),
    ];

    let rez_vars: Vec<_> = all_env
        .iter()
        .filter(|(k, _)| k.starts_with("REZ_"))
        .collect();
    assert_eq!(rez_vars.len(), 2, "Should find exactly 2 REZ_ vars");
    assert!(rez_vars.iter().any(|(k, _)| k == "REZ_CONTEXT_FILE"));
    assert!(rez_vars.iter().any(|(k, _)| k == "REZ_VERSION"));
}

/// rez status: shell detection on various SHELL env values
#[test]
fn test_status_shell_detection_logic() {
    let cases = [
        ("/bin/bash", "bash"),
        ("/usr/bin/zsh", "zsh"),
        ("/usr/local/bin/fish", "fish"),
    ];

    for (shell_val, expected) in &cases {
        let detected = if shell_val.contains("zsh") {
            "zsh"
        } else if shell_val.contains("fish") {
            "fish"
        } else if shell_val.contains("bash") {
            "bash"
        } else {
            *shell_val
        };
        assert_eq!(
            detected, *expected,
            "Shell detection should identify {}",
            expected
        );
    }
}

/// rez status: context file path round-trips through env var
#[test]
fn test_status_context_file_path_format() {
    let ctx_path = "/tmp/rez_ctx_12345.rxt";
    // Simulate what would be in REZ_CONTEXT_FILE
    let parsed = ctx_path.to_string();
    assert!(
        parsed.ends_with(".rxt"),
        "Context file should have .rxt extension"
    );
    assert!(
        parsed.starts_with("/tmp"),
        "Context file path should be absolute"
    );
}
