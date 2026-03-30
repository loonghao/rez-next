//! Suite error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SuiteError {
    #[error("Suite context '{0}' not found")]
    ContextNotFound(String),

    #[error("Tool conflict: tool '{tool}' exists in contexts: {contexts}")]
    ToolConflict { tool: String, contexts: String },

    #[error("Suite already exists at path: {0}")]
    SuiteAlreadyExists(String),

    #[error("Suite not found at path: {0}")]
    SuiteNotFound(String),

    #[error("Invalid suite: {0}")]
    InvalidSuite(String),

    #[error("Context name '{0}' is invalid (must be alphanumeric with dashes/underscores)")]
    InvalidContextName(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Resolution error: {0}")]
    Resolution(String),
}

impl From<SuiteError> for rez_next_common::RezCoreError {
    fn from(e: SuiteError) -> Self {
        rez_next_common::RezCoreError::ExecutionError(e.to_string())
    }
}
