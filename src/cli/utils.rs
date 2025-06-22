//! # CLI Utilities
//!
//! Common utilities and helper functions for CLI commands.

use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::io::{self, Write};

/// Print formatted output with proper error handling
pub fn print_output(content: &str) -> RezCoreResult<()> {
    print!("{}", content);
    io::stdout().flush().map_err(RezCoreError::Io)?;
    Ok(())
}

/// Print formatted error message to stderr
pub fn print_error(message: &str) -> RezCoreResult<()> {
    eprintln!("Error: {}", message);
    io::stderr().flush().map_err(RezCoreError::Io)?;
    Ok(())
}

/// Format a list of items in columns
pub fn format_columns(items: &[String], max_width: usize) -> String {
    if items.is_empty() {
        return String::new();
    }

    // Simple column formatting - can be enhanced later
    let max_item_width = items.iter().map(|s| s.len()).max().unwrap_or(0);
    let columns = if max_item_width > 0 {
        (max_width / (max_item_width + 2)).max(1)
    } else {
        1
    };

    let mut result = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 && i % columns == 0 {
            result.push('\n');
        }
        result.push_str(&format!("{:<width$}", item, width = max_item_width + 2));
    }

    result
}

/// Validate package name format
pub fn validate_package_name(name: &str) -> RezCoreResult<()> {
    if name.is_empty() {
        return Err(RezCoreError::PackageParse(
            "Package name cannot be empty".to_string(),
        ));
    }

    // Basic validation - can be enhanced with proper package name rules
    if name.contains(' ') {
        return Err(RezCoreError::PackageParse(
            "Package name cannot contain spaces".to_string(),
        ));
    }

    Ok(())
}

/// Parse environment variable style arguments (KEY=VALUE)
pub fn parse_env_var(arg: &str) -> RezCoreResult<(String, String)> {
    if let Some(pos) = arg.find('=') {
        let key = arg[..pos].to_string();
        let value = arg[pos + 1..].to_string();

        if key.is_empty() {
            return Err(RezCoreError::RequirementParse(
                "Environment variable key cannot be empty".to_string(),
            ));
        }

        Ok((key, value))
    } else {
        Err(RezCoreError::RequirementParse(format!(
            "Invalid environment variable format: '{}'. Expected KEY=VALUE",
            arg
        )))
    }
}

/// Get terminal width for formatting
pub fn get_terminal_width() -> usize {
    // Default width if we can't determine terminal size
    const DEFAULT_WIDTH: usize = 80;

    // Try to get terminal width from environment or system
    if let Ok(width_str) = std::env::var("COLUMNS") {
        if let Ok(width) = width_str.parse::<usize>() {
            return width;
        }
    }

    // TODO: Use a proper terminal size detection library if needed
    DEFAULT_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_package_name() {
        assert!(validate_package_name("valid_package").is_ok());
        assert!(validate_package_name("package-name").is_ok());
        assert!(validate_package_name("package123").is_ok());

        assert!(validate_package_name("").is_err());
        assert!(validate_package_name("invalid package").is_err());
    }

    #[test]
    fn test_parse_env_var() {
        assert_eq!(
            parse_env_var("KEY=value").unwrap(),
            ("KEY".to_string(), "value".to_string())
        );
        assert_eq!(
            parse_env_var("PATH=/usr/bin").unwrap(),
            ("PATH".to_string(), "/usr/bin".to_string())
        );

        assert!(parse_env_var("invalid").is_err());
        assert!(parse_env_var("=value").is_err());
    }

    #[test]
    fn test_format_columns() {
        let items = vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ];
        let result = format_columns(&items, 80);
        assert!(!result.is_empty());
    }
}
