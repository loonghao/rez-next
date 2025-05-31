//! Error types for the intelligent caching system

use thiserror::Error;
use std::io;

/// Cache-specific error types
#[derive(Error, Debug)]
pub enum CacheError {
    /// I/O error during cache operations
    #[error("Cache I/O error: {0}")]
    Io(#[from] io::Error),

    /// Serialization/deserialization error
    #[error("Cache serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Cache capacity exceeded
    #[error("Cache capacity exceeded: {current} >= {max}")]
    CapacityExceeded { current: usize, max: usize },

    /// Invalid cache configuration
    #[error("Invalid cache configuration: {0}")]
    InvalidConfig(String),

    /// Cache entry not found
    #[error("Cache entry not found: {key}")]
    EntryNotFound { key: String },

    /// Cache entry expired
    #[error("Cache entry expired: {key}")]
    EntryExpired { key: String },

    /// Cache operation timeout
    #[error("Cache operation timeout after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },

    /// Cache corruption detected
    #[error("Cache corruption detected: {details}")]
    Corruption { details: String },

    /// Insufficient memory for cache operation
    #[error("Insufficient memory: requested {requested} bytes, available {available} bytes")]
    InsufficientMemory { requested: u64, available: u64 },

    /// Cache lock contention
    #[error("Cache lock contention: operation failed after {attempts} attempts")]
    LockContention { attempts: u32 },

    /// Predictive preheating error
    #[error("Preheating error: {0}")]
    PreheatingError(String),

    /// Adaptive tuning error
    #[error("Tuning error: {0}")]
    TuningError(String),

    /// Cache level error (L1, L2, etc.)
    #[error("Cache level {level} error: {message}")]
    CacheLevelError { level: String, message: String },

    /// Generic cache error
    #[error("Cache error: {0}")]
    Generic(String),
}

impl CacheError {
    /// Create a new capacity exceeded error
    pub fn capacity_exceeded(current: usize, max: usize) -> Self {
        Self::CapacityExceeded { current, max }
    }

    /// Create a new invalid config error
    pub fn invalid_config<S: Into<String>>(message: S) -> Self {
        Self::InvalidConfig(message.into())
    }

    /// Create a new entry not found error
    pub fn entry_not_found<S: Into<String>>(key: S) -> Self {
        Self::EntryNotFound { key: key.into() }
    }

    /// Create a new entry expired error
    pub fn entry_expired<S: Into<String>>(key: S) -> Self {
        Self::EntryExpired { key: key.into() }
    }

    /// Create a new timeout error
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }

    /// Create a new corruption error
    pub fn corruption<S: Into<String>>(details: S) -> Self {
        Self::Corruption {
            details: details.into(),
        }
    }

    /// Create a new insufficient memory error
    pub fn insufficient_memory(requested: u64, available: u64) -> Self {
        Self::InsufficientMemory {
            requested,
            available,
        }
    }

    /// Create a new lock contention error
    pub fn lock_contention(attempts: u32) -> Self {
        Self::LockContention { attempts }
    }

    /// Create a new preheating error
    pub fn preheating_error<S: Into<String>>(message: S) -> Self {
        Self::PreheatingError(message.into())
    }

    /// Create a new tuning error
    pub fn tuning_error<S: Into<String>>(message: S) -> Self {
        Self::TuningError(message.into())
    }

    /// Create a new cache level error
    pub fn cache_level_error<S: Into<String>>(level: S, message: S) -> Self {
        Self::CacheLevelError {
            level: level.into(),
            message: message.into(),
        }
    }

