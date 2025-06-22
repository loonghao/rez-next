//! Error types for rez-core

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use thiserror::Error;

/// Main error type for rez-core operations
#[derive(Error, Debug)]
pub enum RezCoreError {
    #[error("Version parsing error: {0}")]
    VersionParse(String),

    #[error("Version range error: {0}")]
    VersionRange(String),

    #[error("Package parsing error: {0}")]
    PackageParse(String),

    #[error("Requirement parsing error: {0}")]
    RequirementParse(String),

    #[error("Solver error: {0}")]
    Solver(String),

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Python error: {0}")]
    Python(String),

    #[cfg(feature = "python-bindings")]
    #[error("PyO3 error: {0}")]
    PyO3(#[from] pyo3::PyErr),

    #[error("Rex error: {0}")]
    RexError(String),

    #[error("Context error: {0}")]
    ContextError(String),

    #[error("Build error: {0}")]
    BuildError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("CLI error: {0}")]
    CliError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

/// Result type alias for rez-core operations
pub type RezCoreResult<T> = Result<T, RezCoreError>;

// Create Python exception types
#[cfg(feature = "python-bindings")]
pyo3::create_exception!(rez_core, PyRezCoreError, pyo3::exceptions::PyException);
