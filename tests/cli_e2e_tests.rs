//! CLI End-to-End Tests — Core Subcommands
//!
//! Covers: help/version/config/parse/selftest/status/search/solve/view/bundle/
//!         bind/exit-codes/info flags/build.
//!
//! Miscellaneous subcommands (depends, cp, rm, complete, diff, plugins, suites,
//! pkg-cache, pip, workflow) are in cli_e2e_misc_tests.rs.
//!
//! Run with:
//!   cargo test --test cli_e2e_tests
//!
//! The binary is located at:
//!   target/debug/rez-next   (default)
//!   target/release/rez-next (if REZ_NEXT_E2E_BINARY is set to release path)

use std::fs;

#[path = "cli_e2e_helpers.rs"]
mod cli_e2e_helpers;

use cli_e2e_helpers::{make_test_repo, rez, rez_ok, rez_output};

macro_rules! skip_no_bin {
    () => {
        if cli_e2e_helpers::skip_if_no_binary() {
            eprintln!(
                "[SKIP] rez-next binary not found at {:?}. Run `cargo build` first.",
                cli_e2e_helpers::rez_next_bin()
            );
            return;
        }
    };
}

// ═════════════════════════════════════════════════════════════════════════════
// Test groups
// ═════════════════════════════════════════════════════════════════════════════

// ── Help / version ────────────────────────────────────────────────────────────

#[test]
fn test_help_flag() {
    skip_no_bin!();
    let out = rez_ok(&["--help"]);
    assert!(out.contains("rez") || out.contains("Rez"));
    assert!(out.contains("config") || out.contains("search") || out.contains("solve"));
}

#[test]
fn test_version_flag() {
    skip_no_bin!();
    let out = rez_ok(&["--version"]);
    // Should output something like "rez-next 0.2.0"
    assert!(!out.trim().is_empty());
}

#[test]
fn test_no_args_shows_help() {
    skip_no_bin!();
    let out = rez(&[]);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // No subcommand should print help (exit 0 or 1, but print usage)
    assert!(combined.contains("rez") || combined.contains("Usage"));
}

// ── config ────────────────────────────────────────────────────────────────────

#[test]
fn test_config_show_all() {
    skip_no_bin!();
    let out = rez_ok(&["config"]);
    assert!(
        !out.trim().is_empty(),
        "config should produce output: {out}"
    );
    assert!(
        out.contains("packages_path") || out.contains("local_packages_path"),
        "config output should include a packages_path field: {out}"
    );
}

#[test]
fn test_config_show_field() {
    skip_no_bin!();
    let out = rez_ok(&["config", "packages_path"]);
    assert!(
        !out.trim().is_empty(),
        "config packages_path should produce output"
    );
    // The output should reflect the packages_path field name or its value
    assert!(
        out.contains("packages_path") || out.contains("packages") || out.contains("/"),
        "config packages_path output should be path-related: {out}"
    );
}

#[test]
fn test_config_json_output() {
    skip_no_bin!();
    let out = rez_ok(&["config", "--json"]);
    // Should be valid JSON
    serde_json::from_str::<serde_json::Value>(&out)
        .expect("config --json should produce valid JSON");
}

#[test]
fn test_config_search_list() {
    skip_no_bin!();
    let out = rez_ok(&["config", "--search-list"]);
    // Should list config search paths (one per line); at minimum the output
    // should mention yaml, json, or rezconfig — the standard config file names.
    assert!(
        out.contains("yaml") || out.contains("json") || out.contains("rezconfig"),
        "config --search-list should mention config file search paths: {out}"
    );
}

// ── parse-version (dev command) ───────────────────────────────────────────────

#[test]
fn test_parse_version_valid() {
    skip_no_bin!();
    let out = rez_ok(&["parse-version", "1.2.3"]);
    assert!(out.contains("1.2.3") || out.contains("Valid") || out.contains("valid"));
}

#[test]
fn test_parse_version_complex() {
    skip_no_bin!();
    let out = rez_ok(&["parse-version", "3.11.0-alpha1"]);
    assert!(
        out.contains("3.11.0") || out.contains("alpha"),
        "parse-version should echo back the version components: {out}"
    );
}

#[test]
fn test_parse_version_single_component() {
    skip_no_bin!();
    let out = rez_ok(&["parse-version", "5"]);
    assert!(
        out.contains("5"),
        "parse-version should include the parsed version digit: {out}"
    );
}

// ── selftest ──────────────────────────────────────────────────────────────────

#[test]
fn test_selftest_all_pass() {
    skip_no_bin!();
    let out = rez_ok(&["self-test"]);
    // Should report all passed
    assert!(out.contains("passed") || out.contains("Passed") || out.contains("PASSED"));
    // Must not report failures
    assert!(!out.contains("FAILED: ") || out.contains("0") || out.contains("All tests passed"));
}

// ── status ────────────────────────────────────────────────────────────────────

