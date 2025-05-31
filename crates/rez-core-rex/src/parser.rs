//! Rex command parser

use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rex command types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RexCommand {
    /// Set environment variable
    SetEnv {
        name: String,
        value: String,
    },
    /// Append to environment variable
    AppendEnv {
        name: String,
        value: String,
        separator: String,
    },
    /// Prepend to environment variable
    PrependEnv {
        name: String,
        value: String,
        separator: String,
    },
    /// Unset environment variable
    UnsetEnv {
        name: String,
    },
    /// Set alias
    Alias {
        name: String,
        command: String,
    },
    /// Define function
    Function {
        name: String,
        body: String,
    },
    /// Source script
    Source {
        path: String,
    },
    /// Execute command
    Command {
        command: String,
        args: Vec<String>,
    },
    /// Conditional execution
    If {
        condition: String,
        then_commands: Vec<RexCommand>,
        else_commands: Option<Vec<RexCommand>>,
    },
    /// Comment
    Comment {
        text: String,
    },
}

impl RexCommand {
    /// Execute simulation for benchmarking purposes
    /// This method simulates command execution without actually performing it
    pub fn execute_simulation(&self) -> Result<String, RezCoreError> {
        match self {
            RexCommand::SetEnv { name, value } => {
                // Simulate environment variable setting
                Ok(format!("Simulated: Set {}={}", name, value))
            }
            RexCommand::AppendEnv { name, value, separator } => {
                // Simulate environment variable appending
                Ok(format!("Simulated: Append {} to {} with separator '{}'", value, name, separator))
            }
            RexCommand::PrependEnv { name, value, separator } => {
                // Simulate environment variable prepending
                Ok(format!("Simulated: Prepend {} to {} with separator '{}'", value, name, separator))
            }
            RexCommand::UnsetEnv { name } => {
                // Simulate environment variable unsetting
                Ok(format!("Simulated: Unset {}", name))
            }
            RexCommand::Alias { name, command } => {
                // Simulate alias creation
                Ok(format!("Simulated: Create alias {}={}", name, command))
            }
            RexCommand::Function { name, body } => {
                // Simulate function definition
                Ok(format!("Simulated: Define function {} with body: {}", name, body))
            }
            RexCommand::Source { path } => {
                // Simulate script sourcing
                Ok(format!("Simulated: Source script {}", path))
            }
            RexCommand::Command { command, args } => {
                // Simulate command execution
                let full_command = if args.is_empty() {
                    command.clone()
                } else {
                    format!("{} {}", command, args.join(" "))
                };
                Ok(format!("Simulated: Execute command {}", full_command))
            }
            RexCommand::If { condition, then_commands, else_commands } => {
                // Simulate conditional execution
                let mut result = format!("Simulated: If condition '{}' then {} commands",
                                        condition, then_commands.len());
                if let Some(else_cmds) = else_commands {
                    result.push_str(&format!(" else {} commands", else_cmds.len()));
                }
                Ok(result)
            }
            RexCommand::Comment { text } => {
                // Simulate comment processing
                Ok(format!("Simulated: Comment '{}'", text))
            }
        }
    }
}

/// Rex script representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexScript {
    /// List of commands
    pub commands: Vec<RexCommand>,
    /// Script metadata
    pub metadata: HashMap<String, String>,
}

impl RexScript {
    /// Create a new empty Rex script
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a command to the script
    pub fn add_command(&mut self, command: RexCommand) {
        self.commands.push(command);
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    /// Get command count
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Check if script is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl Default for RexScript {
    fn default() -> Self {
        Self::new()
    }
}

/// Rex parser for parsing Rex command scripts
pub struct RexParser {
    /// Parser configuration
    config: ParserConfig,
}

/// Parser configuration
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Allow shell-specific syntax
    pub allow_shell_syntax: bool,
    /// Default path separator
    pub default_path_separator: String,
    /// Variable expansion enabled
    pub variable_expansion: bool,
    /// Strict mode (fail on unknown commands)
    pub strict_mode: bool,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            allow_shell_syntax: true,
            default_path_separator: if cfg!(windows) { ";" } else { ":" }.to_string(),
            variable_expansion: true,
            strict_mode: false,
        }
    }
}

impl RexParser {
    /// Create a new Rex parser
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse a Rex script from string
    pub fn parse(&self, content: &str) -> Result<RexScript, RezCoreError> {
        let mut script = RexScript::new();
        
        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();
            
            // Skip empty lines
            if line.is_empty() {
                continue;
            }
            
            // Parse command
            match self.parse_line(line) {
                Ok(Some(command)) => script.add_command(command),
                Ok(None) => {}, // Empty or comment line
                Err(e) => {
                    if self.config.strict_mode {
                        return Err(RezCoreError::RexError(
                            format!("Parse error at line {}: {}", line_num + 1, e)
                        ));
                    } else {
                        // In non-strict mode, treat as comment
                        script.add_command(RexCommand::Comment {
                            text: format!("Parse error: {}", line),
                        });
                    }
                }
            }
        }
        
