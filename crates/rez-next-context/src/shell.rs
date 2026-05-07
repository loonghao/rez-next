//! Shell integration and command execution

use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;

/// Supported shell types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ShellType {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// Windows Command Prompt
    Cmd,
    /// PowerShell
    PowerShell,
}

impl ShellType {
    /// Get the shell executable name
    pub fn executable(&self) -> &'static str {
        match self {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::Cmd => "cmd",
            ShellType::PowerShell => "powershell",
        }
    }

    /// Get the shell script extension
    pub fn script_extension(&self) -> &'static str {
        match self {
            ShellType::Bash | ShellType::Zsh => "sh",
            ShellType::Fish => "fish",
            ShellType::Cmd => "bat",
            ShellType::PowerShell => "ps1",
        }
    }

    /// Get the shell command flag for executing scripts
    pub fn command_flag(&self) -> &'static str {
        match self {
            ShellType::Bash | ShellType::Zsh => "-c",
            ShellType::Fish => "-c",
            ShellType::Cmd => "/c",
            ShellType::PowerShell => "-Command",
        }
    }

    /// Detect the current shell from environment
    pub fn detect() -> Self {
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("bash") {
                return ShellType::Bash;
            } else if shell.contains("zsh") {
                return ShellType::Zsh;
            } else if shell.contains("fish") {
                return ShellType::Fish;
            }
        }

        // Check for Windows
        if cfg!(windows) {
            if std::env::var("PSModulePath").is_ok() {
                ShellType::PowerShell
            } else {
                ShellType::Cmd
            }
        } else {
            ShellType::Bash // Default to bash on Unix-like systems
        }
    }
}

/// Shell command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl CommandResult {
    /// Check if the command was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output (stdout + stderr)
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }
}

/// Shell executor for running commands in resolved contexts
#[derive(Debug, Clone)]
pub struct ShellExecutor {
    /// Shell type to use
    shell_type: ShellType,
    /// Working directory
    working_directory: Option<PathBuf>,
    /// Environment variables
    environment: HashMap<String, String>,
    /// Timeout for command execution (in seconds)
    timeout_seconds: u64,
}


impl ShellExecutor {
    /// Create a new shell executor with default shell type
    pub fn new() -> Self {
        Self::with_shell(ShellType::detect())
    }

    /// Create a new shell executor with specified shell type
    pub fn with_shell(shell_type: ShellType) -> Self {
        Self {
            shell_type,
            working_directory: None,
            environment: HashMap::new(),
            timeout_seconds: 300, // 5 minutes default
        }
    }

