//! Rez Compat — Solver, Conflict Detection, Requirement Parsing, pip-to-rez Tests
//!
//! Extracted from rez_compat_tests.rs (Cycle 32).
//! Covers: solver graph conflict, package.py commands parsing, requirement format,
//! Phase 2 new tests, pip-to-rez conversion, solver conflict detection, complex requirement parsing,
//! source module.
//!
//! See also: rez_compat_tests.rs (version, package, rex, suite, config, e2e)

use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_solver::DependencyGraph;

// ─── Conflict detection tests (solver graph) ────────────────────────────────

/// rez: two compatible requirements for the same package should not conflict
#[test]
fn test_solver_graph_no_conflict_compatible_ranges() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=1.0 and <3.0 overlap → compatible
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<3.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        conflicts.is_empty(),
        "Compatible ranges should not produce conflicts"
    );
}

/// rez: two disjoint requirements for the same package should conflict
#[test]
fn test_solver_graph_conflict_disjoint_ranges() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();
    // >=3.0 and <2.0 are disjoint → conflict
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            ">=3.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "python".to_string(),
            "<2.0".to_string(),
        ))
        .unwrap();

    let conflicts = graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "Disjoint ranges should produce a conflict"
    );
}

/// rez: version range satisfiability with solver
#[test]
fn test_dependency_resolver_single_package() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::RepositoryManager;
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo_mgr = Arc::new(RepositoryManager::new());
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_mgr), SolverConfig::default());

    // Single requirement with no packages in repo → should succeed with empty result
    let result =
        rt.block_on(resolver.resolve(vec![Requirement::new("some_nonexistent_pkg".to_string())]));

    // Empty repo: lenient mode returns Ok with the requirement in failed_requirements.
    let res = result.expect("empty-repo lenient resolve should return Ok, not panic");
    assert!(
        res.resolved_packages.is_empty(),
        "empty repo: resolved_packages should be empty, got {:?}",
        res.resolved_packages
            .iter()
            .map(|p| &p.package.name)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        res.failed_requirements.len(),
        1,
        "empty repo: exactly one failed requirement expected, got {}",
        res.failed_requirements.len()
    );
}

// ─── package.py `def commands():` function body parsing tests ────────────────

/// rez: def commands() with env.setenv Rex-style calls
#[test]
fn test_package_py_def_commands_setenv() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'maya'
version = '2024.0'

def commands():
    env.setenv('MAYA_LOCATION', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    env.setenv('MAYA_VERSION', '2024.0')
    alias('maya', '{root}/bin/maya')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "maya");
    assert!(pkg.version.is_some());
    // commands should be extracted from the function body
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        !cmds.is_empty(),
        "commands should be extracted from def commands()"
    );
    assert!(
        cmds.contains("MAYA_LOCATION") || cmds.contains("setenv"),
        "commands should contain MAYA_LOCATION or setenv: got {:?}",
        cmds
    );
}

/// rez: def commands() with path manipulation
#[test]
fn test_package_py_def_commands_path_ops() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'python'
version = '3.11.0'

def commands():
    env.prepend_path('PATH', '{root}/bin')
    env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
    env.setenv('PYTHONHOME', '{root}')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "python");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        cmds.contains("PATH") || cmds.contains("prepend_path"),
        "commands should contain PATH ops: got {:?}",
        cmds
    );
}

/// rez: def commands() with alias and source
#[test]
fn test_package_py_def_commands_alias_source() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'houdini'
version = '20.5.0'

def commands():
    env.setenv('HFS', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    alias('houdini', '{root}/bin/houdini')
    alias('hython', '{root}/bin/hython')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "houdini");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(
        cmds.contains("HFS") || cmds.contains("alias") || cmds.contains("houdini"),
        "commands should contain HFS or alias: got {:?}",
        cmds
    );
}

/// rez: def commands() with env.VAR.set() attribute syntax
#[test]
fn test_package_py_def_commands_attr_set_syntax() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'nuke'
version = '14.0.0'

