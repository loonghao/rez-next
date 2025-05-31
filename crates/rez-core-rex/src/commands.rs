//! Rex command definitions and utilities

use crate::{RexCommand, RexScript};
use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rex command builder for creating commands programmatically
#[derive(Debug, Clone)]
pub struct RexCommandBuilder {
    commands: Vec<RexCommand>,
}

impl RexCommandBuilder {
    /// Create a new command builder
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Add a setenv command
    pub fn setenv(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.commands.push(RexCommand::SetEnv {
            name: name.into(),
            value: value.into(),
        });
        self
    }

    /// Add an appendenv command
    pub fn appendenv(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        separator: impl Into<String>,
    ) -> Self {
        self.commands.push(RexCommand::AppendEnv {
            name: name.into(),
            value: value.into(),
            separator: separator.into(),
        });
        self
    }

    /// Add a prependenv command
    pub fn prependenv(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        separator: impl Into<String>,
    ) -> Self {
        self.commands.push(RexCommand::PrependEnv {
            name: name.into(),
            value: value.into(),
            separator: separator.into(),
        });
        self
    }

    /// Add an unsetenv command
    pub fn unsetenv(mut self, name: impl Into<String>) -> Self {
        self.commands.push(RexCommand::UnsetEnv {
            name: name.into(),
        });
        self
    }

    /// Add an alias command
    pub fn alias(mut self, name: impl Into<String>, command: impl Into<String>) -> Self {
        self.commands.push(RexCommand::Alias {
            name: name.into(),
            command: command.into(),
        });
        self
    }

    /// Add a function command
    pub fn function(mut self, name: impl Into<String>, body: impl Into<String>) -> Self {
        self.commands.push(RexCommand::Function {
            name: name.into(),
            body: body.into(),
        });
        self
    }

    /// Add a source command
    pub fn source(mut self, path: impl Into<String>) -> Self {
        self.commands.push(RexCommand::Source {
            path: path.into(),
        });
        self
    }

    /// Add a command execution
    pub fn command(mut self, command: impl Into<String>, args: Vec<String>) -> Self {
        self.commands.push(RexCommand::Command {
            command: command.into(),
            args,
        });
        self
    }

    /// Add a comment
    pub fn comment(mut self, text: impl Into<String>) -> Self {
        self.commands.push(RexCommand::Comment {
            text: text.into(),
        });
        self
    }

    /// Add an if command
    pub fn if_then(
        mut self,
        condition: impl Into<String>,
        then_commands: Vec<RexCommand>,
    ) -> Self {
        self.commands.push(RexCommand::If {
            condition: condition.into(),
            then_commands,
            else_commands: None,
        });
        self
    }

    /// Add an if-else command
    pub fn if_then_else(
        mut self,
        condition: impl Into<String>,
        then_commands: Vec<RexCommand>,
        else_commands: Vec<RexCommand>,
    ) -> Self {
        self.commands.push(RexCommand::If {
            condition: condition.into(),
            then_commands,
            else_commands: Some(else_commands),
        });
        self
    }

    /// Build the Rex script
    pub fn build(self) -> RexScript {
        RexScript {
            commands: self.commands,
            metadata: HashMap::new(),
        }
    }

    /// Build with metadata
    pub fn build_with_metadata(self, metadata: HashMap<String, String>) -> RexScript {
        RexScript {
            commands: self.commands,
            metadata,
        }
    }
}

impl Default for RexCommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Rex command utilities
pub struct RexCommandUtils;

impl RexCommandUtils {
    /// Convert a Rex command to shell script
    pub fn command_to_shell_script(
        command: &RexCommand,
        shell_type: &rez_core_context::ShellType,
    ) -> Result<String, RezCoreError> {
        match command {
            RexCommand::SetEnv { name, value } => {
                Ok(Self::generate_setenv_script(name, value, shell_type))
            }
            RexCommand::AppendEnv { name, value, separator } => {
                Ok(Self::generate_appendenv_script(name, value, separator, shell_type))
            }
            RexCommand::PrependEnv { name, value, separator } => {
                Ok(Self::generate_prependenv_script(name, value, separator, shell_type))
            }
            RexCommand::UnsetEnv { name } => {
                Ok(Self::generate_unsetenv_script(name, shell_type))
            }
            RexCommand::Alias { name, command } => {
                Ok(Self::generate_alias_script(name, command, shell_type))
            }
            RexCommand::Function { name, body } => {
                Ok(Self::generate_function_script(name, body, shell_type))
            }
            RexCommand::Source { path } => {
                Ok(Self::generate_source_script(path, shell_type))
            }
            RexCommand::Command { command, args } => {
                let full_command = if args.is_empty() {
                    command.clone()
                } else {
                    format!("{} {}", command, args.join(" "))
                };
                Ok(full_command)
            }
            RexCommand::Comment { text } => {
                Ok(Self::generate_comment_script(text, shell_type))
            }
            RexCommand::If { condition, then_commands, else_commands } => {
                Self::generate_if_script(condition, then_commands, else_commands, shell_type)
            }
        }
    }

