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

    mod test_selftest_cy127_version {
        /// Direct unit tests for version subsystem behaviour (does not call selftest())

        #[test]
        fn test_version_parse_semver_three_components() {
            use rez_next_version::Version;
            assert!(Version::parse("1.2.3").is_ok());
            assert!(Version::parse("0.0.0").is_ok());
            assert!(Version::parse("100.200.300").is_ok());
        }

        #[test]
        fn test_version_parse_two_components() {
            use rez_next_version::Version;
            assert!(Version::parse("3.9").is_ok());
            assert!(Version::parse("20.1").is_ok());
        }

        #[test]
        fn test_version_parse_single_component() {
            use rez_next_version::Version;
            assert!(Version::parse("3").is_ok());
        }

        #[test]
        fn test_version_ordering_less_than() {
            use rez_next_version::Version;
            let v1 = Version::parse("1.0.0").unwrap();
            let v2 = Version::parse("2.0.0").unwrap();
            assert!(v1 < v2);
        }

        #[test]
        fn test_version_ordering_equal() {
            use rez_next_version::Version;
            let v1 = Version::parse("1.0.0").unwrap();
            let v2 = Version::parse("1.0.0").unwrap();
            assert_eq!(v1, v2);
        }

        #[test]
        fn test_version_range_empty_matches_any() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("").unwrap();
            let v = Version::parse("99.0").unwrap();
            assert!(range.contains(&v), "empty range must match any version");
        }

        #[test]
        fn test_version_range_gte_lower_bound() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse(">=3.9").unwrap();
            assert!(!range.contains(&Version::parse("3.8").unwrap()));
            assert!(range.contains(&Version::parse("3.9").unwrap()));
            assert!(range.contains(&Version::parse("4.0").unwrap()));
        }

        #[test]
        fn test_version_range_exclusive_upper_bound() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("1.0+<2.0").unwrap();
            assert!(range.contains(&Version::parse("1.0").unwrap()));
            assert!(range.contains(&Version::parse("1.9").unwrap()));
            assert!(!range.contains(&Version::parse("2.0").unwrap()));
        }
    }

    mod test_selftest_cy127_config {
        #[test]
        fn test_config_loads_and_has_version() {
            let cfg = rez_next_common::config::RezCoreConfig::load();
            assert!(!cfg.version.is_empty(), "config version must not be empty");
        }

        #[test]
        fn test_config_packages_path_is_vec() {
            let cfg = rez_next_common::config::RezCoreConfig::load();
            // packages_path must be a Vec (even if empty)
            let _: &Vec<String> = &cfg.packages_path;
        }
    }

    mod test_selftest_cy127_package {
        #[test]
        fn test_package_requirement_parse_simple() {
            use rez_next_package::PackageRequirement;
            assert!(PackageRequirement::parse("python").is_ok());
            assert!(PackageRequirement::parse("maya-2024").is_ok());
        }

        #[test]
        fn test_package_requirement_parse_range() {
            use rez_next_package::PackageRequirement;
            assert!(PackageRequirement::parse("python-3+<4").is_ok());
            assert!(PackageRequirement::parse("houdini>=19.5").is_ok());
        }

        #[test]
        fn test_package_requirement_name_extracted() {
            use rez_next_package::PackageRequirement;
            let req = PackageRequirement::parse("maya-2024").unwrap();
            assert_eq!(req.name(), "maya");
        }

        #[test]
        fn test_package_build_fields_version_and_commands() {
            use rez_next_package::Package;
            use rez_next_version::Version;
            let mut pkg = Package::new("tool".to_string());
            pkg.version = Some(Version::parse("1.0.0").unwrap());
            pkg.commands = Some("env.setenv('TOOL_ROOT', '{root}')".to_string());
            assert!(pkg.version.is_some());
            assert!(pkg.commands.is_some());
        }

        #[test]
        fn test_package_tools_empty_by_default() {
            use rez_next_package::Package;
            let pkg = Package::new("notools".to_string());
            assert!(pkg.tools.is_empty(), "new Package must have no tools by default");
        }

        #[test]
        fn test_package_requires_empty_by_default() {
            use rez_next_package::Package;
            let pkg = Package::new("noreqs".to_string());
            assert!(pkg.requires.is_empty(), "new Package must have no requires by default");
        }
    }

    mod test_selftest_cy127_rex {
        #[test]
        fn test_rex_parse_setenv_returns_one_action() {
            use rez_next_rex::RexParser;
            let parser = RexParser::new();
            let actions = parser.parse("env.setenv('MY_VAR', 'value')").unwrap();
            assert_eq!(actions.len(), 1);
        }

        #[test]
        fn test_rex_parse_prepend_path_returns_one_action() {
            use rez_next_rex::RexParser;
            let parser = RexParser::new();
            let actions = parser.parse("env.prepend_path('PATH', '{root}/bin')").unwrap();
            assert_eq!(actions.len(), 1);
        }

        #[test]
        fn test_rex_execute_sets_env_var() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands(
                    "env.setenv('REZ_TOOL_ROOT', '/opt/tool')",
                    "tool",
                    Some("/opt/tool"),
                    Some("1.0"),
                )
                .unwrap();
            assert!(
                env.vars.contains_key("REZ_TOOL_ROOT"),
                "setenv must create env var"
            );
        }

        #[test]
        fn test_rex_stop_sets_stopped_flag() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("stop('done')", "pkg", None, None)
                .unwrap();
            assert!(env.stopped, "stop() must set env.stopped = true");
        }

        #[test]
        fn test_rex_info_appends_message() {
            use rez_next_rex::RexExecutor;
            let mut exec = RexExecutor::new();
            let env = exec
                .execute_commands("info('hello from rex')", "pkg", None, None)
                .unwrap();
            assert!(!env.info_messages.is_empty(), "info() must append to info_messages");
        }

        #[test]
        fn test_shell_script_bash_contains_export() {
            use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
            let mut env = RexEnvironment::new();
            env.vars.insert("MY_VAR".to_string(), "value".to_string());
            let script = generate_shell_script(&env, &ShellType::Bash);
            assert!(script.contains("export MY_VAR="), "bash script must use export: {script}");
        }

        #[test]
        fn test_shell_script_powershell_contains_env_prefix() {
            use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
            let mut env = RexEnvironment::new();
            env.vars.insert("MY_VAR".to_string(), "value".to_string());
            let script = generate_shell_script(&env, &ShellType::PowerShell);
            assert!(script.contains("$env:MY_VAR"), "powershell script must use $env: prefix: {script}");
        }
    }

    mod test_selftest_cy127_repository {
        #[test]
        fn test_repository_manager_starts_empty() {
            use rez_next_repository::simple_repository::RepositoryManager;
            let mgr = RepositoryManager::new();
            assert_eq!(mgr.repository_count(), 0);
        }
    }

    mod test_selftest_cy128_suite {
        /// Direct unit tests for suite subsystem (does not call selftest())

        #[test]
        fn test_suite_new_has_no_contexts() {
            use rez_next_suites::Suite;
            let s = Suite::new();
            assert_eq!(s.len(), 0, "new Suite must have zero contexts");
        }

        #[test]
        fn test_suite_with_description_stores_value() {
            use rez_next_suites::Suite;
            let s = Suite::new().with_description("my desc");
            assert_eq!(s.description, Some("my desc".to_string()));
        }

        #[test]
        fn test_suite_add_context_increases_len() {
            use rez_next_suites::Suite;
            let mut s = Suite::new();
            s.add_context("ctx_a", vec!["python-3.9".to_string()]).unwrap();
            assert_eq!(s.len(), 1);
        }

        #[test]
        fn test_suite_duplicate_context_returns_err() {
            use rez_next_suites::Suite;
            let mut s = Suite::new();
            s.add_context("ctx", vec![]).unwrap();
            let r = s.add_context("ctx", vec![]);
            assert!(r.is_err(), "duplicate context name must return Err");
        }

        #[test]
        fn test_suite_save_and_is_suite() {
            use rez_next_suites::Suite;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("mytest_suite");
            let mut s = Suite::new().with_description("test");
            let _ = s.add_context("ctx_save", vec![]);
            s.save(&path).unwrap();
            assert!(Suite::is_suite(&path), "saved directory must be detected as suite");
        }

        #[test]
        fn test_suite_load_roundtrip_description() {
            use rez_next_suites::Suite;
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("rt_suite");
            Suite::new().with_description("roundtrip desc").save(&path).unwrap();
            let loaded = Suite::load(&path).unwrap();
            assert_eq!(loaded.description, Some("roundtrip desc".to_string()));
        }
    }

    mod test_selftest_cy128_version_extra {
        /// Additional version edge cases

        #[test]
        fn test_version_range_lt_upper_bound() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("<2.0").unwrap();
            assert!(range.contains(&Version::parse("1.9.9").unwrap()));
            assert!(!range.contains(&Version::parse("2.0").unwrap()));
        }

        #[test]
        fn test_version_display_roundtrip() {
            use rez_next_version::Version;
            let v = Version::parse("3.9.1").unwrap();
            let s = format!("{:?}", v);
            assert!(!s.is_empty(), "version debug must be non-empty: {s}");
        }

        #[test]
        fn test_version_parse_alpha_suffix() {
            use rez_next_version::Version;
            assert!(Version::parse("1.0.0-alpha1").is_ok());
        }
    }

    mod test_selftest_cy129 {
        /// Cycle 129 — solver and repository edge-case coverage

        #[test]
        fn test_solver_resolves_empty_request() {
            use rez_next_solver::{DependencySolver, SolverRequest};
            let solver = DependencySolver::new();
            let request = SolverRequest::new(vec![]);
            let result = solver.resolve(request);
            // empty request should resolve to Ok
            assert!(result.is_ok(), "empty solve request must not error: {:?}", result.err());
        }

        #[test]
        fn test_repository_manager_starts_with_zero() {
            use rez_next_repository::simple_repository::RepositoryManager;
            let mgr = RepositoryManager::new();
            assert_eq!(mgr.repository_count(), 0, "new RepositoryManager must have 0 repositories");
        }

        #[test]
        fn test_version_range_exact_match() {
            use rez_next_version::{Version, VersionRange};
            let range = VersionRange::parse("2.0").unwrap();
            assert!(range.contains(&Version::parse("2.0").unwrap()));
            assert!(!range.contains(&Version::parse("2.1").unwrap()));
        }

        #[test]
        fn test_package_requirement_version_range() {
            use rez_next_package::PackageRequirement;
            let req = PackageRequirement::parse("python-3+<4").unwrap();
            assert_eq!(req.name(), "python", "package name must be 'python'");
        }

        #[test]
        fn test_rex_parser_parses_multiple_commands() {
            use rez_next_rex::RexParser;
            let parser = RexParser::new();
            let script = "env.setenv('A', '1')\nenv.setenv('B', '2')";
            let actions = parser.parse(script).unwrap();
            assert_eq!(actions.len(), 2, "two setenv calls must produce two actions");
        }
    }
}
