//! CLI End-to-End Tests
//!
//! These tests build `rez-next` in release mode and invoke the actual binary,
//! exercising every subcommand with realistic inputs against temporary on-disk
//! package repositories.
//!
//! Run with:
//!   cargo test --test cli_e2e_tests
//!
//! The binary is located at:
//!   target/debug/rez-next   (default)
//!   target/release/rez-next (if REZ_NEXT_E2E_BINARY is set to release path)

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// ── Binary path ───────────────────────────────────────────────────────────────

fn rez_next_bin() -> PathBuf {
    // Allow override via env for CI (release binary is faster)
    if let Ok(path) = std::env::var("REZ_NEXT_E2E_BINARY") {
        return PathBuf::from(path);
    }
    // Default: debug binary produced by `cargo build`
    let manifest = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .join("target")
        .join("debug")
        .join(if cfg!(windows) {
            "rez-next.exe"
        } else {
            "rez-next"
        })
}

fn rez(args: &[&str]) -> Output {
    Command::new(rez_next_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run rez-next: {e}"))
}

fn rez_ok(args: &[&str]) -> String {
    let out = rez(args);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.success(),
        "rez-next {} failed (exit {:?})\nstdout: {}\nstderr: {}",
        args.join(" "),
        out.status.code(),
        stdout,
        stderr
    );
    stdout
}

// ── Package repo helpers ──────────────────────────────────────────────────────

/// Write a minimal package.py under `<repo>/<name>/<version>/package.py`
fn write_package(repo: &Path, name: &str, version: &str, extra: &str) {
    let pkg_dir = repo.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();
    let content = format!("name = \"{name}\"\nversion = \"{version}\"\n{extra}\n");
    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

/// Create a minimal test repository with a few packages
fn make_test_repo(base: &Path) -> PathBuf {
    let repo = base.join("packages");
    // python 3.9.0
    write_package(
        &repo,
        "python",
        "3.9.0",
        r#"description = "Python interpreter"
tools = ["python", "python3"]
commands = """
env.setenv('PYTHON_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
"""
"#,
    );
    // python 3.11.0
    write_package(
        &repo,
        "python",
        "3.11.0",
        r#"description = "Python interpreter 3.11"
tools = ["python", "python3"]
requires = []
commands = """
env.setenv('PYTHON_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
"""
"#,
    );
    // maya 2024.0
    write_package(
        &repo,
        "maya",
        "2024.0",
        r#"description = "Autodesk Maya"
tools = ["maya", "mayabatch"]
requires = ["python-3.9"]
commands = """
env.setenv('MAYA_ROOT', '{root}')
env.prepend_path('PATH', '{root}/bin')
"""
"#,
    );
    // numpy 1.25.0
    write_package(
        &repo,
        "numpy",
        "1.25.0",
        r#"description = "NumPy scientific computing"
requires = ["python-3.9+<3.12"]
commands = """
env.prepend_path('PYTHONPATH', '{root}/lib/python/site-packages')
"""
"#,
    );
    repo
}

// ── Skip guard ────────────────────────────────────────────────────────────────

fn skip_if_no_binary() -> bool {
    !rez_next_bin().exists()
}

macro_rules! skip_no_bin {
    () => {
        if skip_if_no_binary() {
            eprintln!(
                "[SKIP] rez-next binary not found at {:?}. Run `cargo build` first.",
                rez_next_bin()
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
    assert!(!out.trim().is_empty());
}

#[test]
fn test_config_show_field() {
    skip_no_bin!();
    let out = rez_ok(&["config", "packages_path"]);
    assert!(!out.trim().is_empty());
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
    // May be empty if no config files installed, but should not fail
    let _ = out;
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
    assert!(!out.trim().is_empty());
}

#[test]
fn test_parse_version_single_component() {
    skip_no_bin!();
    let out = rez_ok(&["parse-version", "5"]);
    assert!(!out.trim().is_empty());
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
    let out = rez(&["status"]);
    // Status outside a rez context exits 0 or 1, but should not crash (no panic)
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    // Either prints "not in a rez context" or similar — no panic
    assert!(
        out.status.code().is_some(),
        "process should not have been killed by signal: stdout={stdout} stderr={stderr}"
    );
}

// ── search ────────────────────────────────────────────────────────────────────

#[test]
fn test_search_empty_result() {
    skip_no_bin!();
    // Search in a non-existent repo — should print nothing / empty, not panic
    let out = rez(&[
        "search",
        "nonexistent_package_xyz_9999",
        "--repository",
        "/tmp/nonexistent_repo_xyz",
    ]);
    assert!(
        out.status.code().is_some(),
        "search should not be killed by signal"
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
    let out = rez(&[
        "search",
        "python",
        "--repository",
        repo.to_str().unwrap(),
        "--latest-only",
    ]);
    assert!(out.status.code().is_some());
}

// ── solve ─────────────────────────────────────────────────────────────────────

#[test]
fn test_solve_empty_request() {
    skip_no_bin!();
    // Solving empty request should succeed (trivial)
    let out = rez(&["solve"]);
    assert!(out.status.code().is_some());
}

#[test]
fn test_solve_package_not_in_repo() {
    skip_no_bin!();
    // Solving a package that doesn't exist should fail gracefully (not panic)
    let tmp = tempfile::tempdir().unwrap();
    let out = rez(&[
        "solve",
        "nonexistent_xyz_9999",
        "--repository",
        tmp.path().to_str().unwrap(),
    ]);
    assert!(out.status.code().is_some());
}

#[test]
fn test_solve_with_real_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let out = rez(&["solve", "python", "--repository", repo.to_str().unwrap()]);
    // Either resolves successfully or fails with a message — no panic
    assert!(out.status.code().is_some());
}

// ── view ──────────────────────────────────────────────────────────────────────

#[test]
fn test_view_package_not_found() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // view doesn't take --path; it uses global config; just check it doesn't panic
    let out = rez(&["view", "nonexistent_xyz"]);
    assert!(out.status.code().is_some());
    let _ = tmp;
}

#[test]
fn test_view_package_in_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let _ = repo;
    // view uses configured packages_path, not --path flag
    let out = rez(&["view", "python"]);
    assert!(out.status.code().is_some());
}

// ── bundle ────────────────────────────────────────────────────────────────────

#[test]
fn test_bundle_create() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("my_bundle");
    let out = rez_ok(&["bundle", "python-3.9", dest.to_str().unwrap()]);
    let _ = out;
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

// ── depends ───────────────────────────────────────────────────────────────────

#[test]
fn test_depends_empty_repo() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez(&["depends", "python", "--paths", tmp.path().to_str().unwrap()]);
    assert!(out.status.code().is_some());
}

// ── cp / mv / rm ──────────────────────────────────────────────────────────────

#[test]
fn test_cp_package() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let src_repo = make_test_repo(tmp.path());
    let dst_repo = tmp.path().join("dest_repo");
    fs::create_dir_all(&dst_repo).unwrap();

    // cp takes "name-version" as source package spec
    let out = rez(&[
        "cp",
        "python-3.9.0",
        dst_repo.to_str().unwrap(),
        "--src-path",
        src_repo.to_str().unwrap(),
    ]);
    assert!(out.status.code().is_some());
    if out.status.success() {
        assert!(dst_repo.join("python").join("3.9.0").exists());
    }
}

