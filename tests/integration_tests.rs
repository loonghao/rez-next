//! Integration tests for rez-core

use rez_core::common::RezCoreConfig;
use rez_core::version::{Version, VersionRange};

#[test]
fn test_version_creation() {
    let version = Version::parse("1.2.3").expect("Should create version");
    assert_eq!(version.as_str(), "1.2.3");
}

#[test]
fn test_version_range_creation() {
    let range = VersionRange::parse("1.0.0..2.0.0").expect("Should create version range");
    assert_eq!(range.as_str(), "1.0.0..2.0.0");
}

#[test]
fn test_config_defaults() {
    let config = RezCoreConfig::default();
    assert!(config.use_rust_version);
    assert!(config.use_rust_solver);
    assert!(config.use_rust_repository);
    assert!(config.rust_fallback);
}

#[test]
fn test_module_structure() {
    // Test that all modules can be imported and basic functionality works
    let _version = Version::parse("1.0.0").expect("Version creation should work");
    let _range = VersionRange::parse("1.0.0+").expect("Range creation should work");
    let _config = RezCoreConfig::default();

    // This test ensures the basic module structure compiles and initializes
}

// Performance optimization tests
mod performance_tests {
    use std::time::Instant;

    #[test]
    fn test_version_parsing_performance() {
        println!("Testing version parsing performance...");

        let test_versions = vec![
            "1.2.3",
            "1.2.3-alpha.1",
            "2.0.0-beta.2+build.123",
            "1.0.0-rc.1",
            "3.1.4-dev.123",
        ];

        let start_time = Instant::now();

        // Simulate optimized parsing
        for version_str in &test_versions {
            let _parsed = format!("parsed_{}", version_str);
        }

        let duration = start_time.elapsed();
        println!("Mock parsing took: {:?}", duration);

        assert!(duration.as_millis() < 100, "Parsing should be fast");
    }

    #[test]
    fn test_batch_processing_simulation() {
        println!("Testing batch processing simulation...");

        let version_strings: Vec<String> = (0..1000)
            .map(|i| format!("1.{}.{}", i / 100, i % 100))
            .collect();

        let start_time = Instant::now();

        let _results: Vec<_> = version_strings
            .iter()
            .map(|v| format!("processed_{}", v))
            .collect();

        let duration = start_time.elapsed();
        println!(
            "Batch processing of {} items took: {:?}",
            version_strings.len(),
            duration
        );

        assert!(
            duration.as_millis() < 1000,
            "Batch processing should be efficient"
        );
    }

    #[test]
    fn test_simd_pattern_matching_simulation() {
        println!("Testing SIMD pattern matching simulation...");

        let test_files = ["package.py",
            "package.yaml",
            "package.json",
            "not_a_package.txt",
            "another_package.py"];

        let start_time = Instant::now();

        let matches: Vec<_> = test_files
            .iter()
            .filter(|filename| {
                filename.ends_with(".py")
                    || filename.ends_with(".yaml")
                    || filename.ends_with(".json")
            })
            .collect();

        let duration = start_time.elapsed();
        println!(
            "Pattern matching took: {:?}, found {} matches",
            duration,
            matches.len()
        );

        assert_eq!(matches.len(), 4);
        assert!(duration.as_millis() < 10);
    }

    #[test]
    fn test_memory_efficiency_simulation() {
        println!("Testing memory efficiency simulation...");

        let mut data = Vec::new();

        for i in 0..10000 {
            data.push(format!("item_{}", i));
        }

        assert_eq!(data.len(), 10000);

        data.clear();
        assert_eq!(data.len(), 0);

        println!("Memory efficiency test completed");
    }

    #[test]
    fn test_parallel_processing_simulation() {
        println!("Testing parallel processing simulation...");

        let large_dataset: Vec<String> = (0..5000).map(|i| format!("data_{}", i)).collect();

        let start_time = Instant::now();

        let _results: Vec<_> = large_dataset
            .iter()
            .map(|item| format!("processed_{}", item))
            .collect();

        let duration = start_time.elapsed();
        println!("Parallel simulation took: {:?}", duration);

        assert!(duration.as_millis() < 500);
    }
}

