//! Rex bindings for package commands

use crate::{RexCommand, RexScript, RexCommandBuilder};
use rez_core_common::RezCoreError;
use rez_core_package::Package;
use rez_core_context::{ResolvedContext, ShellType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Rex binding generator for packages
#[derive(Debug, Clone)]
pub struct RexBindingGenerator {
    /// Shell type to generate bindings for
    shell_type: ShellType,
    /// Binding configuration
    config: BindingConfig,
}

/// Binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    /// Whether to generate PATH modifications
    pub generate_path_bindings: bool,
    /// Whether to generate tool aliases
    pub generate_tool_aliases: bool,
    /// Whether to generate package-specific environment variables
    pub generate_package_env_vars: bool,
    /// Custom environment variable prefix
    pub env_var_prefix: Option<String>,
    /// Path separator for the target platform
    pub path_separator: String,
    /// Whether to use absolute paths
    pub use_absolute_paths: bool,
}

impl Default for BindingConfig {
    fn default() -> Self {
        Self {
            generate_path_bindings: true,
            generate_tool_aliases: true,
            generate_package_env_vars: true,
            env_var_prefix: None,
            path_separator: if cfg!(windows) { ";" } else { ":" }.to_string(),
            use_absolute_paths: true,
        }
    }
}

impl RexBindingGenerator {
    /// Create a new binding generator
    pub fn new(shell_type: ShellType) -> Self {
        Self {
            shell_type,
            config: BindingConfig::default(),
        }
    }

    /// Create a binding generator with custom configuration
    pub fn with_config(shell_type: ShellType, config: BindingConfig) -> Self {
        Self {
            shell_type,
            config,
        }
    }

    /// Generate Rex bindings for a single package
    pub fn generate_package_bindings(&self, package: &Package) -> Result<RexScript, RezCoreError> {
        let mut builder = RexCommandBuilder::new();

        // Add header comment
        builder = builder.comment(format!("Bindings for package: {}", package.name));
        
        if let Some(ref version) = package.version {
            builder = builder.comment(format!("Version: {}", version.as_str()));
        }

        // Generate package-specific environment variables
        if self.config.generate_package_env_vars {
            builder = self.add_package_env_vars(builder, package)?;
        }

        // Generate PATH modifications for tools
        if self.config.generate_path_bindings && !package.tools.is_empty() {
            builder = self.add_path_bindings(builder, package)?;
        }

        // Generate tool aliases
        if self.config.generate_tool_aliases {
            builder = self.add_tool_aliases(builder, package)?;
        }

        // Parse and add package commands if present
        if let Some(ref commands) = package.commands {
            builder = self.add_package_commands(builder, package, commands)?;
        }

        Ok(builder.build())
    }

    /// Generate Rex bindings for a resolved context
    pub fn generate_context_bindings(&self, context: &ResolvedContext) -> Result<RexScript, RezCoreError> {
        let mut builder = RexCommandBuilder::new();

        // Add header comment
        builder = builder.comment("Generated bindings for resolved context".to_string());
        builder = builder.comment(format!("Context ID: {}", context.id));
        
        if let Some(ref name) = context.name {
            builder = builder.comment(format!("Context name: {}", name));
        }

        // Generate bindings for each package in dependency order
        for package in &context.resolved_packages {
            let package_script = self.generate_package_bindings(package)?;
            
            // Add separator comment
            builder = builder.comment(format!("--- Package: {} ---", package.name));
            
            // Add all commands from the package script
            for command in package_script.commands {
                match command {
                    RexCommand::Comment { .. } => {
                        // Skip package-level comments to avoid duplication
                    }
                    _ => {
                        builder.commands.push(command);
                    }
                }
            }
        }

        // Add context-specific environment variables
        for (name, value) in &context.environment_vars {
            // Skip variables that were already set by packages
            if !self.is_package_generated_var(name) {
                builder = builder.setenv(name, value);
            }
        }

        Ok(builder.build())
    }

    /// Add package-specific environment variables
    fn add_package_env_vars(&self, mut builder: RexCommandBuilder, package: &Package) -> Result<RexCommandBuilder, RezCoreError> {
        let prefix = self.config.env_var_prefix.as_deref().unwrap_or("");
        let package_name_upper = package.name.to_uppercase();

        // Set package root
        let root_var = format!("{}{}_ROOT", prefix, package_name_upper);
        let root_path = format!("/packages/{}", package.name);
        builder = builder.setenv(root_var, root_path);

        // Set package version
        if let Some(ref version) = package.version {
            let version_var = format!("{}{}_VERSION", prefix, package_name_upper);
            builder = builder.setenv(version_var, version.as_str());
        }

        // Set package tools directory
        if !package.tools.is_empty() {
            let tools_var = format!("{}{}_TOOLS", prefix, package_name_upper);
            let tools_path = format!("/packages/{}/bin", package.name);
            builder = builder.setenv(tools_var, tools_path);
        }

        Ok(builder)
    }