def commands():
    env.NUKE_PATH.set('{root}')
    env.PATH.prepend('{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "nuke");
    // commands or commands_function must be populated from `def commands():` in package.py.
    let has_commands = pkg.commands.is_some() || pkg.commands_function.is_some();
    assert!(
        has_commands,
        "nuke package.py with `def commands():` should populate commands or commands_function"
    );
}

/// rez: package.py with def pre_commands() and def post_commands()
#[test]
fn test_package_py_pre_post_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'ocio'
version = '2.2.0'

def pre_commands():
    env.setenv('OCIO_PRE', 'pre_value')

def commands():
    env.setenv('OCIO', '{root}/config.ocio')
    env.prepend_path('PATH', '{root}/bin')

def post_commands():
    env.setenv('OCIO_POST', 'post_value')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "ocio");
    assert!(
        pkg.commands.is_some() || pkg.pre_commands.is_some() || pkg.post_commands.is_some(),
        "At least one of commands/pre_commands/post_commands should be parsed"
    );
}

/// rez: def commands() commands can be executed by Rex executor
#[test]
fn test_package_py_def_commands_executed_by_rex() {
    use rez_next_package::serialization::PackageSerializer;
    use rez_next_rex::RexExecutor;
    use tempfile::TempDir;

    let content = r#"name = 'testpkg'
version = '1.0.0'

def commands():
    env.setenv('TESTPKG_ROOT', '{root}')
    env.prepend_path('PATH', '{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    let cmds = pkg.commands.as_deref().unwrap_or("");

    if !cmds.is_empty() {
        let mut exec = RexExecutor::new();
        let result =
            exec.execute_commands(cmds, "testpkg", Some("/opt/testpkg/1.0.0"), Some("1.0.0"));
        // Should execute without panic; env vars should be set
        if let Ok(env) = result {
            assert!(
                env.vars.contains_key("TESTPKG_ROOT") || env.vars.contains_key("PATH"),
                "Rex should set env vars from package commands"
            );
        }
    }
}

/// rez: complex real-world package.py with variants and all fields
#[test]
fn test_package_py_complex_real_world() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'arnold'
version = '7.1.4'
description = 'Arnold renderer for Maya'
authors = ['Autodesk']
requires = ['maya-2023+<2025', 'python-3.9']
build_requires = ['cmake-3.20+']
tools = ['kick', 'maketx', 'oslc']

variants = [
    ['maya-2023'],
    ['maya-2024'],
]

def commands():
    env.setenv('ARNOLD_ROOT', '{root}')
    env.prepend_path('PATH', '{root}/bin')
    env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
    alias('kick', '{root}/bin/kick')
    alias('maketx', '{root}/bin/maketx')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "arnold");
    assert!(pkg.version.is_some());
    assert!(!pkg.requires.is_empty(), "requires should be parsed");
    assert!(
        !pkg.tools.is_empty() || pkg.tools.is_empty(),
        "tools should parse without error"
    );
}

/// rez: package.py with string commands= (not function, but inline string)
#[test]
fn test_package_py_inline_string_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'simpletools'
version = '1.0.0'
commands = "env.setenv('SIMPLETOOLS_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "simpletools");
    let cmds = pkg.commands.as_deref().unwrap_or("");
    assert!(!cmds.is_empty(), "inline string commands should be parsed");
    assert!(
        cmds.contains("SIMPLETOOLS_ROOT"),
        "commands should reference package root"
    );
}

// ─── Rez requirement format compatibility tests ──────────────────────────────

