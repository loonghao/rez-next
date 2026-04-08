//! Shared helpers for CLI end-to-end tests.
//!
//! Included via `#[path = "cli_e2e_helpers.rs"] mod cli_e2e_helpers;`
//! in both cli_e2e_tests.rs and cli_e2e_misc_tests.rs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

// ── Binary path ───────────────────────────────────────────────────────────────

pub fn rez_next_bin() -> PathBuf {
    if let Ok(path) = std::env::var("REZ_NEXT_E2E_BINARY") {
        return PathBuf::from(path);
    }
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

pub fn rez(args: &[&str]) -> Output {
    Command::new(rez_next_bin())
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run rez-next: {e}"))
}

pub fn rez_ok(args: &[&str]) -> String {
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
pub fn rez_output(args: &[&str]) -> (String, String, Option<i32>) {
    let out = rez(args);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (stdout, stderr, out.status.code())
}

// ── Package repo helpers ──────────────────────────────────────────────────────

/// Write a minimal package.py under `<repo>/<name>/<version>/package.py`
pub fn write_package(repo: &Path, name: &str, version: &str, extra: &str) {
    let pkg_dir = repo.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();
    let content = format!("name = \"{name}\"\nversion = \"{version}\"\n{extra}\n");
    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

/// Create a minimal test repository with a few packages
pub fn make_test_repo(base: &Path) -> PathBuf {
    let repo = base.join("packages");
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

pub fn skip_if_no_binary() -> bool {
    !rez_next_bin().exists()
}

#[macro_export]
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
