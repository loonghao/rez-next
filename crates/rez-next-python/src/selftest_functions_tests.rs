//! Unit tests for selftest_functions.
//!
//! Keep this suite focused on public/selftest contracts rather than repeating
//! every individual helper's implementation detail.

use std::collections::HashSet;

use crate::selftest_functions::{
    collect_selftest_results, selftest, selftest_verbose, summarize_selftest_results,
    SelftestCheckResult,
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

#[test]
fn test_summarize_selftest_empty_list_returns_zero_triple() {
    let (passed, failed, total) = summarize_selftest_results(&[]);
    assert_eq!(passed, 0, "no checks → 0 passed");
    assert_eq!(failed, 0, "no checks → 0 failed");
    assert_eq!(total, 0, "no checks → total 0");
}

#[test]
fn test_summarize_selftest_counts_failures_correctly() {
    let results = vec![
        SelftestCheckResult::new("ok_check", true),
        SelftestCheckResult::new("fail_check", false),
        SelftestCheckResult::new("ok_check_2", true),
    ];
    let (passed, failed, total) = summarize_selftest_results(&results);
    assert_eq!(passed, 2);
    assert_eq!(failed, 1);
    assert_eq!(total, 3);
}

#[test]
fn test_selftest_check_result_debug_contains_name() {
    let result = SelftestCheckResult::new("my_check", false);
    let formatted = format!("{:?}", result);
    assert!(
        formatted.contains("my_check"),
        "Debug output must contain the check name, got: {formatted}"
    );
}

#[test]
fn test_selftest_verbose_count_matches_collect() {
    let verbose = selftest_verbose().expect("selftest_verbose must not error");
    let results = collect_selftest_results();
    assert_eq!(
        verbose.len(),
        results.len(),
        "selftest_verbose() must return one entry per check"
    );
}

#[test]
fn test_selftest_verbose_names_and_flags_match_collect() {
    let verbose = selftest_verbose().expect("selftest_verbose must not error");
    let results = collect_selftest_results();
    for ((name, passed), result) in verbose.iter().zip(results.iter()) {
        assert_eq!(name, result.name, "verbose name must match collect name");
        assert_eq!(*passed, result.passed, "verbose flag must match collect flag for '{name}'");
    }
}
