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

    // ─────── Cycle 135 additions ──────────────────────────────────────────

    #[test]
    fn test_check_shell_powershell_generation_returns_true() {
        assert!(
            super::check_shell_powershell_generation(),
            "check_shell_powershell_generation must return true in healthy runtime"
        );
    }

    #[test]
    fn test_check_suite_load_roundtrip_returns_true() {
        assert!(
            super::check_suite_load_roundtrip(),
            "check_suite_load_roundtrip must return true in healthy runtime"
        );
    }

    #[test]
    fn test_selftest_check_result_copy_trait_works() {
        // SelftestCheckResult derives Copy; copying must yield an independent value
        let original = super::SelftestCheckResult::new("copy_test", true);
        let copied = original;
        assert_eq!(original, copied, "copied SelftestCheckResult must equal original");
    }

    #[test]
    fn test_summarize_selftest_single_pass_entry() {
        let results = vec![super::SelftestCheckResult::new("only_one", true)];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 1);
        assert_eq!(failed, 0);
        assert_eq!(total, 1);
    }

    #[test]
    fn test_summarize_selftest_single_fail_entry() {
        let results = vec![super::SelftestCheckResult::new("only_fail", false)];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 0);
        assert_eq!(failed, 1);
        assert_eq!(total, 1);
    }

    #[test]
    fn test_summarize_selftest_mixed_three_entries() {
        let results = vec![
            super::SelftestCheckResult::new("ok1", true),
            super::SelftestCheckResult::new("fail1", false),
            super::SelftestCheckResult::new("ok2", true),
        ];
        let (passed, failed, total) = summarize_selftest_results(&results);
        assert_eq!(passed, 2);
        assert_eq!(failed, 1);
        assert_eq!(total, 3);
    }

    #[test]
    fn test_check_version_parse_basic_covers_alpha_suffix() {
        // Directly verify that the alpha suffix case passes
        assert!(
            rez_next_version::Version::parse("1.0.0-alpha1").is_ok(),
            "alpha-suffix version must parse"
        );
    }

    #[test]
    fn test_check_package_requirement_houdini_version_range() {
        use rez_next_package::PackageRequirement;
        // houdini>=19.5 uses a lower-bound range format
        assert!(
            PackageRequirement::parse("houdini>=19.5").is_ok(),
            "houdini>=19.5 must parse successfully"
        );
    }

    #[test]
    fn test_check_rex_maya_var_substitution() {
        use rez_next_rex::RexExecutor;
        let mut exec = RexExecutor::new();
        let env = exec
            .execute_commands(
                "env.setenv('MAYA_ROOT', '{root}')",
                "maya",
                Some("/opt/maya/2024.1"),
                Some("2024.1"),
            )
            .expect("maya root setenv must succeed");
        let val = env.vars.get("MAYA_ROOT").cloned().unwrap_or_default();
        assert!(
            val.contains("/opt/maya"),
            "MAYA_ROOT must contain base path after root substitution, got: {val}"
        );
    }

    #[test]
    fn test_check_shell_bash_contains_export_keyword() {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.vars.insert("BASH_CHECK_VAR".to_string(), "1".to_string());
        let script = generate_shell_script(&env, &ShellType::Bash);
        assert!(
            script.contains("export"),
            "bash script must use 'export' keyword, got: {script}"
        );
    }

    #[test]
    fn test_check_shell_bash_contains_alias_keyword() {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.aliases.insert("myalias".to_string(), "/path/to/tool".to_string());
        let script = generate_shell_script(&env, &ShellType::Bash);
        assert!(
            script.contains("alias"),
            "bash script must contain 'alias' keyword when aliases are set, got: {script}"
        );
    }

    #[test]
    fn test_collect_selftest_results_check_names_match_known_set() {
        // All check names must be non-empty strings with no surrounding whitespace
        let results = collect_selftest_results();
        for result in &results {
            assert_eq!(
                result.name.trim(),
                result.name,
                "check name '{}' must not have surrounding whitespace",
                result.name
            );
        }
    }

    #[test]
    fn test_selftest_public_tuple_passed_lte_total() {
        let (passed, _failed, total) = selftest().expect("selftest must not error");
        assert!(
            passed <= total,
            "passed ({passed}) must not exceed total ({total})"
        );
    }

    #[test]
    fn test_selftest_public_tuple_failed_lte_total() {
        let (_passed, failed, total) = selftest().expect("selftest must not error");
        assert!(
            failed <= total,
            "failed ({failed}) must not exceed total ({total})"
        );
    }

    #[test]
    fn test_check_config_version_is_semver_like() {
        // config.version must look like a non-trivial version string
        let cfg = rez_next_common::config::RezCoreConfig::load();
        assert!(
            !cfg.version.is_empty(),
            "config version must be a non-empty string"
        );
        // At minimum it should contain a digit
        assert!(
            cfg.version.chars().any(|c| c.is_ascii_digit()),
            "config version must contain at least one digit, got: '{}'",
            cfg.version
        );
    }

    // ── Cycle 136 additions ───────────────────────────────────────────────────

    #[test]
    fn test_collect_selftest_results_count_is_positive() {
        // There must be at least one check registered in the selftest suite
        let results = collect_selftest_results();
        assert!(
            !results.is_empty(),
            "collect_selftest_results must return at least one entry"
        );
    }

    #[test]
    fn test_selftest_check_result_passed_field() {
        let r = super::SelftestCheckResult::new("my_check", true);
        assert!(r.passed, "passed field must be true when constructed with true");
    }

    #[test]
    fn test_selftest_check_result_failed_field() {
        let r = super::SelftestCheckResult::new("bad_check", false);
        assert!(!r.passed, "passed field must be false when constructed with false");
    }

    #[test]
    fn test_summarize_empty_results_all_zero() {
        let (passed, failed, total) = summarize_selftest_results(&[]);
        assert_eq!(passed, 0, "no results → passed must be 0");
        assert_eq!(failed, 0, "no results → failed must be 0");
        assert_eq!(total, 0, "no results → total must be 0");
    }

    #[test]
    fn test_selftest_passed_plus_failed_equals_total() {
        let (passed, failed, total) = selftest().expect("selftest must succeed");
        assert_eq!(
            passed + failed,
            total,
            "passed ({passed}) + failed ({failed}) must equal total ({total})"
        );
    }
}

