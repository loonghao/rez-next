//! Self-test suite exposed to Python.
//!
//! Equivalent to `rez selftest` — verifies core functionality at runtime.

use pyo3::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SelftestCheckResult {
    name: &'static str,
    passed: bool,
}

impl SelftestCheckResult {
    const fn new(name: &'static str, passed: bool) -> Self {
        Self { name, passed }
    }
}

fn collect_selftest_results() -> Vec<SelftestCheckResult> {
    vec![
        SelftestCheckResult::new("version_parse_basic", check_version_parse_basic()),
        SelftestCheckResult::new("version_range_parse", check_version_range_parse()),
        SelftestCheckResult::new("version_comparison", check_version_comparison()),
        SelftestCheckResult::new("version_range_contains", check_version_range_contains()),
        SelftestCheckResult::new("config_loads", check_config_loads()),
        SelftestCheckResult::new(
            "package_requirement_parse",
            check_package_requirement_parse(),
        ),
        SelftestCheckResult::new(
            "package_requirement_satisfied_by",
            check_package_requirement_satisfied_by(),
        ),
        SelftestCheckResult::new("package_build_fields", check_package_build_fields()),
        SelftestCheckResult::new("rex_parse_setenv", check_rex_parse_setenv()),
        SelftestCheckResult::new("rex_parse_prepend_path", check_rex_parse_prepend_path()),
        SelftestCheckResult::new(
            "rex_execute_maya_commands",
            check_rex_execute_maya_commands(),
        ),
        SelftestCheckResult::new("rex_resetenv_info_stop", check_rex_resetenv_info_stop()),
        SelftestCheckResult::new("shell_bash_generation", check_shell_bash_generation()),
        SelftestCheckResult::new(
            "shell_powershell_generation",
            check_shell_powershell_generation(),
        ),
        SelftestCheckResult::new("suite_create_and_save", check_suite_create_and_save()),
        SelftestCheckResult::new("suite_load_roundtrip", check_suite_load_roundtrip()),
        SelftestCheckResult::new(
            "repository_manager_create",
            check_repository_manager_create(),
        ),
    ]
}

fn summarize_selftest_results(results: &[SelftestCheckResult]) -> (usize, usize, usize) {
    let passed = results.iter().filter(|result| result.passed).count();
    let total = results.len();
    (passed, total - passed, total)
}

fn check_version_parse_basic() -> bool {
    let cases = ["1.0.0", "2.1.3", "1.0.0-alpha1", "3.2.1", "0.0.1", "100.200.300"];
    cases
        .iter()
        .all(|version| rez_next_version::Version::parse(version).is_ok())
}

fn check_version_range_parse() -> bool {
    let cases = ["1.0+<2.0", ">=3.9", "<2.0", "3.9", "1.2.3+<1.3", ""];
    cases
        .iter()
        .all(|range| rez_next_version::VersionRange::parse(range).is_ok())
}

fn check_version_comparison() -> bool {
    use rez_next_version::Version;

    let Ok(v1) = Version::parse("1.0.0") else {
        return false;
    };
    let Ok(v2) = Version::parse("2.0.0") else {
        return false;
    };
    let Ok(v3) = Version::parse("1.0.0") else {
        return false;
    };

    v1 < v2 && v1 == v3 && v2 > v3
}

fn check_version_range_contains() -> bool {
    use rez_next_version::{Version, VersionRange};

    let Ok(range) = VersionRange::parse(">=3.9") else {
        return false;
    };
    let Ok(v39) = Version::parse("3.9") else {
        return false;
    };
    let Ok(v311) = Version::parse("3.11") else {
        return false;
    };
    let Ok(v38) = Version::parse("3.8") else {
        return false;
    };

    range.contains(&v39) && range.contains(&v311) && !range.contains(&v38)
}

fn check_config_loads() -> bool {
    let cfg = rez_next_common::config::RezCoreConfig::load();
    !cfg.version.is_empty()
}