    /// Set the environment variables
    pub fn with_environment(mut self, environment: HashMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    /// Set the working directory
    pub fn with_working_directory(mut self, working_directory: PathBuf) -> Self {
        self.working_directory = Some(working_directory);
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Execute a command and wait for completion
    pub async fn execute(&self, command: &str) -> Result<CommandResult, RezCoreError> {
        let start_time = std::time::Instant::now();

        let mut cmd = AsyncCommand::new(self.shell_type.executable());
        cmd.arg(self.shell_type.command_flag())
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory
        if let Some(ref wd) = self.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_seconds),
            cmd.output(),
        )
        .await
        .map_err(|_| RezCoreError::ExecutionError("Command execution timeout".to_string()))?
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to execute command: {}", e)))?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(CommandResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms,
        })
    }

    /// Execute a command in the background and return the process ID
    pub async fn execute_background(&self, command: &str) -> Result<u32, RezCoreError> {
        let mut cmd = AsyncCommand::new(self.shell_type.executable());
        cmd.arg(self.shell_type.command_flag())
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        // Set working directory
        if let Some(ref wd) = self.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| RezCoreError::ExecutionError(format!("Failed to spawn command: {}", e)))?;

        Ok(child.id().unwrap_or(0))
    }

    /// Execute multiple commands in sequence
    pub async fn execute_batch(
        &self,
        commands: &[String],
    ) -> Result<Vec<CommandResult>, RezCoreError> {
        let mut results = Vec::new();

        for command in commands {
            let result = self.execute(command).await?;
            results.push(result);

            // Stop on first failure if desired
            // if !result.is_success() {
            //     break;
            // }
        }

        Ok(results)
    }

    /// Execute a script file
    pub async fn execute_script(
        &self,
        script_path: &PathBuf,
    ) -> Result<CommandResult, RezCoreError> {
        if !script_path.exists() {
            return Err(RezCoreError::ExecutionError(format!(
                "Script file does not exist: {}",
                script_path.display()
            )));
        }

        let start_time = std::time::Instant::now();

        let mut cmd = AsyncCommand::new(self.shell_type.executable());

        match self.shell_type {
            ShellType::Bash | ShellType::Zsh => {
                cmd.arg(script_path);
            }
            ShellType::Fish => {
                cmd.arg(script_path);
            }
            ShellType::Cmd => {
                cmd.arg("/c").arg(script_path);
            }
            ShellType::PowerShell => {
                cmd.arg("-File").arg(script_path);
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Set working directory
        if let Some(ref wd) = self.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_seconds),
            cmd.output(),
        )
        .await
        .map_err(|_| RezCoreError::ExecutionError("Script execution timeout".to_string()))?
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to execute script: {}", e)))?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(CommandResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms,
        })
    }

    /// Start an interactive shell session
    pub async fn start_interactive_shell(&self) -> Result<(), RezCoreError> {
        let mut cmd = AsyncCommand::new(self.shell_type.executable());

        // Set interactive flags
        match self.shell_type {
            ShellType::Bash => cmd.arg("-i"),
            ShellType::Zsh => cmd.arg("-i"),
            ShellType::Fish => cmd.arg("-i"),
            ShellType::Cmd => &mut cmd, // No special flag needed
            ShellType::PowerShell => cmd.arg("-NoExit"),
        };

        // Set working directory
        if let Some(ref wd) = self.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        for (key, value) in &self.environment {
            cmd.env(key, value);
        }

        // Inherit stdio for interactive session
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = cmd
            .spawn()
            .map_err(|e| RezCoreError::ExecutionError(format!("Failed to start shell: {}", e)))?;

        // Wait for the shell to exit
        let status = child
            .wait()
            .await
            .map_err(|e| RezCoreError::ExecutionError(format!("Shell execution error: {}", e)))?;

        if !status.success() {
            return Err(RezCoreError::ExecutionError(format!(
                "Shell exited with code: {}",
                status.code().unwrap_or(-1)
            )));
        }

        Ok(())
    }

    /// Check if a command exists in the current environment
    pub async fn command_exists(&self, command: &str) -> bool {
        let check_command = match self.shell_type {
            ShellType::Bash | ShellType::Zsh => format!("command -v {}", command),
            ShellType::Fish => format!("command -v {}", command),
            ShellType::Cmd => format!("where {}", command),
            ShellType::PowerShell => {
                format!("Get-Command {} -ErrorAction SilentlyContinue", command)
            }
        };

        match self.execute(&check_command).await {
            Ok(result) => result.is_success() && !result.stdout.trim().is_empty(),
            Err(_) => false,
        }
    }

    /// Get shell information
    pub async fn get_shell_info(&self) -> Result<ShellInfo, RezCoreError> {
        let version_command = match self.shell_type {
            ShellType::Bash => "bash --version",
            ShellType::Zsh => "zsh --version",
            ShellType::Fish => "fish --version",
            ShellType::Cmd => "ver",
            ShellType::PowerShell => "$PSVersionTable.PSVersion",
        };

        let result = self.execute(version_command).await?;

        Ok(ShellInfo {
            shell_type: self.shell_type.clone(),
            version: result
                .stdout
                .lines()
                .next()
                .unwrap_or("unknown")
                .to_string(),
            executable_path: self.shell_type.executable().to_string(),
        })
    }
}

/// Shell information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellInfo {
    /// Shell type
    pub shell_type: ShellType,
    /// Shell version
    pub version: String,
    /// Executable path
    pub executable_path: String,
}