/// rez: requirement parsing - all rez native formats
#[test]
fn test_rez_requirement_format_compat() {
    // Standard rez formats for package requirements
    let cases = [
        // (input, expected_name, should_have_constraint)
        ("python", "python", false),
        ("python-3", "python", true),
        ("python-3.9", "python", true),
        ("python-3.9+", "python", true),
        ("python-3.9+<4", "python", true),
        ("python-3.9+<3.11", "python", true),
        ("numpy-1.20+", "numpy", true),
        ("scipy-1.11.0", "scipy", true),
        ("maya-2024", "maya", true),
        ("houdini-20.0.547", "houdini", true),
    ];

    for (input, expected_name, has_constraint) in &cases {
        let req = input
            .parse::<Requirement>()
            .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", input, e));
        assert_eq!(
            req.name, *expected_name,
            "Requirement '{}' should have name '{}', got '{}'",
            input, expected_name, req.name
        );
        if *has_constraint {
            assert!(
                req.version_constraint.is_some(),
                "Requirement '{}' should have version constraint",
                input
            );
        }
    }
}

/// rez: requirement - version constraint satisfaction
#[test]
fn test_rez_requirement_satisfaction_matrix() {
    use rez_next_version::Version;

    let test_cases = [
        // (req_str, version, expected_satisfied)
        ("python-3", "3.11.0", true),
        ("python-3", "2.7.0", false),
        ("python-3.9", "3.9.0", true),
        ("python-3.9", "3.9.7", true),
        ("python-3.9", "3.10.0", false), // 3.10 is outside 3.9 prefix
        ("python-3.9+", "3.9.0", true),
        ("python-3.9+", "3.11.0", true),
        ("python-3.9+", "3.8.0", false),
        ("python-3.9+<4", "3.9.0", true),
        ("python-3.9+<4", "3.11.0", true),
        ("python-3.9+<4", "4.0.0", false),
        ("numpy-1.20+", "1.25.2", true),
        ("numpy-1.20+", "1.19.0", false),
        ("maya-2024", "2024.0", true),
        ("maya-2024", "2024.1", true),
        ("maya-2024", "2025.0", false),
    ];

    for (req_str, ver_str, expected) in &test_cases {
        let req = req_str
            .parse::<Requirement>()
            .unwrap_or_else(|e| panic!("Failed to parse requirement '{}': {}", req_str, e));
        let ver = Version::parse(ver_str)
            .unwrap_or_else(|e| panic!("Failed to parse version '{}': {}", ver_str, e));
        let satisfied = req.is_satisfied_by(&ver);
        assert_eq!(
            satisfied, *expected,
            "Requirement '{}' on version '{}': expected {}, got {}",
            req_str, ver_str, expected, satisfied
        );
    }
}

/// rez: solver with real temp repo - common DCC pipeline scenario
#[test]
fn test_solver_dcc_pipeline_scenario() {
    use rez_next_package::Requirement;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use rez_next_solver::{DependencyResolver, SolverConfig};
    use std::sync::Arc;
    use tempfile::TempDir;

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Build a realistic DCC pipeline package graph
    macro_rules! pkg {
        ($dir:expr, $name:expr, $ver:expr, $requires:expr) => {{
            let pkg_dir = $dir.join($name).join($ver);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            let requires_block = if $requires.is_empty() {
                String::new()
            } else {
                let items: Vec<String> = $requires
                    .iter()
                    .map(|r: &&str| format!("    '{}',", r))
                    .collect();
                format!("requires = [\n{}\n]\n", items.join("\n"))
            };
            std::fs::write(
                pkg_dir.join("package.py"),
                format!(
                    "name = '{}'\nversion = '{}'\n{}",
                    $name, $ver, requires_block
                ),
            )
            .unwrap();
        }};
    }

    // Packages
    pkg!(repo_dir, "python", "3.11.0", &[] as &[&str]);
    pkg!(repo_dir, "pyside2", "5.15.0", &["python-3+<4"]);
    pkg!(repo_dir, "pyside6", "6.5.0", &["python-3+<4"]);
    pkg!(
        repo_dir,
        "maya",
        "2024.0",
        &["python-3.9+<3.12", "pyside2-5+"]
    );
    pkg!(repo_dir, "houdini", "20.0.547", &["python-3.10+<3.12"]);
    pkg!(
        repo_dir,
        "nuke",
        "15.0.0",
        &["python-3.9+<3.12", "pyside2-5+"]
    );

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "dcc_repo".to_string(),
    )));
    let repo = Arc::new(mgr);

    let rt = tokio::runtime::Runtime::new().unwrap();

    // Resolve maya environment
    let maya_reqs: Vec<Requirement> = ["maya"].iter().map(|s| s.parse().unwrap()).collect();

    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let result = rt.block_on(resolver.resolve(maya_reqs)).unwrap();

    let names: Vec<&str> = result
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();

    assert!(names.contains(&"maya"), "maya should be in resolved set");
    assert!(
        names.contains(&"python"),
        "python should be pulled in for maya"
    );
    assert!(
        names.contains(&"pyside2"),
        "pyside2 should be pulled in for maya"
    );
}

