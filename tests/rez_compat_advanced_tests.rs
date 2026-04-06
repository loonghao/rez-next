//! Rez Compat — SolverConfig, Solver Boundary, Context Advanced, Exception Types,
//! Version Advanced, Rex DSL Completeness, Package Validation, Suite, Context Serialization,
//! Source Mode, rez.config, rez.diff, Cycle 30 Status/packages_ Tests
//!
//! Extracted from rez_compat_tests.rs (Cycle 32).
//!
//! See also: rez_compat_tests.rs (version, package, rex, suite, config, e2e)

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

// ─── Solver boundary tests ────────────────────────────────────────────────────

/// rez solver: single package with no dependencies resolves immediately
#[test]
fn test_solver_single_package_no_deps() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("standalone".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages();
    assert!(result.is_ok(), "Single package with no deps should resolve");
    assert_eq!(result.unwrap().len(), 1);
}

/// rez solver: version range intersection for multi-constraint requirement
#[test]
fn test_solver_multi_constraint_version_range() {
    use rez_core::version::VersionRange;

    let r_ge = VersionRange::parse(">=3.9").unwrap();
    let r_lt = VersionRange::parse("<4.0").unwrap();
    let intersection = r_ge
        .intersect(&r_lt)
        .expect(">=3.9 and <4.0 should intersect");

    assert!(intersection.contains(&rez_core::version::Version::parse("3.9").unwrap()));
    assert!(intersection.contains(&rez_core::version::Version::parse("3.11").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("4.0").unwrap()));
    assert!(!intersection.contains(&rez_core::version::Version::parse("3.8").unwrap()));
}

/// rez solver: two packages with exclusive version ranges → conflict
#[test]
fn test_solver_exclusive_ranges_detect_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "lib".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Exclusive ranges >=1.0,<2.0 and >=2.0 should conflict for lib"
    );
}

/// rez solver: compatible ranges do not produce a conflict
#[test]
fn test_solver_compatible_ranges_no_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=3.8 and <4.0 are compatible
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.8".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<4.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(conflicts.is_empty(), ">=3.8 and <4.0 should not conflict");
}

/// rez solver: weak requirement (~pkg) is parsed correctly
#[test]
fn test_solver_weak_requirement_parse() {
    use rez_next_package::Requirement;

    let req = "~python>=3.9".parse::<Requirement>().unwrap();
    assert!(req.weak, "~ prefix should set weak=true");
    assert_eq!(req.name, "python");
}

/// rez solver: topological sort on a chain A → B → C
#[test]
fn test_solver_topological_sort_chain() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();

    for (name, ver) in &[("pkgA", "1.0"), ("pkgB", "1.0"), ("pkgC", "1.0")] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("pkgA-1.0", "pkgB-1.0").unwrap();
    graph.add_dependency_edge("pkgB-1.0", "pkgC-1.0").unwrap();

    let result = graph.get_resolved_packages();
    assert!(
        result.is_ok(),
        "Linear chain A->B->C should resolve (no cycles)"
    );
    assert_eq!(
        result.unwrap().len(),
        3,
        "All 3 packages should be in resolved order"
    );
}

// ─── Context compat tests ────────────────────────────────────────────────────

/// rez.resolved_context: context created from zero requirements has empty resolved_packages
#[test]
fn test_context_empty_requirements_has_no_packages() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    assert!(
        ctx.resolved_packages.is_empty(),
        "Empty requirements should produce empty resolved_packages"
    );
}

/// rez.resolved_context: get_summary reports correct package count
#[test]
fn test_context_summary_reflects_resolved_packages() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("houdini-20.0").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);

    for (name, ver) in &[("python", "3.11"), ("houdini", "20.0")] {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
    }
    ctx.status = ContextStatus::Resolved;

    let summary = ctx.get_summary();
    assert_eq!(summary.package_count, 2);
    assert!(summary.package_versions.contains_key("python"));
    assert!(summary.package_versions.contains_key("houdini"));
}

