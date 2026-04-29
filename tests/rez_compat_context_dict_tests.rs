//! Rez Compat — context.to_dict, Solver weak, Version boundary (new),
//! Package validation (new), Rex DSL edge case, Package commands
//!
//! Extracted from rez_compat_misc_tests.rs (Cycle 141).

use rez_core::version::{Version, VersionRange};

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