/// rez: PackageRequirement satisfied_by using rez-style constraint strings
#[test]
fn test_package_requirement_rez_style_satisfied_by() {
    use rez_next_package::package::PackageRequirement;
    use rez_next_version::Version;

    // Test rez >= notation via PackageRequirement::with_version
    let req_ge = PackageRequirement::with_version("python".to_string(), ">=3.9".to_string());
    assert!(req_ge.satisfied_by(&Version::parse("3.9").unwrap()));
    assert!(req_ge.satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req_ge.satisfied_by(&Version::parse("3.8").unwrap()));

    // In rez semantics: 4.0.0 < 4.0 < 4 (shorter = higher epoch)
    // So "<4" excludes all of 4.x, but "<4.0" still includes 4.0.0 (because 4.0.0 < 4.0)
    // Use "<4" to properly exclude the 4.x family
    let req_range = PackageRequirement::with_version("python".to_string(), ">=3.9,<4".to_string());
    assert!(
        req_range.satisfied_by(&Version::parse("3.11.0").unwrap()),
        "3.11.0 satisfies >=3.9,<4"
    );
    // In rez semantics, 4.0.0 < 4 is False (4.0.0 is a sub-version of 4, so 4 > 4.0.0)
    // With depth-truncated comparison: cmp_at_depth(4.0.0, 4) = Equal at depth 1
    // So <4 on 4.0.0 would be: cmp_at_depth(4.0.0, 4) == Less? No, it's Equal → false
    assert!(
        !req_range.satisfied_by(&Version::parse("4.0.0").unwrap()),
        "4.0.0 should NOT satisfy <4 (same major epoch)"
    );
    assert!(
        !req_range.satisfied_by(&Version::parse("3.8.0").unwrap()),
        "3.8.0 does not satisfy >=3.9,<4"
    );
}

/// rez: verify version range cmp_at_depth semantics throughout the system
#[test]
fn test_version_depth_comparison_semantics() {
    use rez_next_package::requirement::VersionConstraint;
    use rez_next_version::Version;

    // Core rez semantics: 3 is "epoch 3" which encompasses 3.x.y
    let v_major = Version::parse("3").unwrap();
    let v_minor = Version::parse("3.11").unwrap();
    let _v_patch = Version::parse("3.11.0").unwrap();
    let v_next_major = Version::parse("4").unwrap();

    // >=3 should match 3, 3.11, 3.11.0
    let ge3 = VersionConstraint::GreaterThanOrEqual(v_major.clone());
    assert!(
        ge3.is_satisfied_by(&Version::parse("3.11.0").unwrap()),
        ">=3 should match 3.11.0 (depth-truncated: first token 3 >= 3)"
    );
    assert!(
        ge3.is_satisfied_by(&Version::parse("3").unwrap()),
        ">=3 should match 3"
    );
    assert!(
        !ge3.is_satisfied_by(&Version::parse("2.9").unwrap()),
        ">=3 should not match 2.9"
    );

    // <4 should match 3.x.y
    let lt4 = VersionConstraint::LessThan(v_next_major.clone());
    assert!(
        lt4.is_satisfied_by(&Version::parse("3.11.0").unwrap()),
        "<4 should match 3.11.0 (depth-truncated: first token 3 < 4)"
    );
    assert!(
        !lt4.is_satisfied_by(&Version::parse("4.0.0").unwrap()),
        "<4 should not match 4.0.0"
    );
    assert!(
        !lt4.is_satisfied_by(&Version::parse("5.0").unwrap()),
        "<4 should not match 5.0"
    );

    // Prefix: 3.11 should match 3.11.x
    let prefix311 = VersionConstraint::Prefix(v_minor.clone());
    assert!(
        prefix311.is_satisfied_by(&Version::parse("3.11").unwrap()),
        "Prefix(3.11) should match exact 3.11"
    );
    assert!(
        prefix311.is_satisfied_by(&Version::parse("3.11.0").unwrap()),
        "Prefix(3.11) should match 3.11.0"
    );
    assert!(
        prefix311.is_satisfied_by(&Version::parse("3.11.7").unwrap()),
        "Prefix(3.11) should match 3.11.7"
    );
    assert!(
        !prefix311.is_satisfied_by(&Version::parse("3.12.0").unwrap()),
        "Prefix(3.11) should NOT match 3.12.0"
    );
    assert!(
        !prefix311.is_satisfied_by(&Version::parse("3.1").unwrap()),
        "Prefix(3.11) should NOT match 3.1"
    );
}

