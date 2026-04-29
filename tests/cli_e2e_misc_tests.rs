//! CLI End-to-End Tests — Miscellaneous Subcommands
//!
//! Covers: depends, cp, mv, rm, complete, diff, plugins, suites,
//!         pkg-cache (daemon/status/clean/logs), pip, full workflow integration.
//!
//! Extracted from cli_e2e_tests.rs (Cycle 140).

use std::fs;
use std::process::Command;

#[path = "cli_e2e_helpers.rs"]
mod cli_e2e_helpers;

use cli_e2e_helpers::{make_test_repo, rez_ok, rez_output};

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

// ── depends ───────────────────────────────────────────────────────────────────

#[test]
fn test_depends_empty_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let (stdout, stderr, code) =
        rez_output(&["depends", "python", "--paths", tmp.path().to_str().unwrap()]);
    assert!(
        code.is_some(),
        "depends should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("No packages")
            || combined.contains("python")
            || combined.contains("Error"),
        "depends should produce meaningful output: combined={combined}"
    );
}

// ── cp / mv / rm ──────────────────────────────────────────────────────────────

#[test]
fn test_cp_package() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let src_repo = make_test_repo(tmp.path());
    let dst_repo = tmp.path().join("dest_repo");
    fs::create_dir_all(&dst_repo).unwrap();

    let (stdout, stderr, code) = rez_output(&[
        "cp",
        "python-3.9.0",
        dst_repo.to_str().unwrap(),
        "--src-path",
        src_repo.to_str().unwrap(),
    ]);
    assert!(
        code.is_some(),
        "cp should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    if code == Some(0) {
        assert!(
            stdout.contains("copied") || stdout.contains("Successfully"),
            "cp success message should mention 'copied': {stdout}"
        );
        assert!(
            dst_repo.join("python").join("3.9.0").exists(),
            "cp should create the destination version directory"
        );
    } else {
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("Error") || combined.contains("error"),
            "cp failure should report an error: combined={combined}"
        );
    }
}

#[test]
fn test_rm_nonexistent_package() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez_ok(&[
        "rm",
        "nonexistent_xyz",
        "--paths",
        tmp.path().to_str().unwrap(),
    ]);
    assert!(
        out.contains("No packages found")
            || out.contains("not found")
            || out.contains("nonexistent_xyz"),
        "rm should report that no matching package was found: {out}"
    );
}

// ── complete ──────────────────────────────────────────────────────────────────

#[test]
fn test_complete_bash_script() {
    skip_no_bin!();
    let (stdout, stderr, code) = rez_output(&["complete", "--shell", "bash", "--print-script"]);
    assert!(
        code.is_some(),
        "complete --print-script should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    if code == Some(0) {
        assert!(
            stdout.contains("bash completion") || stdout.contains("_rez"),
            "bash completion script should define a completion function: {stdout}"
        );
        assert!(
            stdout.contains("search") || stdout.contains("solve") || stdout.contains("build"),
            "bash completion script should list rez subcommands: {stdout}"
        );
    }
}

#[test]
fn test_complete_help() {
    skip_no_bin!();
    let out = rez_ok(&["complete", "--help"]);
    assert!(out.contains("shell") || out.contains("complete"));
}

// ── diff ──────────────────────────────────────────────────────────────────────

#[test]
fn test_diff_help() {
    skip_no_bin!();
    let out = rez_ok(&["diff", "--help"]);
    assert!(out.contains("diff") || out.contains("compare"));
}

// ── plugins ───────────────────────────────────────────────────────────────────

#[test]
fn test_plugins_list() {
    skip_no_bin!();
    let out = rez_ok(&["plugins"]);
    assert!(
        !out.contains('\0'),
        "plugins output should not contain NUL bytes: {out}"
    );
}

// ── suites ────────────────────────────────────────────────────────────────────

#[test]
fn test_suites_help() {
    skip_no_bin!();
    let out = rez_ok(&["suites", "--help"]);
    assert!(
        !out.trim().is_empty(),
        "suites --help should produce output"
    );
    assert!(
        out.contains("suite") || out.contains("Suite"),
        "suites --help should mention 'suite': {out}"
    );
}

// ── pkg-cache ─────────────────────────────────────────────────────────────────

#[test]
fn test_pkg_cache_help() {
    skip_no_bin!();
    let out = rez_ok(&["pkg-cache", "--help"]);
    assert!(
        !out.trim().is_empty(),
        "pkg-cache --help should produce output"
    );
    assert!(
        out.contains("cache") || out.contains("Cache"),
        "pkg-cache --help should mention 'cache': {out}"
    );
}

