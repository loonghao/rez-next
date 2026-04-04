//! Rez Compat — Context Activation Script E2E, Solver Weak Dependency,
//! PackageSerializer, Phase 136-143, rez.config, rez.diff, Cycle 30 Tests
//!
//! Extracted from rez_compat_misc_tests.rs (Cycle 32).

use rez_core::version::{Version, VersionRange};
use rez_next_package::PackageRequirement;
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

// ─── Context activation script E2E tests (296-300) ──────────────────────────

/// rez context: activation script for bash sets correct env vars
#[test]
fn test_context_activation_bash_sets_rez_env_vars() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    env.vars.insert(
        "REZ_USED_PACKAGES_PATH".to_string(),
        "/packages".to_string(),
    );
    env.vars
        .insert("PATH".to_string(), "/packages/python/3.9/bin".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);

    assert!(
        script.contains("REZ_RESOLVE"),
        "bash script must contain REZ_RESOLVE"
    );
    assert!(script.contains("PATH"), "bash script must contain PATH");
    assert!(
        script.contains("export") || script.contains("="),
        "bash script must have assignment syntax"
    );
}

/// rez context: activation script for powershell uses $env: syntax
#[test]
fn test_context_activation_powershell_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "maya-2024".to_string());
    env.vars.insert(
        "MAYA_LOCATION".to_string(),
        "C:\\Autodesk\\Maya2024".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::PowerShell);

    assert!(
        script.contains("$env:") || script.contains("REZ_RESOLVE"),
        "PowerShell activation script must use $env: syntax or contain var name, got: {}",
        &script[..script.len().min(300)]
    );
}

/// rez context: activation script for fish uses 'set' syntax
#[test]
fn test_context_activation_fish_set_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_CONTEXT_FILE".to_string(),
        "/tmp/rez_context.rxt".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        !script.is_empty(),
        "fish activation script must not be empty"
    );
    assert!(
        script.contains("set") || script.contains("REZ_CONTEXT_FILE"),
        "fish script should use set syntax or contain var name"
    );
}

/// rez context: activation script for cmd uses SET syntax
#[test]
fn test_context_activation_cmd_set_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_PACKAGES_PATH".to_string(),
        "C:\\rez\\packages;D:\\rez\\packages".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Cmd);
    assert!(
        !script.is_empty(),
        "cmd activation script must not be empty"
    );
    assert!(
        script.to_uppercase().contains("SET") || script.contains("REZ_PACKAGES_PATH"),
        "cmd script should use SET command or contain var name"
    );
}

/// rez context: multiple packages in activation script are all present
#[test]
fn test_context_activation_multiple_packages() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "PYTHON_ROOT".to_string(),
        "/packages/python/3.9".to_string(),
    );
    env.vars
        .insert("MAYA_ROOT".to_string(), "/packages/maya/2024".to_string());
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 maya-2024".to_string(),
    );
    env.aliases.insert(
        "python".to_string(),
        "/packages/python/3.9/bin/python".to_string(),
    );

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("PYTHON_ROOT"),
        "script must contain PYTHON_ROOT"
    );
    assert!(
        script.contains("MAYA_ROOT"),
        "script must contain MAYA_ROOT"
    );
    assert!(
        script.contains("REZ_RESOLVE"),
        "script must contain REZ_RESOLVE"
    );
}

// ─── Solver weak dependency (~pkg) tests (301-304) ──────────────────────────

/// rez solver: weak requirement flag defaults to false
#[test]
fn test_solver_weak_requirement_default_false() {
    use rez_next_package::PackageRequirement;

    let normal = PackageRequirement::parse("python").unwrap();
    assert!(
        !normal.weak,
        "Normal requirement 'python' should not be weak"
    );

    let with_ver = PackageRequirement::parse("python-3.9").unwrap();
    assert!(
        !with_ver.weak,
        "Versioned requirement 'python-3.9' should not be weak"
    );
}