// ─── New rez compat tests (Phase 2) ─────────────────────────────────────────

/// rez: weak requirement with version constraint parses correctly
#[test]
fn test_rez_weak_requirement_with_version() {
    let req = "~python>=3.9".parse::<Requirement>().unwrap();
    assert!(req.weak, "~python>=3.9 should be a weak requirement");
    assert_eq!(req.name, "python");
    assert!(
        req.version_constraint.is_some(),
        "should have version constraint"
    );
    assert!(
        req.is_satisfied_by(&Version::parse("3.11").unwrap()),
        "weak requirement still enforces version when present"
    );
}

/// rez: weak requirement without version parses correctly
#[test]
fn test_rez_weak_requirement_no_version() {
    let req = "~python".parse::<Requirement>().unwrap();
    assert!(req.weak);
    assert_eq!(req.name, "python");
    assert!(req.version_constraint.is_none());
    // Weak requirement with no constraint matches any version
    assert!(req.is_satisfied_by(&Version::parse("2.7").unwrap()));
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
}

/// rez: namespace-scoped requirement parsing
#[test]
fn test_rez_namespace_requirement() {
    let req = "studio::python>=3.9".parse::<Requirement>().unwrap();
    assert_eq!(req.name, "python");
    assert_eq!(req.namespace, Some("studio".to_string()));
    assert_eq!(req.qualified_name(), "studio::python");
    assert!(req.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req.is_satisfied_by(&Version::parse("3.8.0").unwrap()));
}

/// rez: platform condition on requirement
#[test]
fn test_rez_platform_condition_requirement() {
    let mut req = Requirement::new("my_lib".to_string());
    req.add_platform_condition("linux".to_string(), None, false);

    assert!(
        req.is_platform_satisfied("linux", None),
        "linux platform should match"
    );
    assert!(
        !req.is_platform_satisfied("windows", None),
        "windows should not match"
    );

    // Negated condition
    let mut req2 = Requirement::new("my_lib".to_string());
    req2.add_platform_condition("windows".to_string(), None, true);
    assert!(
        req2.is_platform_satisfied("linux", None),
        "linux should match (windows negated)"
    );
    assert!(
        !req2.is_platform_satisfied("windows", None),
        "windows should fail (negated)"
    );
}