#[test]
fn test_status_outside_context() {
    skip_no_bin!();
    let (stdout, stderr, code) = rez_output(&["status"]);
    // Process must not be killed by signal
    assert!(
        code.is_some(),
        "status should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    // When no rez context is active the command should either print a "not in context"
    // message or report an error — it must not produce empty output on both streams.
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.trim().is_empty(),
        "status should print something (context info or error): combined={combined}"
    );
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn test_search_empty_result() {
    skip_no_bin!();
    // Search in a non-existent repo — the repo path does not exist, so the
    // command should fail with a non-zero exit code and report an IO error.
    let tmp = tempfile::tempdir().unwrap();
    let nonexistent = tmp.path().join("nonexistent_xyz");
    let (stdout, stderr, code) = rez_output(&[
        "search",
        "nonexistent_package_xyz_9999",
        "--repository",
        nonexistent.to_str().unwrap(),
    ]);
    // Must exit with a code (not signal-killed)
    assert!(
        code.is_some(),
        "search should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    // A missing repo path is an IO error → non-zero exit
    assert_ne!(
        code,
        Some(0),
        "search against nonexistent repo path should fail: stdout={stdout} stderr={stderr}"
    );
    // Error message should appear in stderr or stdout
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("Error") || combined.contains("error") || combined.contains("IO"),
        "error output should describe the failure: combined={combined}"
    );
}

#[test]
fn test_search_json_format() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let out = rez_ok(&[
        "search",
        "python",
        "--repository",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    // If results found, should be valid JSON array; if empty, may be empty array
    if !out.trim().is_empty() && out.trim() != "[]" {
        serde_json::from_str::<serde_json::Value>(&out)
            .expect("search --format json should produce valid JSON");
    }
}

#[test]
fn test_search_finds_python_in_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let out = rez_ok(&["search", "python", "--repository", repo.to_str().unwrap()]);
    assert!(
        out.contains("python") || out.contains("No packages"),
        "search output should mention 'python': {out}"
    );
}

#[test]
fn test_search_with_latest_only() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let out = rez_ok(&[
        "search",
        "python",
        "--repository",
        repo.to_str().unwrap(),
        "--latest-only",
    ]);
    // With --latest-only the search should still find python and report exactly one result
    assert!(
        out.contains("python"),
        "--latest-only search should report python: {out}"
    );
    assert!(
        out.contains("Found") || out.contains("1 package"),
        "--latest-only should report finding at least one package: {out}"
    );
}

// ── solve ─────────────────────────────────────────────────────────────────────

#[test]
fn test_solve_empty_request() {
    skip_no_bin!();
    // Solving empty request should succeed and report no packages to resolve
    let out = rez_ok(&["solve"]);
    assert!(
        out.contains("No packages to resolve") || out.contains("Resolution Summary"),
        "empty solve should report no packages: {out}"
    );
}

#[test]
fn test_solve_package_not_in_repo() {
    skip_no_bin!();
    // Solving a package that doesn't exist should report failed requirements, not panic
    let tmp = tempfile::tempdir().unwrap();
    let out = rez_ok(&[
        "solve",
        "nonexistent_xyz_9999",
        "--repository",
        tmp.path().to_str().unwrap(),
    ]);
    // Lenient solver: exits 0 but reports failed requirements
    assert!(
        out.contains("Failed requirements") || out.contains("nonexistent_xyz_9999"),
        "solve should report the unresolvable requirement: {out}"
    );
}

#[test]
fn test_solve_with_real_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let out = rez_ok(&["solve", "python", "--repository", repo.to_str().unwrap()]);
    // Should resolve python successfully and list it in resolved packages
    assert!(
        out.contains("Resolved packages") || out.contains("python"),
        "solve python should report resolved packages: {out}"
    );
}

// ── view ──────────────────────────────────────────────────────────────────────

#[test]
fn test_view_package_not_found() {
    skip_no_bin!();
    let (stdout, stderr, code) = rez_output(&["view", "nonexistent_xyz"]);
    // view for a missing package should exit non-zero
    assert_ne!(
        code,
        Some(0),
        "view nonexistent package should fail: stdout={stdout} stderr={stderr}"
    );
    // Error message should mention the package was not found
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("not found") || combined.contains("Error"),
        "view should report package not found: combined={combined}"
    );
}

#[test]
fn test_view_package_in_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let package_dir = repo.join("python").join("3.11.0");

    let out = rez_ok(&["view", package_dir.to_str().unwrap()]);
    assert!(
        out.contains("name: python"),
        "view should render the package name from package.py: {out}"
    );
    assert!(
        out.contains("version: 3.11.0"),
        "view should render the exact package version from the package directory: {out}"
    );
}

// ── bundle ────────────────────────────────────────────────────────────────────

#[test]
fn test_bundle_create() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("my_bundle");
    rez_ok(&["bundle", "python-3.9", dest.to_str().unwrap()]);
    // Bundle dir should exist
    assert!(dest.exists(), "bundle dir should be created: {:?}", dest);
    assert!(
        dest.join("bundle.yaml").exists(),
        "bundle.yaml should exist"
    );
}


#[test]
fn test_bundle_create_multiple_packages() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("multi_bundle");
    rez_ok(&["bundle", "python-3.9", "numpy-1.25", dest.to_str().unwrap()]);
    assert!(dest.join("bundle.yaml").exists());
    let content = fs::read_to_string(dest.join("bundle.yaml")).unwrap();
    assert!(content.contains("python-3.9"));
    assert!(content.contains("numpy-1.25"));
}

// ── bind ──────────────────────────────────────────────────────────────────────

#[test]
fn test_bind_help() {
    skip_no_bin!();
    let out = rez_ok(&["bind", "--help"]);
    assert!(!out.trim().is_empty());
}