fn check_package_requirement_parse() -> bool {
    use rez_next_package::PackageRequirement;

    PackageRequirement::parse("python-3.9").is_ok()
        && PackageRequirement::parse("maya").is_ok()
        && PackageRequirement::parse("houdini>=19.5").is_ok()
        && PackageRequirement::parse("python-3+<4").is_ok()
}

fn check_package_requirement_satisfied_by() -> bool {
    use rez_next_package::PackageRequirement;
    use rez_next_version::Version;

    let Ok(req) = PackageRequirement::parse("python-3.9") else {
        return false;
    };
    let Ok(version) = Version::parse("3.9") else {
        return false;
    };

    req.satisfied_by(&version)
}

fn check_package_build_fields() -> bool {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let Ok(version) = Version::parse("1.0.0") else {
        return false;
    };

    let mut pkg = Package::new("testpkg".to_string());
    pkg.version = Some(version);
    pkg.commands = Some("env.setenv('MY_ROOT', '{root}')".to_string());
    pkg.tools = vec!["mytool".to_string()];
    pkg.requires = vec!["python-3.9".to_string()];

    pkg.version.is_some() && !pkg.tools.is_empty() && pkg.commands.is_some()
}

fn check_rex_parse_setenv() -> bool {
    use rez_next_rex::RexParser;

    RexParser::new()
        .parse("env.setenv('MY_VAR', 'value')")
        .map(|actions| actions.len() == 1)
        .unwrap_or(false)
}

fn check_rex_parse_prepend_path() -> bool {
    use rez_next_rex::RexParser;

    RexParser::new()
        .parse("env.prepend_path('PATH', '{root}/bin')")
        .map(|actions| actions.len() == 1)
        .unwrap_or(false)
}