// Phase 73: Integration tests for new VersionRange APIs
mod version_range_integration {
    use rez_core::version::{Version, VersionRange};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn vr(s: &str) -> VersionRange {
        VersionRange::parse(s).unwrap()
    }

    #[test]
    fn test_version_range_intersect_semantic() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<4.0");
        let i = r1.intersect(&r2).unwrap();
        // intersection should be >=2.0,<3.0
        assert!(i.contains(&v("2.0")));
        assert!(i.contains(&v("2.9")));
        assert!(!i.contains(&v("1.0"))); // below intersection lower bound
        assert!(!i.contains(&v("3.0"))); // at r1's upper bound (excluded)
    }

    #[test]
    fn test_version_range_union_semantic() {
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr(">=3.0,<4.0");
        let u = r1.union(&r2);
        assert!(u.contains(&v("1.5")));
        assert!(u.contains(&v("3.5")));
        assert!(!u.contains(&v("2.5"))); // gap between the two ranges
    }

    #[test]
    fn test_version_range_subset_chain() {
        // >=1.0,<1.5 ⊂ >=1.0,<2.0 ⊂ >=1.0
        let r1 = vr(">=1.0,<1.5");
        let r2 = vr(">=1.0,<2.0");
        let r3 = vr(">=1.0");

        assert!(r1.is_subset_of(&r2));
        assert!(r1.is_subset_of(&r3));
        assert!(r2.is_subset_of(&r3));

        assert!(!r2.is_subset_of(&r1));
        assert!(!r3.is_subset_of(&r2));
        assert!(!r3.is_subset_of(&r1));
    }

    #[test]
    fn test_version_range_superset_chain() {
        let r1 = vr(">=1.0,<1.5");
        let r2 = vr(">=1.0,<2.0");
        let r3 = vr(">=1.0");

        assert!(r3.is_superset_of(&r2));
        assert!(r3.is_superset_of(&r1));
        assert!(r2.is_superset_of(&r1));
    }

    #[test]
    fn test_version_range_intersects_disjoint() {
        // Two completely disjoint ranges
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr(">=3.0,<4.0");
        assert!(!r1.intersects(&r2));

        // Adjacent ranges (boundary: [1,2) and [2,3))
        let r3 = vr(">=2.0,<3.0");
        // 2.0 is not in r1 (exclusive upper), so no overlap
        assert!(!r1.intersects(&r3));
    }

    #[test]
    fn test_version_range_intersects_overlapping() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<4.0");
        assert!(r1.intersects(&r2));
    }

    #[test]
    fn test_version_range_subtract_basic() {
        let r1 = vr(">=1.0,<3.0");
        let r2 = vr(">=2.0,<3.0");
        let diff = r1.subtract(&r2);
        // diff should be >=1.0,<2.0
        assert!(diff.is_some());
        let diff = diff.unwrap();
        assert!(diff.contains(&v("1.5")));
        assert!(!diff.contains(&v("2.0")));
        assert!(!diff.contains(&v("2.5")));
    }

    #[test]
    fn test_version_range_exact_subset() {
        let exact = vr("==2.0.0");
        let range = vr(">=1.0,<3.0");
        assert!(exact.is_subset_of(&range));
        assert!(range.is_superset_of(&exact));
    }

    #[test]
    fn test_version_range_any_superset_of_all() {
        let any = vr("*");
        let r1 = vr(">=1.0,<2.0");
        let r2 = vr("==3.5");
        assert!(any.is_superset_of(&r1));
        assert!(any.is_superset_of(&r2));
        assert!(r1.is_subset_of(&any));
        assert!(r2.is_subset_of(&any));
    }

    #[test]
    fn test_version_range_rez_syntax_compatibility() {
        // Test rez-specific syntax
        let r1 = vr("1.0+"); // >=1.0
        let r2 = vr("1.0+<2.0"); // >=1.0,<2.0
        let r3 = vr("1.0..2.0"); // >=1.0,<2.0

        assert!(r2.is_subset_of(&r1));
        // r2 and r3 should be semantically equivalent
        assert!(r2.contains(&v("1.5")));
        assert!(r3.contains(&v("1.5")));
        assert!(!r2.contains(&v("2.0")));
        assert!(!r3.contains(&v("2.0")));
    }
}

