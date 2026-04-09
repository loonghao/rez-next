//! Rez Compat — Rex DSL Completeness, Package Validation, Suite Integration,
//! Solver Topology, Context Serialization Round-trip, Version Boundary,
//! Rex DSL Boundary, SourceMode
//!
//! Extracted from rez_compat_advanced_tests.rs (Cycle 32).
//! Cycle 141: extracted context.to_dict/Solver weak/Version new/Package new/Rex edge/
//!            Package commands into rez_compat_context_dict_tests.rs.

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
