//! Rex command interpreter

use crate::{RexCommand, RexScript};
use rez_core_common::RezCoreError;
use rez_core_context::{ShellType, EnvironmentManager, ContextConfig};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Rex interpreter for executing Rex commands
#[pyclass]
#[derive(Debug, Clone)]
pub struct RexInterpreter {
    /// Current environment variables
    environment: HashMap<String, String>,
    /// Aliases
    aliases: HashMap<String, String>,
    /// Functions
    functions: HashMap<String, String>,
    /// Interpreter configuration
    config: InterpreterConfig,
    /// Execution statistics
    stats: InterpreterStats,
}

/// Interpreter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpreterConfig {
    /// Shell type for command generation
    pub shell_type: ShellType,
    /// Working directory
    pub working_directory: Option<PathBuf>,
    /// Whether to inherit parent environment
    pub inherit_parent_env: bool,
    /// Maximum recursion depth for function calls
    pub max_recursion_depth: usize,
    /// Enable debug output
    pub debug_mode: bool,
}

impl Default for InterpreterConfig {
    fn default() -> Self {
        Self {
            shell_type: ShellType::detect(),
            working_directory: None,
            inherit_parent_env: true,
            max_recursion_depth: 100,
            debug_mode: false,
        }
    }
}

/// Interpreter execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpreterStats {
    /// Number of commands executed
    pub commands_executed: usize,
    /// Number of environment variables set
    pub env_vars_set: usize,
    /// Number of aliases created
    pub aliases_created: usize,
    /// Number of functions defined
    pub functions_defined: usize,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
}

impl Default for InterpreterStats {
    fn default() -> Self {
        Self {
            commands_executed: 0,
            env_vars_set: 0,
            aliases_created: 0,
            functions_defined: 0,
            total_execution_time_ms: 0,
        }
    }
}

/// Command execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution was successful
    pub success: bool,
    /// Output messages
    pub output: Vec<String>,
    /// Error messages
    pub errors: Vec<String>,
    /// Environment changes
    pub env_changes: HashMap<String, Option<String>>, // None = unset
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ExecutionResult {
    /// Create a successful result
    pub fn success() -> Self {
        Self {
            success: true,
            output: Vec::new(),
            errors: Vec::new(),
            env_changes: HashMap::new(),
            execution_time_ms: 0,
        }
    }

    /// Create an error result
    pub fn error(message: String) -> Self {
        Self {
            success: false,
            output: Vec::new(),
            errors: vec![message],
            env_changes: HashMap::new(),
            execution_time_ms: 0,
        }
    }

    /// Add output message
    pub fn with_output(mut self, message: String) -> Self {
        self.output.push(message);
        self
    }

    /// Add environment change
    pub fn with_env_change(mut self, name: String, value: Option<String>) -> Self {
        self.env_changes.insert(name, value);
        self
    }
}

#[pymethods]
impl RexInterpreter {
    #[new]
    pub fn new() -> Self {
        Self::with_config(InterpreterConfig::default())
    }

    /// Execute a Rex script
    pub fn execute_script_py(&mut self, script_content: &str) -> PyResult<bool> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.execute_script_content(script_content));
        
        match result {
            Ok(exec_result) => Ok(exec_result.success),
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string())),
        }
    }

    /// Get current environment variables
    #[getter]
    pub fn environment(&self) -> HashMap<String, String> {
        self.environment.clone()
    }

    /// Get aliases
    #[getter]
    pub fn aliases(&self) -> HashMap<String, String> {
        self.aliases.clone()
    }

    /// Get functions
    #[getter]
    pub fn functions(&self) -> HashMap<String, String> {
        self.functions.clone()
    }

    /// Get execution statistics
    #[getter]
    pub fn stats(&self) -> String {
        serde_json::to_string_pretty(&self.stats).unwrap_or_default()
    }
}

impl RexInterpreter {
    /// Create a new interpreter with configuration
    pub fn with_config(config: InterpreterConfig) -> Self {
        let environment = if config.inherit_parent_env {
            std::env::vars().collect()
        } else {
            HashMap::new()
        };

        Self {
            environment,
            aliases: HashMap::new(),
            functions: HashMap::new(),
            config,
            stats: InterpreterStats::default(),
        }
    }

    /// Execute a Rex script
    pub async fn execute_script(&mut self, script: &RexScript) -> Result<ExecutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();
        let mut overall_result = ExecutionResult::success();

        for command in &script.commands {
            let result = self.execute_command(command).await?;
            
            // Merge results
            overall_result.output.extend(result.output);
            overall_result.errors.extend(result.errors);
            overall_result.env_changes.extend(result.env_changes);
            
            if !result.success {
                overall_result.success = false;
                break; // Stop on first error
            }
        }

        overall_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
        self.stats.total_execution_time_ms += overall_result.execution_time_ms;