    /// Generate setenv script
    fn generate_setenv_script(
        name: &str,
        value: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("export {}=\"{}\"", name, Self::escape_bash_value(value))
            }
            rez_core_context::ShellType::Fish => {
                format!("set -x {} \"{}\"", name, Self::escape_fish_value(value))
            }
            rez_core_context::ShellType::Cmd => {
                format!("set {}={}", name, value)
            }
            rez_core_context::ShellType::PowerShell => {
                format!("$env:{} = \"{}\"", name, Self::escape_powershell_value(value))
            }
        }
    }

    /// Generate appendenv script
    fn generate_appendenv_script(
        name: &str,
        value: &str,
        separator: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!(
                    "export {}=\"${{{}}}{}{}\"",
                    name,
                    name,
                    separator,
                    Self::escape_bash_value(value)
                )
            }
            rez_core_context::ShellType::Fish => {
                format!("set -x {} ${}{}\"{}\"", name, name, separator, Self::escape_fish_value(value))
            }
            rez_core_context::ShellType::Cmd => {
                format!("set {}=%{}%{}{}", name, name, separator, value)
            }
            rez_core_context::ShellType::PowerShell => {
                format!(
                    "$env:{} = $env:{} + \"{}\" + \"{}\"",
                    name,
                    name,
                    separator,
                    Self::escape_powershell_value(value)
                )
            }
        }
    }

    /// Generate prependenv script
    fn generate_prependenv_script(
        name: &str,
        value: &str,
        separator: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!(
                    "export {}=\"{}{}${{{}}}\"",
                    name,
                    Self::escape_bash_value(value),
                    separator,
                    name
                )
            }
            rez_core_context::ShellType::Fish => {
                format!("set -x {} \"{}\"{}${}", name, Self::escape_fish_value(value), separator, name)
            }
            rez_core_context::ShellType::Cmd => {
                format!("set {}={}{}{}", name, value, separator, format!("%{}%", name))
            }
            rez_core_context::ShellType::PowerShell => {
                format!(
                    "$env:{} = \"{}\" + \"{}\" + $env:{}",
                    name,
                    Self::escape_powershell_value(value),
                    separator,
                    name
                )
            }
        }
    }

    /// Generate unsetenv script
    fn generate_unsetenv_script(
        name: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("unset {}", name)
            }
            rez_core_context::ShellType::Fish => {
                format!("set -e {}", name)
            }
            rez_core_context::ShellType::Cmd => {
                format!("set {}=", name)
            }
            rez_core_context::ShellType::PowerShell => {
                format!("Remove-Item Env:{} -ErrorAction SilentlyContinue", name)
            }
        }
    }

    /// Generate alias script
    fn generate_alias_script(
        name: &str,
        command: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("alias {}=\"{}\"", name, Self::escape_bash_value(command))
            }
            rez_core_context::ShellType::Fish => {
                format!("alias {} \"{}\"", name, Self::escape_fish_value(command))
            }
            rez_core_context::ShellType::Cmd => {
                format!("doskey {}={}", name, command)
            }
            rez_core_context::ShellType::PowerShell => {
                format!("Set-Alias {} \"{}\"", name, Self::escape_powershell_value(command))
            }
        }
    }

    /// Generate function script
    fn generate_function_script(
        name: &str,
        body: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("function {} {{ {}; }}", name, body)
            }
            rez_core_context::ShellType::Fish => {
                format!("function {}\n    {}\nend", name, body)
            }
            rez_core_context::ShellType::Cmd => {
                // CMD doesn't have functions, use a label instead
                format!(":{}\n{}\ngoto :eof", name, body)
            }
            rez_core_context::ShellType::PowerShell => {
                format!("function {} {{ {} }}", name, body)
            }
        }
    }

    /// Generate source script
    fn generate_source_script(
        path: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("source \"{}\"", path)
            }
            rez_core_context::ShellType::Fish => {
                format!("source \"{}\"", path)
            }
            rez_core_context::ShellType::Cmd => {
                format!("call \"{}\"", path)
            }
            rez_core_context::ShellType::PowerShell => {
                format!(". \"{}\"", path)
            }
        }
    }

    /// Generate comment script
    fn generate_comment_script(
        text: &str,
        shell_type: &rez_core_context::ShellType,
    ) -> String {
        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                format!("# {}", text)
            }
            rez_core_context::ShellType::Fish => {
                format!("# {}", text)
            }
            rez_core_context::ShellType::Cmd => {
                format!("REM {}", text)
            }
            rez_core_context::ShellType::PowerShell => {
                format!("# {}", text)
            }
        }
    }

    /// Generate if script
    fn generate_if_script(
        condition: &str,
        then_commands: &[RexCommand],
        else_commands: &Option<Vec<RexCommand>>,
        shell_type: &rez_core_context::ShellType,
    ) -> Result<String, RezCoreError> {
        let mut script = String::new();

        match shell_type {
            rez_core_context::ShellType::Bash | rez_core_context::ShellType::Zsh => {
                script.push_str(&format!("if [[ {} ]]; then\n", condition));
                for cmd in then_commands {
                    script.push_str("    ");
                    script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                    script.push('\n');
                }
                if let Some(ref else_cmds) = else_commands {
                    script.push_str("else\n");
                    for cmd in else_cmds {
                        script.push_str("    ");
                        script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                        script.push('\n');
                    }
                }
                script.push_str("fi");
            }
            rez_core_context::ShellType::Fish => {
                script.push_str(&format!("if {}\n", condition));
                for cmd in then_commands {
                    script.push_str("    ");
                    script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                    script.push('\n');
                }
                if let Some(ref else_cmds) = else_commands {
                    script.push_str("else\n");
                    for cmd in else_cmds {
                        script.push_str("    ");
                        script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                        script.push('\n');
                    }
                }
                script.push_str("end");
            }
            rez_core_context::ShellType::Cmd => {
                script.push_str(&format!("if {} (\n", condition));
                for cmd in then_commands {
                    script.push_str("    ");
                    script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                    script.push('\n');
                }
                if let Some(ref else_cmds) = else_commands {
                    script.push_str(") else (\n");
                    for cmd in else_cmds {
                        script.push_str("    ");
                        script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                        script.push('\n');
                    }
                }
                script.push(')');
            }
            rez_core_context::ShellType::PowerShell => {
                script.push_str(&format!("if ({}) {{\n", condition));
                for cmd in then_commands {
                    script.push_str("    ");
                    script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                    script.push('\n');
                }
                if let Some(ref else_cmds) = else_commands {
                    script.push_str("} else {\n");
                    for cmd in else_cmds {
                        script.push_str("    ");
                        script.push_str(&Self::command_to_shell_script(cmd, shell_type)?);
                        script.push('\n');
                    }
                }
                script.push('}');
            }
        }

        Ok(script)
    }

    /// Escape value for bash
    fn escape_bash_value(value: &str) -> String {
        value.replace("\"", "\\\"").replace("$", "\\$").replace("`", "\\`")
    }

    /// Escape value for fish
    fn escape_fish_value(value: &str) -> String {
        value.replace("\"", "\\\"").replace("$", "\\$")
    }

    /// Escape value for PowerShell
    fn escape_powershell_value(value: &str) -> String {
        value.replace("\"", "`\"").replace("$", "`$")
    }

    /// Convert a Rex script to shell script
    pub fn script_to_shell_script(
        script: &RexScript,
        shell_type: &rez_core_context::ShellType,
    ) -> Result<String, RezCoreError> {
        let mut shell_script = String::new();

        // Add header
        match shell_type {
            rez_core_context::ShellType::Bash => {
                shell_script.push_str("#!/bin/bash\n# Generated by rez-core\n\n");
            }
            rez_core_context::ShellType::Zsh => {
                shell_script.push_str("#!/bin/zsh\n# Generated by rez-core\n\n");
            }
            rez_core_context::ShellType::Fish => {
                shell_script.push_str("#!/usr/bin/env fish\n# Generated by rez-core\n\n");
            }
            rez_core_context::ShellType::Cmd => {
                shell_script.push_str("@echo off\nREM Generated by rez-core\n\n");
            }
            rez_core_context::ShellType::PowerShell => {
                shell_script.push_str("# Generated by rez-core\n\n");
            }
        }

        // Convert each command
        for command in &script.commands {
            let command_script = Self::command_to_shell_script(command, shell_type)?;
            shell_script.push_str(&command_script);
            shell_script.push('\n');
        }

        Ok(shell_script)
    }
}