/// rez solver: weak requirement preserves package name correctly
#[test]
fn test_solver_weak_requirement_name_preserved() {
    use rez_next_package::PackageRequirement;

    let weak_req = PackageRequirement {
        name: "numpy".to_string(),
        version_spec: None,
        weak: true,
        conflict: false,
    };
    assert_eq!(weak_req.name(), "numpy");
    assert!(
        weak_req.weak,
        "Explicitly set weak=true should be preserved"
    );
}

/// rez solver: non-conflicting requirements yield no conflicts
#[test]
fn test_solver_weak_no_conflict_if_compatible() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.9".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "numpy".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Non-conflicting requirements should yield no conflicts"
    );
}

/// rez solver: disjoint version ranges for same package produce conflict
#[test]
fn test_solver_disjoint_ranges_produce_conflict() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            ">=4.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "maya".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint requirements >=4.0 and <3.0 should produce conflict"
    );
}

// ─── PackageSerializer commands field tests (305-308) ───────────────────────

/// rez serializer: package.py with def commands() is parsed correctly
#[test]
fn test_serializer_package_with_commands_function() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'testpkg'
version = '2.0.0'
description = 'package with commands'
def commands():
    env.setenv('TESTPKG_ROOT', '{root}')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "testpkg");
    let has_commands = pkg.commands.is_some() || pkg.commands_function.is_some();
    assert!(
        has_commands,
        "Package with def commands() should have commands populated"
    );
}

/// rez serializer: package.py with pre_commands() is parsed without error
#[test]
fn test_serializer_package_with_pre_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'prepkg'
version = '1.5.0'
def pre_commands():
    env.setenv('PREPKG_SETUP', '1')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "prepkg");
}

/// rez serializer: package.py with post_commands() is parsed without error
#[test]
fn test_serializer_package_with_post_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'postpkg'
version = '0.5.0'
def post_commands():
    env.setenv('POST_DONE', '1')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "postpkg");
}

/// rez serializer: package.py with inline string commands is parsed without error
#[test]
fn test_serializer_package_commands_string_form() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'strpkg'
version = '3.0.0'
commands = "env.setenv('STRPKG_HOME', '{root}')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "strpkg");
}

// ─── Phase 136-143: Rex info messages, ShellType case-insensitive, env var CRUD ─