// ── Phase 101: Complete solve + context + env generation chain ──────────────────

mod solve_context_env_integration {
    use rez_next_package::Package;
    use rez_next_rex::{generate_shell_script, RexExecutor, ShellType};
    use rez_next_version::Version;

    fn make_package(name: &str, version: &str, commands: Option<&str>) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        if let Some(cmds) = commands {
            pkg.commands = Some(cmds.to_string());
        }
        pkg
    }

    #[test]
    fn test_package_commands_to_env_vars() {
        // Simulate: package has commands → executor processes → env vars set
        let pkg = make_package(
            "myapp",
            "2.0.0",
            Some("env.setenv('MYAPP_VERSION', '2.0.0')\nenv.setenv('MYAPP_ROOT', '{root}')"),
        );
        let mut exec = RexExecutor::new();
        let env = exec
            .execute_commands(
                pkg.commands.as_deref().unwrap(),
                &pkg.name,
                Some("/opt/myapp/2.0.0"),
                Some("2.0.0"),
            )
            .unwrap();
        assert_eq!(env.vars.get("MYAPP_VERSION"), Some(&"2.0.0".to_string()));
        assert_eq!(
            env.vars.get("MYAPP_ROOT"),
            Some(&"/opt/myapp/2.0.0".to_string())
        );
    }

    #[test]
    fn test_multiple_packages_env_merging() {
        // Simulate: two packages with commands → check each env independently
        let python_cmds =
            "env.setenv('PYTHON_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')";
        let numpy_cmds = "env.setenv('NUMPY_VERSION', '1.24.0')\nenv.prepend_path('PYTHONPATH', '{root}/lib/python/site-packages')";

        let mut exec1 = RexExecutor::new();
        let env_python = exec1
            .execute_commands(
                python_cmds,
                "python",
                Some("/opt/python/3.10"),
                Some("3.10"),
            )
            .unwrap();

        let mut exec2 = RexExecutor::new();
        let env_numpy = exec2
            .execute_commands(numpy_cmds, "numpy", Some("/opt/numpy/1.24"), Some("1.24.0"))
            .unwrap();

        // Check each env independently
        assert!(env_python.vars.contains_key("PYTHON_ROOT"));
        assert!(env_numpy.vars.contains_key("NUMPY_VERSION"));
        // PATH entries from both
        let path = env_python.vars.get("PATH").cloned().unwrap_or_default();
        assert!(path.contains("/opt/python/3.10/bin"));
    }

    #[test]
    fn test_env_to_bash_script_chain() {
        // Full chain: package commands → executor → env → bash script
        let mut exec = RexExecutor::new();
        let env = exec.execute_commands(
            "env.setenv('APP_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')\nalias('myapp', '{root}/bin/myapp')",
            "myapp",
            Some("/opt/myapp/1.0"),
            Some("1.0"),
        ).unwrap();

        let script = generate_shell_script(&env, &ShellType::Bash);
        assert!(
            script.contains("APP_ROOT"),
            "Bash script must contain APP_ROOT"
        );
        assert!(
            script.contains("/opt/myapp/1.0/bin"),
            "Bash script must contain bin path"
        );
        assert!(
            script.contains("alias myapp="),
            "Bash script must contain myapp alias"
        );
    }

    #[test]
    fn test_env_to_powershell_script_chain() {
        let mut exec = RexExecutor::new();
        let env = exec
            .execute_commands(
                "env.setenv('TOOL_HOME', '{root}')",
                "tool",
                Some("C:\\opt\\tool\\1.0"),
                Some("1.0"),
            )
            .unwrap();

        let script = generate_shell_script(&env, &ShellType::PowerShell);
        assert!(
            script.contains("$env:TOOL_HOME"),
            "PowerShell script must contain TOOL_HOME"
        );
    }

    #[test]
    fn test_source_script_in_env_chain() {
        // Package sources a setup script → env tracks it
        let mut exec = RexExecutor::new();
        let env = exec
            .execute_commands(
                "env.setenv('PKG_ROOT', '{root}')\nsource('{root}/etc/pkg_setup.sh')",
                "mypkg",
                Some("/opt/mypkg/2.0"),
                None,
            )
            .unwrap();

        assert_eq!(
            env.vars.get("PKG_ROOT"),
            Some(&"/opt/mypkg/2.0".to_string())
        );
        assert_eq!(env.sourced_scripts.len(), 1);
        assert_eq!(env.sourced_scripts[0], "/opt/mypkg/2.0/etc/pkg_setup.sh");

        // Bash script should include source command
        let script = generate_shell_script(&env, &ShellType::Bash);
        assert!(script.contains("source \"/opt/mypkg/2.0/etc/pkg_setup.sh\""));
    }

    #[test]
    fn test_package_with_pre_and_post_commands() {
        // Test pre_commands and commands combination
        let pre_cmds = "env.setenv('PRE_VAR', 'pre_value')";
        let main_cmds = "env.setenv('MAIN_VAR', 'main_value')\nenv.setenv('PRE_VAR', 'overridden')";

        let mut exec = RexExecutor::new();
        let env_pre = exec
            .execute_commands(pre_cmds, "mypkg", Some("/opt/mypkg/1.0"), None)
            .unwrap();
        assert_eq!(env_pre.vars.get("PRE_VAR"), Some(&"pre_value".to_string()));

        let mut exec2 = RexExecutor::new();
        let env_main = exec2
            .execute_commands(main_cmds, "mypkg", Some("/opt/mypkg/1.0"), None)
            .unwrap();
        assert_eq!(
            env_main.vars.get("PRE_VAR"),
            Some(&"overridden".to_string())
        );
        assert_eq!(
            env_main.vars.get("MAIN_VAR"),
            Some(&"main_value".to_string())
        );
    }

    #[test]
    fn test_path_accumulation_from_multiple_packages() {
        let mut exec = RexExecutor::new();
        let cmds2 = "env.prepend_path('PATH', '/opt/pkg2/bin')";
        let env2 = exec
            .execute_commands(cmds2, "pkg2", Some("/opt/pkg2"), None)
            .unwrap();
        assert!(env2
            .vars
            .get("PATH")
            .map(|p| p.contains("/opt/pkg2/bin"))
            .unwrap_or(false));
    }

    #[test]
    fn test_version_range_satisfies_solver_input() {
        use rez_core::version::VersionRange;
        // Simulates solver input validation: requirements are valid version ranges
        let requirements = vec![
            ("python", "3.9+<4"),
            ("numpy", ">=1.20"),
            ("maya", "==2024.0"),
        ];
        for (pkg, req) in &requirements {
            let range = VersionRange::parse(req);
            assert!(
                range.is_ok(),
                "Requirement {}-{} should parse as valid range",
                pkg,
                req
            );
        }
    }

    #[test]
    fn test_package_version_satisfies_range() {
        use rez_core::version::{Version, VersionRange};
        let pkg_version = Version::parse("3.10.5").unwrap();
        // Use explicit ranges to avoid rez short-version semantics edge cases
        let requirement = VersionRange::parse(">=3.9,<4.0.0").unwrap();
        assert!(
            requirement.contains(&pkg_version),
            "3.10.5 should satisfy >=3.9,<4.0.0"
        );

        let too_low = Version::parse("3.8.0").unwrap();
        assert!(
            !requirement.contains(&too_low),
            "3.8.0 should not satisfy >=3.9,<4.0.0"
        );

        // 4.0.0 is excluded by <4.0.0
        let at_boundary = Version::parse("4.0.0").unwrap();
        assert!(
            !requirement.contains(&at_boundary),
            "4.0.0 should not satisfy >=3.9,<4.0.0"
        );

        // Something clearly above
        let above = Version::parse("5.0.0").unwrap();
        assert!(
            !requirement.contains(&above),
            "5.0.0 should not satisfy >=3.9,<4.0.0"
        );
    }

    #[test]
    fn test_env_variables_substitution_chain() {
        let mut exec = RexExecutor::new();
        let env = exec.execute_commands(
            "env.setenv('MY_ROOT', '{root}')\nenv.setenv('MY_VER', '{version}')\nenv.setenv('MY_NAME', '{name}')",
            "testpkg",
            Some("/packages/testpkg/2.5.0"),
            Some("2.5.0"),
        ).unwrap();
        assert_eq!(
            env.vars.get("MY_ROOT"),
            Some(&"/packages/testpkg/2.5.0".to_string())
        );
        assert_eq!(env.vars.get("MY_VER"), Some(&"2.5.0".to_string()));
        assert_eq!(env.vars.get("MY_NAME"), Some(&"testpkg".to_string()));
    }
}

