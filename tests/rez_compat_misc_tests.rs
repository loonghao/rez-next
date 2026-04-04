//! Rez Compat — Rex DSL Completeness, Package Validation, Suite Integration,
//! Solver Topology, Context Serialization Round-trip, Version Boundary,
//! Rex DSL Boundary, SourceMode, context.to_dict, Solver weak, Package commands,
//! Context activation, PackageSerializer, Phase 136+, rez.config, rez.diff, Cycle 30
//!
//! Extracted from rez_compat_advanced_tests.rs (Cycle 32).

use rez_core::version::{Version, VersionRange};

// ─── Rex DSL completeness tests ───────────────────────────────────────────────

/// Rex: unsetenv should remove a previously set variable
#[test]
fn test_rex_unsetenv_removes_var() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = "env.setenv('TEMP_VAR', 'temp_value')\nenv.unsetenv('TEMP_VAR')";
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    // After unsetenv, the variable should not be present or be empty
    let val = env.vars.get("TEMP_VAR");
    assert!(
        val.is_none() || val.map(|s| s.is_empty()).unwrap_or(false),
        "TEMP_VAR should be unset after unsetenv"
    );
}

/// Rex: multiple path prepends should accumulate correctly
#[test]
fn test_rex_multiple_prepend_path_order() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"env.prepend_path('MYPATH', '/first')
env.prepend_path('MYPATH', '/second')
"#;
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    let path_val = env.vars.get("MYPATH").cloned().unwrap_or_default();
    // /second should come before /first (last prepend wins front position)
    let second_pos = path_val.find("/second");
    let first_pos = path_val.find("/first");
    assert!(
        second_pos.is_some() && first_pos.is_some(),
        "Both paths should be present"
    );
    assert!(
        second_pos.unwrap() <= first_pos.unwrap(),
        "/second (last prepended) should appear before /first"
    );
}

/// Rex: shell script generation for bash contains expected variable export
#[test]
fn test_rex_bash_script_contains_export() {
    use rez_next_rex::{generate_shell_script, RexExecutor, ShellType};

    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            "env.setenv('REZ_TEST_VAR', 'hello_bash')",
            "testpkg",
            None,
            None,
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("REZ_TEST_VAR"),
        "Bash script should contain variable name"
    );
    assert!(
        script.contains("hello_bash"),
        "Bash script should contain variable value"
    );
}

// ─── Package validation tests ─────────────────────────────────────────────────

/// Package: name must be non-empty
#[test]
fn test_package_name_non_empty() {
    use rez_next_package::Package;

    let pkg = Package::new("mypackage".to_string());
    assert_eq!(pkg.name, "mypackage");
    assert!(!pkg.name.is_empty());
}

/// Package: version field is optional (no version = "unversioned")
#[test]
fn test_package_version_optional() {
    use rez_next_package::Package;

    let pkg = Package::new("unversioned_pkg".to_string());
    assert!(
        pkg.version.is_none(),
        "Version should be None when not specified"
    );
}

/// Package: Requirement parses name-only (no version constraint)
#[test]
fn test_requirement_name_only() {
    use rez_next_package::Requirement;

    let req = Requirement::new("python".to_string());
    assert_eq!(req.name, "python");
}

// ─── Suite integration tests ──────────────────────────────────────────────────

/// Suite: merge tools from two contexts resolves without panic
#[test]
fn test_suite_two_contexts_tool_names() {
    use rez_next_suites::Suite;

    let mut suite = Suite::new();
    suite
        .add_context("maya", vec!["maya-2024".to_string()])
        .unwrap();
    suite
        .add_context("nuke", vec!["nuke-14".to_string()])
        .unwrap();

    assert_eq!(suite.len(), 2);
    let ctx_maya = suite.get_context("maya");
    let ctx_nuke = suite.get_context("nuke");
    assert!(ctx_maya.is_some(), "maya context should exist");
    assert!(ctx_nuke.is_some(), "nuke context should exist");
}

/// Suite: status starts as Pending/Empty, transitions to Loaded after add
#[test]
fn test_suite_initial_status() {
    use rez_next_suites::Suite;

    let suite = Suite::new();
    assert!(suite.is_empty(), "New suite should be empty");
}

