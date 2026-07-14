//! Logging utilities aligned with rez.utils.logging.
//!
//! ## Lessons from Rez Issues:
//! - Avoid excessive debug output that can slow down resolution.
//! - Use structured logging for machine-parseable output.

use std::fmt;

/// Log levels matching rez's logging levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Debug information (most verbose).
    Debug = 0,
    /// Informational messages.
    Info = 1,
    /// Warning messages.
    Warning = 2,
    /// Error messages.
    Error = 3,
    /// Critical errors.
    Critical = 4,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Simple logger wrapper that supports rez-style log formatting.
#[derive(Debug, Clone)]
pub struct Logger {
    /// Current log level.
    level: LogLevel,
    /// Whether to include timestamps.
    timestamps: bool,
    /// Module name for context.
    module: Option<String>,
}

impl Logger {
    /// Create a new logger with the given level.
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            timestamps: false,
            module: None,
        }
    }

    /// Set the module name for context in log messages.
    pub fn with_module(mut self, module: &str) -> Self {
        self.module = Some(module.to_string());
        self
    }

    /// Enable timestamps in log output.
    pub fn with_timestamps(mut self) -> Self {
        self.timestamps = true;
        self
    }

    /// Log a message at the given level.
    pub fn log(&self, level: LogLevel, message: &str) {
        if level >= self.level {
            let module = self.module.as_deref().unwrap_or("rez_next");
            eprintln!("[{}] {}: {}", level, module, message);
        }
    }

    /// Log a debug message.
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }

    /// Log an info message.
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }

    /// Log a warning message.
    pub fn warning(&self, message: &str) {
        self.log(LogLevel::Warning, message);
    }

    /// Log an error message.
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }

    /// Log a critical message.
    pub fn critical(&self, message: &str) {
        self.log(LogLevel::Critical, message);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Warning < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Critical);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "DEBUG");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn test_logger_filters_by_level() {
        let logger = Logger::new(LogLevel::Warning);
        // These should not panic - they're filtered out
        logger.debug("should not appear");
        logger.info("should not appear");
    }

    #[test]
    fn test_logger_with_module() {
        let logger = Logger::new(LogLevel::Debug).with_module("test_module");
        logger.debug("test message");
    }
}
