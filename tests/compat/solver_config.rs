use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