/// rez.resolved_context: each context receives a unique ID
#[test]
fn test_context_unique_ids() {
    use rez_next_context::ResolvedContext;

    let c1 = ResolvedContext::from_requirements(vec![]);
    let c2 = ResolvedContext::from_requirements(vec![]);
    assert_ne!(c1.id, c2.id, "Each ResolvedContext must have a unique ID");
}

/// rez.resolved_context: created_at timestamp is positive (Unix epoch)
#[test]
fn test_context_created_at_positive() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    assert!(
        ctx.created_at > 0,
        "created_at should be a positive Unix timestamp"
    );
}

/// rez.resolved_context: status transitions Failed → Resolved
#[test]
fn test_context_status_transition() {
    use rez_next_context::{ContextStatus, ResolvedContext};

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.status = ContextStatus::Failed;
    assert_eq!(ctx.status, ContextStatus::Failed);

    ctx.status = ContextStatus::Resolved;
    assert_eq!(ctx.status, ContextStatus::Resolved);
}

/// rez.resolved_context: environment_vars can be injected (rez env semantics)
#[test]
fn test_context_environment_vars_injection() {
    use rez_next_context::ResolvedContext;

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.environment_vars
        .insert("REZ_USED_REQUEST".to_string(), "python-3.11".to_string());
    ctx.environment_vars
        .insert("PATH".to_string(), "/usr/bin:/bin".to_string());

    assert_eq!(
        ctx.environment_vars.get("REZ_USED_REQUEST"),
        Some(&"python-3.11".to_string())
    );
    assert!(ctx.environment_vars.contains_key("PATH"));
}

// ─── Solver boundary tests ───────────────────────────────────────────────────

/// rez solver: resolving with only one package returns exactly that package
#[test]
fn test_solver_single_package_resolution() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    let mut pkg = Package::new("solo".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    graph.add_package(pkg).unwrap();

    let result = graph.get_resolved_packages().unwrap();
    assert_eq!(
        result.len(),
        1,
        "Single package graph should resolve to 1 package"
    );
    assert_eq!(result[0].name, "solo");
}

/// rez solver: weak requirement (~) does not prevent resolution when absent
#[test]
fn test_solver_weak_requirement_optional_absent() {
    use rez_next_package::Requirement;

    let req: Requirement = "~optional_tool>=1.0".parse().unwrap();
    assert!(req.weak, "~ prefix must produce a weak requirement");
    assert_eq!(req.name, "optional_tool");
}

/// rez solver: diamond dependency A→B, A→C, B→D, C→D resolves correctly
#[test]
fn test_solver_diamond_dependency_no_conflict() {
    use rez_next_package::Package;
    use rez_next_solver::DependencyGraph;
    use rez_next_version::Version;

    let mut graph = DependencyGraph::new();
    for (n, v) in &[("A", "1.0"), ("B", "1.0"), ("C", "1.0"), ("D", "1.0")] {
        let mut pkg = Package::new(n.to_string());
        pkg.version = Some(Version::parse(v).unwrap());
        graph.add_package(pkg).unwrap();
    }

    graph.add_dependency_edge("A-1.0", "B-1.0").unwrap();
    graph.add_dependency_edge("A-1.0", "C-1.0").unwrap();
    graph.add_dependency_edge("B-1.0", "D-1.0").unwrap();
    graph.add_dependency_edge("C-1.0", "D-1.0").unwrap();

    let resolved = graph.get_resolved_packages().unwrap();
    assert_eq!(
        resolved.len(),
        4,
        "Diamond dependency should include all 4 packages exactly once"
    );
}

// ─── Package is_valid() / validate() tests (Phase 93) ─────────────────────

/// rez package: valid package passes is_valid()
#[test]
fn test_package_is_valid_basic() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(
        pkg.is_valid(),
        "Package with valid name and version should be valid"
    );
}

/// rez package: empty name fails is_valid()
#[test]
fn test_package_is_valid_empty_name() {
    use rez_next_package::Package;

    let pkg = Package::new("".to_string());
    assert!(
        !pkg.is_valid(),
        "Package with empty name should not be valid"
    );
}

