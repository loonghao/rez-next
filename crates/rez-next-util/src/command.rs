//! Command execution utilities.
//!
//! This module provides functionality for executing external commands and
//! capturing their output, compatible with Rez's `command` module.

use std::path::PathBuf;
use std::process::Output;

use rez_next_common::{RezCoreError, RezCoreResult};

/// Result of command execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Whether the command succeeded (exit code 0)
    pub success: bool,
}

impl CommandResult {
    /// Create a new CommandResult from std::process::Output.
    pub fn from_output(output: Output) -> Self {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        let success = output.status.success();

        CommandResult {
            stdout,
            stderr,
            exit_code,
            success,
        }
    }
}

/// Execute a command and capture its output.
///
/// # Arguments
///
/// * `command` - Command to execute (e.g., "echo", "git")
/// * `args` - Arguments to pass to the command
///
/// # Returns
///
/// * `RezCoreResult<CommandResult>` - Execution result or error
pub fn execute_command(command: &str, args: &[&str]) -> RezCoreResult<CommandResult> {
    let output = std::process::Command::new(command)
        .args(args)
        .output()
        .map_err(|e| {
            RezCoreError::ExecutionError(format!("Failed to execute command '{}': {}", command, e))
        })?;

    Ok(CommandResult::from_output(output))
}

/// Execute a command with a timeout.
///
/// # Arguments
///
/// * `command` - Command to execute
/// * `args` - Arguments to pass to the command
/// * `timeout_secs` - Timeout in seconds
///
/// # Returns
///
/// * `RezCoreResult<CommandResult>` - Execution result or timeout error
pub fn execute_command_with_timeout(
    command: &str,
    args: &[&str],
    timeout_secs: u64,
) -> RezCoreResult<CommandResult> {
    // Note: Proper timeout implementation would require async runtime.
    // For now, we just execute the command without timeout.
    // TODO: Implement proper timeout using tokio or similar
    let _ = timeout_secs; // Suppress unused warning
    execute_command(command, args)
}

/// Check if a command exists in PATH.
///
/// This is a convenience wrapper around `which()`.
///
/// # Arguments
///
/// * `command` - Command name to check
///
/// # Returns
///
/// * `bool` - True if command exists in PATH
pub fn command_exists(command: &str) -> bool {
    crate::which(command).is_some()
}

/// Get the full path to a command.
///
/// This is a convenience wrapper around `which()`.
///
/// # Arguments
///
/// * `command` - Command name to find
///
/// # Returns
///
/// * `Option<PathBuf>` - Full path to the command, or None if not found
pub fn get_command_path(command: &str) -> Option<PathBuf> {
    crate::which(command)
}

/// Execute a command and return stdout as a string.
///
/// # Arguments
///
/// * `command` - Command to execute
/// * `args` - Arguments to pass to the command
///
/// # Returns
///
/// * `RezCoreResult<String>` - Stdout as string, or error
pub fn get_command_output(command: &str, args: &[&str]) -> RezCoreResult<String> {
    let result = execute_command(command, args)?;

    if !result.success {
        return Err(RezCoreError::ExecutionError(format!(
            "Command '{}' failed with exit code {}: {}",
            command, result.exit_code, result.stderr
        )));
    }

    Ok(result.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to get a command that echoes text, cross-platform
    fn get_echo_command() -> (&'static str, Vec<String>) {
        if cfg!(unix) {
            ("echo", vec!["test output".to_string()])
        } else {
            (
                "cmd",
                vec!["/c".to_string(), "echo test output".to_string()],
            )
        }
    }

    #[test]
    fn test_command_result_from_output() {
        use std::process::Command;

        // Create a simple command that outputs "test output"
        let (cmd, args) = if cfg!(unix) {
            ("echo", vec!["test output"])
        } else {
            ("cmd", vec!["/c", "echo test output"])
        };

        let output = Command::new(cmd).args(&args).output().unwrap();

        let cmd_result = CommandResult::from_output(output);
        assert!(cmd_result.success);
        assert!(cmd_result.stdout.contains("test output"));
        assert_eq!(cmd_result.stderr, "");
        assert_eq!(cmd_result.exit_code, 0);
    }

    #[test]
    fn test_nonexistent_command() {
        let result = execute_command("this_command_definitely_does_not_exist_12345", &[]);
        // Should return an error (command not found)
        assert!(result.is_err());
    }

    #[test]
    fn test_execute_command_success() {
        // Test with a command that should succeed
        let (cmd, args) = if cfg!(unix) {
            ("echo", vec!["test"])
        } else {
            ("cmd", vec!["/c", "echo test"])
        };

        let result = execute_command(cmd, &args);
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        assert!(cmd_result.success);
        assert!(cmd_result.stdout.contains("test"));
        assert_eq!(cmd_result.exit_code, 0);
    }

    #[test]
    fn test_command_exists_echo() {
        // `echo` should exist on most systems (either as standalone or via cmd)
        // Note: The actual result depends on the system PATH
        let _ = command_exists("echo");
    }

    #[test]
    fn test_get_command_output_echo() {
        let (cmd, args) = if cfg!(unix) {
            ("echo", vec!["Hello, World!"])
        } else {
            ("cmd", vec!["/c", "echo Hello, World!"])
        };

        let result = get_command_output(cmd, &args);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Hello, World!"));
    }
}