    /// Create a generic cache error
    pub fn generic<S: Into<String>>(message: S) -> Self {
        Self::Generic(message.into())
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            CacheError::Io(_) => false,
            CacheError::Serialization(_) => false,
            CacheError::CapacityExceeded { .. } => true,
            CacheError::InvalidConfig(_) => false,
            CacheError::EntryNotFound { .. } => true,
            CacheError::EntryExpired { .. } => true,
            CacheError::Timeout { .. } => true,
            CacheError::Corruption { .. } => false,
            CacheError::InsufficientMemory { .. } => true,
            CacheError::LockContention { .. } => true,
            CacheError::PreheatingError(_) => true,
            CacheError::TuningError(_) => true,
            CacheError::CacheLevelError { .. } => true,
            CacheError::Generic(_) => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CacheError::Io(_) => ErrorSeverity::Critical,
            CacheError::Serialization(_) => ErrorSeverity::High,
            CacheError::CapacityExceeded { .. } => ErrorSeverity::Medium,
            CacheError::InvalidConfig(_) => ErrorSeverity::High,
            CacheError::EntryNotFound { .. } => ErrorSeverity::Low,
            CacheError::EntryExpired { .. } => ErrorSeverity::Low,
            CacheError::Timeout { .. } => ErrorSeverity::Medium,
            CacheError::Corruption { .. } => ErrorSeverity::Critical,
            CacheError::InsufficientMemory { .. } => ErrorSeverity::High,
            CacheError::LockContention { .. } => ErrorSeverity::Medium,
            CacheError::PreheatingError(_) => ErrorSeverity::Low,
            CacheError::TuningError(_) => ErrorSeverity::Low,
            CacheError::CacheLevelError { .. } => ErrorSeverity::Medium,
            CacheError::Generic(_) => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Low severity - operation can continue
    Low,
    /// Medium severity - may affect performance
    Medium,
    /// High severity - significant impact
    High,
    /// Critical severity - system may be unstable
    Critical,
}

impl ErrorSeverity {
    /// Get severity as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorSeverity::Low => "LOW",
            ErrorSeverity::Medium => "MEDIUM",
            ErrorSeverity::High => "HIGH",
            ErrorSeverity::Critical => "CRITICAL",
        }
    }

    /// Check if the severity requires immediate attention
    pub fn requires_immediate_attention(&self) -> bool {
        matches!(self, ErrorSeverity::High | ErrorSeverity::Critical)
    }
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Error context for better error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation that failed
    pub operation: String,
    /// Cache level where error occurred
    pub cache_level: Option<String>,
    /// Additional context information
    pub context: std::collections::HashMap<String, String>,
    /// Timestamp when error occurred
    pub timestamp: u64,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            cache_level: None,
            context: std::collections::HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    /// Set the cache level
    pub fn with_cache_level<S: Into<String>>(mut self, level: S) -> Self {
        self.cache_level = Some(level.into());
        self
    }

    /// Add context information
    pub fn with_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Enhanced cache error with context
#[derive(Debug)]
pub struct ContextualCacheError {
    /// The underlying cache error
    pub error: CacheError,
    /// Error context
    pub context: ErrorContext,
}

impl ContextualCacheError {
    /// Create a new contextual error
    pub fn new(error: CacheError, context: ErrorContext) -> Self {
        Self { error, context }
    }

    /// Get the error severity
    pub fn severity(&self) -> ErrorSeverity {
        self.error.severity()
    }

    /// Check if the error is recoverable
    pub fn is_recoverable(&self) -> bool {
        self.error.is_recoverable()
    }
}

impl std::fmt::Display for ContextualCacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache error in operation '{}': {} (severity: {})",
            self.context.operation,
            self.error,
            self.severity().as_str()
        )?;

        if let Some(ref level) = self.context.cache_level {
            write!(f, " [cache level: {}]", level)?;
        }

        if !self.context.context.is_empty() {
            write!(f, " [context: {:?}]", self.context.context)?;
        }

        Ok(())
    }
}

impl std::error::Error for ContextualCacheError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_error_creation() {
        let error = CacheError::capacity_exceeded(100, 50);
        assert!(matches!(error, CacheError::CapacityExceeded { current: 100, max: 50 }));
        assert!(error.is_recoverable());
        assert_eq!(error.severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("get_operation")
            .with_cache_level("L1")
            .with_context("key", "test_key");

        assert_eq!(context.operation, "get_operation");
        assert_eq!(context.cache_level, Some("L1".to_string()));
        assert_eq!(context.context.get("key"), Some(&"test_key".to_string()));
    }

    #[test]
    fn test_contextual_error() {
        let error = CacheError::timeout(1000);
        let context = ErrorContext::new("put_operation");
        let contextual_error = ContextualCacheError::new(error, context);

        assert_eq!(contextual_error.severity(), ErrorSeverity::Medium);
        assert!(contextual_error.is_recoverable());
    }

    #[test]
    fn test_error_severity() {
        assert!(ErrorSeverity::Critical.requires_immediate_attention());
        assert!(ErrorSeverity::High.requires_immediate_attention());
        assert!(!ErrorSeverity::Medium.requires_immediate_attention());
        assert!(!ErrorSeverity::Low.requires_immediate_attention());
    }
}
