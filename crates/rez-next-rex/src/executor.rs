//! Rex executor: runs package commands and builds environment

use crate::actions::{RexAction, RexActionType};
use crate::RexEnvironment;
use rez_next_common::RezCoreError;
use std::collections::HashMap;

/// Rex executor: processes package commands and generates environment actions
#[derive(Debug)]
pub struct RexExecutor {
    /// Context variables available to commands (e.g., {root}, {version})
    context_vars: HashMap<String, String>,
    /// Generated actions
    actions: Vec<RexAction>,
}

impl RexExecutor {
    pub fn new() -> Self {
        Self {
            context_vars: HashMap::new(),
            actions: Vec::new(),
        }
    }

    /// Set a context variable (e.g., root path, version)
    pub fn set_context_var(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.context_vars.insert(name.into(), value.into());
    }

    /// Execute a package's commands string and return the resulting environment
    pub fn execute_commands(
        &mut self,
        commands: &str,
        package_name: &str,
        root: Option<&str>,
        version: Option<&str>,
    ) -> Result<RexEnvironment, RezCoreError> {
        // Set default context vars
        if let Some(root) = root {
            self.context_vars
                .insert("root".to_string(), root.to_string());
        }
        if let Some(version) = version {
            self.context_vars
                .insert("version".to_string(), version.to_string());
        }
        self.context_vars
            .insert("name".to_string(), package_name.to_string());

        // Parse and execute commands
        let parser = crate::parser::RexParser::new();
        let raw_actions = parser.parse(commands)?;

        // Expand variables in actions
        for mut action in raw_actions {
            action = self.expand_action_vars(action);
            action.source_package = Some(package_name.to_string());
            self.actions.push(action);
        }

        let mut env = RexEnvironment::new();
        env.apply(&self.actions);
        Ok(env)
    }

    /// Get all generated actions
    pub fn get_actions(&self) -> &[RexAction] {
        &self.actions
    }

    /// Clear all actions
    pub fn clear(&mut self) {
        self.actions.clear();
    }

    /// Expand {variable} references in action values
    fn expand_action_vars(&self, action: RexAction) -> RexAction {
        let expand = |s: &str| -> String {
            let mut result = s.to_string();
            for (key, value) in &self.context_vars {
                result = result.replace(&format!("{{{}}}", key), value);
                // Also support $NAME style
                result = result.replace(&format!("${}", key.to_uppercase()), value);
            }
            result
        };

        let new_action_type = match action.action_type {
            RexActionType::Setenv { name, value } => RexActionType::Setenv {
                name,
                value: expand(&value),
            },
            RexActionType::PrependPath {
                name,
                value,
                separator,
            } => RexActionType::PrependPath {
                name,
                value: expand(&value),
                separator,
            },
            RexActionType::AppendPath {
                name,
                value,
                separator,
            } => RexActionType::AppendPath {
                name,
                value: expand(&value),
                separator,
            },
            RexActionType::SetenvIfEmpty { name, value } => RexActionType::SetenvIfEmpty {
                name,
                value: expand(&value),
            },
            RexActionType::Alias { name, value } => RexActionType::Alias {
                name,
                value: expand(&value),
            },
            RexActionType::Command { cmd } => RexActionType::Command { cmd: expand(&cmd) },
            RexActionType::Source { path } => RexActionType::Source {
                path: expand(&path),
            },
            RexActionType::Info { message } => RexActionType::Info {
                message: expand(&message),
            },
            RexActionType::Error { message } => RexActionType::Error {
                message: expand(&message),
            },
            RexActionType::Stop { message } => RexActionType::Stop {
                message: message.map(|m| expand(&m)),
            },
            other => other,
        };

        RexAction {
            action_type: new_action_type,
            source_package: action.source_package,
        }
    }
}

impl Default for RexExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod tests;