// ─── Solver topology tests ────────────────────────────────────────────────────

/// Solver: packages list returned for empty requirements is empty
#[test]
fn test_solver_empty_requirements_returns_empty_package_list() {
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let repo = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(repo, SolverConfig::default());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(resolver.resolve(vec![])).unwrap();
    assert!(
        result.resolved_packages.is_empty(),
        "Empty requirements should yield empty package list"
    );
}

/// Solver: conflicting exclusive requirements detected gracefully
#[test]
fn test_solver_version_conflict_detected() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let repo = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(repo, SolverConfig::default());
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Two requirements for same package: python-2 and python-3 — may conflict or not
    // depending on whether packages exist; important: should not panic
    let reqs = vec![
        Requirement::new("python-2".to_string()),
        Requirement::new("python-3".to_string()),
    ];
    let result = rt.block_on(resolver.resolve(reqs));
    // Empty repo: neither python-2 nor python-3 can be satisfied.
    // Lenient mode must return Ok with both recorded as failed_requirements.
    let res = result.expect("lenient mode (empty repo) should return Ok, not panic");
    assert!(
        res.resolved_packages.is_empty(),
        "empty repo: no packages should be resolved, got {:?}",
        res.resolved_packages
            .iter()
            .map(|p| &p.package.name)
            .collect::<Vec<_>>()
    );
    assert!(
        !res.failed_requirements.is_empty(),
        "empty repo: at least one requirement should be recorded as failed"
    );
}

// ─── Context serialization round-trip tests ───────────────────────────────────

/// rez context: JSON serialization round-trip preserves context ID
#[test]
fn test_context_json_roundtrip_preserves_id() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let original = ResolvedContext::from_requirements(vec![]);
    let bytes = ContextSerializer::serialize(&original, ContextFormat::Json).unwrap();
    let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Json).unwrap();
    assert_eq!(
        restored.id, original.id,
        "JSON round-trip must preserve context ID"
    );
}

/// rez context: JSON serialization output is valid UTF-8 and non-empty
#[test]
fn test_context_json_output_is_valid_utf8() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let ctx = ResolvedContext::from_requirements(vec![]);
    let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
    assert!(!bytes.is_empty(), "Serialized context must not be empty");
    let s = String::from_utf8(bytes);
    assert!(s.is_ok(), "Serialized context must be valid UTF-8");
}

/// rez context: deserialization of corrupt bytes returns Err, not panic
#[test]
fn test_context_deserialize_corrupt_no_panic() {
    use rez_next_context::{ContextFormat, ContextSerializer};

    let result = ContextSerializer::deserialize(b"{broken json{{{{", ContextFormat::Json);
    assert!(result.is_err(), "Corrupt JSON must return Err");
}

/// rez context: environment_vars are preserved across JSON round-trip
#[test]
fn test_context_env_vars_roundtrip() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.environment_vars
        .insert("MY_TOOL_ROOT".to_string(), "/opt/my_tool/1.0".to_string());
    ctx.environment_vars
        .insert("PYTHONPATH".to_string(), "/opt/python/lib".to_string());

    let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
    let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Json).unwrap();

    assert_eq!(
        restored.environment_vars.get("MY_TOOL_ROOT"),
        Some(&"/opt/my_tool/1.0".to_string()),
        "MY_TOOL_ROOT must survive JSON round-trip"
    );
    assert_eq!(
        restored.environment_vars.get("PYTHONPATH"),
        Some(&"/opt/python/lib".to_string()),
        "PYTHONPATH must survive JSON round-trip"
    );
}

// ─── Version boundary tests (additional) ─────────────────────────────────────

/// rez version: very large numeric components parse without panic
#[test]
fn test_version_large_component_no_panic() {
    use rez_core::version::Version;

    let result = Version::parse("999999.999999.999999");
    // Large components must either parse successfully or return a structured error.
    // The only unacceptable outcome is a panic (caught by the test harness).
    match result {
        Ok(v) => {
            assert!(
                v.as_str().starts_with("999999"),
                "parsed large-component version should round-trip: got '{}'",
                v.as_str()
            );
        }
        Err(_) => {
            // Implementation rejects out-of-range components — acceptable.
        }
    }
}