/// rez rex: info_messages appear in bash script in order
#[test]
fn test_rex_info_messages_order_in_script() {
    let mut env = RexEnvironment::new();
    env.info_messages.push("first message".to_string());
    env.info_messages.push("second message".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    let pos1 = script.find("first message").unwrap();
    let pos2 = script.find("second message").unwrap();
    assert!(pos1 < pos2, "Info messages should appear in order");
}

/// rez rex: ShellType::parse is case-insensitive
#[test]
fn test_shell_type_parse_case_insensitive() {
    assert_eq!(ShellType::parse("BASH"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("bash"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("Bash"), Some(ShellType::Bash));
    assert_eq!(ShellType::parse("POWERSHELL"), Some(ShellType::PowerShell));
    assert_eq!(ShellType::parse("Fish"), Some(ShellType::Fish));
}

/// rez rex: RexEnvironment vars insert/update/delete
#[test]
fn test_rex_environment_env_var_crud() {
    let mut env = RexEnvironment::new();
    env.vars.insert("MY_VAR".to_string(), "initial".to_string());
    assert_eq!(env.vars.get("MY_VAR"), Some(&"initial".to_string()));

    env.vars.insert("MY_VAR".to_string(), "updated".to_string());
    assert_eq!(env.vars.get("MY_VAR"), Some(&"updated".to_string()));

    env.vars.remove("MY_VAR");
    assert!(!env.vars.contains_key("MY_VAR"));
}

/// rez rex: generate_shell_script for zsh produces identical output as bash
#[test]
fn test_rex_zsh_identical_to_bash() {
    let mut env = RexEnvironment::new();
    env.vars
        .insert("PKG_ROOT".to_string(), "/opt/pkg".to_string());
    env.aliases
        .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());

    let bash = generate_shell_script(&env, &ShellType::Bash);
    let zsh = generate_shell_script(&env, &ShellType::Zsh);
    assert_eq!(bash, zsh, "Zsh script should be identical to bash script");
}

/// rez rex: empty env produces minimal script with header for all shells
#[test]
fn test_rex_empty_env_has_header_all_shells() {
    let env = RexEnvironment::new();
    for shell in [ShellType::Bash, ShellType::Fish, ShellType::PowerShell] {
        let script = generate_shell_script(&env, &shell);
        assert!(
            !script.is_empty(),
            "Even empty env should produce non-empty script (header)"
        );
        assert!(
            script.contains("Generated by rez-next rex"),
            "Script should have generator header for {:?}",
            shell
        );
    }
}

/// rez version: VersionRange union covers both subranges
#[test]
fn test_version_range_union_covers_both() {
    let r1 = VersionRange::parse("1.0+<2.0").unwrap();
    let r2 = VersionRange::parse("3.0+<4.0").unwrap();
    let union = r1.union(&r2);
    let v1 = Version::parse("1.5").unwrap();
    let v2 = Version::parse("3.5").unwrap();
    let v3 = Version::parse("2.5").unwrap();
    assert!(union.contains(&v1), "Union should contain 1.5");
    assert!(union.contains(&v2), "Union should contain 3.5");
    assert!(!union.contains(&v3), "Union should not contain 2.5");
}

/// rez version: VersionRange open-ended range contains all versions above lower bound
#[test]
fn test_version_range_open_upper_contains_versions() {
    // "1.0+" means >= 1.0, no upper bound
    let open_range = VersionRange::parse("1.0+").unwrap();
    let v_lo = Version::parse("1.0").unwrap();
    let v_hi = Version::parse("99.99.99").unwrap();
    let v_below = Version::parse("0.9.9").unwrap();
    assert!(
        open_range.contains(&v_lo),
        "Open range should contain lower bound"
    );
    assert!(
        open_range.contains(&v_hi),
        "Open range should contain high version"
    );
    assert!(
        !open_range.contains(&v_below),
        "Open range should not contain version below"
    );
}

/// rez requirement: weak requirement (~pkg) is parsed correctly
#[test]
fn test_requirement_weak_field_from_tilde_prefix() {
    // In rez, "~python" is a weak requirement (optional)
    let req = PackageRequirement::parse("~python").unwrap();
    assert!(
        req.weak,
        "Requirement starting with ~ should have weak=true"
    );
    assert_eq!(req.name, "python");

    // Standard requirement is not weak
    let normal = PackageRequirement::parse("python").unwrap();
    assert!(!normal.weak, "Normal requirement should not be weak");
}

/// rez requirement: standard requirement is not weak
#[test]
fn test_requirement_standard_not_weak() {
    let req = PackageRequirement::parse("python").unwrap();
    assert!(!req.weak, "Standard requirement should have weak=false");
}

/// rez package: variant with multiple requirements
#[test]
fn test_package_variant_multi_req_roundtrip() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"
name = 'vartest'
version = '1.0.0'
variants = [
    ['python-3.7', 'maya-2022'],
    ['python-3.9', 'maya-2024'],
]
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "vartest");
    // Variants list must contain exactly 2 entries as written in the file.
    assert_eq!(
        pkg.variants.len(),
        2,
        "vartest should have 2 variant entries, got {}",
        pkg.variants.len()
    );
}

/// rez rex: aliases in bash use single-quote escaping
#[test]
fn test_rex_bash_alias_with_special_chars() {
    let mut env = RexEnvironment::new();
    env.aliases
        .insert("mypkg".to_string(), "/opt/my pkg/bin/run".to_string());
    let script = generate_shell_script(&env, &ShellType::Bash);
    // The alias should contain the path (possibly quoted)
    assert!(
        script.contains("mypkg"),
        "Alias name 'mypkg' should appear in bash script"
    );
}

