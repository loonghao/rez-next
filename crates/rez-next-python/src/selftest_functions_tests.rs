//! Unit tests for selftest_functions.

use std::collections::HashSet;

use crate::selftest_functions::{
    collect_selftest_results, selftest, summarize_selftest_results, SelftestCheckResult,
};

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
    let results = vec![
        SelftestCheckResult::new("check_a", true),
        SelftestCheckResult::new("check_b", true),
        SelftestCheckResult::new("check_c", true),
    ];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 3);
    assert_eq!(failed, 0);
    assert_eq!(total, 3);
}

#[test]
fn test_summarize_selftest_counts_failures_correctly() {
    let results = vec![
        SelftestCheckResult::new("ok_check", true),
        SelftestCheckResult::new("fail_check", false),
    ];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 1);
    assert_eq!(failed, 1);
    assert_eq!(total, 2);
}

#[test]
fn test_selftest_check_result_equality() {
    let a = SelftestCheckResult::new("version_parse_basic", true);
    let b = SelftestCheckResult::new("version_parse_basic", true);
    assert_eq!(a, b, "two identical SelftestCheckResult structs must be equal");
}

#[test]
fn test_selftest_check_result_debug_format() {
    let r = SelftestCheckResult::new("my_check", false);
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
        SelftestCheckResult::new("fail_a", false),
        SelftestCheckResult::new("fail_b", false),
    ];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 0);
    assert_eq!(failed, 2);
    assert_eq!(total, 2);
}

#[test]
fn test_selftest_check_result_not_equal_when_name_differs() {
    let a = SelftestCheckResult::new("check_alpha", true);
    let b = SelftestCheckResult::new("check_beta", true);
    assert_ne!(a, b, "different names must not be equal");
}

#[test]
fn test_selftest_check_result_not_equal_when_pass_flag_differs() {
    let a = SelftestCheckResult::new("same_name", true);
    let b = SelftestCheckResult::new("same_name", false);
    assert_ne!(a, b, "different pass flags must not be equal");
}

#[test]
fn test_selftest_check_count_is_stable_at_expected_minimum() {
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
        crate::selftest_functions::check_version_parse_basic(),
        "check_version_parse_basic must return true in healthy runtime"
    );
}

#[test]
fn test_check_version_range_parse_returns_true() {
    assert!(
        crate::selftest_functions::check_version_range_parse(),
        "check_version_range_parse must return true in healthy runtime"
    );
}

#[test]
fn test_check_version_comparison_returns_true() {
    assert!(
        crate::selftest_functions::check_version_comparison(),
        "check_version_comparison must return true in healthy runtime"
    );
}

#[test]
fn test_check_version_range_contains_returns_true() {
    assert!(
        crate::selftest_functions::check_version_range_contains(),
        "check_version_range_contains must return true in healthy runtime"
    );
}

#[test]
fn test_check_config_loads_returns_true() {
    assert!(
        crate::selftest_functions::check_config_loads(),
        "check_config_loads must return true in healthy runtime"
    );
}

#[test]
fn test_check_package_requirement_parse_returns_true() {
    assert!(
        crate::selftest_functions::check_package_requirement_parse(),
        "check_package_requirement_parse must return true in healthy runtime"
    );
}

#[test]
fn test_check_package_requirement_satisfied_by_returns_true() {
    assert!(
        crate::selftest_functions::check_package_requirement_satisfied_by(),
        "check_package_requirement_satisfied_by must return true in healthy runtime"
    );
}

#[test]
fn test_check_package_build_fields_returns_true() {
    assert!(
        crate::selftest_functions::check_package_build_fields(),
        "check_package_build_fields must return true in healthy runtime"
    );
}

#[test]
fn test_check_rex_parse_setenv_returns_true() {
    assert!(
        crate::selftest_functions::check_rex_parse_setenv(),
        "check_rex_parse_setenv must return true in healthy runtime"
    );
}

#[test]
fn test_check_rex_parse_prepend_path_returns_true() {
    assert!(
        crate::selftest_functions::check_rex_parse_prepend_path(),
        "check_rex_parse_prepend_path must return true in healthy runtime"
    );
}