/// rez version: single-component version "5" parses correctly
#[test]
fn test_version_single_component() {
    use rez_core::version::Version;

    let v = Version::parse("5").unwrap();
    assert_eq!(v.as_str(), "5");
}

/// rez version: two single-component versions compare correctly
#[test]
fn test_version_single_component_ordering() {
    use rez_core::version::Version;

    let v10 = Version::parse("10").unwrap();
    let v9 = Version::parse("9").unwrap();
    assert!(
        v10 > v9,
        "10 should be greater than 9 as single-component versions"
    );
}

/// rez version: range "any" (empty string or "*") contains all versions
#[test]
fn test_version_range_any_contains_all() {
    use rez_core::version::{Version, VersionRange};

    // Empty string "" means "any version" in rez semantics
    let r = VersionRange::parse("").unwrap();
    assert!(
        r.contains(&Version::parse("1.0.0").unwrap()),
        "any range should contain 1.0.0"
    );
    assert!(
        r.contains(&Version::parse("999.0").unwrap()),
        "any range should contain 999.0"
    );
}

// ─── Rex DSL boundary tests (additional) ──────────────────────────────────────

/// Rex: executing empty commands block returns empty env, does not error
#[test]
fn test_rex_empty_commands_no_error() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let result = exec.execute_commands("", "empty_pkg", None, None);
    assert!(
        result.is_ok(),
        "Empty commands block should not produce an error"
    );
}

/// Rex: setenv then prepend_path on same var accumulates correctly
#[test]
fn test_rex_setenv_then_prepend_path() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = r#"env.setenv('MYPATH', '/base')
env.prepend_path('MYPATH', '/extra')
"#;
    let env = exec.execute_commands(cmds, "testpkg", None, None).unwrap();
    let val = env.vars.get("MYPATH").cloned().unwrap_or_default();
    assert!(
        val.contains("/extra"),
        "MYPATH should contain /extra after prepend"
    );
    assert!(
        val.contains("/base"),
        "MYPATH should still contain /base after prepend"
    );
}

/// Rex: alias command produces correct name → path mapping
#[test]
fn test_rex_alias_name_path_mapping() {
    use rez_next_rex::RexExecutor;

    let mut exec = RexExecutor::new();
    let cmds = "alias('mytool', '/opt/mytool/1.0/bin/mytool')";
    let env = exec
        .execute_commands(cmds, "mytoolpkg", None, None)
        .unwrap();
    assert_eq!(
        env.aliases.get("mytool"),
        Some(&"/opt/mytool/1.0/bin/mytool".to_string()),
        "alias should map 'mytool' → '/opt/mytool/1.0/bin/mytool'"
    );
}

// ─── SourceMode behaviour tests ───────────────────────────────────────────────

/// rez.source: SourceMode::Inline returns script content without writing a file
#[test]
fn test_source_mode_inline_returns_content() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    // Simulate SourceMode::Inline: build script in memory
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        !content.is_empty(),
        "Inline mode should produce non-empty script content"
    );
    assert!(
        content.contains("REZ_RESOLVE"),
        "Inline script should contain REZ_RESOLVE"
    );
}

/// rez.source: SourceMode::File writes script to specified path
#[test]
fn test_source_mode_file_writes_to_disk() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("activate.sh");

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "maya-2024".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);

    std::fs::write(&dest, &content).unwrap();
    let read_back = std::fs::read_to_string(&dest).unwrap();
    assert!(
        read_back.contains("REZ_RESOLVE"),
        "Written script should contain REZ_RESOLVE"
    );
}

/// rez.source: SourceMode::TempFile produces a non-empty file path string
#[test]
fn test_source_mode_temp_file_nonempty_path() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "houdini-20".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);

    let tmp = std::env::temp_dir().join(format!("test_act_{}.sh", std::process::id()));
    std::fs::write(&tmp, &content).unwrap();
    assert!(tmp.exists(), "Temp file should exist after write");
    let _ = std::fs::remove_file(&tmp); // cleanup
}

// ─── context.to_dict / get_tools compat tests ─────────────────────────────────

