//! Rex action types

use serde::{Deserialize, Serialize};

/// A single Rex action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexAction {
    /// The type of action
    pub action_type: RexActionType,
    /// Source package that generated this action (if any)
    pub source_package: Option<String>,
}

/// Types of Rex actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RexActionType {
    /// Set an environment variable
    Setenv { name: String, value: String },
    /// Unset an environment variable
    Unsetenv { name: String },
    /// Prepend to a path-like environment variable
    PrependPath {
        name: String,
        value: String,
        separator: Option<String>,
    },
    /// Append to a path-like environment variable
    AppendPath {
        name: String,
        value: String,
        separator: Option<String>,
    },
    /// Set env var only if not already set
    SetenvIfEmpty { name: String, value: String },
    /// Create an alias
    Alias { name: String, value: String },
    /// Execute a command
    Command { cmd: String },
    /// Source a shell script file
    Source { path: String },
    /// Comment (ignored)
    Comment { text: String },
    /// Reset an environment variable to its original (pre-context) value
    Resetenv { name: String },
    /// Informational message (rez info() - no env effect)
    Info { message: String },
    /// Error message (rez error() - raises on non-strict mode)
    Error { message: String },
    /// Stop execution of commands (rez stop())
    Stop { message: Option<String> },
}

impl RexAction {
    pub fn setenv(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::Setenv {
                name: name.into(),
                value: value.into(),
            },
            source_package: None,
        }
    }

    pub fn unsetenv(name: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::Unsetenv { name: name.into() },
            source_package: None,
        }
    }

    pub fn prepend_path(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::PrependPath {
                name: name.into(),
                value: value.into(),
                separator: None,
            },
            source_package: None,
        }
    }

    pub fn append_path(name: impl Into<String>, value: impl Into<String>) -> Self {
        RexAction {
            action_type: RexActionType::AppendPath {
                name: name.into(),
                value: value.into(),
                separator: None,
            },
            source_package: None,
        }
    }

    pub fn with_source(mut self, package: impl Into<String>) -> Self {
        self.source_package = Some(package.into());
        self
    }
}