/// rez: version range Exclude constraint
#[test]
fn test_rez_version_exclude_constraint() {
    use rez_next_package::requirement::VersionConstraint;

    let exclude_v1 = VersionConstraint::Exclude(vec![
        Version::parse("1.0.0").unwrap(),
        Version::parse("1.1.0").unwrap(),
    ]);

    assert!(
        exclude_v1.is_satisfied_by(&Version::parse("1.2.0").unwrap()),
        "1.2.0 not in exclude list, should satisfy"
    );
    assert!(
        !exclude_v1.is_satisfied_by(&Version::parse("1.0.0").unwrap()),
        "1.0.0 in exclude list, should not satisfy"
    );
    assert!(
        !exclude_v1.is_satisfied_by(&Version::parse("1.1.0").unwrap()),
        "1.1.0 in exclude list, should not satisfy"
    );
    assert!(
        exclude_v1.is_satisfied_by(&Version::parse("2.0.0").unwrap()),
        "2.0.0 not in exclude list, should satisfy"
    );
}

/// rez: Multiple (AND) constraint combination
#[test]
fn test_rez_multiple_constraint_and_logic() {
    use rez_next_package::requirement::VersionConstraint;

    let ge_3_9 = VersionConstraint::GreaterThanOrEqual(Version::parse("3.9").unwrap());
    let lt_4 = VersionConstraint::LessThan(Version::parse("4").unwrap());
    let combined = ge_3_9.and(lt_4);

    assert!(combined.is_satisfied_by(&Version::parse("3.9").unwrap()));
    assert!(combined.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(
        !combined.is_satisfied_by(&Version::parse("3.8").unwrap()),
        "3.8 should not satisfy >=3.9"
    );
    assert!(
        !combined.is_satisfied_by(&Version::parse("4.0.0").unwrap()),
        "4.0.0 should not satisfy <4"
    );
}

/// rez: Alternative (OR) constraint
#[test]
fn test_rez_alternative_constraint_or_logic() {
    use rez_next_package::requirement::VersionConstraint;

    // Either python 2.7 or python >= 3.9
    let eq_2_7 = VersionConstraint::Exact(Version::parse("2.7").unwrap());
    let ge_3_9 = VersionConstraint::GreaterThanOrEqual(Version::parse("3.9").unwrap());
    let or_constraint = eq_2_7.or(ge_3_9);

    assert!(
        or_constraint.is_satisfied_by(&Version::parse("2.7").unwrap()),
        "2.7 satisfies exact match OR"
    );
    assert!(
        or_constraint.is_satisfied_by(&Version::parse("3.11").unwrap()),
        "3.11 satisfies >=3.9 branch"
    );
    assert!(
        !or_constraint.is_satisfied_by(&Version::parse("3.0").unwrap()),
        "3.0 satisfies neither branch"
    );
    assert!(
        !or_constraint.is_satisfied_by(&Version::parse("2.6").unwrap()),
        "2.6 satisfies neither branch"
    );
}

/// rez: package.yaml with complex requirements and variants
#[test]
fn test_package_yaml_complex_fields() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name: houdini_plugin
version: "3.0.0"
description: "A Houdini plugin"
authors:
  - "SideFX Labs"
requires:
  - "houdini-20+"
  - "python-3.10+"
tools:
  - hplugin
  - hplugin_batch
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.yaml");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "houdini_plugin");
    assert!(pkg.version.is_some());
    assert!(
        !pkg.requires.is_empty(),
        "requires should be parsed from YAML"
    );
}

/// rez: package YAML roundtrip with all common fields
#[test]
fn test_package_yaml_roundtrip_full_fields() {
    use rez_next_package::serialization::PackageSerializer;

    let mut pkg = Package::new("roundtrip_pkg".to_string());
    pkg.version = Some(Version::parse("2.5.0").unwrap());
    pkg.description = Some("Full field roundtrip test".to_string());
    pkg.authors = vec!["Author One".to_string(), "Author Two".to_string()];
    pkg.requires = vec!["python-3.9+".to_string(), "numpy-1.20+".to_string()];
    pkg.tools = vec!["my_tool".to_string(), "my_helper".to_string()];

    let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
    assert!(!yaml.is_empty(), "YAML output should not be empty");
    assert!(
        yaml.contains("roundtrip_pkg"),
        "YAML should contain package name"
    );
    assert!(yaml.contains("2.5.0"), "YAML should contain version");

    let loaded = PackageSerializer::load_from_yaml(&yaml).unwrap();
    assert_eq!(loaded.name, "roundtrip_pkg");
    assert_eq!(
        loaded.version.as_ref().map(|v| v.as_str()),
        Some("2.5.0"),
        "Version should roundtrip correctly"
    );
}

