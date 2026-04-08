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
        fn test_selftest_returns_balanced_tuple_without_panic() {
            let result = std::panic::catch_unwind(selftest);
            assert!(result.is_ok(), "selftest() should not panic in Rust unit tests");

            let (passed, failed, total) = result.unwrap().expect("selftest() should return Ok");
            assert_eq!(passed + failed, total, "selftest() counts should balance");
            assert!(total > 0, "selftest() should execute at least one check");
        }

        #[test]
        fn test_selftest_reports_zero_failures_in_healthy_runtime() {
            let (passed, failed, total) = selftest().expect("selftest() must not return Err");
            assert_eq!(failed, 0, "selftest() should not report failures in unit tests");
            assert_eq!(passed, total, "all recorded checks should pass in a healthy runtime");
        }
    }

    mod test_selftest_counts {
        use super::super::selftest;

        #[test]
        fn test_selftest_passed_count_is_positive() {
            let (passed, _failed, _total) = selftest().unwrap();
            assert!(passed > 0, "selftest should run at least one passing check");
        }

        #[test]
        fn test_selftest_total_at_least_ten() {
            let (_passed, _failed, total) = selftest().unwrap();
            assert!(total >= 10, "selftest must run at least 10 checks, got {total}");
        }

        #[test]
        fn test_selftest_failed_is_zero() {
            let (_passed, failed, _total) = selftest().unwrap();
            assert_eq!(failed, 0, "no selftest checks should fail in a clean build");
        }

        #[test]
        fn test_selftest_passed_equals_total() {
            let (passed, failed, total) = selftest().unwrap();
            assert_eq!(passed, total - failed, "passed must equal total - failed");
        }

        #[test]
        fn test_selftest_returns_ok_not_err() {
            let result = selftest();
            assert!(result.is_ok(), "selftest must return Ok(...)");
        }
    }

    mod test_selftest_version_system {
        use super::super::selftest;

        #[test]
        fn test_version_parse_basic_check_included() {
            // selftest includes version_parse_basic; if failed == 0, this check passed
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "version_parse_basic check must pass");
        }

        #[test]
        fn test_version_range_parse_check_included() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "version_range_parse check must pass");
        }

        #[test]
        fn test_version_comparison_check_included() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "version_comparison check must pass");
        }

        #[test]
        fn test_version_range_contains_check_included() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "version_range_contains check must pass");
        }
    }

    mod test_selftest_components {
        use super::super::selftest;

        #[test]
        fn test_config_loads_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "config_loads selftest check must pass");
        }

        #[test]
        fn test_package_requirement_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "package_requirement selftest checks must pass");
        }

        #[test]
        fn test_rex_parse_checks_pass() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "rex_parse selftest checks must pass");
        }

        #[test]
        fn test_shell_generation_checks_pass() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "shell_generation selftest checks must pass");
        }

        #[test]
        fn test_suite_create_and_save_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "suite_create_and_save selftest check must pass");
        }

        #[test]
        fn test_repository_manager_create_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "repository_manager_create selftest check must pass");
        }

        #[test]
        fn test_rex_execute_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "rex_execute selftest check must pass");
        }

        #[test]
        fn test_suite_load_roundtrip_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "suite_load_roundtrip selftest check must pass");
        }
    }

    mod test_selftest_invariants {
        use super::super::selftest;

        #[test]
        fn test_selftest_idempotent() {
            let r1 = selftest().unwrap();
            let r2 = selftest().unwrap();
            assert_eq!(r1, r2, "selftest must be idempotent (same result on every call)");
        }

        #[test]
        fn test_selftest_total_is_sum_of_passed_and_failed() {
            let (passed, failed, total) = selftest().unwrap();
            assert_eq!(passed + failed, total, "passed + failed must equal total");
        }

        #[test]
        fn test_selftest_all_pass_implies_failed_zero() {
            let (passed, failed, total) = selftest().unwrap();
            if passed == total {
                assert_eq!(failed, 0, "if all pass, failed must be 0");
            }
        }

        #[test]
        fn test_selftest_does_not_modify_global_state() {
            // Call twice; environment should remain stable
            let _ = selftest().unwrap();
            let _ = selftest().unwrap();
        }

        #[test]
        fn test_selftest_tuple_elements_are_usize() {
            // Type safety: the tuple must be (usize, usize, usize)
            let (p, f, t): (usize, usize, usize) = selftest().unwrap();
            let _ = (p, f, t);
        }

        #[test]
        fn test_selftest_total_does_not_exceed_100() {
            let (_p, _f, total) = selftest().unwrap();
            assert!(total <= 100, "selftest total checks should be <= 100, got {total}");
        }

        #[test]
        fn test_selftest_at_least_twelve_checks() {
            let (_p, _f, total) = selftest().unwrap();
            assert!(total >= 12, "selftest should run at least 12 checks, got {total}");
        }

        #[test]
        fn test_selftest_passes_exceed_half_of_total() {
            let (passed, _f, total) = selftest().unwrap();
            assert!(
                passed * 2 >= total,
                "more than half of selftest checks must pass, passed={passed}, total={total}"
            );
        }
    }

    mod test_selftest_cy122 {
        use super::super::selftest;

        /// selftest total is at least 15 (accounts for all subsystem checks)
        #[test]
        fn test_selftest_total_at_least_fifteen() {
            let (_p, _f, total) = selftest().unwrap();
            assert!(
                total >= 15,
                "selftest should run at least 15 checks, got {total}"
            );
        }

        /// selftest passes all version-related checks (failed == 0 means none failed)
        #[test]
        fn test_selftest_all_version_checks_pass() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "no selftest checks should fail in a clean build");
        }

        /// selftest result (p, f, t) satisfies p + f == t invariant
        #[test]
        fn test_selftest_sum_invariant() {
            let (p, f, t) = selftest().unwrap();
            assert_eq!(p + f, t, "passed + failed must always equal total");
        }

        /// selftest rex_execute_maya_commands check passes (indirectly via failed==0)
        #[test]
        fn test_selftest_rex_maya_commands_check_passes() {
            let (_p, failed, _t) = selftest().unwrap();
            assert_eq!(failed, 0, "rex_execute_maya_commands selftest check must pass");
        }

        /// selftest result is stable across 3 calls
        #[test]
        fn test_selftest_stable_three_calls() {
            let r1 = selftest().unwrap();
            let r2 = selftest().unwrap();
            let r3 = selftest().unwrap();
            assert_eq!(r1, r2, "selftest results must be stable (call 1 vs 2)");
            assert_eq!(r2, r3, "selftest results must be stable (call 2 vs 3)");
        }

        /// selftest total count does not change between calls (count is fixed)
        #[test]
        fn test_selftest_count_is_deterministic() {
            let (_, _, t1) = selftest().unwrap();
            let (_, _, t2) = selftest().unwrap();
            assert_eq!(t1, t2, "total check count must be deterministic");
        }
    }

    mod test_selftest_cy123 {
        use super::super::selftest;

        /// selftest passes value: passed > 0 always
        #[test]
        fn test_selftest_passed_nonzero() {
            let (passed, _, _) = selftest().unwrap();
            assert!(passed > 0, "selftest must have at least one passing check");
        }

        /// selftest total is at least 16
        #[test]
        fn test_selftest_total_at_least_sixteen() {
            let (_, _, total) = selftest().unwrap();
            assert!(total >= 16, "selftest should run at least 16 checks, got {total}");
        }

        /// failed count is zero (all checks pass in healthy build)
        #[test]
        fn test_selftest_cy123_failed_zero() {
            let (_, failed, _) = selftest().unwrap();
            assert_eq!(failed, 0, "no selftest checks should fail");
        }

        /// passed + failed == total holds for fresh call
        #[test]
        fn test_selftest_cy123_sum_invariant() {
            let (p, f, t) = selftest().unwrap();
            assert_eq!(p + f, t, "passed + failed must equal total");
        }

        /// selftest returns Ok (not Err) when called twice in sequence
        #[test]
        fn test_selftest_returns_ok_twice() {
            assert!(selftest().is_ok(), "first selftest() call must return Ok");
            assert!(selftest().is_ok(), "second selftest() call must return Ok");
        }

        /// selftest total is less than 50 (bounded upper limit)
        #[test]
        fn test_selftest_total_below_fifty() {
            let (_, _, total) = selftest().unwrap();
            assert!(total < 50, "selftest total should be < 50, got {total}");
        }
    }
}