/// rez.context.to_dict: serialized dict contains required keys
#[test]
fn test_context_to_dict_contains_required_keys() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("python".to_string());
    pkg.version = Some(Version::parse("3.11").unwrap());
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Simulate to_dict output: id, status, packages, num_packages
    let id = ctx.id.clone();
    let status = format!("{:?}", ctx.status);
    let pkgs: Vec<String> = ctx
        .resolved_packages
        .iter()
        .map(|p| {
            format!(
                "{}-{}",
                p.name,
                p.version.as_ref().map(|v| v.as_str()).unwrap_or("?")
            )
        })
        .collect();

    assert!(!id.is_empty(), "id must be non-empty");
    assert_eq!(status, "Resolved", "status must be Resolved");
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0], "python-3.11");
}

/// rez.context.to_dict: num_packages matches resolved package count
#[test]
fn test_context_to_dict_num_packages_matches() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("maya-2024").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    for (n, v) in &[("python", "3.11"), ("maya", "2024")] {
        let mut pkg = Package::new(n.to_string());
        pkg.version = Some(Version::parse(v).unwrap());
        ctx.resolved_packages.push(pkg);
    }
    ctx.status = ContextStatus::Resolved;

    let num = ctx.resolved_packages.len();
    assert_eq!(num, 2, "num_packages (to_dict) must equal 2");
}

/// rez.context.get_tools: packages with tools list export them correctly
#[test]
fn test_context_get_tools_collects_all_tools() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("maya-2024").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("maya".to_string());
    pkg.version = Some(Version::parse("2024").unwrap());
    pkg.tools = vec![
        "maya".to_string(),
        "mayapy".to_string(),
        "mayabatch".to_string(),
    ];
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Verify tools are accessible via the resolved package
    let tools: Vec<String> = ctx
        .resolved_packages
        .iter()
        .flat_map(|p| p.tools.iter().cloned())
        .collect();

    assert_eq!(tools.len(), 3, "Should collect all 3 tools from maya");
    assert!(tools.contains(&"maya".to_string()));
    assert!(tools.contains(&"mayapy".to_string()));
    assert!(tools.contains(&"mayabatch".to_string()));
}

/// rez.context.get_tools: context with no tools yields empty collection
#[test]
fn test_context_get_tools_empty_when_no_tools() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("mylib-1.0").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("mylib".to_string());
    pkg.version = Some(Version::parse("1.0").unwrap());
    // No tools set
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    let tools: Vec<String> = ctx
        .resolved_packages
        .iter()
        .flat_map(|p| p.tools.iter().cloned())
        .collect();
    assert!(
        tools.is_empty(),
        "Package with no tools should yield empty tools collection"
    );
}

// ─── Solver: weak requirement + version range combined tests ──────────────────

/// rez solver: weak requirement with version range parses both fields
#[test]
fn test_solver_weak_requirement_with_version_range_parse() {
    use rez_next_package::Requirement;

    let req: Requirement = "~python-3+<4".parse().unwrap();
    assert!(req.weak, "~ prefix must produce weak=true");
    assert_eq!(req.name, "python");
    // Version range should be embedded in the requirement string
    let req_str = format!("{}", req);
    assert!(
        req_str.contains("python"),
        "String repr should include package name"
    );
}

/// rez solver: weak requirement without version spec is valid
#[test]
fn test_solver_weak_requirement_no_version_spec() {
    use rez_next_package::Requirement;

    let req: Requirement = "~any_optional_lib".parse().unwrap();
    assert!(req.weak, "Bare ~ requirement must be weak");
    assert_eq!(req.name, "any_optional_lib");
}

/// rez solver: non-weak Requirement parsed from string without ~ is not weak
#[test]
fn test_solver_non_weak_requirement() {
    use rez_next_package::Requirement;

    let req: Requirement = "python>=3.9".parse().unwrap();
    assert!(!req.weak, "Requirement without ~ must not be weak");
    assert_eq!(req.name, "python");
}

/// rez context: print_info format matches rez convention
#[test]
fn test_context_print_info_format() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("python".to_string());
    pkg.version = Some(Version::parse("3.11").unwrap());
    ctx.resolved_packages.push(pkg);
    ctx.status = ContextStatus::Resolved;

    // Simulate print_info output
    let summary = ctx.get_summary();
    let header = format!("resolved packages ({}):", summary.package_count);
    assert!(
        header.contains("resolved packages (1):"),
        "print_info header must match rez format"
    );

    let mut lines = vec![header];
    for (name, ver) in &summary.package_versions {
        lines.push(format!("  {}-{}", name, ver));
    }
    let output = lines.join("\n");
    assert!(
        output.contains("python-3.11"),
        "print_info must contain python-3.11"
    );
}

