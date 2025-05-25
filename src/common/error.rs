//! Error types for rez-core

use thiserror::Error;

/// Main error type for rez-core operations
#[derive(Error, Debug)]
pub enum RezCoreError {
    #[error("Version parsing error: {0}")]
    VersionParse(String),
    
    #[error("Version range error: {0}")]
    VersionRange(String),
    
    #[error("Solver error: {0}")]
    Solver(String),
    
    #[error("Repository error: {0}")]
    Repository(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    
    #[error("Python error: {0}")]
    Python(String),
}

/// Result type alias for rez-core operations
pub type RezCoreResult<T> = Result<T, RezCoreError>;