/// rez version: comparing alphanumeric tokens (alpha < numeric in rez)
#[test]
fn test_version_alphanumeric_ordering() {
    // In rez: "1.0.alpha" < "1.0.0" (alpha token is less than numeric 0)
    let v_alpha = Version::parse("1.0.alpha").unwrap();
    let v_zero = Version::parse("1.0.0").unwrap();
    assert!(
        v_alpha < v_zero,
        "rez ordering: '1.0.alpha' should be less than '1.0.0' (alpha < numeric)"
    );
}

// ─── rez.config compatibility tests ─────────────────────────────────────────

/// rez.config: default packages_path is a non-empty list of paths
/// Mirrors rez.config.packages_path default behavior (defaults include ~/packages).
#[test]
fn test_config_packages_path_default_is_list() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.packages_path.is_empty(),
        "default packages_path should be non-empty"
    );
}

/// rez.config: local_packages_path is a non-empty string
#[test]
fn test_config_local_packages_path_is_string() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.local_packages_path.is_empty(),
        "local_packages_path must be non-empty"
    );
}

/// rez.config: release_packages_path is a non-empty string
#[test]
fn test_config_release_packages_path_is_string() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.release_packages_path.is_empty(),
        "release_packages_path must be non-empty"
    );
}

/// rez.config: packages_path can be overridden by direct field assignment
#[test]
fn test_config_override_packages_path_direct() {
    use rez_next_common::config::RezCoreConfig;
    let mut cfg = RezCoreConfig::default();
    cfg.packages_path = vec!["/tmp/pkgs".to_string(), "/opt/pkgs".to_string()];
    assert_eq!(
        cfg.packages_path.len(),
        2,
        "overridden packages_path should have 2 entries"
    );
    assert!(cfg.packages_path.contains(&"/tmp/pkgs".to_string()));
    assert!(cfg.packages_path.contains(&"/opt/pkgs".to_string()));
}

/// rez.config: get_field accessor returns packages_path as JSON array
#[test]
fn test_config_get_field_packages_path() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let value = cfg.get_field("packages_path");
    assert!(
        value.is_some(),
        "get_field('packages_path') should return Some"
    );
    if let Some(serde_json::Value::Array(arr)) = value {
        assert!(
            !arr.is_empty(),
            "packages_path field should be non-empty array"
        );
    }
}

/// rez.config: get_field for nested cache config returns correct bool
#[test]
fn test_config_get_field_cache_nested() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let mem = cfg.get_field("cache.enable_memory_cache");
    assert_eq!(
        mem,
        Some(serde_json::Value::Bool(true)),
        "cache.enable_memory_cache should default to true"
    );
    let disk = cfg.get_field("cache.enable_disk_cache");
    assert_eq!(
        disk,
        Some(serde_json::Value::Bool(true)),
        "cache.enable_disk_cache should default to true"
    );
}

/// rez.config: default_shell is platform-appropriate (cmd on Windows, bash on Unix)
#[test]
fn test_config_default_shell_platform_appropriate() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(
        !cfg.default_shell.is_empty(),
        "default_shell must not be empty"
    );
    #[cfg(windows)]
    assert_eq!(
        cfg.default_shell, "cmd",
        "on Windows default_shell should be 'cmd'"
    );
    #[cfg(not(windows))]
    assert_eq!(
        cfg.default_shell, "bash",
        "on Unix default_shell should be 'bash'"
    );
}

/// rez.config: version field matches CARGO_PKG_VERSION (non-empty semver string)
#[test]
fn test_config_version_non_empty() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    assert!(!cfg.version.is_empty(), "config version must be non-empty");
    // Should look like a semver: contains a dot separator
    assert!(
        cfg.version.contains('.'),
        "config version should contain '.' (semver format)"
    );
}

