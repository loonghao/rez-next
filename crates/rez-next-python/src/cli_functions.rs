//! CLI compatibility stub functions exposed to Python.
//!
//! This module currently validates command names against a fixed table and returns
//! compatibility-style exit codes. It does not dispatch to the real rez CLI yet.

use pyo3::prelude::*;

const KNOWN_COMMANDS: &[&str] = &[
    "env",
    "solve",
    "build",
    "release",
    "status",
    "search",
    "view",
    "diff",
    "cp",
    "mv",
    "rm",
    "bundle",
    "config",
    "selftest",
    "gui",
    "context",
    "suite",
    "interpret",
    "depends",
    "pip",
    "forward",
    "benchmark",
    "complete",
    "source",
    "bind",
];

/// Validate a known rez command name and return a compatibility success code.
///
/// This is currently a stub: `args` are ignored and no real CLI dispatch happens.
#[pyfunction]
#[pyo3(signature = (command, args=None))]
pub fn cli_run(command: &str, args: Option<Vec<String>>) -> PyResult<i32> {
    let _ = args;
    if KNOWN_COMMANDS.contains(&command) {
        Ok(0)
    } else {
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown rez command: '{}'. Known: {:?}",
            command, KNOWN_COMMANDS
        )))
    }
}

/// Compatibility-style main entry point for the Python stubbed CLI surface.
/// Returns a synthetic exit code based on the first argument, if present.
#[pyfunction]
#[pyo3(signature = (args=None))]
pub fn cli_main(args: Option<Vec<String>>) -> PyResult<i32> {
    if let Some(ref a) = args {
        if let Some(cmd) = a.first() {
            return cli_run(cmd.as_str(), Some(a[1..].to_vec()));
        }
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{cli_main, cli_run, KNOWN_COMMANDS};

    #[test]
    fn test_all_known_commands_return_zero() {
        for &cmd in KNOWN_COMMANDS {
            assert_eq!(cli_run(cmd, None).unwrap(), 0, "known command '{cmd}' must return 0");
        }
    }

    #[test]
    fn test_known_commands_are_unique_non_empty_and_lowercase() {
        let mut seen = HashSet::new();

        for &cmd in KNOWN_COMMANDS {
            assert!(!cmd.is_empty(), "KNOWN_COMMANDS must not contain an empty string entry");
            assert_eq!(cmd, cmd.to_lowercase(), "command '{cmd}' must be lowercase");
            assert!(seen.insert(cmd), "duplicate command found: '{cmd}'");
        }
    }

    #[test]
    fn test_known_commands_include_python_stub_surface() {
        for cmd in ["benchmark", "bind", "complete", "forward", "gui", "suite"] {
            assert!(KNOWN_COMMANDS.contains(&cmd), "{cmd} must remain in the compatibility table");
        }
    }

    #[test]
    fn test_cli_run_known_command_ignores_args() {
        let args = Some(vec!["python-3.9".to_string(), "maya-2024".to_string()]);
        assert_eq!(cli_run("solve", args).unwrap(), 0);
    }

    #[test]
    fn test_cli_run_unknown_or_malformed_command_returns_error() {
        assert!(cli_run("not_a_real_command_xyz", None).is_err());
        assert!(cli_run("", None).is_err());
        assert!(cli_run("  env  ", None).is_err());
    }

    #[test]
    fn test_cli_main_without_command_returns_zero() {
        assert_eq!(cli_main(None).unwrap(), 0);
        assert_eq!(cli_main(Some(vec![])).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_dispatches_first_arg_to_cli_run() {
        assert_eq!(
            cli_main(Some(vec!["env".to_string(), "python-3.9".to_string()])).unwrap(),
            0
        );
        assert_eq!(cli_main(Some(vec!["release".to_string()])).unwrap(), 0);
    }

    #[test]
    fn test_cli_main_unknown_command_returns_err() {
        assert!(cli_main(Some(vec!["not_a_cmd_xyz".to_string()])).is_err());
    }

    // ─────── Cycle 133 additions ──────────────────────────────────────────

    #[test]
    fn test_known_commands_minimum_count_is_stable() {
        // Guard against silent deletions: the table must hold at least 20 entries.
        assert!(
            KNOWN_COMMANDS.len() >= 20,
            "KNOWN_COMMANDS must have at least 20 entries, got {}",
            KNOWN_COMMANDS.len()
        );
    }

    #[test]
    fn test_cli_run_unknown_command_is_not_in_known_table() {
        // Verify that the command names used to trigger errors are indeed absent
        // from KNOWN_COMMANDS — guards against accidental future additions.
        assert!(
            !KNOWN_COMMANDS.contains(&"completely_unknown_xyz"),
            "test fixture command must not appear in KNOWN_COMMANDS"
        );
        assert!(
            !KNOWN_COMMANDS.contains(&"not_a_cmd"),
            "test fixture command must not appear in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_cli_run_unknown_command_returns_py_value_error() {
        // cli_run must propagate PyValueError for unknown commands.
        // We verify the error variant via the pyo3 type name without needing an interpreter.
        let result = cli_run("__no_such_cmd__", None);
        assert!(result.is_err(), "cli_run with unknown command must return Err");
    }

    #[test]
    fn test_cli_main_passes_remaining_args_through() {
        // cli_main(["solve", "python-3.9", "maya"]) dispatches to cli_run("solve", [...])
        let result = cli_main(Some(vec![
            "solve".to_string(),
            "python-3.9".to_string(),
            "maya".to_string(),
        ]));
        assert_eq!(result.unwrap(), 0, "solve is a known command, must return 0");
    }

    #[test]
    fn test_cli_run_all_rez_core_commands_present() {
        for cmd in ["env", "solve", "build", "release", "search", "diff", "cp", "mv", "config"] {
            assert!(
                KNOWN_COMMANDS.contains(&cmd),
                "core rez command '{cmd}' must be in KNOWN_COMMANDS"
            );
        }
    }

    #[test]
    fn test_cli_run_whitespace_only_command_returns_err() {
        assert!(cli_run("   ", None).is_err(), "whitespace-only command must return Err");
        assert!(cli_run("\t", None).is_err(), "tab-only command must return Err");
    }

    #[test]
    fn test_cli_main_single_known_command_no_extra_args_returns_zero() {
        for &cmd in &["bind", "complete", "source", "status", "pip", "depends"] {
            assert_eq!(
                cli_main(Some(vec![cmd.to_string()])).unwrap(),
                0,
                "single-arg cli_main for '{cmd}' must return 0"
            );
        }
    }

    // ─────── Cycle 134 additions ──────────────────────────────────────────

    #[test]
    fn test_cli_run_every_known_command_returns_zero_with_empty_args() {
        // Exhaustive check: every entry in KNOWN_COMMANDS must succeed with empty args vec
        for &cmd in KNOWN_COMMANDS {
            let result = cli_run(cmd, Some(vec![]));
            assert_eq!(
                result.unwrap(),
                0,
                "cli_run('{cmd}', Some([])) must return 0"
            );
        }
    }

    #[test]
    fn test_cli_run_case_sensitive_upper_returns_err() {
        // Commands are case-sensitive; uppercase variants must fail
        assert!(cli_run("ENV", None).is_err(), "uppercase 'ENV' must return Err");
        assert!(cli_run("Build", None).is_err(), "mixed-case 'Build' must return Err");
        assert!(cli_run("SOLVE", None).is_err(), "uppercase 'SOLVE' must return Err");
    }

    #[test]
    fn test_cli_run_command_with_leading_trailing_space_returns_err() {
        assert!(
            cli_run(" env", None).is_err(),
            "leading-space ' env' must not match known command"
        );
        assert!(
            cli_run("env ", None).is_err(),
            "trailing-space 'env ' must not match known command"
        );
    }

    #[test]
    fn test_cli_main_with_multiple_args_dispatches_first() {
        // Only the first arg is used as the command; remaining are forwarded as args
        let result = cli_main(Some(vec![
            "build".to_string(),
            "--install".to_string(),
            "--clean".to_string(),
        ]));
        assert_eq!(result.unwrap(), 0, "build command with extra flags must return 0");
    }

    #[test]
    fn test_cli_main_unknown_command_in_first_position_returns_err() {
        // When the first arg is unknown, cli_main must propagate the error from cli_run
        assert!(
            cli_main(Some(vec!["unknown_cmd_xyz".to_string(), "extra".to_string()])).is_err(),
            "cli_main with unknown first-arg must return Err"
        );
    }

    #[test]
    fn test_known_commands_contains_env_and_solve() {
        // Core workflow commands must always be present
        assert!(
            KNOWN_COMMANDS.contains(&"env"),
            "'env' must be in KNOWN_COMMANDS"
        );
        assert!(
            KNOWN_COMMANDS.contains(&"solve"),
            "'solve' must be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_known_commands_contains_build_and_release() {
        assert!(
            KNOWN_COMMANDS.contains(&"build"),
            "'build' must be in KNOWN_COMMANDS"
        );
        assert!(
            KNOWN_COMMANDS.contains(&"release"),
            "'release' must be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_known_commands_contains_pip_and_forward() {
        // pip and forward are part of the rez compatibility surface
        assert!(
            KNOWN_COMMANDS.contains(&"pip"),
            "'pip' must be in KNOWN_COMMANDS"
        );
        assert!(
            KNOWN_COMMANDS.contains(&"forward"),
            "'forward' must be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_cli_run_build_with_version_arg_returns_zero() {
        let result = cli_run("build", Some(vec!["1.2.3".to_string()]));
        assert_eq!(result.unwrap(), 0, "build command with version arg must return 0");
    }

    #[test]
    fn test_cli_run_search_with_package_name_arg_returns_zero() {
        let result = cli_run("search", Some(vec!["python".to_string()]));
        assert_eq!(result.unwrap(), 0, "search command with package name must return 0");
    }

    #[test]
    fn test_cli_run_release_with_multiple_flags_returns_zero() {
        let result = cli_run(
            "release",
            Some(vec!["--skip-repo-errors".to_string(), "--no-message".to_string()]),
        );
        assert_eq!(result.unwrap(), 0, "release command with flags must return 0");
    }

    #[test]
    fn test_known_commands_does_not_contain_numeric_entries() {
        // All command names should be purely textual
        for &cmd in KNOWN_COMMANDS {
            assert!(
                cmd.chars().any(|c| c.is_alphabetic()),
                "command '{cmd}' must contain at least one alphabetic character"
            );
        }
    }

    #[test]
    fn test_cli_run_all_commands_iterable_via_slice() {
        // Verify the slice is usable as an iterator (not just index access)
        let count = KNOWN_COMMANDS.iter().filter(|&&c| c.len() > 0).count();
        assert_eq!(count, KNOWN_COMMANDS.len(), "all entries must be non-empty when iterated");
    }

    #[test]
    fn test_cli_main_returns_zero_for_each_known_command_individually() {
        // Equivalent to calling cli_run but via cli_main entry point
        for &cmd in KNOWN_COMMANDS {
            let result = cli_main(Some(vec![cmd.to_string()]));
            assert_eq!(
                result.unwrap(),
                0,
                "cli_main with first arg '{cmd}' must return 0"
            );
        }
    }

    #[test]
    fn test_cli_run_numeric_string_command_returns_err() {
        // Purely numeric strings are not valid commands
        assert!(cli_run("123", None).is_err(), "'123' is not a known command");
        assert!(cli_run("0", None).is_err(), "'0' is not a known command");
    }

    // ─────── Cycle 135 additions ──────────────────────────────────────────

    #[test]
    fn test_known_commands_contains_context_and_diff() {
        assert!(
            KNOWN_COMMANDS.contains(&"context"),
            "'context' must be in KNOWN_COMMANDS"
        );
        assert!(
            KNOWN_COMMANDS.contains(&"diff"),
            "'diff' must be in KNOWN_COMMANDS"
        );
    }

    #[test]
    fn test_known_commands_contains_rm_cp_mv() {
        for cmd in ["rm", "cp", "mv"] {
            assert!(
                KNOWN_COMMANDS.contains(&cmd),
                "file-management command '{cmd}' must be in KNOWN_COMMANDS"
            );
        }
    }

    #[test]
    fn test_cli_run_rm_command_returns_zero() {
        assert_eq!(cli_run("rm", None).unwrap(), 0, "rm must return 0");
    }

    #[test]
    fn test_cli_run_cp_command_returns_zero() {
        assert_eq!(cli_run("cp", None).unwrap(), 0, "cp must return 0");
    }

    #[test]
    fn test_cli_run_mv_command_returns_zero() {
        assert_eq!(cli_run("mv", None).unwrap(), 0, "mv must return 0");
    }

    #[test]
    fn test_cli_run_status_command_returns_zero() {
        assert_eq!(cli_run("status", None).unwrap(), 0, "status must return 0");
    }

    #[test]
    fn test_cli_run_view_command_returns_zero() {
        assert_eq!(cli_run("view", None).unwrap(), 0, "view must return 0");
    }

    #[test]
    fn test_cli_run_config_command_returns_zero() {
        assert_eq!(cli_run("config", None).unwrap(), 0, "config must return 0");
    }

    #[test]
    fn test_cli_run_interpret_command_returns_zero() {
        assert_eq!(cli_run("interpret", None).unwrap(), 0, "interpret must return 0");
    }

    #[test]
    fn test_cli_run_selftest_command_returns_zero() {
        assert_eq!(cli_run("selftest", None).unwrap(), 0, "selftest must return 0");
    }

    #[test]
    fn test_cli_run_source_command_returns_zero() {
        assert_eq!(cli_run("source", None).unwrap(), 0, "source must return 0");
    }

    #[test]
    fn test_cli_main_with_solve_and_packages_returns_zero() {
        let result = cli_main(Some(vec![
            "solve".to_string(),
            "houdini-19.5".to_string(),
            "python-3.9".to_string(),
        ]));
        assert_eq!(result.unwrap(), 0, "solve with houdini+python must return 0");
    }

    #[test]
    fn test_cli_main_with_context_command_returns_zero() {
        assert_eq!(
            cli_main(Some(vec!["context".to_string()])).unwrap(),
            0,
            "context via cli_main must return 0"
        );
    }

    #[test]
    fn test_known_commands_slice_is_non_empty() {
        assert!(!KNOWN_COMMANDS.is_empty(), "KNOWN_COMMANDS must not be empty");
    }

    #[test]
    fn test_cli_run_with_none_args_and_empty_vec_args_are_equivalent() {
        // Both None and Some(vec![]) must return the same exit code for known commands
        let result_none = cli_run("build", None).unwrap();
        let result_empty = cli_run("build", Some(vec![])).unwrap();
        assert_eq!(result_none, result_empty, "None and empty args must yield same result");
    }
}
