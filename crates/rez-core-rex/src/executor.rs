//! Rex command executor

use crate::{RexScript, RexInterpreter, RexBindingGenerator, ExecutionResult};
use rez_core_common::RezCoreError;
use rez_core_context::{ResolvedContext, ShellExecutor, ShellType, CommandResult};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Rex executor for running Rex scripts in resolved contexts
#[pyclass]
#[derive(Debug)]
pub struct RexExecutor {
    /// The resolved context to execute in
    context: ResolvedContext,
    /// Rex interpreter
    interpreter: RexInterpreter,
    /// Shell executor
    shell_executor: ShellExecutor,
    /// Executor configuration
    config: ExecutorConfig,
    /// Execution statistics
    stats: ExecutorStats,
}

/// Executor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorConfig {
    /// Shell type for execution
    pub shell_type: ShellType,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Execution timeout in seconds
    pub timeout_seconds: u64,
    /// Whether to generate bindings automatically
    pub auto_generate_bindings: bool,
    /// Whether to validate scripts before execution
    pub validate_before_execution: bool,
    /// Debug mode
    pub debug_mode: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            shell_type: ShellType::detect(),
            working_directory: None,
            timeout_seconds: 300, // 5 minutes
            auto_generate_bindings: true,
            validate_before_execution: true,
            debug_mode: false,
        }
    }
}

/// Executor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutorStats {
    /// Number of scripts executed
    pub scripts_executed: usize,
    /// Number of commands executed
    pub commands_executed: usize,
    /// Number of successful executions
    pub successful_executions: usize,
    /// Number of failed executions
    pub failed_executions: usize,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Average execution time in milliseconds
    pub avg_execution_time_ms: f64,
}

impl Default for ExecutorStats {
    fn default() -> Self {
        Self {
            scripts_executed: 0,
            commands_executed: 0,
            successful_executions: 0,
            failed_executions: 0,
            total_execution_time_ms: 0,
            avg_execution_time_ms: 0.0,
        }
    }
}

/// Script execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Rex execution result
    pub rex_result: ExecutionResult,
    /// Shell execution result (if shell commands were run)
    pub shell_result: Option<CommandResult>,
    /// Generated shell script (if any)
    pub generated_script: Option<String>,
    /// Execution metadata
    pub metadata: HashMap<String, String>,
}

