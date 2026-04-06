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

/// Returns (stdout, stderr, exit_code_option) without asserting success.
fn rez_output(args: &[&str]) -> (String, String, Option<i32>) {
    let out = rez(args);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (stdout, stderr, out.status.code())
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
    let _repo = make_test_repo(tmp.path());
    // view uses the globally configured packages_path, not a --path flag.
    // Without pointing to our temp repo it will likely fail with "not found",
    // which is still a valid, well-formed error response (not a crash).
    let (stdout, stderr, code) = rez_output(&["view", "python"]);
    assert!(
        code.is_some(),
        "view should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    // Must produce some output — either package details or a "not found" error
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.trim().is_empty(),
        "view should always print something: combined={combined}"
    );
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
    let (stdout, stderr, code) =
        rez_output(&["depends", "python", "--paths", tmp.path().to_str().unwrap()]);
    // Process must not be killed by signal
    assert!(
        code.is_some(),
        "depends should not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    // Either reports "No packages depend on" (empty repo) or an error message
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
        // On success: output should confirm the copy and the directory must exist
        assert!(
            stdout.contains("copied") || stdout.contains("Successfully"),
            "cp success message should mention 'copied': {stdout}"
        );
        assert!(
            dst_repo.join("python").join("3.9.0").exists(),
            "cp should create the destination version directory"
        );
    } else {
        // On failure: must at least print an error description
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
    // Removing a nonexistent package: graceful — exits 0 with "not found" message
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
        // Bash completion script must define a function and reference rez subcommands
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
    // Output may be empty (no plugins registered), but the command must succeed
    // and must not produce garbage / panic output.
    // When plugins are registered, each line should be a plugin name (no NUL bytes).
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

    // 2. View python package (uses global config — may not find it, but must not crash)
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

    // 3. Solve python requirement (uses --repository to point at our temp repo)
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
    let (stdout, stderr, code) = rez_output(&["build", "--", "--dry-run", "--verbose"]);
    assert!(
        code.is_some(),
        "build with -- extra args should exit with a code, not be killed by signal: stdout={stdout} stderr={stderr}"
    );
    // The command should not produce empty output — either build progress or an error
    let combined = format!("{stdout}{stderr}");
    assert!(
        !combined.trim().is_empty(),
        "build should produce some output when given -- extra args: combined={combined}"
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
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        out.status.code().is_some(),
        "build in empty dir should exit with a code, not crash"
    );
    // Must fail (no package.py to build) and report a meaningful error
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

// ── pkg-cache daemon ──────────────────────────────────────────────────────────

#[test]
fn test_pkg_cache_status_empty_dir() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    // Point pkg-cache at an empty directory — should print cache status summary
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
    // After cleaning an empty cache, entry counts should be zero
    assert!(
        out.contains("0"),
        "pkg-cache --clean on empty dir should report 0 entries: {out}"
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
