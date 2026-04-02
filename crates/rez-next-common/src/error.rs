//! Error types for rez-core

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse_error_display() {
        let e = RezCoreError::VersionParse("bad version".to_string());
        assert!(e.to_string().contains("bad version"));
        assert!(e.to_string().contains("Version parsing error"));
    }

    #[test]
    fn test_version_range_error_display() {
        let e = RezCoreError::VersionRange("invalid range".to_string());
        assert!(e.to_string().contains("invalid range"));
    }

    #[test]
    fn test_package_parse_error_display() {
        let e = RezCoreError::PackageParse("missing name".to_string());
        assert!(e.to_string().contains("missing name"));
    }

    #[test]
    fn test_requirement_parse_error_display() {
        let e = RezCoreError::RequirementParse("bad req".to_string());
        assert!(e.to_string().contains("bad req"));
        assert!(e.to_string().contains("Requirement parsing error"));
    }

    #[test]
    fn test_solver_error_display() {
        let e = RezCoreError::Solver("conflict detected".to_string());
        assert!(e.to_string().contains("conflict detected"));
        assert!(e.to_string().contains("Solver error"));
    }

    #[test]
    fn test_repository_error_display() {
        let e = RezCoreError::Repository("not found".to_string());
        assert!(e.to_string().contains("not found"));
    }

    #[test]
    fn test_cache_error_display() {
        let e = RezCoreError::Cache("cache miss".to_string());
        assert!(e.to_string().contains("cache miss"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let rez_err: RezCoreError = io_err.into();
        assert!(rez_err.to_string().contains("IO error"));
        assert!(rez_err.to_string().contains("file not found"));
    }

    #[test]
    fn test_serde_error_from() {
        let bad_json = "{ invalid }";
        let serde_err: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
        let rez_err: RezCoreError = serde_err.unwrap_err().into();
        assert!(rez_err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_rex_error_display() {
        let e = RezCoreError::RexError("script failed".to_string());
        assert!(e.to_string().contains("Rex error"));
        assert!(e.to_string().contains("script failed"));
    }

    #[test]
    fn test_context_error_display() {
        let e = RezCoreError::ContextError("context not resolved".to_string());
        assert!(e.to_string().contains("Context error"));
    }

    #[test]
    fn test_build_error_display() {
        let e = RezCoreError::BuildError("build failed".to_string());
        assert!(e.to_string().contains("Build error"));
        assert!(e.to_string().contains("build failed"));
    }

    #[test]
    fn test_execution_error_display() {
        let e = RezCoreError::ExecutionError("process exited 1".to_string());
        assert!(e.to_string().contains("Execution error"));
    }

    #[test]
    fn test_cli_error_display() {
        let e = RezCoreError::CliError("unknown subcommand".to_string());
        assert!(e.to_string().contains("CLI error"));
    }

    #[test]
    fn test_config_error_display() {
        let e = RezCoreError::ConfigError("missing packages_path".to_string());
        assert!(e.to_string().contains("Configuration error"));
        assert!(e.to_string().contains("missing packages_path"));
    }

    #[test]
    fn test_error_is_debug() {
        let e = RezCoreError::Solver("test".to_string());
        let debug = format!("{e:?}");
        assert!(!debug.is_empty());
    }

    #[test]
    fn test_rez_core_result_ok() {
        let result: RezCoreResult<i32> = Ok(42);
        assert!(result.is_ok());
        let val = match result {
            Ok(v) => v,
            Err(_) => unreachable!("result should be Ok"),
        };
        assert_eq!(val, 42);
    }

    #[test]
    fn test_rez_core_result_err() {
        let result: RezCoreResult<i32> = Err(RezCoreError::Solver("test".to_string()));
        assert!(result.is_err());
    }
}