fn check_rex_execute_maya_commands() -> bool {
    use rez_next_rex::RexExecutor;

    let commands = "env.setenv('MAYA_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')";
    let mut exec = RexExecutor::new();
    exec.execute_commands(commands, "maya", Some("/opt/maya/2024.1"), Some("2024.1"))
        .map(|env| {
            env.vars
                .get("MAYA_ROOT")
                .map(|value| value.contains("/opt/maya"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn check_rex_resetenv_info_stop() -> bool {
    use rez_next_rex::RexExecutor;

    let commands = "info('test message')\nresetenv('OLD_VAR')\nstop('done')";
    let mut exec = RexExecutor::new();
    exec.execute_commands(commands, "pkg", None, None)
        .map(|env| env.stopped && !env.info_messages.is_empty())
        .unwrap_or(false)
}

fn check_shell_bash_generation() -> bool {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
    env.aliases
        .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);
    script.contains("export MY_ROOT=") && script.contains("alias pkg=")
}

fn check_shell_powershell_generation() -> bool {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    script.contains("$env:MY_ROOT")
}

fn check_suite_create_and_save() -> bool {
    use rez_next_suites::Suite;

    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let suite_path = dir.path().join("test_suite");

    let mut suite = Suite::new().with_description("rez-next selftest suite");
    if suite
        .add_context("dev", vec!["python-3.9".to_string()])
        .is_err()
    {
        return false;
    }

    suite.save(&suite_path).is_ok() && Suite::is_suite(&suite_path)
}

fn check_suite_load_roundtrip() -> bool {
    use rez_next_suites::Suite;

    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let suite_path = dir.path().join("roundtrip_suite");

    let mut suite = Suite::new().with_description("roundtrip");
    if suite
        .add_context("ctx", vec!["python-3.10".to_string()])
        .is_err()
    {
        return false;
    }
    if suite.save(&suite_path).is_err() {
        return false;
    }

    Suite::load(&suite_path)
        .map(|suite| suite.description == Some("roundtrip".to_string()) && suite.len() == 1)
        .unwrap_or(false)
}

fn check_repository_manager_create() -> bool {
    use rez_next_repository::simple_repository::RepositoryManager;

    let mgr = RepositoryManager::new();
    mgr.repository_count() == 0
}

/// Run basic self-tests and return (passed, failed, total) counts.
/// Equivalent to `rez selftest`
#[pyfunction]
pub fn selftest() -> PyResult<(usize, usize, usize)> {
    Ok(summarize_selftest_results(&collect_selftest_results()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{collect_selftest_results, selftest, summarize_selftest_results};

    #[test]
    fn test_selftest_returns_balanced_tuple_without_panic() {
        let result = std::panic::catch_unwind(selftest);
        assert!(result.is_ok(), "selftest() should not panic in Rust unit tests");

        let (passed, failed, total) = result.unwrap().expect("selftest() should return Ok");
        assert_eq!(passed + failed, total, "selftest() counts should balance");
        assert!(total > 0, "selftest() should execute at least one check");
    }

    #[test]
    fn test_selftest_results_have_unique_non_empty_names() {
        let results = collect_selftest_results();
        let mut names = HashSet::new();

        assert!(!results.is_empty(), "selftest should expose at least one check");

        for result in &results {
            assert!(!result.name.is_empty(), "selftest check names must be non-empty");
            assert!(names.insert(result.name), "duplicate selftest check name: {}", result.name);
        }
    }

    #[test]
    fn test_selftest_results_all_pass_in_healthy_runtime() {
        let results = collect_selftest_results();
        let failing: Vec<_> = results
            .iter()
            .filter(|result| !result.passed)
            .map(|result| result.name)
            .collect();

        assert!(failing.is_empty(), "selftest should not report failures: {failing:?}");
    }

    #[test]
    fn test_selftest_public_api_matches_internal_summary() {
        let results = collect_selftest_results();
        assert_eq!(selftest().unwrap(), summarize_selftest_results(&results));
    }

    #[test]
    fn test_selftest_includes_core_contract_checks() {
        let names: HashSet<_> = collect_selftest_results()
            .into_iter()
            .map(|result| result.name)
            .collect();

        for name in [
            "version_parse_basic",
            "config_loads",
            "package_requirement_parse",
            "rex_execute_maya_commands",
            "suite_load_roundtrip",
            "repository_manager_create",
        ] {
            assert!(names.contains(name), "missing expected selftest check: {name}");
        }
    }

    // ─────── Cycle 132 additions ──────────────────────────────────────────

    #[test]
    fn test_selftest_check_results_are_cloneable() {
        // SelftestCheckResult derives Clone; calling collect twice should be safe
        let first = collect_selftest_results();
        let second = collect_selftest_results();
        assert_eq!(
            first.len(),
            second.len(),
            "collect_selftest_results must be deterministic"
        );
        for (a, b) in first.iter().zip(second.iter()) {
            assert_eq!(a.name, b.name, "check names must be stable across calls");
            assert_eq!(a.passed, b.passed, "check results must be stable across calls");
        }
    }

    #[test]
    fn test_summarize_selftest_no_failures_when_all_pass() {
        // Build a synthetic all-passing result list and verify summary
        let results = vec![
            super::SelftestCheckResult::new("check_a", true),
            super::SelftestCheckResult::new("check_b", true),
            super::SelftestCheckResult::new("check_c", true),
        ];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 3);
        assert_eq!(failed, 0);
        assert_eq!(total, 3);
    }

    #[test]
    fn test_summarize_selftest_counts_failures_correctly() {
        // Build a synthetic result list with 1 failure and verify summary
        let results = vec![
            super::SelftestCheckResult::new("ok_check", true),
            super::SelftestCheckResult::new("fail_check", false),
        ];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 1);
        assert_eq!(failed, 1);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_selftest_check_result_equality() {
        // SelftestCheckResult derives PartialEq; equal structs must compare equal
        let a = super::SelftestCheckResult::new("version_parse_basic", true);
        let b = super::SelftestCheckResult::new("version_parse_basic", true);
        assert_eq!(a, b, "two identical SelftestCheckResult structs must be equal");
    }

    #[test]
    fn test_selftest_check_result_debug_format() {
        // SelftestCheckResult derives Debug; must not panic when formatted
        let r = super::SelftestCheckResult::new("my_check", false);
        let formatted = format!("{:?}", r);
        assert!(
            formatted.contains("my_check"),
            "Debug output must contain the check name, got: {formatted}"
        );
    }

    // ─────── Cycle 133 additions ──────────────────────────────────────────

    #[test]
    fn test_summarize_selftest_empty_list_returns_zero_triple() {
        let (passed, failed, total) = summarize_selftest_results(&[]);
        assert_eq!(passed, 0, "no checks → 0 passed");
        assert_eq!(failed, 0, "no checks → 0 failed");
        assert_eq!(total, 0, "no checks → total 0");
    }

    #[test]
    fn test_summarize_selftest_all_fail_case() {
        let results = vec![
            super::SelftestCheckResult::new("fail_a", false),
            super::SelftestCheckResult::new("fail_b", false),
        ];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 0);
        assert_eq!(failed, 2);
        assert_eq!(total, 2);
    }

    #[test]
    fn test_selftest_check_result_not_equal_when_name_differs() {
        let a = super::SelftestCheckResult::new("check_alpha", true);
        let b = super::SelftestCheckResult::new("check_beta", true);
        assert_ne!(a, b, "different names must not be equal");
    }

    #[test]
    fn test_selftest_check_result_not_equal_when_pass_flag_differs() {
        let a = super::SelftestCheckResult::new("same_name", true);
        let b = super::SelftestCheckResult::new("same_name", false);
        assert_ne!(a, b, "different pass flags must not be equal");
    }

    #[test]
    fn test_selftest_check_count_is_stable_at_expected_minimum() {
        // The check list currently has 17 entries; guard against silent deletion
        let results = collect_selftest_results();
        assert!(
            results.len() >= 15,
            "collect_selftest_results() must return at least 15 checks, got {}",
            results.len()
        );
    }

    // ─────── Cycle 134 additions ──────────────────────────────────────────

    #[test]
    fn test_check_version_parse_basic_returns_true() {
        assert!(
            super::check_version_parse_basic(),
            "check_version_parse_basic must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_version_range_parse_returns_true() {
        assert!(
            super::check_version_range_parse(),
            "check_version_range_parse must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_version_comparison_returns_true() {
        assert!(
            super::check_version_comparison(),
            "check_version_comparison must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_version_range_contains_returns_true() {
        assert!(
            super::check_version_range_contains(),
            "check_version_range_contains must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_config_loads_returns_true() {
        assert!(
            super::check_config_loads(),
            "check_config_loads must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_package_requirement_parse_returns_true() {
        assert!(
            super::check_package_requirement_parse(),
            "check_package_requirement_parse must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_package_requirement_satisfied_by_returns_true() {
        assert!(
            super::check_package_requirement_satisfied_by(),
            "check_package_requirement_satisfied_by must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_package_build_fields_returns_true() {
        assert!(
            super::check_package_build_fields(),
            "check_package_build_fields must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_rex_parse_setenv_returns_true() {
        assert!(
            super::check_rex_parse_setenv(),
            "check_rex_parse_setenv must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_rex_parse_prepend_path_returns_true() {
        assert!(
            super::check_rex_parse_prepend_path(),
            "check_rex_parse_prepend_path must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_rex_execute_maya_commands_returns_true() {
        assert!(
            super::check_rex_execute_maya_commands(),
            "check_rex_execute_maya_commands must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_rex_resetenv_info_stop_returns_true() {
        assert!(
            super::check_rex_resetenv_info_stop(),
            "check_rex_resetenv_info_stop must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_shell_bash_generation_returns_true() {
        assert!(
            super::check_shell_bash_generation(),
            "check_shell_bash_generation must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_suite_create_and_save_returns_true() {
        assert!(
            super::check_suite_create_and_save(),
            "check_suite_create_and_save must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_repository_manager_create_returns_true() {
        assert!(
            super::check_repository_manager_create(),
            "check_repository_manager_create must return true in healthy runtime"
        );
    }
}

