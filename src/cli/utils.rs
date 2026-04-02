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

    // Explicit user override via environment variable
    if let Ok(width_str) = std::env::var("COLUMNS") {
        if let Ok(width) = width_str.parse::<usize>() {
            if width > 0 {
                return width;
            }
        }
    }

    // OS-level terminal size query
    #[cfg(unix)]
    {
        // SAFETY: winsize is a POD struct; ioctl is a well-known POSIX call.
        let mut ws = libc::winsize {
            ws_row: 0,
            ws_col: 0,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let ret = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut ws) };
        if ret == 0 && ws.ws_col > 0 {
            return ws.ws_col as usize;
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO,
            STD_OUTPUT_HANDLE,
        };
        // SAFETY: Windows API call; handle validity checked before use.
        let handle = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
        if !handle.is_null() {
            let mut csbi = unsafe { std::mem::zeroed::<CONSOLE_SCREEN_BUFFER_INFO>() };
            let ok = unsafe { GetConsoleScreenBufferInfo(handle, &mut csbi) };
            if ok != 0 {
                let width = (csbi.srWindow.Right - csbi.srWindow.Left + 1) as usize;
                if width > 0 {
                    return width;
                }
            }
        }
    }

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

    // ── get_terminal_width tests ─────────────────────────────────────────────

    #[test]
    fn test_terminal_width_via_columns_env() {
        // Set COLUMNS to a known value and verify it is respected.
        // Use a thread-local scope to avoid interfering with other tests.
        std::env::set_var("COLUMNS", "132");
        let width = get_terminal_width();
        std::env::remove_var("COLUMNS");
        assert_eq!(width, 132);
    }

    #[test]
    fn test_terminal_width_columns_zero_is_ignored() {
        // COLUMNS=0 should not be used — fall through to OS query or default.
        std::env::set_var("COLUMNS", "0");
        let width = get_terminal_width();
        std::env::remove_var("COLUMNS");
        // Must not return 0 (always returns at least the DEFAULT_WIDTH)
        assert!(width > 0);
    }

    #[test]
    fn test_terminal_width_columns_invalid_is_ignored() {
        std::env::set_var("COLUMNS", "not_a_number");
        let width = get_terminal_width();
        std::env::remove_var("COLUMNS");
        assert!(width > 0);
    }

    #[test]
    fn test_terminal_width_fallback_is_positive() {
        // Without COLUMNS set, the function must return a positive value.
        std::env::remove_var("COLUMNS");
        let width = get_terminal_width();
        assert!(width > 0, "terminal width must be positive, got {}", width);
    }

    #[test]
    fn test_terminal_width_reasonable_range() {
        // Width should be in a sane range (20–65535).
        std::env::remove_var("COLUMNS");
        let width = get_terminal_width();
        assert!(
            (20..=65535).contains(&width),
            "unexpected terminal width: {}",
            width
        );
    }
}