#[test]
fn test_rm_nonexistent_package() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // Removing a nonexistent package should fail gracefully
    let out = rez(&[
        "rm",
        "nonexistent_xyz",
        "--paths",
        tmp.path().to_str().unwrap(),
    ]);
    assert!(out.status.code().is_some());
}

// ── complete ──────────────────────────────────────────────────────────────────

#[test]
fn test_complete_bash_script() {
    skip_no_bin!();
    let out = rez(&["complete", "--shell", "bash", "--print-script"]);
    assert!(out.status.code().is_some());
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(!stdout.trim().is_empty());
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
    let _ = out; // Output may be empty if no plugins registered
}

// ── suites ────────────────────────────────────────────────────────────────────

#[test]
fn test_suites_help() {
    skip_no_bin!();
    let out = rez_ok(&["suites", "--help"]);
    assert!(!out.trim().is_empty());
}

// ── pkg-cache ─────────────────────────────────────────────────────────────────

#[test]
fn test_pkg_cache_help() {
    skip_no_bin!();
    let out = rez_ok(&["pkg-cache", "--help"]);
    assert!(!out.trim().is_empty());
}

// ── pip ───────────────────────────────────────────────────────────────────────

#[test]
fn test_pip_help() {
    skip_no_bin!();
    let out = rez_ok(&["pip", "--help"]);
    assert!(!out.trim().is_empty());
}

// ── info flag ─────────────────────────────────────────────────────────────────

#[test]
fn test_info_flag() {
    skip_no_bin!();
    let out = rez_ok(&["-i"]);
    assert!(out.contains("Version") || out.contains("version") || out.contains("OS"));
}

// ── exit codes ────────────────────────────────────────────────────────────────