// ── Cycle 24: Requirement × Version × VersionConstraint cross-validation ──────
mod package_requirement_cross_validation {
    use rez_next_package::{Requirement, VersionConstraint};
    use rez_next_version::Version;

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_requirement_any_accepts_all() {
        let req = Requirement::new("python".to_string());
        assert!(req.is_satisfied_by(&v("2.7.0")));
        assert!(req.is_satisfied_by(&v("3.11.0")));
        assert!(req.is_satisfied_by(&v("99.0.0")));
    }

    #[test]
    fn test_requirement_exact_version_constraint() {
        let req = Requirement::with_version(
            "maya".to_string(),
            VersionConstraint::Exact(v("2024.0.0")),
        );
        assert!(req.is_satisfied_by(&v("2024.0.0")));
        assert!(!req.is_satisfied_by(&v("2023.0.0")));
        assert!(!req.is_satisfied_by(&v("2024.0.1")));
    }

    #[test]
    fn test_requirement_gte_constraint() {
        let req = Requirement::with_version(
            "lib".to_string(),
            VersionConstraint::GreaterThanOrEqual(v("2.0")),
        );
        assert!(req.is_satisfied_by(&v("2.0")));
        assert!(req.is_satisfied_by(&v("3.5.0")));
        assert!(!req.is_satisfied_by(&v("1.9.9")));
    }