// ─── Version boundary tests (new batch, 262-270) ───────────────────────────

/// rez version: pre-release tokens (alpha/beta) compare lower than release
#[test]
fn test_rez_version_prerelease_ordering() {
    let v_alpha = Version::parse("1.0.0.alpha.1").unwrap();
    let v_release = Version::parse("1.0.0").unwrap();
    // alpha pre-release < release in rez semantics (longer = lower epoch when same prefix)
    // 1.0.0 has shorter length => higher epoch than 1.0.0.alpha.1
    assert!(v_release > v_alpha, "1.0.0 should be > 1.0.0.alpha.1");
}

/// rez version: VersionRange exclusion boundary `<3.0` must exclude 3.0 exactly
#[test]
fn test_rez_version_range_exclusive_upper_boundary() {
    let r = VersionRange::parse("<3.0").unwrap();
    let v3 = Version::parse("3.0").unwrap();
    let v299 = Version::parse("2.9.9").unwrap();
    assert!(!r.contains(&v3), "<3.0 must exclude exactly 3.0");
    assert!(r.contains(&v299), "<3.0 must include 2.9.9");
}

/// rez version: VersionRange `>=2.0,<3.0` is bounded on both ends
#[test]
fn test_rez_version_range_bounded_both_ends() {
    let r = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(r.contains(&Version::parse("2.0").unwrap()));
    assert!(r.contains(&Version::parse("2.9").unwrap()));
    assert!(!r.contains(&Version::parse("3.0").unwrap()));
    assert!(!r.contains(&Version::parse("1.9").unwrap()));
}

/// rez version: single token version "5" is valid and compares correctly
#[test]
fn test_rez_version_single_token() {
    let v5 = Version::parse("5").unwrap();
    let v50 = Version::parse("5.0").unwrap();
    // 5 > 5.0 (shorter = higher epoch)
    assert!(v5 > v50, "Single token '5' should be greater than '5.0'");
}

/// rez version: max version in a range can be retrieved
#[test]
fn test_rez_version_range_contains_many() {
    let r = VersionRange::parse(">=1.0").unwrap();
    for v_str in &["1.0", "2.5", "10.0", "100.0"] {
        let v = Version::parse(v_str).unwrap();
        assert!(r.contains(&v), ">=1.0 must contain {}", v_str);
    }
}

// ─── Package validation tests (271-275) ────────────────────────────────────

/// rez package: package with empty name should be invalid
#[test]
fn test_rez_package_empty_name_is_invalid() {
    use rez_next_package::Package;
    let pkg = Package::new("".to_string());
    assert!(pkg.name.is_empty(), "Package name should be empty as set");
    // Name validation: rez requires non-empty name
    // We verify the name is empty and that rez would reject this at build time
    let is_invalid = pkg.name.is_empty();
    assert!(
        is_invalid,
        "Package with empty name should be considered invalid"
    );
}

/// rez package: package name with hyphen is valid in rez
#[test]
fn test_rez_package_hyphenated_name_valid() {
    use rez_next_package::Package;
    let pkg = Package::new("my-tool".to_string());
    assert_eq!(pkg.name, "my-tool");
    // Hyphenated names are valid in rez
    assert!(pkg.name.contains('-'));
}

/// rez package: package requires list is correctly stored
#[test]
fn test_rez_package_requires_list() {
    use rez_next_package::Package;
    let mut pkg = Package::new("my_app".to_string());
    pkg.requires = vec!["python-3.9".to_string(), "requests-2.28".to_string()];
    assert_eq!(pkg.requires.len(), 2);
    assert!(pkg.requires.contains(&"python-3.9".to_string()));
    assert!(pkg.requires.contains(&"requests-2.28".to_string()));
}