        Ok(script)
    }

    /// Parse a single line
    fn parse_line(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let line = line.trim();
        
        // Handle comments
        if line.starts_with('#') {
            return Ok(Some(RexCommand::Comment {
                text: line[1..].trim().to_string(),
            }));
        }
        
        // Handle empty lines
        if line.is_empty() {
            return Ok(None);
        }
        
        // Parse different command types
        if line.starts_with("setenv ") {
            self.parse_setenv(line)
        } else if line.starts_with("appendenv ") {
            self.parse_appendenv(line)
        } else if line.starts_with("prependenv ") {
            self.parse_prependenv(line)
        } else if line.starts_with("unsetenv ") {
            self.parse_unsetenv(line)
        } else if line.starts_with("alias ") {
            self.parse_alias(line)
        } else if line.starts_with("function ") {
            self.parse_function(line)
        } else if line.starts_with("source ") {
            self.parse_source(line)
        } else if self.config.allow_shell_syntax {
            // Try to parse as shell command
            self.parse_shell_command(line)
        } else {
            Err(RezCoreError::RexError(
                format!("Unknown command: {}", line)
            ))
        }
    }

    /// Parse setenv command
    fn parse_setenv(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(&line[7..]); // Skip "setenv "
        
        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "setenv requires name and value".to_string()
            ));
        }
        
        let name = parts[0].clone();
        let value = parts[1..].join(" ");
        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };
        
        Ok(Some(RexCommand::SetEnv {
            name,
            value: expanded_value,
        }))
    }

    /// Parse appendenv command
    fn parse_appendenv(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(&line[10..]); // Skip "appendenv "
        
        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "appendenv requires name and value".to_string()
            ));
        }
        
        let name = parts[0].clone();
        let value = parts[1].clone();
        let separator = if parts.len() > 2 {
            parts[2].clone()
        } else {
            self.get_default_separator(&name)
        };
        
        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };
        
        Ok(Some(RexCommand::AppendEnv {
            name,
            value: expanded_value,
            separator,
        }))
    }

    /// Parse prependenv command
    fn parse_prependenv(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(&line[11..]); // Skip "prependenv "
        
        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "prependenv requires name and value".to_string()
            ));
        }
        
        let name = parts[0].clone();
        let value = parts[1].clone();
        let separator = if parts.len() > 2 {
            parts[2].clone()
        } else {
            self.get_default_separator(&name)
        };
        
        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };
        
        Ok(Some(RexCommand::PrependEnv {
            name,
            value: expanded_value,
            separator,
        }))
    }

    /// Parse unsetenv command
    fn parse_unsetenv(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(&line[9..]); // Skip "unsetenv "
        
        if parts.is_empty() {
            return Err(RezCoreError::RexError(
                "unsetenv requires variable name".to_string()
            ));
        }
        
        Ok(Some(RexCommand::UnsetEnv {
            name: parts[0].clone(),
        }))
    }

    /// Parse alias command
    fn parse_alias(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let content = &line[6..]; // Skip "alias "
        
        if let Some(eq_pos) = content.find('=') {
            let name = content[..eq_pos].trim().to_string();
            let command = content[eq_pos + 1..].trim().to_string();
            
            // Remove quotes if present
            let command = if (command.starts_with('"') && command.ends_with('"')) ||
                            (command.starts_with('\'') && command.ends_with('\'')) {
                command[1..command.len()-1].to_string()
            } else {
                command
            };
            
            Ok(Some(RexCommand::Alias { name, command }))
        } else {
            Err(RezCoreError::RexError(
                "alias requires name=command format".to_string()
            ))
        }
    }

    /// Parse function command
    fn parse_function(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let content = &line[9..]; // Skip "function "
        
        if let Some(brace_pos) = content.find('{') {
            let name = content[..brace_pos].trim().to_string();
            let body_start = brace_pos + 1;
            
            if let Some(close_brace) = content.rfind('}') {
                let body = content[body_start..close_brace].trim().to_string();
                Ok(Some(RexCommand::Function { name, body }))
            } else {
                Err(RezCoreError::RexError(
                    "function missing closing brace".to_string()
                ))
            }
        } else {
            Err(RezCoreError::RexError(
                "function requires name { body } format".to_string()
            ))
        }
    }

    /// Parse source command
    fn parse_source(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(&line[7..]); // Skip "source "
        
        if parts.is_empty() {
            return Err(RezCoreError::RexError(
                "source requires file path".to_string()
            ));
        }
        
        let path = if self.config.variable_expansion {
            self.expand_variables(&parts[0])?
        } else {
            parts[0].clone()
        };
        
        Ok(Some(RexCommand::Source { path }))
    }

    /// Parse shell command
    fn parse_shell_command(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line(line);
        
        if parts.is_empty() {
            return Ok(None);
        }
        
        let command = parts[0].clone();
        let args = parts[1..].to_vec();
        
        Ok(Some(RexCommand::Command { command, args }))
    }

    /// Split command line into parts, respecting quotes
    fn split_command_line(&self, line: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current_part = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        let mut chars = line.chars().peekable();
        
        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                ch if in_quotes && ch == quote_char => {
                    in_quotes = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current_part.is_empty() {
                        parts.push(current_part.clone());
                        current_part.clear();
                    }
                }
                _ => {
                    current_part.push(ch);
                }
            }
        }
        
        if !current_part.is_empty() {
            parts.push(current_part);
        }
        
        parts
    }

    /// Get default separator for environment variable
    fn get_default_separator(&self, var_name: &str) -> String {
        match var_name.to_uppercase().as_str() {
            "PATH" | "LD_LIBRARY_PATH" | "PYTHONPATH" | "CLASSPATH" => {
                self.config.default_path_separator.clone()
            }
            _ => " ".to_string(),
        }
    }

    /// Expand variables in a string
    fn expand_variables(&self, value: &str) -> Result<String, RezCoreError> {
        shellexpand::env(value)
            .map(|expanded| expanded.to_string())
            .map_err(|e| RezCoreError::RexError(
                format!("Variable expansion error: {}", e)
            ))
    }
}

impl Default for RexParser {
    fn default() -> Self {
        Self::new()
    }
}
