//! Shared command runner utilities for build system implementations.
//!
//! This module avoids shell-specific inline fragments (`2>&1`, `|| echo`, quoting) by
//! providing typed helpers that handle stderr merging and optional-command fallback in
//! Rust rather than in the shell command string.

use crate::{BuildStep, BuildStepResult};
use rez_next_common::RezCoreError;
use rez_next_context::ShellExecutor;

/// Run `cmd` via `executor`.  If the command fails (Err *or* non-zero exit) and
/// `optional` is `true`, return a successful `BuildStepResult` with `fallback_msg`
/// as output rather than propagating the failure.
///
/// This replaces the pattern:
/// ```text
/// executor.execute("npm run build 2>&1 || echo 'No build script'").await
/// ```
/// with:
/// ```rust,ignore
/// run_cmd(&executor, BuildStep::Compiling, "npm run build", true, "No build script").await
/// ```
///
/// The `2>&1` redirect is intentionally omitted — stderr is captured separately
/// by `ShellExecutor` and surfaced through `BuildStepResult::errors`.
pub async fn run_cmd(
    executor: &ShellExecutor,
    step: BuildStep,
    cmd: &str,
    optional: bool,
    fallback_msg: &str,
) -> Result<BuildStepResult, RezCoreError> {
    match executor.execute(cmd).await {
        Ok(r) if r.is_success() => Ok(BuildStepResult {
            step,
            success: true,
            output: r.stdout,
            errors: r.stderr,
            duration_ms: r.execution_time_ms,
        }),
        Ok(r) if optional => Ok(BuildStepResult {
            step,
            success: true,
            output: fallback_msg.to_string(),
            errors: r.stderr,
            duration_ms: r.execution_time_ms,
        }),
        Ok(r) => Ok(BuildStepResult {
            step,
            success: false,
            output: r.stdout,
            errors: r.stderr,
            duration_ms: r.execution_time_ms,
        }),
        Err(_) if optional => Ok(BuildStepResult {
            step,
            success: true,
            output: fallback_msg.to_string(),
            errors: String::new(),
            duration_ms: 0,
        }),
        Err(e) => Err(e),
    }
}

/// Build a `make install` command string using the platform-appropriate
/// variable assignment form.
///
/// On all supported platforms, GNU Make accepts `VAR=value` on the command
/// line, so this does not need to be shell-specific.  However, paths that
/// contain spaces must be quoted; this helper adds double-quotes around the
/// destination path.
pub fn make_install_cmd(destdir: &std::path::Path) -> String {
    format!("make install DESTDIR=\"{}\"", destdir.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_install_cmd_formats_destdir_with_quotes() {
        let path = std::path::Path::new("/opt/packages/mylib/1.0.0");
        let cmd = make_install_cmd(path);
        assert!(
            cmd.starts_with("make install DESTDIR="),
            "must start with make install DESTDIR="
        );
        assert!(
            cmd.contains("/opt/packages/mylib/1.0.0"),
            "must contain the path"
        );
        assert!(cmd.contains('"'), "path must be quoted");
    }

    #[test]
    fn make_install_cmd_path_with_spaces() {
        let path = std::path::Path::new("/opt/my packages/1.0");
        let cmd = make_install_cmd(path);
        assert!(cmd.contains('"'), "path with spaces must be quoted");
    }
}
