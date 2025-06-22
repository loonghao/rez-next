//! Context execution and command spawning

use crate::{CommandResult, ResolvedContext, ShellExecutor, ShellType};
use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command as AsyncCommand;

/// Context execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Shell type to use for execution
    pub shell_type: ShellType,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Whether to inherit parent environment
    pub inherit_parent_env: bool,
    /// Additional environment variables
    pub additional_env_vars: HashMap<String, String>,
    /// Execution timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to capture output
    pub capture_output: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            shell_type: ShellType::detect(),
            working_directory: None,
            inherit_parent_env: true,
            additional_env_vars: HashMap::new(),
            timeout_seconds: 300, // 5 minutes
            capture_output: true,
        }
    }
}

/// Context executor for running commands in resolved contexts
#[derive(Debug)]
pub struct ContextExecutor {
    /// The resolved context to execute in
    context: ResolvedContext,
    /// Execution configuration
    config: ExecutionConfig,
    /// Shell executor
    shell_executor: ShellExecutor,
}

impl ContextExecutor {
    /// Create a new context executor
    pub fn new(context: ResolvedContext) -> Self {
        let config = ExecutionConfig::default();
        Self::with_config(context, config)
    }

    /// Create a context executor with custom configuration
    pub fn with_config(context: ResolvedContext, config: ExecutionConfig) -> Self {
        let mut environment = context.environment_vars.clone();

        // Add additional environment variables
        environment.extend(config.additional_env_vars.clone());

        let shell_executor = ShellExecutor::with_shell(config.shell_type.clone())
            .with_environment(environment)
            .with_timeout(config.timeout_seconds);

        let shell_executor = if let Some(ref wd) = config.working_directory {
            shell_executor.with_working_directory(wd.clone())
        } else {
            shell_executor
        };

        Self {
            context,
            config,
            shell_executor,
        }
    }

    /// Execute a command in the context
    pub async fn execute(&self, command: &str) -> Result<CommandResult, RezCoreError> {
        self.shell_executor.execute(command).await
    }

    /// Execute a command in the background
    pub async fn execute_background(&self, command: &str) -> Result<u32, RezCoreError> {
        self.shell_executor.execute_background(command).await
    }

    /// Execute multiple commands in sequence
    pub async fn execute_batch(
        &self,
        commands: &[String],
    ) -> Result<Vec<CommandResult>, RezCoreError> {
        self.shell_executor.execute_batch(commands).await
    }

    /// Execute a script file in the context
    pub async fn execute_script(
        &self,
        script_path: &PathBuf,
    ) -> Result<CommandResult, RezCoreError> {
        self.shell_executor.execute_script(script_path).await
    }

    /// Start an interactive shell in the context
    pub async fn start_interactive_shell(&self) -> Result<(), RezCoreError> {
        self.shell_executor.start_interactive_shell().await
    }

    /// Spawn a new process in the context
    pub async fn spawn_process(
        &self,
        program: &str,
        args: &[String],
    ) -> Result<SpawnedProcess, RezCoreError> {
        let mut cmd = AsyncCommand::new(program);
        cmd.args(args);

        // Set working directory
        if let Some(ref wd) = self.config.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        if self.config.inherit_parent_env {
            // Inherit parent environment and override with context environment
            for (key, value) in &self.context.environment_vars {
                cmd.env(key, value);
            }
        } else {
            // Clear environment and set only context environment
            cmd.env_clear();
            for (key, value) in &self.context.environment_vars {
                cmd.env(key, value);
            }
        }

        // Add additional environment variables
        for (key, value) in &self.config.additional_env_vars {
            cmd.env(key, value);
        }

        // Configure stdio
        if self.config.capture_output {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }

        let child = cmd.spawn().map_err(|e| {
            RezCoreError::ExecutionError(format!("Failed to spawn process {}: {}", program, e))
        })?;

        Ok(SpawnedProcess {
            child,
            program: program.to_string(),
            args: args.to_vec(),
            start_time: std::time::Instant::now(),
        })
    }

    /// Check if a command/tool is available in the context
    pub async fn command_exists(&self, command: &str) -> bool {
        self.shell_executor.command_exists(command).await
    }

    /// Get all available tools in the context
    pub fn get_available_tools(&self) -> Vec<String> {
        self.context.get_all_tools()
    }

    /// Get context information
    pub fn get_context(&self) -> &ResolvedContext {
        &self.context
    }

    /// Get execution configuration
    pub fn get_config(&self) -> &ExecutionConfig {
        &self.config
    }

