//! # Rex (Rez Execution) Command Language
//!
//! Rex is rez's DSL for modifying the shell environment when entering a package context.
//! This crate implements the Rex command executor, compatible with rez's rex module.
//!
//! ## Supported Commands
//!
//! - `env.setenv("VAR", "value")` — set an environment variable
//! - `env.unsetenv("VAR")` — unset an environment variable
//! - `env.prepend_path("PATH", "/usr/bin")` — prepend to a path variable
//! - `env.append_path("PATH", "/usr/bin")` — append to a path variable
//! - `env.setenv_if_empty("VAR", "default")` — set only if not already set
//! - `setenv("VAR", "value")` — shorthand
//! - `appendenv("VAR", "value")` — append to any env var
//! - `prependenv("VAR", "value")` — prepend to any env var
//! - `unsetenv("VAR")` — shorthand
//! - `alias("name", "command")` — create a command alias
//! - `command("cmd arg1 arg2")` — run a command

use std::collections::HashMap;

pub mod executor;
pub mod actions;
pub mod parser;
pub mod shell;

pub use executor::RexExecutor;
pub use actions::{RexAction, RexActionType};
pub use parser::RexParser;
pub use shell::{ShellType, generate_shell_script};

/// Environment state after applying Rex commands
#[derive(Debug, Clone, Default)]
pub struct RexEnvironment {
    /// Environment variables (name -> value)
    pub vars: HashMap<String, String>,
    /// Aliases (name -> command)
    pub aliases: HashMap<String, String>,
    /// Commands to execute on shell startup
    pub startup_commands: Vec<String>,
    /// Scripts sourced during environment setup
    pub sourced_scripts: Vec<String>,
    /// Info messages emitted by info() calls
    pub info_messages: Vec<String>,
    /// Whether a stop() was encountered
    pub stopped: bool,
    /// Stop message if any
    pub stop_message: Option<String>,
}

impl RexEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply all Rex actions to this environment
    pub fn apply(&mut self, actions: &[RexAction]) {
        for action in actions {
            self.apply_action(action);
        }
    }

    fn apply_action(&mut self, action: &RexAction) {
        match &action.action_type {
            RexActionType::Setenv { name, value } => {
                self.vars.insert(name.clone(), value.clone());
            }
            RexActionType::Unsetenv { name } => {
                self.vars.remove(name);
            }
            RexActionType::PrependPath { name, value, separator } => {
                let sep = separator.as_deref().unwrap_or(get_path_sep());
                let current = self.vars.get(name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", value, sep, current)
                };
                self.vars.insert(name.clone(), new_value);
            }
            RexActionType::AppendPath { name, value, separator } => {
                let sep = separator.as_deref().unwrap_or(get_path_sep());
                let current = self.vars.get(name).cloned().unwrap_or_default();
                let new_value = if current.is_empty() {
                    value.clone()
                } else {
                    format!("{}{}{}", current, sep, value)
                };
                self.vars.insert(name.clone(), new_value);
            }
            RexActionType::SetenvIfEmpty { name, value } => {
                if !self.vars.contains_key(name) {
                    self.vars.insert(name.clone(), value.clone());
                }
            }
            RexActionType::Alias { name, value } => {
                self.aliases.insert(name.clone(), value.clone());
            }
            RexActionType::Command { cmd } => {
                self.startup_commands.push(cmd.clone());
            }
            RexActionType::Source { path } => {
                self.sourced_scripts.push(path.clone());
            }
            RexActionType::Comment { .. } => {} // Ignore comments
            RexActionType::Resetenv { name } => {
                // Reset to empty (original value restoration requires process env snapshot)
                self.vars.remove(name);
            }
            RexActionType::Info { message } => {
                self.info_messages.push(message.clone());
            }
            RexActionType::Error { message } => {
                // In non-strict mode, record as info message
                self.info_messages.push(format!("[error] {}", message));
            }
            RexActionType::Stop { message } => {
                self.stopped = true;
                self.stop_message = message.clone();
            }
        }
    }

    /// Merge with base environment (base values are not overwritten by existing env)
    pub fn merge_with_base(&mut self, base: &HashMap<String, String>) {
        for (key, value) in base {
            if !self.vars.contains_key(key) {
                self.vars.insert(key.clone(), value.clone());
            }
        }
    }
}

fn get_path_sep() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}