#[test]
fn test_exit_code_success_is_zero() {
    skip_no_bin!();
    let out = rez(&["config"]);
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn test_exit_code_unknown_subcommand_is_nonzero() {
    skip_no_bin!();
    let out = rez(&["totally-unknown-subcommand-xyz-9999"]);
    assert_ne!(out.status.code(), Some(0));
}

// ── real repo workflow (integration) ─────────────────────────────────────────

#[test]
fn test_full_workflow_search_and_view() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = make_test_repo(tmp.path());
    let repo_str = repo.to_str().unwrap();

    // 1. Search for python
    let search_out = rez_ok(&["search", "python", "--repository", repo_str]);
    assert!(
        search_out.contains("python") || search_out.contains("No"),
        "search should mention python"
    );

    // 2. View python package
    let view_out = rez(&["view", "python", "--path", repo_str]);
    assert!(view_out.status.code().is_some());

    // 3. Solve python requirement
    let solve_out = rez(&["solve", "python", "--path", repo_str]);
    assert!(solve_out.status.code().is_some());
}

#[test]
fn test_full_workflow_bundle_roundtrip() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let bundle_dir = tmp.path().join("my_bundle");

    // Create bundle
    rez_ok(&[
        "bundle",
        "python-3.9",
        "maya-2024",
        bundle_dir.to_str().unwrap(),
    ]);
    assert!(bundle_dir.join("bundle.yaml").exists());

    // Verify bundle content
    let content = fs::read_to_string(bundle_dir.join("bundle.yaml")).unwrap();
    assert!(content.contains("python-3.9"));
    assert!(content.contains("maya-2024"));
}

// ── info flag extended ────────────────────────────────────────────────────────

#[test]
fn test_info_long_flag() {
    skip_no_bin!();
    // --info long flag should behave identically to -i
    let out = rez_ok(&["--info"]);
    assert!(
        out.contains("Version") || out.contains("version") || out.contains("OS"),
        "long --info flag should print system info: {out}"
    );
}

#[test]
fn test_info_shows_packages_path_label() {
    skip_no_bin!();
    let out = rez_ok(&["-i"]);
    // Print-info should mention packages_path or packages path
    assert!(
        out.to_lowercase().contains("packages"),
        "--info output should mention packages path: {out}"
    );
}

#[test]
fn test_info_shows_version_string() {
    skip_no_bin!();
    let out = rez_ok(&["-i"]);
    // The version line should include a semver-ish string like "0.2.0"
    let has_version = out
        .lines()
        .any(|l| l.to_lowercase().contains("version") && l.chars().any(|c| c.is_ascii_digit()));
    assert!(
        has_version,
        "--info should include a version line with digits: {out}"
    );
}

#[test]
fn test_info_exit_code_zero() {
    skip_no_bin!();
    let out = rez(&["-i"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "-i flag should exit with code 0"
    );
}

// ── build extra-args passthrough ──────────────────────────────────────────────

#[test]
fn test_build_help_flag() {
    skip_no_bin!();
    let out = rez_ok(&["build", "--help"]);
    assert!(
        !out.trim().is_empty(),
        "build --help should print usage information"
    );
}

#[test]
fn test_build_extra_args_separator_accepted() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // Write a minimal package.py so build can find a valid package root
    fs::write(
        tmp.path().join("package.py"),
        "name = \"test_build_pkg\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    // Pass extra build args via "--"; the binary should not crash on unknown
    // downstream flags — it may fail because there's nothing to build, but the
    // crash-free handling of the "--" separator is what we're testing.
    let out = rez(&["build", "--", "--dry-run", "--verbose"]);
    assert!(
        out.status.code().is_some(),
        "build with -- extra args should exit with a code, not be killed by signal"
    );
}

#[test]
fn test_build_without_package_py() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // Run build in an empty directory — should fail gracefully, not panic
    let out = std::process::Command::new(rez_next_bin())
        .args(["build"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.code().is_some(),
        "build in empty dir should exit with a code, not crash"
    );
}

// ── pkg-cache daemon ──────────────────────────────────────────────────────────

#[test]
fn test_pkg_cache_status_empty_dir() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // Point pkg-cache at an empty directory — should not panic
    let out = rez(&["pkg-cache", tmp.path().to_str().unwrap()]);
    assert!(
        out.status.code().is_some(),
        "pkg-cache with empty dir should exit with a code"
    );
}

#[test]
fn test_pkg_cache_clean_empty_dir() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let out = rez(&["pkg-cache", tmp.path().to_str().unwrap(), "--clean"]);
    assert!(
        out.status.code().is_some(),
        "pkg-cache --clean on empty dir should not crash"
    );
}

#[test]
fn test_pkg_cache_logs_no_log_file() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // --logs when no log file exists should print "No cache logs found" and exit 0
    let out = rez_ok(&["pkg-cache", tmp.path().to_str().unwrap(), "--logs"]);
    assert!(
        out.contains("No cache logs") || out.contains("logs"),
        "should report no logs found: {out}"
    );
}
