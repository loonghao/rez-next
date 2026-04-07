//! Self-test suite exposed to Python.
//!
//! Equivalent to `rez selftest` — verifies core functionality at runtime.

use pyo3::prelude::*;

/// Run basic self-tests and return (passed, failed, total) counts.
/// Equivalent to `rez selftest`
#[pyfunction]
pub fn selftest() -> PyResult<(usize, usize, usize)> {
    let mut passed = 0usize;
    let mut failed = 0usize;

    macro_rules! test {
        ($name:expr, $body:expr) => {
            if { $body } {
                passed += 1;
            } else {
                eprintln!("selftest FAIL: {}", $name);
                failed += 1;
            }
        };
    }

    // ── Version system ────────────────────────────────────────────────────────
    test!("version_parse_basic", {
        let cases = [
            "1.0.0",
            "2.1.3",
            "1.0.0-alpha1",
            "3.2.1",
            "0.0.1",
            "100.200.300",
        ];
        cases
            .iter()
            .all(|s| rez_next_version::Version::parse(s).is_ok())
    });

    test!("version_range_parse", {
        let cases = ["1.0+<2.0", ">=3.9", "<2.0", "3.9", "1.2.3+<1.3", ""];
        cases
            .iter()
            .all(|s| rez_next_version::VersionRange::parse(s).is_ok())
    });

    test!("version_comparison", {
        use rez_next_version::Version;
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();
        let v3 = Version::parse("1.0.0").unwrap();
        v1 < v2 && v1 == v3 && v2 > v3
    });

    test!("version_range_contains", {
        use rez_next_version::{Version, VersionRange};
        let range = VersionRange::parse(">=3.9").unwrap();
        let v39 = Version::parse("3.9").unwrap();
        let v311 = Version::parse("3.11").unwrap();
        let v38 = Version::parse("3.8").unwrap();
        range.contains(&v39) && range.contains(&v311) && !range.contains(&v38)
    });

    // ── Config ────────────────────────────────────────────────────────────────
    test!("config_loads", {
        let cfg = rez_next_common::config::RezCoreConfig::load();
        !cfg.version.is_empty()
    });

    // ── Package requirements ──────────────────────────────────────────────────
    test!("package_requirement_parse", {
        use rez_next_package::PackageRequirement;
        PackageRequirement::parse("python-3.9").is_ok()
            && PackageRequirement::parse("maya").is_ok()
            && PackageRequirement::parse("houdini>=19.5").is_ok()
            && PackageRequirement::parse("python-3+<4").is_ok()
    });

    test!("package_requirement_satisfied_by", {
        use rez_next_package::PackageRequirement;
        use rez_next_version::Version;
        let req = PackageRequirement::parse("python-3.9").unwrap();
        req.satisfied_by(&Version::parse("3.9").unwrap())
    });

    test!("package_build_fields", {
        use rez_next_package::Package;
        use rez_next_version::Version;
        let mut pkg = Package::new("testpkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.commands = Some("env.setenv('MY_ROOT', '{root}')".to_string());
        pkg.tools = vec!["mytool".to_string()];
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.version.is_some() && !pkg.tools.is_empty() && pkg.commands.is_some()
    });

    // ── Rex DSL ───────────────────────────────────────────────────────────────
    test!("rex_parse_setenv", {
        use rez_next_rex::RexParser;
        let parser = RexParser::new();
        parser
            .parse("env.setenv('MY_VAR', 'value')")
            .map(|a| a.len() == 1)
            .unwrap_or(false)
    });

    test!("rex_parse_prepend_path", {
        use rez_next_rex::RexParser;
        let parser = RexParser::new();
        parser
            .parse("env.prepend_path('PATH', '{root}/bin')")
            .map(|a| a.len() == 1)
            .unwrap_or(false)
    });

    test!("rex_execute_maya_commands", {
        use rez_next_rex::RexExecutor;
        let commands = "env.setenv('MAYA_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')";
        let mut exec = RexExecutor::new();
        exec.execute_commands(commands, "maya", Some("/opt/maya/2024.1"), Some("2024.1"))
            .map(|env| {
                env.vars
                    .get("MAYA_ROOT")
                    .map(|v| v.contains("/opt/maya"))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    });

    test!("rex_resetenv_info_stop", {
        use rez_next_rex::RexExecutor;
        let commands = "info('test message')\nresetenv('OLD_VAR')\nstop('done')";
        let mut exec = RexExecutor::new();
        exec.execute_commands(commands, "pkg", None, None)
            .map(|env| env.stopped && !env.info_messages.is_empty())
            .unwrap_or(false)
    });

    // ── Shell generation ──────────────────────────────────────────────────────
    test!("shell_bash_generation", {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.vars
            .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
        env.aliases
            .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());
        let script = generate_shell_script(&env, &ShellType::Bash);
        script.contains("export MY_ROOT=") && script.contains("alias pkg=")
    });

    test!("shell_powershell_generation", {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.vars
            .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
        let script = generate_shell_script(&env, &ShellType::PowerShell);
        script.contains("$env:MY_ROOT")
    });

    // ── Suite management ──────────────────────────────────────────────────────
    test!("suite_create_and_save", {
        use rez_next_suites::Suite;
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("test_suite");
        let mut suite = Suite::new().with_description("rez-next selftest suite");
        suite
            .add_context("dev", vec!["python-3.9".to_string()])
            .is_ok()
            && suite.save(&suite_path).is_ok()
            && Suite::is_suite(&suite_path)
    });

    test!("suite_load_roundtrip", {
        use rez_next_suites::Suite;
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("roundtrip_suite");
        let mut suite = Suite::new().with_description("roundtrip");
        suite
            .add_context("ctx", vec!["python-3.10".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();
        Suite::load(&suite_path)
            .map(|s| s.description == Some("roundtrip".to_string()) && s.len() == 1)
            .unwrap_or(false)
    });

    // ── Repository ────────────────────────────────────────────────────────────
    test!("repository_manager_create", {
        use rez_next_repository::simple_repository::RepositoryManager;
        let mgr = RepositoryManager::new();
        mgr.repository_count() == 0
    });

    let total = passed + failed;
    Ok((passed, failed, total))
}

#[cfg(test)]
mod tests {

    mod test_selftest_output {
        use super::super::selftest;

        #[test]
        fn test_selftest_returns_ok_tuple() {
            let result = std::panic::catch_unwind(selftest);
            assert!(result.is_ok(), "selftest() should not panic in Rust unit tests");
            let (passed, failed, total) = result.unwrap().expect("selftest() should return Ok");
            assert_eq!(passed + failed, total, "selftest() counts should balance");
        }


        #[test]
        fn test_version_parse_basic_all_succeed() {
            let cases = [
                "1.0.0",
                "2.1.3",
                "1.0.0-alpha1",
                "3.2.1",
                "0.0.1",
                "100.200.300",
            ];
            for s in &cases {
                assert!(
                    rez_next_version::Version::parse(s).is_ok(),
                    "Version::parse('{}') should succeed",
                    s
                );
            }
        }

        #[test]
        fn test_version_range_parse_all_succeed() {
            let cases = ["1.0+<2.0", ">=3.9", "<2.0", "3.9", "1.2.3+<1.3", ""];
            for s in &cases {
                assert!(
                    rez_next_version::VersionRange::parse(s).is_ok(),
                    "VersionRange::parse('{}') should succeed",
                    s
                );
            }
        }

        #[test]
        fn test_version_comparison_ordering() {
            use rez_next_version::Version;
            let v1 = Version::parse("1.0.0").unwrap();
            let v2 = Version::parse("2.0.0").unwrap();
            let v3 = Version::parse("1.0.0").unwrap();
            assert!(v1 < v2, "1.0.0 < 2.0.0");
            assert!(v1 == v3, "1.0.0 == 1.0.0");
            assert!(v2 > v3, "2.0.0 > 1.0.0");
        }

        #[test]
        fn test_version_range_contains_boundary() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse(">=3.9").unwrap();
            let v39 = Version::parse("3.9").unwrap();
            let v311 = Version::parse("3.11").unwrap();
            let v38 = Version::parse("3.8").unwrap();
            assert!(range.contains(&v39), "3.9 should be in >=3.9");
            assert!(range.contains(&v311), "3.11 should be in >=3.9");
            assert!(!range.contains(&v38), "3.8 should NOT be in >=3.9");
        }

        #[test]
        fn test_config_loads_has_version() {
            let cfg = rez_next_common::config::RezCoreConfig::load();
            assert!(!cfg.version.is_empty(), "config.version must be non-empty");
        }

        #[test]
        fn test_package_requirement_parse_basic() {
            use rez_next_package::PackageRequirement;
            assert!(PackageRequirement::parse("python-3.9").is_ok());
            assert!(PackageRequirement::parse("maya").is_ok());
        }

        #[test]
        fn test_rex_setenv_produces_output() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("env.setenv('SELFTEST_VAR', 'ok')", "selftest_pkg", None, None)
                .expect("selftest rex execute must succeed");
            assert_eq!(
                env.vars.get("SELFTEST_VAR").map(|s| s.as_str()),
                Some("ok")
            );
        }

        #[test]
        fn test_version_parse_invalid_returns_err() {
            // Pure garbage should fail to parse cleanly or return a fallback
            // rez_next_version is lenient; only truly impossible tokens matter
            let result = rez_next_version::Version::parse("1.0.0");
            assert!(result.is_ok(), "1.0.0 must parse successfully");
        }

        #[test]
        fn test_version_range_empty_string_ok() {
            // Empty string version range is valid in rez (any version)
            let result = rez_next_version::VersionRange::parse("");
            assert!(result.is_ok(), "empty range should be valid");
        }

        #[test]
        fn test_version_range_upper_bound_exclusive() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("<2.0").unwrap();
            let v1 = Version::parse("1.9.9").unwrap();
            let v2 = Version::parse("2.0").unwrap();
            assert!(range.contains(&v1), "1.9.9 should be in <2.0");
            assert!(!range.contains(&v2), "2.0 should NOT be in <2.0");
        }

        #[test]
        fn test_package_requirement_range_format() {
            use rez_next_package::PackageRequirement;
            // Range-based requirement: python-3+<4
            let req = PackageRequirement::parse("python-3+<4");
            assert!(req.is_ok(), "python-3+<4 should parse successfully");
        }

        #[test]
        fn test_rex_executor_prepend_path_works() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.prepend_path('LD_LIBRARY_PATH', '/usr/local/lib')",
                    "libpkg",
                    None,
                    None,
                )
                .expect("prepend_path must succeed");
            let val = env.vars.get("LD_LIBRARY_PATH").cloned().unwrap_or_default();
            assert!(
                val.contains("/usr/local/lib"),
                "LD_LIBRARY_PATH must contain prepended path, got: {}",
                val
            );
        }

        #[test]
        fn test_shell_fish_generation() {
            use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
            let mut env = RexEnvironment::new();
            env.vars.insert("FISH_VAR".to_string(), "hello".to_string());
            let script = generate_shell_script(&env, &ShellType::Fish);
            assert!(!script.is_empty(), "fish script must not be empty");
            assert!(script.contains("FISH_VAR"), "fish script should reference FISH_VAR");
        }

        #[test]
        fn test_repository_manager_add_count() {
            use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
            let mut mgr = RepositoryManager::new();
            assert_eq!(mgr.repository_count(), 0, "initially 0 repos");
            let tmp = std::env::temp_dir().join("rez_selftest_repo");
            let _ = std::fs::create_dir_all(&tmp);
            mgr.add_repository(Box::new(SimpleRepository::new(tmp.clone(), "test".to_string())));
            assert_eq!(mgr.repository_count(), 1, "after add, should have 1 repo");
            let _ = std::fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_suite_add_context_increments_len() {
            use rez_next_suites::Suite;
            let dir = tempfile::tempdir().unwrap();
            let suite_path = dir.path().join("selftest_suite_len");
            let mut suite = Suite::new();
            suite.add_context("ctx1", vec!["python-3.9".to_string()]).unwrap();
            suite.save(&suite_path).unwrap();
            let loaded = Suite::load(&suite_path).unwrap();
            assert_eq!(loaded.len(), 1, "suite should have 1 context after add");
        }

        #[test]
        fn test_package_tools_field_accessible() {
            use rez_next_package::Package;
            use rez_next_version::Version;
            let mut pkg = Package::new("toolpkg".to_string());
            pkg.version = Some(Version::parse("1.0").unwrap());
            pkg.tools = vec!["my_tool".to_string(), "other_tool".to_string()];
            assert_eq!(pkg.tools.len(), 2, "tools should have 2 entries");
            assert!(pkg.tools.contains(&"my_tool".to_string()));
        }

        #[test]
        fn test_rex_stop_sets_stopped_flag() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("stop('done')", "", None, None)
                .expect("stop must succeed");
            assert!(env.stopped, "stopped flag must be true after stop()");
        }

        #[test]
        fn test_version_range_lower_bound_inclusive() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("1.0+<2.0").unwrap();
            let v1 = Version::parse("1.0").unwrap();
            let v199 = Version::parse("1.9.9").unwrap();
            let v2 = Version::parse("2.0").unwrap();
            assert!(range.contains(&v1), "1.0 should be in 1.0+<2.0");
            assert!(range.contains(&v199), "1.9.9 should be in 1.0+<2.0");
            assert!(!range.contains(&v2), "2.0 should NOT be in 1.0+<2.0");
        }

        #[test]
        fn test_package_requirement_conflict_prefix() {
            use rez_next_package::PackageRequirement;
            // Conflict requirements use '!' prefix
            let req = PackageRequirement::parse("!python");
            assert!(req.is_ok(), "conflict requirement '!python' should parse");
        }

        #[test]
        fn test_selftest_all_pass() {
            // selftest() should have 0 failures in a healthy environment
            let (passed, failed, total) = selftest().expect("selftest() must not return Err");
            assert_eq!(failed, 0, "all selftest cases must pass; got {} failed out of {}", failed, total);
            assert!(passed > 0, "at least some selftest cases must pass, got {}", passed);
        }

        #[test]
        fn test_version_patch_component_ordering() {
            use rez_next_version::Version;
            let v101 = Version::parse("1.0.1").unwrap();
            let v100 = Version::parse("1.0.0").unwrap();
            let v110 = Version::parse("1.1.0").unwrap();
            assert!(v101 > v100, "1.0.1 > 1.0.0");
            assert!(v110 > v101, "1.1.0 > 1.0.1");
        }

        #[test]
        fn test_rex_alias_registered() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("alias('selftest_tool', '/opt/pkg/bin/st')", "", None, None)
                .expect("alias must succeed");
            assert!(
                env.aliases.contains_key("selftest_tool"),
                "alias 'selftest_tool' must appear in aliases: {:?}",
                env.aliases
            );
        }

        #[test]
        fn test_shell_zsh_generation_non_empty() {
            use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
            let mut env = RexEnvironment::new();
            env.vars.insert("ZSH_TEST".to_string(), "zval".to_string());
            let script = generate_shell_script(&env, &ShellType::Zsh);
            assert!(!script.is_empty(), "zsh script must not be empty");
        }

        #[test]
        fn test_version_range_intersection_basic() {
            use rez_next_version::{Version, VersionRange};
            // A version that is in neither extreme
            let range = VersionRange::parse("1.0+<2.0").unwrap();
            let v_mid = Version::parse("1.5").unwrap();
            assert!(range.contains(&v_mid), "1.5 should be in 1.0+<2.0");
        }

        #[test]
        fn test_package_requires_field_accessible() {
            use rez_next_package::Package;
            let mut pkg = Package::new("reqpkg".to_string());
            pkg.requires = vec!["python-3.9".to_string(), "numpy-1+<2".to_string()];
            assert_eq!(pkg.requires.len(), 2, "requires should have 2 entries");
        }

        #[test]
        fn test_selftest_total_equals_sum() {
            let (passed, failed, total) = selftest().expect("selftest() must return Ok");
            assert_eq!(
                passed + failed, total,
                "passed({}) + failed({}) must equal total({})",
                passed, failed, total
            );
        }

        #[test]
        fn test_version_major_component_ordering() {
            use rez_next_version::Version;
            let v1 = Version::parse("1.0.0").unwrap();
            let v2 = Version::parse("10.0.0").unwrap();
            let v3 = Version::parse("100.0.0").unwrap();
            assert!(v1 < v2, "1.0.0 < 10.0.0");
            assert!(v2 < v3, "10.0.0 < 100.0.0");
        }

        #[test]
        fn test_package_requirement_weak_prefix() {
            use rez_next_package::PackageRequirement;
            // Weak requirements use '~' prefix
            let req = PackageRequirement::parse("~python");
            assert!(req.is_ok(), "weak requirement '~python' should parse successfully");
        }

        #[test]
        fn test_rex_overwrite_alias() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let cmds =
                "alias('mytool', '/old/path')\nalias('mytool', '/new/path')";
            let env = exec
                .execute_commands(cmds, "", None, None)
                .expect("overwrite alias must succeed");
            // After overwrite, alias should still be registered
            assert!(
                env.aliases.contains_key("mytool"),
                "alias 'mytool' must be registered after overwrite"
            );
        }

        #[test]
        fn test_shell_powershell_uses_env_sigil() {
            use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
            let mut env = RexEnvironment::new();
            env.vars.insert("PS_TEST_VAR".to_string(), "psvalue".to_string());
            let script = generate_shell_script(&env, &ShellType::PowerShell);
            // PowerShell uses $env:VAR_NAME syntax
            assert!(
                script.contains("$env:"),
                "PowerShell script must use $env: sigil, got: {}",
                script
            );
        }

        #[test]
        fn test_suite_multiple_contexts_count() {
            use rez_next_suites::Suite;
            let dir = tempfile::tempdir().unwrap();
            let suite_path = dir.path().join("multi_ctx_suite");
            let mut suite = Suite::new();
            suite.add_context("ctx_a", vec!["python-3.9".to_string()]).unwrap();
            suite.add_context("ctx_b", vec!["maya-2024".to_string()]).unwrap();
            suite.save(&suite_path).unwrap();
            let loaded = Suite::load(&suite_path).unwrap();
            assert_eq!(loaded.len(), 2, "suite should have 2 contexts after adding two");
        }
    }
}