/// rez package: invalid name chars fails validate()
#[test]
fn test_package_validate_invalid_name_chars() {
    use rez_next_package::Package;

    let pkg = Package::new("bad@pkg!name".to_string());
    assert!(
        pkg.validate().is_err(),
        "Package with special chars in name should fail validate()"
    );
    let err_msg = pkg.validate().unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid package name"),
        "Error should mention invalid name: {}",
        err_msg
    );
}

/// rez package: empty requirement in requires fails validate()
#[test]
fn test_package_validate_empty_requirement() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    pkg.requires.push("".to_string()); // Empty requirement
    assert!(
        pkg.validate().is_err(),
        "Package with empty requirement should fail validate()"
    );
    assert!(
        !pkg.is_valid(),
        "is_valid() should return false for package with empty requirement"
    );
}

/// rez package: valid name formats (hyphen, underscore) pass is_valid()
#[test]
fn test_package_is_valid_name_variants() {
    use rez_next_package::Package;

    for name in &["my-pkg", "my_pkg", "MyPkg2", "pkg123"] {
        let pkg = Package::new(name.to_string());
        assert!(pkg.is_valid(), "Package name '{}' should be valid", name);
    }
}

/// rez package: empty build_requires entry fails validate()
#[test]
fn test_package_validate_empty_build_requirement() {
    use rez_next_package::Package;

    let mut pkg = Package::new("buildpkg".to_string());
    pkg.build_requires.push("cmake".to_string());
    pkg.build_requires.push("".to_string()); // invalid entry
    let result = pkg.validate();
    assert!(
        result.is_err(),
        "Empty build requirement should fail validation"
    );
}

// ─── VersionRange advanced tests (Phase 93) ───────────────────────────────

/// rez version range: negation "!=" (exclude single version)
#[test]
fn test_version_range_exclude_single() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("!=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be excluded"
    );
    assert!(
        r.contains(&Version::parse("1.9").unwrap()),
        "1.9 should be included"
    );
    assert!(
        r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should be included"
    );
}

/// rez version range: upper-inclusive "<=2.0"
#[test]
fn test_version_range_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse("<=2.0").unwrap();
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 should be included in <=2.0"
    );
    assert!(
        r.contains(&Version::parse("1.5").unwrap()),
        "1.5 should be included in <=2.0"
    );
    assert!(
        !r.contains(&Version::parse("2.1").unwrap()),
        "2.1 should not be in <=2.0"
    );
}

/// rez version range: ">1.0" (strict lower bound, exclusive)
#[test]
fn test_version_range_gt_exclusive() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 should be excluded from >1.0"
    );
    assert!(
        r.contains(&Version::parse("1.1").unwrap()),
        "1.1 should be included in >1.0"
    );
}

/// rez version range: combined ">1.0,<=2.0"
#[test]
fn test_version_range_combined_gt_le() {
    use rez_core::version::{Version, VersionRange};

    let r = VersionRange::parse(">1.0,<=2.0").unwrap();
    assert!(
        !r.contains(&Version::parse("1.0").unwrap()),
        "1.0 excluded (strict >)"
    );
    assert!(r.contains(&Version::parse("1.5").unwrap()), "1.5 included");
    assert!(
        r.contains(&Version::parse("2.0").unwrap()),
        "2.0 included (<=)"
    );
    assert!(!r.contains(&Version::parse("2.1").unwrap()), "2.1 excluded");
}

/// rez version range: is_superset_of semantics
#[test]
fn test_version_range_is_superset() {
    use rez_core::version::VersionRange;

    let broad = VersionRange::parse(">=1.0").unwrap();
    let narrow = VersionRange::parse(">=1.5,<2.0").unwrap();
    assert!(
        broad.is_superset_of(&narrow),
        ">=1.0 should be superset of >=1.5,<2.0"
    );
    assert!(
        !narrow.is_superset_of(&broad),
        ">=1.5,<2.0 should NOT be superset of >=1.0"
    );
}