// ── pip ───────────────────────────────────────────────────────────────────────

#[test]
fn test_pip_help() {
    skip_no_bin!();
    let out = rez_ok(&["pip", "--help"]);
    assert!(!out.trim().is_empty(), "pip --help should produce output");
    assert!(
        out.contains("pip") || out.contains("install") || out.contains("package"),
        "pip --help should mention pip-related terms: {out}"
    );
}

// ── real repo workflow (integration) ─────────────────────────────────────────

#[test]
fn test_full_workflow_search_and_view() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let repo_str = repo.to_str().unwrap();

    let search_out = rez_ok(&["search", "python", "--repository", repo_str]);
    assert!(
        search_out.contains("python") || search_out.contains("No"),
        "search should mention python"
    );

    let (view_stdout, view_stderr, view_code) = rez_output(&["view", "python", "--path", repo_str]);
    assert!(
        view_code.is_some(),
        "view should exit with a code, not be killed by signal: stdout={view_stdout} stderr={view_stderr}"
    );
    let view_combined = format!("{view_stdout}{view_stderr}");
    assert!(
        !view_combined.trim().is_empty(),
        "view should produce some output: combined={view_combined}"
    );

    let (solve_stdout, solve_stderr, solve_code) =
        rez_output(&["solve", "python", "--repository", repo_str]);
    assert!(
        solve_code.is_some(),
        "solve should exit with a code: stdout={solve_stdout} stderr={solve_stderr}"
    );
    let solve_combined = format!("{solve_stdout}{solve_stderr}");
    assert!(
        solve_combined.contains("python") || solve_combined.contains("Resolution"),
        "solve output should mention python or resolution: combined={solve_combined}"
    );
}

#[test]
fn test_full_workflow_bundle_roundtrip() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let bundle_dir = tmp.path().join("my_bundle");

    rez_ok(&[
        "bundle",
        "python-3.9",
        "maya-2024",
        bundle_dir.to_str().unwrap(),
    ]);
    assert!(bundle_dir.join("bundle.yaml").exists());

    let content = fs::read_to_string(bundle_dir.join("bundle.yaml")).unwrap();
    assert!(content.contains("python-3.9"));
    assert!(content.contains("maya-2024"));
}

// ── pkg-cache daemon ──────────────────────────────────────────────────────────

#[test]
fn test_pkg_cache_status_empty_dir() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez_ok(&["pkg-cache", tmp.path().to_str().unwrap()]);
    assert!(
        out.contains("Cache") || out.contains("cache"),
        "pkg-cache status should include 'Cache' in output: {out}"
    );
    assert!(
        out.contains("entries") || out.contains("No cached"),
        "pkg-cache status should report entry count or empty cache: {out}"
    );
}

#[test]
fn test_pkg_cache_clean_empty_dir() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez_ok(&["pkg-cache", tmp.path().to_str().unwrap(), "--clean"]);
    assert!(
        out.contains("cleaning") || out.contains("Cleaning") || out.contains("completed"),
        "pkg-cache --clean should report cleaning activity: {out}"
    );
    assert!(
        out.contains("0"),
        "pkg-cache --clean on empty dir should report 0 entries: {out}"
    );
}

#[test]
fn test_pkg_cache_logs_no_log_file() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez_ok(&["pkg-cache", tmp.path().to_str().unwrap(), "--logs"]);
    assert!(
        out.contains("No cache logs") || out.contains("logs"),
        "should report no logs found: {out}"
    );
}

// ── build extra-args passthrough ──────────────────────────────────────────────

#[test]
fn test_build_without_package_py() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = std::process::Command::new(cli_e2e_helpers::rez_next_bin())
        .args(["build"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.code().is_some(),
        "build in empty dir should exit with a code, not crash"
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "build in dir without package.py should fail: stdout={stdout} stderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("package")
            || combined.contains("Error")
            || combined.contains("not found"),
        "build failure should mention package.py or an error: combined={combined}"
    );
}

#[test]
fn test_build_extra_args_separator_accepted() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("package.py"),
        "name = \"test_build_pkg\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    let out = Command::new(cli_e2e_helpers::rez_next_bin())
        .args(["build", "--", "--dry-run", "--verbose"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.code().is_some(),
        "build with -- extra args should exit with a code: stdout={stdout} stderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.contains("No package.py or package.yaml found in current directory"),
        "build should discover the temporary package.py: combined={combined}"
    );
    assert!(
        !combined.trim().is_empty(),
        "build should produce output when given -- extra args: combined={combined}"
    );
}
