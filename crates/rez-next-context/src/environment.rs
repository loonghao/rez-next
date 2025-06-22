//! Environment variable generation and management

use crate::{ContextConfig, PathStrategy, ShellType};
use rez_next_common::RezCoreError;
use rez_next_package::Package;
// use pyo3::prelude::*;  // Temporarily disabled due to DLL issues
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};

/// Environment manager for generating package environments
// #[pyclass]  // Temporarily disabled due to DLL issues
#[derive(Debug, Clone)]
pub struct EnvironmentManager {
    /// Configuration for environment generation
    config: ContextConfig,
    /// Base environment variables
    base_env: HashMap<String, String>,
}

/// Environment variable operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvOperation {
    /// Set a variable to a value
    Set(String),
    /// Prepend to a variable (with separator)
    Prepend(String, String), // value, separator
    /// Append to a variable (with separator)
    Append(String, String), // value, separator
    /// Unset a variable
    Unset,
}

/// Environment variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVarDefinition {
    /// Variable name
    pub name: String,
    /// Operation to perform
    pub operation: EnvOperation,
    /// Source package (if any)
    pub source_package: Option<String>,
    /// Priority (higher = applied later)
    pub priority: i32,
}

// Python methods temporarily disabled due to DLL issues
/*
#[pymethods]
impl EnvironmentManager {
    #[new]
    pub fn new_py() -> Self {
        Self::new(ContextConfig::default())
    }

    /// Generate environment variables for packages
    #[cfg(feature = "python-bindings")]
    pub fn generate_environment_py(&self, packages: Vec<Package>) -> PyResult<HashMap<String, String>> {
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.generate_environment(&packages));

        result.map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
    }

    /// Get base environment variables
    #[getter]
    pub fn base_env(&self) -> HashMap<String, String> {
        self.base_env.clone()
    }
}
*/

impl EnvironmentManager {
    /// Create a new environment manager
    pub fn new(config: ContextConfig) -> Self {
        let base_env = if config.inherit_parent_env {
            env::vars().collect()
        } else {
            HashMap::new()
        };

        Self { config, base_env }
    }

    /// Generate environment variables for a list of packages
    pub async fn generate_environment(
        &self,
        packages: &[Package],
    ) -> Result<HashMap<String, String>, RezCoreError> {
        let mut env_vars = self.base_env.clone();
        let mut env_definitions = Vec::new();

        // Collect environment variable definitions from packages
        for (index, package) in packages.iter().enumerate() {
            let package_env_defs = self.extract_package_env_definitions(package, index as i32)?;
            env_definitions.extend(package_env_defs);
        }

        // Add additional environment variables from config
        for (name, value) in &self.config.additional_env_vars {
            env_definitions.push(EnvVarDefinition {
                name: name.clone(),
                operation: EnvOperation::Set(value.clone()),
                source_package: None,
                priority: 1000, // High priority for user-defined vars
            });
        }

        // Sort by priority (lower priority first)
        env_definitions.sort_by_key(|def| def.priority);

        // Apply environment variable definitions
        for env_def in env_definitions {
            self.apply_env_definition(&mut env_vars, &env_def)?;
        }

        // Handle PATH modifications
        self.apply_path_modifications(&mut env_vars, packages)?;

        // Unset specified variables
        for var_name in &self.config.unset_vars {
            env_vars.remove(var_name);
        }

        Ok(env_vars)
    }

    /// Extract environment variable definitions from a package
    fn extract_package_env_definitions(
        &self,
        package: &Package,
        priority: i32,
    ) -> Result<Vec<EnvVarDefinition>, RezCoreError> {
        let mut definitions = Vec::new();

        // Set package-specific environment variables
        definitions.push(EnvVarDefinition {
            name: format!("{}_ROOT", package.name.to_uppercase()),
            operation: EnvOperation::Set(format!("/packages/{}", package.name)),
            source_package: Some(package.name.clone()),
            priority,
        });

        if let Some(ref version) = package.version {
            definitions.push(EnvVarDefinition {
                name: format!("{}_VERSION", package.name.to_uppercase()),
                operation: EnvOperation::Set(version.as_str().to_string()),
                source_package: Some(package.name.clone()),
                priority,
            });
        }

        // Add tools to PATH (will be handled separately in apply_path_modifications)

        // Parse commands for environment variable operations
        if let Some(ref commands) = package.commands {
            let command_env_defs =
                self.parse_commands_for_env_vars(commands, &package.name, priority)?;
            definitions.extend(command_env_defs);
        }

        Ok(definitions)
    }