    /// Update execution configuration
    pub fn set_config(&mut self, config: ExecutionConfig) {
        self.config = config;
        // Recreate shell executor with new config
        let mut environment = self.context.environment_vars.clone();
        environment.extend(self.config.additional_env_vars.clone());

        self.shell_executor = ShellExecutor::with_shell(self.config.shell_type.clone())
            .with_environment(environment)
            .with_timeout(self.config.timeout_seconds);

        if let Some(ref wd) = self.config.working_directory {
            self.shell_executor = self
                .shell_executor
                .clone()
                .with_working_directory(wd.clone());
        }
    }

    /// Execute a package-specific command
    pub async fn execute_package_command(
        &self,
        package_name: &str,
        command: &str,
    ) -> Result<CommandResult, RezCoreError> {
        // Check if package exists in context
        if !self.context.contains_package(package_name) {
            return Err(RezCoreError::ExecutionError(format!(
                "Package {} not found in context",
                package_name
            )));
        }

        // Get package-specific environment
        let package_env_var = format!("{}_ROOT", package_name.to_uppercase());
        let package_root = self.context.environment_vars.get(&package_env_var);

        if package_root.is_none() {
            return Err(RezCoreError::ExecutionError(format!(
                "Package root not found for {}",
                package_name
            )));
        }

        // Execute the command (could be enhanced to look in package-specific paths)
        self.execute(command).await
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> ExecutionStats {
        ExecutionStats {
            context_id: self.context.id.clone(),
            package_count: self.context.resolved_packages.len(),
            env_var_count: self.context.environment_vars.len(),
            tool_count: self.get_available_tools().len(),
            shell_type: self.config.shell_type.clone(),
            working_directory: self.config.working_directory.clone(),
        }
    }
}

/// Spawned process handle
pub struct SpawnedProcess {
    /// The child process
    child: tokio::process::Child,
    /// Program name
    program: String,
    /// Program arguments
    args: Vec<String>,
    /// Start time
    start_time: std::time::Instant,
}

impl SpawnedProcess {
    /// Get the process ID
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Wait for the process to complete
    pub async fn wait(mut self) -> Result<ProcessResult, RezCoreError> {
        let output = self.child.wait_with_output().await.map_err(|e| {
            RezCoreError::ExecutionError(format!("Failed to wait for process: {}", e))
        })?;

        let execution_time_ms = self.start_time.elapsed().as_millis() as u64;

        Ok(ProcessResult {
            program: self.program,
            args: self.args,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time_ms,
        })
    }

    /// Kill the process
    pub async fn kill(&mut self) -> Result<(), RezCoreError> {
        self.child
            .kill()
            .await
            .map_err(|e| RezCoreError::ExecutionError(format!("Failed to kill process: {}", e)))
    }

    /// Try to kill the process
    pub fn try_kill(&mut self) -> Result<(), RezCoreError> {
        self.child
            .start_kill()
            .map_err(|e| RezCoreError::ExecutionError(format!("Failed to kill process: {}", e)))
    }
}

/// Process execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResult {
    /// Program name
    pub program: String,
    /// Program arguments
    pub args: Vec<String>,
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ProcessResult {
    /// Check if the process was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Get combined output
    pub fn combined_output(&self) -> String {
        if self.stderr.is_empty() {
            self.stdout.clone()
        } else if self.stdout.is_empty() {
            self.stderr.clone()
        } else {
            format!("{}\n{}", self.stdout, self.stderr)
        }
    }

    /// Get command line representation
    pub fn command_line(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Context ID
    pub context_id: String,
    /// Number of packages in context
    pub package_count: usize,
    /// Number of environment variables
    pub env_var_count: usize,
    /// Number of available tools
    pub tool_count: usize,
    /// Shell type being used
    pub shell_type: ShellType,
    /// Working directory
    pub working_directory: Option<PathBuf>,
}

/// Context execution builder for fluent API
#[derive(Debug)]
pub struct ContextExecutionBuilder {
    context: ResolvedContext,
    config: ExecutionConfig,
}

impl ContextExecutionBuilder {
    /// Create a new execution builder
    pub fn new(context: ResolvedContext) -> Self {
        Self {
            context,
            config: ExecutionConfig::default(),
        }
    }

    /// Set shell type
    pub fn with_shell(mut self, shell_type: ShellType) -> Self {
        self.config.shell_type = shell_type;
        self
    }

    /// Set working directory
    pub fn with_working_directory(mut self, working_directory: PathBuf) -> Self {
        self.config.working_directory = Some(working_directory);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.timeout_seconds = timeout_seconds;
        self
    }

    /// Add environment variable
    pub fn with_env_var(mut self, name: String, value: String) -> Self {
        self.config.additional_env_vars.insert(name, value);
        self
    }

    /// Set whether to capture output
    pub fn with_capture_output(mut self, capture: bool) -> Self {
        self.config.capture_output = capture;
        self
    }

    /// Build the context executor
    pub fn build(self) -> ContextExecutor {
        ContextExecutor::with_config(self.context, self.config)
    }
}
