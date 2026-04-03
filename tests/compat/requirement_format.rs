use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