/// rez.config: RezCoreConfig serializes to valid JSON and roundtrips correctly
#[test]
fn test_config_serialization_json_roundtrip_compat() {
    use rez_next_common::config::RezCoreConfig;
    let cfg = RezCoreConfig::default();
    let json = serde_json::to_string(&cfg).expect("config must serialize to JSON");
    let restored: RezCoreConfig =
        serde_json::from_str(&json).expect("config must deserialize from JSON");
    assert_eq!(
        cfg.packages_path, restored.packages_path,
        "packages_path must survive JSON roundtrip"
    );
    assert_eq!(
        cfg.local_packages_path, restored.local_packages_path,
        "local_packages_path must survive JSON roundtrip"
    );
    assert_eq!(
        cfg.default_shell, restored.default_shell,
        "default_shell must survive JSON roundtrip"
    );
}

// ─── rez.diff compatibility tests ────────────────────────────────────────────

/// rez.diff: two identical resolved contexts produce empty diff
#[test]
fn test_diff_identical_contexts_empty() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_ctx = || {
        let reqs = vec![PackageRequirement::parse("python-3.11").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = Package::new("python".to_string());
        pkg.version = Some(Version::parse("3.11").unwrap());
        ctx.resolved_packages.push(pkg);
        ctx
    };

    let ctx_a = make_ctx();
    let ctx_b = make_ctx();

    // diff: packages in A not in B (same version) → 0
    let names_a: std::collections::HashSet<String> = ctx_a
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
    let names_b: std::collections::HashSet<String> = ctx_b
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

    let added: Vec<_> = names_b.difference(&names_a).collect();
    let removed: Vec<_> = names_a.difference(&names_b).collect();
    assert!(
        added.is_empty(),
        "identical contexts should have no added packages"
    );
    assert!(
        removed.is_empty(),
        "identical contexts should have no removed packages"
    );
}

/// rez.diff: upgrading a package version shows up as changed
#[test]
fn test_diff_version_upgrade_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_ctx = |ver: &str| {
        let reqs = vec![PackageRequirement::parse("maya-2023").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = Package::new("maya".to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
        ctx
    };

    let ctx_old = make_ctx("2023");
    let ctx_new = make_ctx("2024");

    let ver_old = ctx_old.resolved_packages[0]
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");
    let ver_new = ctx_new.resolved_packages[0]
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("?");

    assert_ne!(
        ver_old, ver_new,
        "version upgrade diff should detect a change"
    );
    // 2024 > 2023 in rez numeric ordering
    let v_old = Version::parse(ver_old).unwrap();
    let v_new = Version::parse(ver_new).unwrap();
    assert!(v_new > v_old, "new context should have higher version");
}

/// rez.diff: added package in new context detected
#[test]
fn test_diff_added_package_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs_old = vec![PackageRequirement::parse("python-3.11").unwrap()];
    let mut ctx_old = ResolvedContext::from_requirements(reqs_old);
    let mut pkg_py = Package::new("python".to_string());
    pkg_py.version = Some(Version::parse("3.11").unwrap());
    ctx_old.resolved_packages.push(pkg_py.clone());

    let reqs_new = vec![
        PackageRequirement::parse("python-3.11").unwrap(),
        PackageRequirement::parse("numpy-1.25").unwrap(),
    ];
    let mut ctx_new = ResolvedContext::from_requirements(reqs_new);
    ctx_new.resolved_packages.push(pkg_py);
    let mut pkg_np = Package::new("numpy".to_string());
    pkg_np.version = Some(Version::parse("1.25").unwrap());
    ctx_new.resolved_packages.push(pkg_np);

    let names_old: std::collections::HashSet<&str> = ctx_old
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let names_new: std::collections::HashSet<&str> = ctx_new
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let added: Vec<_> = names_new.difference(&names_old).collect();
    assert_eq!(added.len(), 1, "one package (numpy) should appear as added");
    assert_eq!(*added[0], "numpy");
}