#[test]
fn test_check_rex_execute_maya_commands_returns_true() {
    assert!(
        crate::selftest_functions::check_rex_execute_maya_commands(),
        "check_rex_execute_maya_commands must return true in healthy runtime"
    );
}

#[test]
fn test_check_rex_resetenv_info_stop_returns_true() {
    assert!(
        crate::selftest_functions::check_rex_resetenv_info_stop(),
        "check_rex_resetenv_info_stop must return true in healthy runtime"
    );
}

#[test]
fn test_check_shell_bash_generation_returns_true() {
    assert!(
        crate::selftest_functions::check_shell_bash_generation(),
        "check_shell_bash_generation must return true in healthy runtime"
    );
}

#[test]
fn test_check_suite_create_and_save_returns_true() {
    assert!(
        crate::selftest_functions::check_suite_create_and_save(),
        "check_suite_create_and_save must return true in healthy runtime"
    );
}

#[test]
fn test_check_repository_manager_create_returns_true() {
    assert!(
        crate::selftest_functions::check_repository_manager_create(),
        "check_repository_manager_create must return true in healthy runtime"
    );
}

// ─────── Cycle 135 additions ──────────────────────────────────────────

#[test]
fn test_check_shell_powershell_generation_returns_true() {
    assert!(
        crate::selftest_functions::check_shell_powershell_generation(),
        "check_shell_powershell_generation must return true in healthy runtime"
    );
}

#[test]
fn test_check_suite_load_roundtrip_returns_true() {
    assert!(
        crate::selftest_functions::check_suite_load_roundtrip(),
        "check_suite_load_roundtrip must return true in healthy runtime"
    );
}

#[test]
fn test_selftest_check_result_copy_trait_works() {
    let original = SelftestCheckResult::new("copy_test", true);
    let copied = original;
    assert_eq!(original, copied, "copied SelftestCheckResult must equal original");
}

#[test]
fn test_summarize_selftest_single_pass_entry() {
    let results = vec![SelftestCheckResult::new("only_one", true)];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 1);
    assert_eq!(failed, 0);
    assert_eq!(total, 1);
}

#[test]
fn test_summarize_selftest_single_fail_entry() {
    let results = vec![SelftestCheckResult::new("only_fail", false)];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 0);
    assert_eq!(failed, 1);
    assert_eq!(total, 1);
}

#[test]
fn test_summarize_selftest_mixed_three_entries() {
    let results = vec![
        SelftestCheckResult::new("ok1", true),
        SelftestCheckResult::new("fail1", false),
        SelftestCheckResult::new("ok2", true),
    ];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 2);
    assert_eq!(failed, 1);
    assert_eq!(total, 3);
}

#[test]
fn test_check_version_parse_basic_covers_alpha_suffix() {
    assert!(
        rez_next_version::Version::parse("1.0.0-alpha1").is_ok(),
        "alpha-suffix version must parse"
    );
}

#[test]
fn test_check_package_requirement_houdini_version_range() {
    use rez_next_package::PackageRequirement;
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
    let cfg = rez_next_common::config::RezCoreConfig::load();
    assert!(
        !cfg.version.is_empty(),
        "config version must be a non-empty string"
    );
    assert!(
        cfg.version.chars().any(|c| c.is_ascii_digit()),
        "config version must contain at least one digit, got: '{}'",
        cfg.version
    );
}

// ── Cycle 136 additions ───────────────────────────────────────────────────

#[test]
fn test_collect_selftest_results_count_is_positive() {
    let results = collect_selftest_results();
    assert!(
        !results.is_empty(),
        "collect_selftest_results must return at least one entry"
    );
}

#[test]
fn test_selftest_check_result_passed_field() {
    let r = SelftestCheckResult::new("my_check", true);
    assert!(r.passed, "passed field must be true when constructed with true");
}

#[test]
fn test_selftest_check_result_failed_field() {
    let r = SelftestCheckResult::new("bad_check", false);
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