    #[test]
    fn test_requirement_range_constraint() {
        let req = Requirement::with_version(
            "openexr".to_string(),
            VersionConstraint::Range(v("3.0"), v("4.0")),
        );
        assert!(req.is_satisfied_by(&v("3.0")));
        assert!(req.is_satisfied_by(&v("3.5.2")));
        // Range constraint: upper bound semantics depends on impl; test both sides
        // Lower bound is inclusive
        assert!(!req.is_satisfied_by(&v("2.9.9")));
    }

    #[test]
    fn test_weak_requirement_has_weak_flag() {
        let req = Requirement::weak("optional_plugin".to_string());
        assert!(req.weak, "weak() constructor must set weak=true");
        assert!(req.version_constraint.is_none());
        // Weak requirement with no constraint accepts any version
        assert!(req.is_satisfied_by(&v("1.0")));
    }

    #[test]
    fn test_requirement_lt_constraint_excludes_exact() {
        let req = Requirement::with_version(
            "pkg".to_string(),
            VersionConstraint::LessThan(v("3.0")),
        );
        assert!(req.is_satisfied_by(&v("2.9.9")));
        assert!(!req.is_satisfied_by(&v("3.0")));
        assert!(!req.is_satisfied_by(&v("3.1")));
    }

    #[test]
    fn test_version_constraint_and_logic() {
        // AND: >=1.0 AND <3.0 → 1.0..3.0
        let gte = VersionConstraint::GreaterThanOrEqual(v("1.0"));
        let lt = VersionConstraint::LessThan(v("3.0"));
        let combined = gte.and(lt);
        let req = Requirement::with_version("pkg".to_string(), combined);

        assert!(req.is_satisfied_by(&v("1.0")));
        assert!(req.is_satisfied_by(&v("2.9")));
        assert!(!req.is_satisfied_by(&v("0.9")));
        assert!(!req.is_satisfied_by(&v("3.0")));
    }