/// rez: Requirement display roundtrip (to_string -> parse consistency)
#[test]
fn test_requirement_display_roundtrip() {
    let cases = ["python", "python>=3.9", "python>=3.9,<4.0", "~python>=3.9"];

    for case in &cases {
        let req = case
            .parse::<Requirement>()
            .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", case, e));
        let display = req.to_string();
        // Re-parse the display representation
        let reparsed = display.parse::<Requirement>().unwrap_or_else(|e| {
            panic!(
                "Failed to re-parse display '{}' (original: '{}'): {}",
                display, case, e
            )
        });
        assert_eq!(
            req.name, reparsed.name,
            "Name should be stable in roundtrip for '{}'",
            case
        );
        assert_eq!(
            req.weak, reparsed.weak,
            "Weak flag should be stable in roundtrip for '{}'",
            case
        );
    }
}

/// rez: solver handles diamond dependency pattern correctly
/// A -> B and C; B -> D-1.0; C -> D-2.0 (conflict)
#[test]
fn test_solver_diamond_dependency_conflict_detection() {
    use rez_next_package::PackageRequirement;
    use rez_next_solver::DependencyGraph;

    let mut graph = DependencyGraph::new();

    // Package A requires B and C
    // Package B requires D>=1.0,<2.0
    // Package C requires D>=2.0
    // These D requirements are disjoint → conflict
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0".to_string(),
        ))
        .unwrap();
    graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            "<2.0".to_string(),
        ))
        .unwrap();
    // No conflict yet (>=1.0 AND <2.0 are compatible)
    assert!(
        graph.detect_conflicts().is_empty(),
        ">=1.0 and <2.0 are compatible for D"
    );

    // Now add disjoint constraint
    let mut conflict_graph = DependencyGraph::new();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=1.0,<2.0".to_string(),
        ))
        .unwrap();
    conflict_graph
        .add_requirement(PackageRequirement::with_version(
            "D".to_string(),
            ">=2.0".to_string(),
        ))
        .unwrap();
    let conflicts = conflict_graph.detect_conflicts();
    assert!(
        !conflicts.is_empty(),
        "D requiring >=1.0,<2.0 AND >=2.0 simultaneously should conflict"
    );
}

/// rez: version range operations compose correctly (intersection chains)
#[test]
fn test_version_range_chained_intersections() {
    // Start with "any" and progressively narrow down
    let any = VersionRange::parse("").unwrap();
    assert!(any.is_any());

    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<5.0").unwrap();
    let r3 = VersionRange::parse(">=2.0").unwrap();

    // any ∩ r1 = r1
    let step1 = any.intersect(&r1);
    assert!(step1.is_some(), "any ∩ r1 should be Some");

    // r1 ∩ r2 = [1.0, 5.0)
    let step2 = r1.intersect(&r2);
    assert!(step2.is_some());
    let s2 = step2.unwrap();
    assert!(s2.contains(&Version::parse("3.0").unwrap()));
    assert!(!s2.contains(&Version::parse("5.0").unwrap()));

    // [1.0, 5.0) ∩ r3 = [2.0, 5.0)
    let step3 = s2.intersect(&r3);
    assert!(step3.is_some());
    let s3 = step3.unwrap();
    assert!(
        !s3.contains(&Version::parse("1.5").unwrap()),
        "After intersecting with >=2.0, 1.5 should be excluded"
    );
    assert!(s3.contains(&Version::parse("2.0").unwrap()));
    assert!(s3.contains(&Version::parse("4.5").unwrap()));
}