    /// Parse package commands for environment variable operations
    fn parse_commands_for_env_vars(
        &self,
        commands: &str,
        package_name: &str,
        priority: i32,
    ) -> Result<Vec<EnvVarDefinition>, RezCoreError> {
        let mut definitions = Vec::new();

        // Simple command parsing (in a real implementation, this would be more sophisticated)
        for line in commands.lines() {
            let line = line.trim();

            if line.starts_with("export ") {
                // Parse export statements
                if let Some(env_def) = self.parse_export_statement(line, package_name, priority)? {
                    definitions.push(env_def);
                }
            } else if line.starts_with("setenv ") {
                // Parse setenv statements (csh/tcsh style)
                if let Some(env_def) = self.parse_setenv_statement(line, package_name, priority)? {
                    definitions.push(env_def);
                }
            }
        }

        Ok(definitions)
    }

    /// Parse an export statement
    fn parse_export_statement(
        &self,
        line: &str,
        package_name: &str,
        priority: i32,
    ) -> Result<Option<EnvVarDefinition>, RezCoreError> {
        // Simple regex-based parsing
        let export_regex = regex::Regex::new(r#"export\s+([A-Z_][A-Z0-9_]*)\s*=\s*"?([^"]*)"?"#)
            .map_err(|e| RezCoreError::ContextError(format!("Regex error: {}", e)))?;

        if let Some(captures) = export_regex.captures(line) {
            let var_name = captures.get(1).unwrap().as_str().to_string();
            let var_value = captures.get(2).unwrap().as_str().to_string();

            // Expand variables in the value
            let expanded_value = self.expand_variables(&var_value)?;

            Ok(Some(EnvVarDefinition {
                name: var_name,
                operation: EnvOperation::Set(expanded_value),
                source_package: Some(package_name.to_string()),
                priority,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse a setenv statement
    fn parse_setenv_statement(
        &self,
        line: &str,
        package_name: &str,
        priority: i32,
    ) -> Result<Option<EnvVarDefinition>, RezCoreError> {
        // Simple parsing for setenv VAR value
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == "setenv" {
            let var_name = parts[1].to_string();
            let var_value = parts[2..].join(" ");

            // Remove quotes if present
            let var_value = var_value.trim_matches('"').trim_matches('\'');
            let expanded_value = self.expand_variables(var_value)?;

            Ok(Some(EnvVarDefinition {
                name: var_name,
                operation: EnvOperation::Set(expanded_value),
                source_package: Some(package_name.to_string()),
                priority,
            }))
        } else {
            Ok(None)
        }
    }

    /// Expand variables in a value string
    fn expand_variables(&self, value: &str) -> Result<String, RezCoreError> {
        // Simple variable expansion (${VAR} and $VAR)
        let expanded = shellexpand::env(value)
            .map_err(|e| RezCoreError::ContextError(format!("Variable expansion error: {}", e)))?;

        Ok(expanded.to_string())
    }

    /// Apply an environment variable definition
    fn apply_env_definition(
        &self,
        env_vars: &mut HashMap<String, String>,
        env_def: &EnvVarDefinition,
    ) -> Result<(), RezCoreError> {
        match &env_def.operation {
            EnvOperation::Set(value) => {
                env_vars.insert(env_def.name.clone(), value.clone());
            }
            EnvOperation::Prepend(value, separator) => {
                let current = env_vars.get(&env_def.name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", value, separator, current)
                };
                env_vars.insert(env_def.name.clone(), new_value);
            }
            EnvOperation::Append(value, separator) => {
                let current = env_vars.get(&env_def.name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", current, separator, value)
                };
                env_vars.insert(env_def.name.clone(), new_value);
            }
            EnvOperation::Unset => {
                env_vars.remove(&env_def.name);
            }
        }

        Ok(())
    }

    /// Apply PATH modifications based on package tools
    fn apply_path_modifications(
        &self,
        env_vars: &mut HashMap<String, String>,
        packages: &[Package],
    ) -> Result<(), RezCoreError> {
        if self.config.path_strategy == PathStrategy::NoModify {
            return Ok(());
        }

        let mut tool_paths = Vec::new();

        // Collect tool paths from packages
        for package in packages {
            for tool in &package.tools {
                let tool_path = format!("/packages/{}/bin", package.name);
                if !tool_paths.contains(&tool_path) {
                    tool_paths.push(tool_path);
                }
            }
        }

        if tool_paths.is_empty() {
            return Ok(());
        }

        let path_separator = self.get_path_separator();
        let new_path_segment = tool_paths.join(&path_separator);
        let current_path = env_vars.get("PATH").cloned().unwrap_or_default();

        let new_path = match self.config.path_strategy {
            PathStrategy::Prepend => {
                if current_path.is_empty() {
                    new_path_segment
                } else {
                    format!("{}{}{}", new_path_segment, path_separator, current_path)
                }
            }
            PathStrategy::Append => {
                if current_path.is_empty() {
                    new_path_segment
                } else {
                    format!("{}{}{}", current_path, path_separator, new_path_segment)
                }
            }
            PathStrategy::Replace => new_path_segment,
            PathStrategy::NoModify => current_path,
        };

        env_vars.insert("PATH".to_string(), new_path);
        Ok(())
    }

    /// Get the appropriate path separator for the current platform
    fn get_path_separator(&self) -> String {
        match self.config.shell_type {
            ShellType::Cmd | ShellType::PowerShell => ";".to_string(),
            _ => ":".to_string(),
        }
    }

    /// Generate shell script for environment setup
    pub fn generate_shell_script(
        &self,
        env_vars: &HashMap<String, String>,
    ) -> Result<String, RezCoreError> {
        let mut script = String::new();

        match self.config.shell_type {
            ShellType::Bash => {
                script.push_str("#!/bin/bash\n");
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in env_vars {
                    script.push_str(&format!(
                        "export {}=\"{}\"\n",
                        name,
                        self.escape_bash_value(value)
                    ));
                }
            }
            ShellType::Cmd => {
                script.push_str("@echo off\n");
                script.push_str("REM Generated by rez-core\n\n");

                for (name, value) in env_vars {
                    script.push_str(&format!("set {}={}\n", name, value));
                }
            }
            ShellType::PowerShell => {
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in env_vars {
                    script.push_str(&format!(
                        "$env:{} = \"{}\"\n",
                        name,
                        self.escape_powershell_value(value)
                    ));
                }
            }
            ShellType::Zsh => {
                script.push_str("#!/bin/zsh\n");
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in env_vars {
                    script.push_str(&format!(
                        "export {}=\"{}\"\n",
                        name,
                        self.escape_bash_value(value)
                    ));
                }
            }
            ShellType::Fish => {
                script.push_str("#!/usr/bin/env fish\n");
                script.push_str("# Generated by rez-core\n\n");

                for (name, value) in env_vars {
                    script.push_str(&format!(
                        "set -x {} \"{}\"\n",
                        name,
                        self.escape_fish_value(value)
                    ));
                }
            }
        }

        Ok(script)
    }

    /// Escape a value for bash/zsh
    fn escape_bash_value(&self, value: &str) -> String {
        value
            .replace("\"", "\\\"")
            .replace("$", "\\$")
            .replace("`", "\\`")
    }

    /// Escape a value for PowerShell
    fn escape_powershell_value(&self, value: &str) -> String {
        value.replace("\"", "`\"").replace("$", "`$")
    }

    /// Escape a value for fish shell
    fn escape_fish_value(&self, value: &str) -> String {
        value.replace("\"", "\\\"").replace("$", "\\$")
    }

    /// Get environment variable differences from base environment
    pub fn get_env_diff(&self, env_vars: &HashMap<String, String>) -> EnvDiff {
        let mut added = HashMap::new();
        let mut modified = HashMap::new();
        let mut removed = Vec::new();

        // Find added and modified variables
        for (name, value) in env_vars {
            match self.base_env.get(name) {
                Some(base_value) => {
                    if base_value != value {
                        modified.insert(name.clone(), (base_value.clone(), value.clone()));
                    }
                }
                None => {
                    added.insert(name.clone(), value.clone());
                }
            }
        }

        // Find removed variables
        for name in self.base_env.keys() {
            if !env_vars.contains_key(name) {
                removed.push(name.clone());
            }
        }

        EnvDiff {
            added,
            modified,
            removed,
        }
    }
}

/// Environment variable differences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvDiff {
    /// Added variables
    pub added: HashMap<String, String>,
    /// Modified variables (old_value, new_value)
    pub modified: HashMap<String, (String, String)>,
    /// Removed variables
    pub removed: Vec<String>,
}

impl EnvDiff {
    /// Check if there are any differences
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }

    /// Get the total number of changes
    pub fn change_count(&self) -> usize {
        self.added.len() + self.modified.len() + self.removed.len()
    }
}