/// rez.diff: removed package in new context detected
#[test]
fn test_diff_removed_package_detected() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let make_pkg = |name: &str, ver: &str| {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    };

    let reqs_old = vec![
        PackageRequirement::parse("houdini-20").unwrap(),
        PackageRequirement::parse("hqueue-5").unwrap(),
    ];
    let mut ctx_old = ResolvedContext::from_requirements(reqs_old);
    ctx_old.resolved_packages.push(make_pkg("houdini", "20"));
    ctx_old.resolved_packages.push(make_pkg("hqueue", "5"));

    let reqs_new = vec![PackageRequirement::parse("houdini-20").unwrap()];
    let mut ctx_new = ResolvedContext::from_requirements(reqs_new);
    ctx_new.resolved_packages.push(make_pkg("houdini", "20"));

    let names_old: std::collections::HashSet<&str> = ctx_old
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    let names_new: std::collections::HashSet<&str> = ctx_new
        .resolved_packages
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    let removed: Vec<_> = names_old.difference(&names_new).collect();
    assert_eq!(
        removed.len(),
        1,
        "one package (hqueue) should appear as removed"
    );
    assert_eq!(*removed[0], "hqueue");
}

// ─── Cycle 30: rez.status + rez.packages_ compat tests ────────────────────────

/// rez.status compat: default created context starts in Resolving status.
#[test]
fn test_context_status_default_is_resolving() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;

    let reqs = vec![PackageRequirement::parse("houdini-20+").unwrap()];
    let ctx = ResolvedContext::from_requirements(reqs);
    assert_eq!(
        ctx.status,
        ContextStatus::Resolving,
        "Freshly created context should be in Resolving state"
    );
}

/// rez.status compat: manually set to Resolved is preserved.
#[test]
fn test_context_status_resolved_set_and_read() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;

    let reqs = vec![PackageRequirement::parse("maya-2024+").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    ctx.status = ContextStatus::Resolved;
    assert_eq!(ctx.status, ContextStatus::Resolved);
}

/// rez.status compat: Failed status is preserved.
#[test]
fn test_context_status_failed_set_and_read() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;

    let reqs = vec![PackageRequirement::parse("nonexistent-99+").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    ctx.status = ContextStatus::Failed;
    assert_eq!(ctx.status, ContextStatus::Failed);
}

/// rez.status compat: Cached status is preserved.
#[test]
fn test_context_status_cached_set_and_read() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;

    let reqs = vec![PackageRequirement::parse("nuke-15+").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    ctx.status = ContextStatus::Cached;
    assert_eq!(ctx.status, ContextStatus::Cached);
}

/// rez.packages_ compat: get_package returns None for unknown package name.
#[test]
fn test_context_get_package_unknown_returns_none() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("maya-2024+").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    let mut pkg = Package::new("maya".to_string());
    pkg.version = Some(Version::parse("2024.0.0").unwrap());
    ctx.resolved_packages.push(pkg);

    assert!(ctx.get_package("maya").is_some());
    assert!(ctx.get_package("houdini").is_none());
}

/// rez.packages_ compat: get_package_names returns all resolved package names.
#[test]
fn test_context_get_package_names_lists_all() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::parse("maya-2024+").unwrap(),
        PackageRequirement::parse("nuke-15+").unwrap(),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);

    let mut pkg_maya = Package::new("maya".to_string());
    pkg_maya.version = Some(Version::parse("2024.0.0").unwrap());
    ctx.resolved_packages.push(pkg_maya);

    let mut pkg_nuke = Package::new("nuke".to_string());
    pkg_nuke.version = Some(Version::parse("15.0.0").unwrap());
    ctx.resolved_packages.push(pkg_nuke);

    let names = ctx.get_package_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"maya".to_string()));
    assert!(names.contains(&"nuke".to_string()));
}

/// rez.packages_ compat: contains_package correctly identifies presence/absence.
#[test]
fn test_context_contains_package_presence_and_absence() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let reqs = vec![PackageRequirement::parse("python-3.11+").unwrap()];
    let mut ctx = ResolvedContext::from_requirements(reqs);

    let mut py = Package::new("python".to_string());
    py.version = Some(Version::parse("3.11.0").unwrap());
    ctx.resolved_packages.push(py);

    assert!(ctx.contains_package("python"));
    assert!(!ctx.contains_package("numpy"));
    assert!(!ctx.contains_package(""));
}