    /// Add PATH bindings for package tools
    fn add_path_bindings(&self, mut builder: RexCommandBuilder, package: &Package) -> Result<RexCommandBuilder, RezCoreError> {
        if package.tools.is_empty() {
            return Ok(builder);
        }

        let tools_path = if self.config.use_absolute_paths {
            format!("/packages/{}/bin", package.name)
        } else {
            format!("${{REZ_PACKAGE_ROOT}}/bin")
        };

        builder = builder.prependenv("PATH", tools_path, self.config.path_separator.clone());

        Ok(builder)
    }

    /// Add tool aliases
    fn add_tool_aliases(&self, mut builder: RexCommandBuilder, package: &Package) -> Result<RexCommandBuilder, RezCoreError> {
        for tool in &package.tools {
            let tool_path = if self.config.use_absolute_paths {
                format!("/packages/{}/bin/{}", package.name, tool)
            } else {
                format!("${{REZ_PACKAGE_ROOT}}/bin/{}", tool)
            };

            // Create alias for the tool
            builder = builder.alias(tool.clone(), tool_path);
        }

        Ok(builder)
    }

    /// Add package commands
    fn add_package_commands(&self, mut builder: RexCommandBuilder, package: &Package, commands: &str) -> Result<RexCommandBuilder, RezCoreError> {
        // Parse the commands string and convert to Rex commands
        let parsed_commands = self.parse_package_commands(package, commands)?;
        
        for command in parsed_commands {
            builder.commands.push(command);
        }

        Ok(builder)
    }

    /// Parse package commands into Rex commands
    fn parse_package_commands(&self, package: &Package, commands: &str) -> Result<Vec<RexCommand>, RezCoreError> {
        let mut rex_commands = Vec::new();

        for line in commands.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Try to parse as Rex command or convert shell command
            let rex_command = self.convert_shell_to_rex(package, line)?;
            rex_commands.push(rex_command);
        }

        Ok(rex_commands)
    }

    /// Convert shell command to Rex command
    fn convert_shell_to_rex(&self, package: &Package, line: &str) -> Result<RexCommand, RezCoreError> {
        // Handle common shell patterns and convert to Rex commands
        
        if line.starts_with("export ") {
            // Parse export statement
            self.parse_export_to_rex(line)
        } else if line.starts_with("alias ") {
            // Parse alias statement
            self.parse_alias_to_rex(line)
        } else if line.contains("PATH=") || line.contains("PATH:") {
            // Parse PATH modification
            self.parse_path_modification_to_rex(line)
        } else {
            // Treat as regular command
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                return Ok(RexCommand::Comment { text: line.to_string() });
            }

            let command = parts[0].to_string();
            let args = parts[1..].iter().map(|s| s.to_string()).collect();
            
            Ok(RexCommand::Command { command, args })
        }
    }

    /// Parse export statement to Rex command
    fn parse_export_to_rex(&self, line: &str) -> Result<RexCommand, RezCoreError> {
        // Simple regex-based parsing for export statements
        let export_content = &line[7..]; // Skip "export "
        
        if let Some(eq_pos) = export_content.find('=') {
            let var_name = export_content[..eq_pos].trim().to_string();
            let var_value = export_content[eq_pos + 1..].trim().to_string();
            
            // Remove quotes if present
            let var_value = if (var_value.starts_with('"') && var_value.ends_with('"')) ||
                              (var_value.starts_with('\'') && var_value.ends_with('\'')) {
                var_value[1..var_value.len()-1].to_string()
            } else {
                var_value
            };

            Ok(RexCommand::SetEnv {
                name: var_name,
                value: var_value,
            })
        } else {
            Err(RezCoreError::RexError(
                format!("Invalid export statement: {}", line)
            ))
        }
    }

    /// Parse alias statement to Rex command
    fn parse_alias_to_rex(&self, line: &str) -> Result<RexCommand, RezCoreError> {
        let alias_content = &line[6..]; // Skip "alias "
        
        if let Some(eq_pos) = alias_content.find('=') {
            let alias_name = alias_content[..eq_pos].trim().to_string();
            let alias_command = alias_content[eq_pos + 1..].trim().to_string();
            
            // Remove quotes if present
            let alias_command = if (alias_command.starts_with('"') && alias_command.ends_with('"')) ||
                                  (alias_command.starts_with('\'') && alias_command.ends_with('\'')) {
                alias_command[1..alias_command.len()-1].to_string()
            } else {
                alias_command
            };

            Ok(RexCommand::Alias {
                name: alias_name,
                command: alias_command,
            })
        } else {
            Err(RezCoreError::RexError(
                format!("Invalid alias statement: {}", line)
            ))
        }
    }

    /// Parse PATH modification to Rex command
    fn parse_path_modification_to_rex(&self, line: &str) -> Result<RexCommand, RezCoreError> {
        // This is a simplified parser for PATH modifications
        // In a real implementation, this would be more sophisticated
        
        if line.contains("PATH=") {
            // Direct PATH assignment
            if let Some(eq_pos) = line.find("PATH=") {
                let path_value = &line[eq_pos + 5..];
                return Ok(RexCommand::SetEnv {
                    name: "PATH".to_string(),
                    value: path_value.to_string(),
                });
            }
        } else if line.contains("PATH:") || line.contains("$PATH") {
            // PATH modification (prepend/append)
            // This is a simplified heuristic
            if line.contains("$PATH:") {
                // Prepend
                let new_path = line.replace("$PATH:", "").replace("export PATH=", "").trim().to_string();
                return Ok(RexCommand::PrependEnv {
                    name: "PATH".to_string(),
                    value: new_path,
                    separator: self.config.path_separator.clone(),
                });
            } else if line.contains(":$PATH") {
                // Append
                let new_path = line.replace(":$PATH", "").replace("export PATH=", "").trim().to_string();
                return Ok(RexCommand::AppendEnv {
                    name: "PATH".to_string(),
                    value: new_path,
                    separator: self.config.path_separator.clone(),
                });
            }
        }

        // Fallback: treat as comment
        Ok(RexCommand::Comment {
            text: format!("Unparsed PATH modification: {}", line),
        })
    }

    /// Check if an environment variable is generated by package bindings
    fn is_package_generated_var(&self, var_name: &str) -> bool {
        let upper_name = var_name.to_uppercase();
        upper_name.ends_with("_ROOT") || 
        upper_name.ends_with("_VERSION") || 
        upper_name.ends_with("_TOOLS") ||
        upper_name == "PATH"
    }

    /// Generate bindings for a specific shell type
    pub fn generate_for_shell(&self, context: &ResolvedContext, target_shell: ShellType) -> Result<String, RezCoreError> {
        // Create a new generator for the target shell
        let target_generator = RexBindingGenerator::with_config(target_shell.clone(), self.config.clone());
        
        // Generate the Rex script
        let rex_script = target_generator.generate_context_bindings(context)?;
        
        // Convert to shell script
        crate::RexCommandUtils::script_to_shell_script(&rex_script, &target_shell)
    }
}