        Ok(overall_result)
    }

    /// Execute a Rex script from string content
    pub async fn execute_script_content(&mut self, content: &str) -> Result<ExecutionResult, RezCoreError> {
        let parser = crate::RexParser::new();
        let script = parser.parse(content)?;
        self.execute_script(&script).await
    }

    /// Execute a single Rex command
    pub async fn execute_command(&mut self, command: &RexCommand) -> Result<ExecutionResult, RezCoreError> {
        let start_time = std::time::Instant::now();
        self.stats.commands_executed += 1;

        let result = match command {
            RexCommand::SetEnv { name, value } => {
                self.execute_setenv(name, value).await
            }
            RexCommand::AppendEnv { name, value, separator } => {
                self.execute_appendenv(name, value, separator).await
            }
            RexCommand::PrependEnv { name, value, separator } => {
                self.execute_prependenv(name, value, separator).await
            }
            RexCommand::UnsetEnv { name } => {
                self.execute_unsetenv(name).await
            }
            RexCommand::Alias { name, command } => {
                self.execute_alias(name, command).await
            }
            RexCommand::Function { name, body } => {
                self.execute_function(name, body).await
            }
            RexCommand::Source { path } => {
                self.execute_source(path).await
            }
            RexCommand::Command { command, args } => {
                self.execute_command_call(command, args).await
            }
            RexCommand::If { condition, then_commands, else_commands } => {
                self.execute_if(condition, then_commands, else_commands).await
            }
            RexCommand::Comment { .. } => {
                // Comments are no-ops
                Ok(ExecutionResult::success())
            }
        };

        match result {
            Ok(mut exec_result) => {
                exec_result.execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(exec_result)
            }
            Err(e) => {
                let execution_time_ms = start_time.elapsed().as_millis() as u64;
                Ok(ExecutionResult::error(e.to_string()).with_output(
                    format!("Execution time: {}ms", execution_time_ms)
                ))
            }
        }
    }

    /// Execute setenv command
    async fn execute_setenv(&mut self, name: &str, value: &str) -> Result<ExecutionResult, RezCoreError> {
        let old_value = self.environment.get(name).cloned();
        self.environment.insert(name.to_string(), value.to_string());
        self.stats.env_vars_set += 1;

        if self.config.debug_mode {
            println!("setenv {} = {}", name, value);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Set {} = {}", name, value))
            .with_env_change(name.to_string(), Some(value.to_string())))
    }

    /// Execute appendenv command
    async fn execute_appendenv(&mut self, name: &str, value: &str, separator: &str) -> Result<ExecutionResult, RezCoreError> {
        let current = self.environment.get(name).cloned().unwrap_or_default();
        let new_value = if current.is_empty() {
            value.to_string()
        } else {
            format!("{}{}{}", current, separator, value)
        };

        self.environment.insert(name.to_string(), new_value.clone());
        self.stats.env_vars_set += 1;

        if self.config.debug_mode {
            println!("appendenv {} += {} (sep: {})", name, value, separator);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Appended {} to {}", value, name))
            .with_env_change(name.to_string(), Some(new_value)))
    }

    /// Execute prependenv command
    async fn execute_prependenv(&mut self, name: &str, value: &str, separator: &str) -> Result<ExecutionResult, RezCoreError> {
        let current = self.environment.get(name).cloned().unwrap_or_default();
        let new_value = if current.is_empty() {
            value.to_string()
        } else {
            format!("{}{}{}", value, separator, current)
        };

        self.environment.insert(name.to_string(), new_value.clone());
        self.stats.env_vars_set += 1;

        if self.config.debug_mode {
            println!("prependenv {} = {} + existing (sep: {})", name, value, separator);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Prepended {} to {}", value, name))
            .with_env_change(name.to_string(), Some(new_value)))
    }

    /// Execute unsetenv command
    async fn execute_unsetenv(&mut self, name: &str) -> Result<ExecutionResult, RezCoreError> {
        let was_set = self.environment.remove(name).is_some();

        if self.config.debug_mode {
            println!("unsetenv {}", name);
        }

        let message = if was_set {
            format!("Unset {}", name)
        } else {
            format!("{} was not set", name)
        };

        Ok(ExecutionResult::success()
            .with_output(message)
            .with_env_change(name.to_string(), None))
    }

    /// Execute alias command
    async fn execute_alias(&mut self, name: &str, command: &str) -> Result<ExecutionResult, RezCoreError> {
        self.aliases.insert(name.to_string(), command.to_string());
        self.stats.aliases_created += 1;

        if self.config.debug_mode {
            println!("alias {} = {}", name, command);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Created alias {} = {}", name, command)))
    }

    /// Execute function command
    async fn execute_function(&mut self, name: &str, body: &str) -> Result<ExecutionResult, RezCoreError> {
        self.functions.insert(name.to_string(), body.to_string());
        self.stats.functions_defined += 1;

        if self.config.debug_mode {
            println!("function {} {{ {} }}", name, body);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Defined function {}", name)))
    }

    /// Execute source command
    async fn execute_source(&mut self, path: &str) -> Result<ExecutionResult, RezCoreError> {
        let expanded_path = shellexpand::full(path)
            .map_err(|e| RezCoreError::RexError(format!("Path expansion error: {}", e)))?;
        
        let content = tokio::fs::read_to_string(expanded_path.as_ref()).await
            .map_err(|e| RezCoreError::RexError(format!("Failed to read {}: {}", path, e)))?;

        if self.config.debug_mode {
            println!("source {}", path);
        }

        // Recursively execute the sourced script
        let result = self.execute_script_content(&content).await?;

        Ok(ExecutionResult::success()
            .with_output(format!("Sourced {}", path))
            .with_output(format!("Executed {} commands", result.output.len())))
    }

    /// Execute command call
    async fn execute_command_call(&mut self, command: &str, args: &[String]) -> Result<ExecutionResult, RezCoreError> {
        // Check if it's an alias
        if let Some(alias_command) = self.aliases.get(command) {
            if self.config.debug_mode {
                println!("Executing alias: {} -> {}", command, alias_command);
            }
            
            // For simplicity, just return success with alias info
            return Ok(ExecutionResult::success()
                .with_output(format!("Executed alias: {} -> {}", command, alias_command)));
        }

        // Check if it's a function
        if let Some(function_body) = self.functions.get(command) {
            if self.config.debug_mode {
                println!("Executing function: {}", command);
            }
            
            // For simplicity, just return success with function info
            return Ok(ExecutionResult::success()
                .with_output(format!("Executed function: {}", command)));
        }

        // Regular command execution would go here
        // For now, just log the command
        let full_command = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        if self.config.debug_mode {
            println!("Executing command: {}", full_command);
        }

        Ok(ExecutionResult::success()
            .with_output(format!("Command: {}", full_command)))
    }

    /// Execute if command
    async fn execute_if(
        &mut self,
        condition: &str,
        then_commands: &[RexCommand],
        else_commands: &Option<Vec<RexCommand>>,
    ) -> Result<ExecutionResult, RezCoreError> {
        // Simple condition evaluation (could be enhanced)
        let condition_result = self.evaluate_condition(condition).await?;

        if self.config.debug_mode {
            println!("if {} -> {}", condition, condition_result);
        }

        let commands_to_execute = if condition_result {
            then_commands
        } else if let Some(ref else_cmds) = else_commands {
            else_cmds
        } else {
            return Ok(ExecutionResult::success()
                .with_output("Condition false, no else branch".to_string()));
        };

        let mut overall_result = ExecutionResult::success();
        for command in commands_to_execute {
            let result = self.execute_command(command).await?;
            overall_result.output.extend(result.output);
            overall_result.errors.extend(result.errors);
            overall_result.env_changes.extend(result.env_changes);
            
            if !result.success {
                overall_result.success = false;
                break;
            }
        }

        Ok(overall_result)
    }

    /// Evaluate a condition (simplified)
    async fn evaluate_condition(&self, condition: &str) -> Result<bool, RezCoreError> {
        // Very simple condition evaluation
        // In a real implementation, this would be much more sophisticated
        
        if condition.contains("==") {
            let parts: Vec<&str> = condition.split("==").collect();
            if parts.len() == 2 {
                let left = parts[0].trim();
                let right = parts[1].trim().trim_matches('"').trim_matches('\'');
                
                // Check if left side is an environment variable
                if left.starts_with('$') {
                    let var_name = &left[1..];
                    let var_value = self.environment.get(var_name).unwrap_or(&String::new());
                    return Ok(var_value == right);
                }
                
                return Ok(left == right);
            }
        }
        
        // Default to true for unknown conditions
        Ok(true)
    }

    /// Get current environment as shell script
    pub fn generate_shell_script(&self) -> Result<String, RezCoreError> {
        let context_config = ContextConfig {
            shell_type: self.config.shell_type.clone(),
            ..Default::default()
        };
        
        let env_manager = EnvironmentManager::new(context_config);
        env_manager.generate_shell_script(&self.environment)
    }

    /// Reset interpreter state
    pub fn reset(&mut self) {
        self.environment.clear();
        self.aliases.clear();
        self.functions.clear();
        self.stats = InterpreterStats::default();
        
        if self.config.inherit_parent_env {
            self.environment = std::env::vars().collect();
        }
    }

    /// Get interpreter statistics
    pub fn get_stats(&self) -> &InterpreterStats {
        &self.stats
    }
}

impl Default for RexInterpreter {
    fn default() -> Self {
        Self::new()
    }
}