impl Default for ShellExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_executable() {
        assert_eq!(ShellType::Bash.executable(), "bash");
        assert_eq!(ShellType::Zsh.executable(), "zsh");
        assert_eq!(ShellType::Fish.executable(), "fish");
        assert_eq!(ShellType::Cmd.executable(), "cmd");
        assert_eq!(ShellType::PowerShell.executable(), "powershell");
    }

    #[test]
    fn test_shell_type_script_extension() {
        assert_eq!(ShellType::Bash.script_extension(), "sh");
        assert_eq!(ShellType::Zsh.script_extension(), "sh");
        assert_eq!(ShellType::Fish.script_extension(), "fish");
        assert_eq!(ShellType::Cmd.script_extension(), "bat");
        assert_eq!(ShellType::PowerShell.script_extension(), "ps1");
    }

    #[test]
    fn test_shell_type_command_flag() {
        assert_eq!(ShellType::Bash.command_flag(), "-c");
        assert_eq!(ShellType::Zsh.command_flag(), "-c");
        assert_eq!(ShellType::Fish.command_flag(), "-c");
        assert_eq!(ShellType::Cmd.command_flag(), "/c");
        assert_eq!(ShellType::PowerShell.command_flag(), "-Command");
    }

    #[test]
    fn test_shell_type_detect() {
        let detected = ShellType::detect();

        // On Windows, should be Cmd or PowerShell
        // On Unix, should be Bash, Zsh, or Fish
        match detected {
            ShellType::Bash
            | ShellType::Zsh
            | ShellType::Fish
            | ShellType::Cmd
            | ShellType::PowerShell => (),
        }
    }

    #[test]
    fn test_command_result_is_success() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "output".to_string(),
            stderr: String::new(),
            execution_time_ms: 100,
        };

        assert!(result.is_success());
    }

    #[test]
    fn test_command_result_is_failure() {
        let result = CommandResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            execution_time_ms: 50,
        };

        assert!(!result.is_success());
    }

    #[test]
    fn test_command_result_combined_output() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: "warn".to_string(),
            execution_time_ms: 100,
        };

        assert_eq!(result.combined_output(), "hello\nwarn");
    }

    #[test]
    fn test_command_result_combined_output_no_stderr() {
        let result = CommandResult {
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: String::new(),
            execution_time_ms: 100,
        };

        assert_eq!(result.combined_output(), "hello");
    }

    #[test]
    fn test_command_result_combined_output_no_stdout() {
        let result = CommandResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            execution_time_ms: 50,
        };

        assert_eq!(result.combined_output(), "error");
    }

    #[test]
    fn test_shell_info_creation() {
        let info = ShellInfo {
            shell_type: ShellType::Bash,
            version: "5.0".to_string(),
            executable_path: "bash".to_string(),
        };

        assert_eq!(info.shell_type, ShellType::Bash);
        assert_eq!(info.version, "5.0");
        assert_eq!(info.executable_path, "bash");
    }

    #[test]
    fn test_shell_executor_new() {
        let executor = ShellExecutor::new();

        // Just verify it creates successfully
        let _ = executor;
    }

    #[test]
    fn test_shell_executor_with_shell() {
        let executor = ShellExecutor::with_shell(ShellType::PowerShell);

        // Just verify it creates successfully
        let _ = executor;
    }

    #[test]
    fn test_shell_executor_with_working_directory() {
        let executor = ShellExecutor::new().with_working_directory(PathBuf::from("/tmp"));

        // Just verify it creates successfully
        let _ = executor;
    }

    #[test]
    fn test_shell_executor_with_timeout() {
        let executor = ShellExecutor::new().with_timeout(600);

        // Just verify it creates successfully
        let _ = executor;
    }

    #[test]
    fn test_shell_executor_fluent_api() {
        let executor = ShellExecutor::with_shell(ShellType::Bash)
            .with_working_directory(PathBuf::from("/work"))
            .with_timeout(120)
            .with_environment(HashMap::new());

        // Just verify it creates successfully
        let _ = executor;
    }
}