/// Rex binding utilities
pub struct RexBindingUtils;

impl RexBindingUtils {
    /// Generate bindings for all common shell types
    pub fn generate_all_shell_bindings(context: &ResolvedContext) -> Result<HashMap<ShellType, String>, RezCoreError> {
        let mut bindings = HashMap::new();
        let generator = RexBindingGenerator::new(ShellType::Bash);

        let shell_types = vec![
            ShellType::Bash,
            ShellType::Zsh,
            ShellType::Fish,
            ShellType::Cmd,
            ShellType::PowerShell,
        ];

        for shell_type in shell_types {
            let binding_script = generator.generate_for_shell(context, shell_type.clone())?;
            bindings.insert(shell_type, binding_script);
        }

        Ok(bindings)
    }

    /// Save bindings to files
    pub async fn save_bindings_to_files(
        bindings: &HashMap<ShellType, String>,
        output_dir: &PathBuf,
    ) -> Result<(), RezCoreError> {
        tokio::fs::create_dir_all(output_dir).await
            .map_err(|e| RezCoreError::RexError(format!("Failed to create output directory: {}", e)))?;

        for (shell_type, script) in bindings {
            let filename = match shell_type {
                ShellType::Bash => "bindings.sh",
                ShellType::Zsh => "bindings.zsh",
                ShellType::Fish => "bindings.fish",
                ShellType::Cmd => "bindings.bat",
                ShellType::PowerShell => "bindings.ps1",
            };

            let file_path = output_dir.join(filename);
            tokio::fs::write(&file_path, script).await
                .map_err(|e| RezCoreError::RexError(
                    format!("Failed to write {}: {}", file_path.display(), e)
                ))?;
        }

        Ok(())
    }

    /// Validate Rex bindings
    pub fn validate_bindings(script: &RexScript) -> Result<BindingValidation, RezCoreError> {
        let mut validation = BindingValidation::default();

        for command in &script.commands {
            match command {
                RexCommand::SetEnv { name, .. } => {
                    validation.env_vars_set += 1;
                    if name.is_empty() {
                        validation.errors.push("Empty environment variable name".to_string());
                    }
                }
                RexCommand::AppendEnv { name, .. } | RexCommand::PrependEnv { name, .. } => {
                    validation.path_modifications += 1;
                    if name.is_empty() {
                        validation.errors.push("Empty environment variable name".to_string());
                    }
                }
                RexCommand::Alias { name, command } => {
                    validation.aliases_created += 1;
                    if name.is_empty() || command.is_empty() {
                        validation.errors.push("Empty alias name or command".to_string());
                    }
                }
                RexCommand::Function { name, body } => {
                    validation.functions_defined += 1;
                    if name.is_empty() || body.is_empty() {
                        validation.errors.push("Empty function name or body".to_string());
                    }
                }
                _ => {}
            }
        }

        validation.is_valid = validation.errors.is_empty();
        Ok(validation)
    }
}

/// Binding validation result
#[derive(Debug, Clone, Default)]
pub struct BindingValidation {
    /// Whether the bindings are valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Number of environment variables set
    pub env_vars_set: usize,
    /// Number of PATH modifications
    pub path_modifications: usize,
    /// Number of aliases created
    pub aliases_created: usize,
    /// Number of functions defined
    pub functions_defined: usize,
}