impl ScriptExecutionResult {
    /// Create a successful result
    pub fn success(rex_result: ExecutionResult) -> Self {
        Self {
            success: true,
            rex_result,
            shell_result: None,
            generated_script: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failure(rex_result: ExecutionResult) -> Self {
        Self {
            success: false,
            rex_result,
            shell_result: None,
            generated_script: None,
            metadata: HashMap::new(),
        }
    }

    /// Add shell result
    pub fn with_shell_result(mut self, shell_result: CommandResult) -> Self {
        self.shell_result = Some(shell_result);
        self
    }

    /// Add generated script
    pub fn with_generated_script(mut self, script: String) -> Self {
        self.generated_script = Some(script);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

#[pymethods]
impl RexExecutor {
    #[new]
    pub fn new(context: ResolvedContext) -> Self {
        Self::with_config(context, ExecutorConfig::default())
    }

    /// Execute a Rex script from string
    pub fn execute_script_py(&mut self, script_content: &str) -> PyResult<bool> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.execute_script_content(script_content));
        
        match result {
            Ok(exec_result) => Ok(exec_result.success),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Execute context bindings
    pub fn execute_context_bindings_py(&mut self) -> PyResult<bool> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.execute_context_bindings());
        
        match result {
            Ok(exec_result) => Ok(exec_result.success),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Get execution statistics
    #[getter]
    pub fn stats(&self) -> String {
        serde_json::to_string_pretty(&self.stats).unwrap_or_default()
    }

    /// Get context ID
    #[getter]
    pub fn context_id(&self) -> String {
        self.context.id.clone()
    }
}

impl RexExecutor {
    /// Create a new Rex executor with configuration
    pub fn with_config(context: ResolvedContext, config: ExecutorConfig) -> Self {
        let interpreter_config = crate::InterpreterConfig {
            shell_type: config.shell_type.clone(),
            working_directory: config.working_directory.clone(),
            debug_mode: config.debug_mode,
            ..Default::default()
        };

        let interpreter = RexInterpreter::with_config(interpreter_config);
        
        let shell_executor = ShellExecutor::with_shell(config.shell_type.clone())
            .with_environment(context.environment_vars.clone())
            .with_timeout(config.timeout_seconds);

        let shell_executor = if let Some(ref wd) = config.working_directory {
            shell_executor.with_working_directory(wd.clone())
        } else {
            shell_executor
        };

        Self {
            context,
            interpreter,
            shell_executor,
            config,
            stats: ExecutorStats::default(),
        }
    }

    /// Execute a Rex script
    pub async fn execute_script(&mut self, script: &RexScript) -> Result<ScriptExecutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();
        self.stats.scripts_executed += 1;

        // Validate script if configured
        if self.config.validate_before_execution {
            self.validate_script(script)?;
        }

        // Execute the script with the interpreter
        let rex_result = self.interpreter.execute_script(script).await?;

        // Update statistics
        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        self.stats.total_execution_time_ms += execution_time_ms;
        self.stats.commands_executed += script.commands.len();

        let mut script_result = if rex_result.success {
            self.stats.successful_executions += 1;
            ScriptExecutionResult::success(rex_result)
        } else {
            self.stats.failed_executions += 1;
            ScriptExecutionResult::failure(rex_result)
        };

        // Update average execution time
        if self.stats.scripts_executed > 0 {
            self.stats.avg_execution_time_ms = 
                self.stats.total_execution_time_ms as f64 / self.stats.scripts_executed as f64;
        }

        // Add execution metadata
        script_result = script_result
            .with_metadata("execution_time_ms".to_string(), execution_time_ms.to_string())
            .with_metadata("context_id".to_string(), self.context.id.clone());

        Ok(script_result)
    }

    /// Execute a Rex script from string content
    pub async fn execute_script_content(&mut self, content: &str) -> Result<ScriptExecutionResult, RezCoreError> {
        let parser = crate::RexParser::new();
        let script = parser.parse(content)?;
        self.execute_script(&script).await
    }

    /// Execute context bindings
    pub async fn execute_context_bindings(&mut self) -> Result<ScriptExecutionResult, RezCoreError> {
        if !self.config.auto_generate_bindings {
            return Err(RezCoreError::RexError(
                "Auto-generate bindings is disabled".to_string()
            ));
        }

        // Generate bindings for the context
        let binding_generator = RexBindingGenerator::new(self.config.shell_type.clone());
        let binding_script = binding_generator.generate_context_bindings(&self.context)?;

        // Execute the binding script
        let mut result = self.execute_script(&binding_script).await?;

        // Generate shell script for the bindings
        let shell_script = crate::RexCommandUtils::script_to_shell_script(
            &binding_script,
            &self.config.shell_type,
        )?;

        result = result.with_generated_script(shell_script);
        result = result.with_metadata("binding_type".to_string(), "context_bindings".to_string());

        Ok(result)
    }

    /// Execute a shell script generated from Rex commands
    pub async fn execute_shell_script(&mut self, script_content: &str) -> Result<CommandResult, RezCoreError> {
        self.shell_executor.execute(script_content).await
    }

    /// Execute package-specific bindings
    pub async fn execute_package_bindings(&mut self, package_name: &str) -> Result<ScriptExecutionResult, RezCoreError> {
        // Find the package in the context
        let package = self.context.resolved_packages.iter()
            .find(|p| p.name == package_name)
            .ok_or_else(|| RezCoreError::RexError(
                format!("Package {} not found in context", package_name)
            ))?;

        // Generate bindings for the specific package
        let binding_generator = RexBindingGenerator::new(self.config.shell_type.clone());
        let binding_script = binding_generator.generate_package_bindings(package)?;

        // Execute the binding script
        let mut result = self.execute_script(&binding_script).await?;

        result = result.with_metadata("binding_type".to_string(), "package_bindings".to_string());
        result = result.with_metadata("package_name".to_string(), package_name.to_string());

        Ok(result)
    }

    /// Validate a Rex script
    fn validate_script(&self, script: &RexScript) -> Result<(), RezCoreError> {
        if script.commands.is_empty() {
            return Err(RezCoreError::RexError("Empty script".to_string()));
        }

        // Additional validation logic could go here
        // For example, checking for dangerous commands, syntax validation, etc.

        Ok(())
    }

    /// Get current environment from interpreter
    pub fn get_current_environment(&self) -> HashMap<String, String> {
        self.interpreter.environment.clone()
    }

    /// Get available aliases
    pub fn get_aliases(&self) -> HashMap<String, String> {
        self.interpreter.aliases.clone()
    }

    /// Get defined functions
    pub fn get_functions(&self) -> HashMap<String, String> {
        self.interpreter.functions.clone()
    }

    /// Reset executor state
    pub fn reset(&mut self) {
        self.interpreter.reset();
        self.stats = ExecutorStats::default();
    }

    /// Update executor configuration
    pub fn set_config(&mut self, config: ExecutorConfig) {
        self.config = config;
        
        // Update interpreter configuration
        let interpreter_config = crate::InterpreterConfig {
            shell_type: self.config.shell_type.clone(),
            working_directory: self.config.working_directory.clone(),
            debug_mode: self.config.debug_mode,
            ..Default::default()
        };

        self.interpreter = RexInterpreter::with_config(interpreter_config);

        // Update shell executor
        self.shell_executor = ShellExecutor::with_shell(self.config.shell_type.clone())
            .with_environment(self.context.environment_vars.clone())
            .with_timeout(self.config.timeout_seconds);

        if let Some(ref wd) = self.config.working_directory {
            self.shell_executor = self.shell_executor.with_working_directory(wd.clone());
        }
    }

    /// Get executor statistics
    pub fn get_stats(&self) -> &ExecutorStats {
        &self.stats
    }

    /// Generate shell script for current environment
    pub fn generate_environment_script(&self) -> Result<String, RezCoreError> {
        self.interpreter.generate_shell_script()
    }

    /// Execute a single Rex command
    pub async fn execute_command(&mut self, command: &crate::RexCommand) -> Result<ExecutionResult, RezCoreError> {
        self.interpreter.execute_command(command).await
    }

    /// Check if a command exists in the current environment
    pub async fn command_exists(&self, command: &str) -> bool {
        self.shell_executor.command_exists(command).await
    }

    /// Get context information
    pub fn get_context(&self) -> &ResolvedContext {
        &self.context
    }
}

/// Rex executor builder for fluent API
#[derive(Debug)]
pub struct RexExecutorBuilder {
    context: ResolvedContext,
    config: ExecutorConfig,
}

impl RexExecutorBuilder {
    /// Create a new executor builder
    pub fn new(context: ResolvedContext) -> Self {
        Self {
            context,
            config: ExecutorConfig::default(),
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

    /// Enable/disable auto-generation of bindings
    pub fn with_auto_bindings(mut self, auto_generate: bool) -> Self {
        self.config.auto_generate_bindings = auto_generate;
        self
    }

    /// Enable/disable script validation
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.config.validate_before_execution = validate;
        self
    }

    /// Enable/disable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.config.debug_mode = debug;
        self
    }

    /// Build the Rex executor
    pub fn build(self) -> RexExecutor {
        RexExecutor::with_config(self.context, self.config)
    }
}