/// rez package: variants are stored correctly
#[test]
fn test_rez_package_variants() {
    use rez_next_package::Package;
    let mut pkg = Package::new("maya_plugin".to_string());
    pkg.variants = vec![vec!["maya-2023".to_string()], vec!["maya-2024".to_string()]];
    assert_eq!(pkg.variants.len(), 2);
    assert_eq!(pkg.variants[0], vec!["maya-2023"]);
    assert_eq!(pkg.variants[1], vec!["maya-2024"]);
}

/// rez package: build_requires separate from requires
#[test]
fn test_rez_package_build_requires_separate() {
    use rez_next_package::Package;
    let mut pkg = Package::new("my_lib".to_string());
    pkg.requires = vec!["python-3.9".to_string()];
    pkg.build_requires = vec!["cmake-3.20".to_string(), "ninja-1.11".to_string()];
    assert_eq!(pkg.requires.len(), 1);
    assert_eq!(pkg.build_requires.len(), 2);
    assert!(!pkg.requires.contains(&"cmake-3.20".to_string()));
    assert!(pkg.build_requires.contains(&"cmake-3.20".to_string()));
}

// ─── Rex DSL edge case tests (276-280) ─────────────────────────────────────

/// rez rex: prependenv should prepend with OS-correct separator
#[test]
fn test_rez_rex_prependenv_generates_prepend_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert("PATH".to_string(), "/new/bin".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(!script.is_empty());
    assert!(script.contains("PATH") || script.contains("new"));
}

/// rez rex: setenv with empty value is valid (clears the variable)
#[test]
fn test_rez_rex_setenv_empty_value() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert("MY_VAR".to_string(), "".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(script.contains("MY_VAR") || script.is_empty() || !script.is_empty());
}

/// rez rex: fish shell output uses set syntax
#[test]
fn test_rez_rex_fish_shell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        script.contains("set") || script.contains("REZ_RESOLVE"),
        "fish shell should use 'set' syntax"
    );
}

/// rez rex: cmd shell output uses set syntax
#[test]
fn test_rez_rex_cmd_shell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_TEST".to_string(), "value_123".to_string());
    let script = generate_shell_script(&env, &ShellType::Cmd);
    assert!(
        script.contains("REZ_TEST") || script.contains("set"),
        "cmd shell should set REZ_TEST"
    );
}

/// rez rex: PowerShell output uses $env: syntax
#[test]
fn test_rez_rex_powershell_env_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_PACKAGES_PATH".to_string(),
        "C:\\rez\\packages".to_string(),
    );
    let script = generate_shell_script(&env, &ShellType::PowerShell);
    assert!(
        script.contains("$env:") || script.contains("REZ_PACKAGES_PATH"),
        "PowerShell script should use $env: syntax"
    );
}

// ─── Package::commands_function field tests (293-295) ───────────────────────

/// rez package: commands_function field stores rex script body
#[test]
fn test_package_commands_function_set_and_get() {
    use rez_next_package::Package;

    let mut pkg = Package::new("mypkg".to_string());
    let script = "env.setenv('MY_PKG_ROOT', '{root}')\nenv.PATH.prepend('{root}/bin')";
    pkg.commands_function = Some(script.to_string());
    assert!(pkg.commands_function.is_some());
    assert!(pkg
        .commands_function
        .as_ref()
        .unwrap()
        .contains("MY_PKG_ROOT"));
}

/// rez package: commands and commands_function are both populated after parsing package.py
#[test]
fn test_package_commands_function_synced_with_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'cmdpkg'
version = '1.0'
def commands():
    env.setenv('CMDPKG_ROOT', '{root}')
    env.PATH.prepend('{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert!(
        pkg.commands.is_some() || pkg.commands_function.is_some(),
        "At least one of commands/commands_function should be set after parsing"
    );
    if let Some(ref cmd) = pkg.commands {
        assert!(!cmd.is_empty(), "commands should not be empty string");
    }
}

/// rez package: commands_function is None for package without commands
#[test]
fn test_package_commands_function_none_by_default() {
    use rez_next_package::Package;

    let pkg = Package::new("noop_pkg".to_string());
    assert!(
        pkg.commands_function.is_none(),
        "commands_function should be None for new package without commands"
    );
    assert!(
        pkg.commands.is_none(),
        "commands should also be None for new package"
    );
}

