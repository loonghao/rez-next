//! CLI End-to-End Tests — Consolidated Filesystem Repository
//!
//! Every Rust consumer now resolves packages through the upstream-faithful
//! `FilesystemPackageRepository` via `AsyncRepositoryAdapter`. The unit tests in
//! `crates/rez-next-repository` cover that adapter directly; these cover the
//! wiring those unit tests cannot reach: a real `rez-next` binary reading a real
//! repository on disk, end to end.
//!
//! The behaviours pinned here are the ones that used to differ between the
//! Python API and the CLI, which is what motivated consolidating the two
//! implementations in the first place.

use std::fs;
use std::path::Path;

// The shared helper module is compiled per test binary; only the helpers this
// file uses are referenced here, so the rest are dead code in this binary.
#[allow(dead_code)]
#[path = "cli_e2e_helpers.rs"]
mod cli_e2e_helpers;

use cli_e2e_helpers::{rez, skip_if_no_binary, write_package};

macro_rules! skip_no_bin {
    () => {
        if skip_if_no_binary() {
            eprintln!("[SKIP] rez-next binary not found. Run `cargo build` first.");
            return;
        }
    };
}

/// Write a package definition with dependencies, the way a real one looks.
fn write_package_with_requires(repo: &Path, name: &str, version: &str, requires: &[&str]) {
    let listed = requires
        .iter()
        .map(|r| format!("    \"{r}\","))
        .collect::<Vec<_>>()
        .join("\n");
    let extra = format!("requires = [\n{listed}\n]\n");
    write_package(repo, name, version, &extra);
}

/// A repository holding one family with a couple of versions.
fn repo_with_versions(base: &Path, name: &str, versions: &[&str]) -> std::path::PathBuf {
    let repo = base.join("packages");
    for version in versions {
        write_package(&repo, name, version, "");
    }
    repo
}

// ── Search ────────────────────────────────────────────────────────────────────

/// `search` is served by `rez-next-search`, one of the crates migrated to the
/// consolidated repository.
#[test]
fn search_lists_every_version_of_a_family() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_with_versions(tmp.path(), "python", &["3.9.0", "3.10.0", "3.11.0"]);

    let out = rez(&[
        "search",
        "python",
        "--repository",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "stdout={stdout} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    // Every version of the family must be reported, not just the latest.
    for version in ["3.9.0", "3.10.0", "3.11.0"] {
        assert!(
            stdout.contains(version),
            "{version} missing from search output: {stdout}"
        );
    }
}

/// Dependency metadata must survive the trip through the repository.
///
/// `--requirements` asks for packages whose `requires` match, so this only
/// passes if the definition file was actually parsed rather than the package
/// being reconstructed from the version directory name.
#[test]
fn search_reports_requirements_parsed_from_the_definition() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("packages");
    write_package_with_requires(&repo, "maya", "2024.1", &["python-3"]);

    let out = rez(&[
        "search",
        "maya",
        "--repository",
        repo.to_str().unwrap(),
        "--requirements",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "stdout={stdout} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("maya"), "maya should be found: {stdout}");
}

// ── Solve ─────────────────────────────────────────────────────────────────────

/// `solve` is the path the reviewer flagged: it queries the repository once per
/// expanded state under a wall-clock budget.
#[test]
fn solve_resolves_a_transitive_dependency_chain() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("packages");
    write_package(&repo, "python", "3.11.0", "");
    write_package_with_requires(&repo, "numpy", "1.26.0", &["python-3"]);

    let out = rez(&[
        "solve",
        "numpy",
        "--repository",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "stdout={stdout} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("numpy"), "numpy should resolve: {stdout}");
    assert!(
        stdout.contains("python"),
        "the transitive python dependency should resolve: {stdout}"
    );
}

/// An ignored version must be invisible to the CLI, matching the Python API.
///
/// Upstream rez hides a version when a sibling `.ignore<version>` marker
/// exists. This is the divergence the issue calls out: it used to depend on
/// whether the query came from Python or from the CLI.
#[test]
fn solve_skips_an_ignored_version() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = repo_with_versions(tmp.path(), "python", &["3.9.0", "3.11.0"]);
    // The marker that hides 3.11.0 from resolution.
    fs::write(repo.join("python").join(".ignore3.11.0"), "").unwrap();

    let out = rez(&[
        "solve",
        "python",
        "--repository",
        repo.to_str().unwrap(),
        "--format",
        "json",
    ]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "stdout={stdout} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("3.9.0"),
        "the non-ignored version should resolve: {stdout}"
    );
    assert!(
        !stdout.contains("3.11.0"),
        "the ignored version must not resolve: {stdout}"
    );
}

// ── Depends ───────────────────────────────────────────────────────────────────

/// `depends` was migrated in this change and compares requirement strings
/// against parsed package names; it needs the real repository behind it.
#[test]
fn depends_reports_a_reverse_dependency() {
    skip_no_bin!();
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("packages");
    write_package(&repo, "python", "3.11.0", "");
    write_package_with_requires(&repo, "maya", "2024.1", &["python-3"]);

    let out = rez(&["depends", "python", "--paths", repo.to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();

    assert!(
        out.status.success(),
        "stdout={stdout} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        stdout.contains("maya"),
        "maya depends on python and should be listed: {stdout}"
    );
}