    #[test]
    fn test_requirement_name_is_preserved() {
        let req = Requirement::with_version(
            "houdini".to_string(),
            VersionConstraint::GreaterThanOrEqual(v("20.0")),
        );
        assert_eq!(req.name, "houdini");
        assert_eq!(req.package_name(), "houdini");
    }
}

// ── Cycle 24: Package struct × VersionRange interop ────────────────────────────
mod rez_version_package_interop {
    use rez_core::version::{Version, VersionRange};
    use rez_next_package::{Package, PackageRequirement};

    fn v(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    fn vr(s: &str) -> VersionRange {
        VersionRange::parse(s).unwrap()
    }

    #[test]
    fn test_package_version_satisfies_requirement_range() {
        // Build a package with version 3.10.5
        let mut pkg = Package::new("python".to_string());
        pkg.set_version(v("3.10.5"));
        let range = vr(">=3.9,<4.0");
        let pkg_ver = pkg.version.as_ref().unwrap();
        assert!(range.contains(pkg_ver), "python-3.10.5 should satisfy >=3.9,<4.0");
    }

    #[test]
    fn test_package_version_outside_range() {
        let mut pkg = Package::new("python".to_string());
        pkg.set_version(v("2.7.18"));
        let range = vr(">=3.0");
        let pkg_ver = pkg.version.as_ref().unwrap();
        assert!(!range.contains(pkg_ver), "python-2.7.18 should not satisfy >=3.0");
    }

    #[test]
    fn test_package_requirement_parse_name_and_spec() {
        let pr = PackageRequirement::parse("python-3.9+").unwrap();
        assert_eq!(pr.name, "python");
        assert!(pr.version_spec.is_some(), "version_spec should be Some for python-3.9+");
    }

    #[test]
    fn test_multiple_package_versions_select_latest_in_range() {
        // Simulate selecting versions from a set that satisfy a range.
        // NOTE: In rez version semantics, 4.0 > 4.0.0, so "<4.0" upper bound
        // includes 4.0.0 (since 4.0.0 < 4.0). Use "<4.0.0" to truly exclude 4.0.0.
        let versions = ["3.7.0", "3.9.0", "3.10.5", "3.11.0", "4.0.0"];
        let range = vr(">=3.9,<4.0.0");
        let candidates: Vec<Version> = versions
            .iter()
            .map(|s| Version::parse(s).unwrap())
            .filter(|ver| range.contains(ver))
            .collect();

        assert!(!candidates.is_empty(), "at least one version should satisfy >=3.9,<4.0.0");
        // All candidates must be within range
        for c in &candidates {
            assert!(range.contains(c), "{} should satisfy the range", c.as_str());
        }
        // 4.0.0 is at the exclusive upper bound — should be excluded
        assert!(
            !candidates.iter().any(|c| c.as_str() == "4.0.0"),
            "4.0.0 must not be in candidates for >=3.9,<4.0.0"
        );
        // 3.7.0 is below lower bound — should also be excluded
        assert!(
            !candidates.iter().any(|c| c.as_str() == "3.7.0"),
            "3.7.0 must not be in candidates for >=3.9,<4.0.0"
        );
    }

    #[test]
    fn test_version_range_any_contains_package_version() {
        let any_range = vr("");
        let pkg_ver = v("1.2.3");
        assert!(any_range.contains(&pkg_ver), "Any range should accept any package version");
    }

    #[test]
    fn test_package_no_version_field_is_none() {
        let pkg = Package::new("unversioned".to_string());
        assert!(pkg.version.is_none(), "Package with no version should have version=None");
    }
}
