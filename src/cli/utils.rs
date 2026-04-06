//! # CLI Utilities
//!
//! Common utilities and helper functions for CLI commands.

use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::io::{self, Write};
use std::path::PathBuf;

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

/// Split a paths string into individual [`PathBuf`] entries using the
/// OS-appropriate path-list separator.
///
/// On **Windows** the separator is `;` (same as the `PATH` env-var convention).
/// On **Unix/macOS** the separator is `:`.
///
/// This avoids the common bug of splitting on `:` on Windows, which would break
/// paths like `C:\packages\foo` (the drive letter `C` would become a separate
/// entry).
///
/// Surrounding whitespace around each entry is trimmed.
///
/// # Examples
/// ```
/// // On Unix: "/pkg/a:/pkg/b" → [PathBuf::from("/pkg/a"), PathBuf::from("/pkg/b")]
/// // On Windows: "C:\\pkg\\a;D:\\pkg\\b" → [PathBuf::from("C:\\pkg\\a"), PathBuf::from("D:\\pkg\\b")]
/// ```
pub fn split_package_paths(paths_str: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    const PATH_LIST_SEP: char = ';';
    #[cfg(not(windows))]
    const PATH_LIST_SEP: char = ':';

    paths_str
        .split(PATH_LIST_SEP)
        .map(|p| PathBuf::from(p.trim()))
        .filter(|p| !p.as_os_str().is_empty())
        .collect()
}

/// Expand `~` at the start of a path string to the user's home directory.
///
/// Returns a [`PathBuf`].  If the path does not start with `~` it is returned
/// unchanged.  Works on both Unix (reads `HOME`) and Windows (reads
/// `USERPROFILE`).
///
/// # Examples
/// ```
/// use std::path::PathBuf;
/// // When HOME/USERPROFILE is set, "~/foo" is expanded.
/// ```
pub fn expand_home_path(p: &str) -> PathBuf {
    if p == "~" {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            return PathBuf::from(home);
        }
    } else if let Some(rest) = p.strip_prefix("~/").or_else(|| p.strip_prefix("~\\")) {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            return PathBuf::from(home).join(rest);
        }
    }
    PathBuf::from(p)
}

/// Expand `~` at the start of a path string and return a [`String`].
///
/// Thin wrapper around [`expand_home_path`] for callers that need a `String`.
pub fn expand_home_str(p: &str) -> String {
    expand_home_path(p).to_string_lossy().into_owned()
}

/// Parse an ISO 8601 date/datetime string or relative-time expression into a
/// Unix timestamp (seconds since epoch, **signed**).
///
/// Supported formats:
/// - ISO 8601 datetime: `2024-01-01T12:00:00`
/// - ISO 8601 date:     `2024-01-01`
/// - Relative:          `1d` (days), `2w` (weeks), `1m` (months ≈ 30 d),
///   `1y` (years ≈ 365 d) — anchored to *now* in UTC
///
/// Returns `None` if `s` does not match any supported format.
pub fn parse_timestamp(s: &str) -> Option<i64> {
    // ISO datetime: YYYY-MM-DDTHH:MM:SS
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp());
    }
    // ISO date: YYYY-MM-DD
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }
    // Relative time: <number><unit>
    parse_relative_time(s)
}

/// Parse a relative time string (`1d`, `2w`, `3m`, `1y`) into a past Unix
/// timestamp (seconds).  Returns `None` for unknown formats.
pub fn parse_relative_time(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (digits, unit) = s.split_at(s.len() - 1);
    let n: i64 = digits.parse().ok()?;
    let seconds_ago: i64 = match unit {
        "d" | "D" => n * 86_400,
        "w" | "W" => n * 7 * 86_400,
        "m" | "M" => n * 30 * 86_400,
        "y" | "Y" => n * 365 * 86_400,
        _ => return None,
    };
    let now = chrono::Utc::now().timestamp();
    Some(now - seconds_ago)
}

/// Parse a time specification into a Unix timestamp (`u64`, seconds since
/// epoch).
///
/// Like [`parse_timestamp`] but returns a [`RezCoreResult<u64>`], making it
/// suitable for CLI argument validation that must not silently succeed.
pub fn parse_time_spec(spec: &str) -> RezCoreResult<u64> {
    parse_timestamp(spec)
        .and_then(|ts| if ts >= 0 { Some(ts as u64) } else { None })
        .ok_or_else(|| {
            RezCoreError::CliError(format!(
                "Cannot parse time spec '{spec}'. Expected: 1d/2w/1m/1y or YYYY-MM-DD[THH:MM:SS]"
            ))
        })
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
            GetConsoleScreenBufferInfo, GetStdHandle, CONSOLE_SCREEN_BUFFER_INFO, STD_OUTPUT_HANDLE,
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
    fn test_split_package_paths_single() {
        let result = split_package_paths("/usr/local/packages");
        assert_eq!(result, vec![std::path::PathBuf::from("/usr/local/packages")]);
    }

    #[test]
    fn test_split_package_paths_empty() {
        let result = split_package_paths("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_split_package_paths_with_whitespace() {
        #[cfg(not(windows))]
        {
            let result = split_package_paths(" /pkg/a : /pkg/b ");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0], std::path::PathBuf::from("/pkg/a"));
            assert_eq!(result[1], std::path::PathBuf::from("/pkg/b"));
        }
        #[cfg(windows)]
        {
            let result = split_package_paths(r" C:\pkg\a ; D:\pkg\b ");
            assert_eq!(result.len(), 2);
            assert_eq!(result[0], std::path::PathBuf::from(r"C:\pkg\a"));
            assert_eq!(result[1], std::path::PathBuf::from(r"D:\pkg\b"));
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_split_package_paths_windows_drive_letters() {
        // Ensure Windows drive letters are NOT split by ':'
        let result = split_package_paths(r"C:\packages\rez;D:\local\packages");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], std::path::PathBuf::from(r"C:\packages\rez"));
        assert_eq!(result[1], std::path::PathBuf::from(r"D:\local\packages"));
    }

    #[test]
    #[cfg(not(windows))]
    fn test_split_package_paths_unix_colon_separator() {
        let result = split_package_paths("/opt/rez/packages:/home/user/packages");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], std::path::PathBuf::from("/opt/rez/packages"));
        assert_eq!(result[1], std::path::PathBuf::from("/home/user/packages"));
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
        unsafe {
            std::env::set_var("COLUMNS", "132");
        }
        let width = get_terminal_width();
        unsafe {
            std::env::remove_var("COLUMNS");
        }
        assert_eq!(width, 132);
    }

    #[test]
    fn test_terminal_width_columns_zero_is_ignored() {
        // COLUMNS=0 should not be used — fall through to OS query or default.
        unsafe {
            std::env::set_var("COLUMNS", "0");
        }
        let width = get_terminal_width();
        unsafe {
            std::env::remove_var("COLUMNS");
        }
        // Must not return 0 (always returns at least the DEFAULT_WIDTH)
        assert!(width > 0);
    }

    #[test]
    fn test_terminal_width_columns_invalid_is_ignored() {
        unsafe {
            std::env::set_var("COLUMNS", "not_a_number");
        }
        let width = get_terminal_width();
        unsafe {
            std::env::remove_var("COLUMNS");
        }
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

    // ── expand_home_path tests ───────────────────────────────────────────────

    #[test]
    fn test_expand_home_path_absolute_unchanged() {
        let p = expand_home_path("/usr/local/packages");
        assert_eq!(p, std::path::PathBuf::from("/usr/local/packages"));
    }

    #[test]
    fn test_expand_home_path_relative_unchanged() {
        let p = expand_home_path("relative/path");
        assert_eq!(p, std::path::PathBuf::from("relative/path"));
    }

    #[test]
    fn test_expand_home_path_tilde_only() {
        // Just "~" should expand to home dir if set, or stay as "~" if not
        let p = expand_home_path("~");
        // Must not be empty
        assert!(!p.as_os_str().is_empty());
    }

    #[test]
    fn test_expand_home_path_tilde_slash() {
        // "~/foo" should result in a path containing "foo" as the last component
        let p = expand_home_path("~/mypackages");
        let last = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        assert_eq!(last, "mypackages");
    }

    #[test]
    fn test_expand_home_str_absolute_unchanged() {
        let s = expand_home_str("/absolute/path");
        assert_eq!(s, "/absolute/path");
    }

    #[test]
    fn test_expand_home_str_relative_unchanged() {
        let s = expand_home_str("relative/path");
        assert_eq!(s, "relative/path");
    }

    // ── parse_timestamp tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_timestamp_iso_datetime() {
        let ts = parse_timestamp("2024-01-15T10:30:00");
        assert!(ts.is_some());
        assert_eq!(ts.unwrap(), 1705314600);
    }

    #[test]
    fn test_parse_timestamp_iso_date() {
        let ts = parse_timestamp("2024-01-15");
        assert!(ts.is_some());
        assert_eq!(ts.unwrap(), 1705276800);
    }

    #[test]
    fn test_parse_timestamp_relative_1d() {
        let ts = parse_timestamp("1d");
        assert!(ts.is_some());
        let now = chrono::Utc::now().timestamp();
        let diff = now - ts.unwrap();
        // Should be within ±5 seconds of 86_400
        assert!((86_395..=86_405).contains(&diff));
    }

    #[test]
    fn test_parse_timestamp_relative_2w() {
        let ts = parse_timestamp("2w");
        assert!(ts.is_some());
        let now = chrono::Utc::now().timestamp();
        let diff = now - ts.unwrap();
        assert!((2 * 7 * 86_400 - 5..=2 * 7 * 86_400 + 5).contains(&diff));
    }

    #[test]
    fn test_parse_timestamp_invalid() {
        assert!(parse_timestamp("not-a-date").is_none());
        assert!(parse_timestamp("").is_none());
        assert!(parse_timestamp("abc").is_none());
    }

    #[test]
    fn test_parse_relative_time_units() {
        let now = chrono::Utc::now().timestamp();
        assert!(parse_relative_time("1d").is_some());
        assert!(parse_relative_time("1D").is_some());
        assert!(parse_relative_time("1w").is_some());
        assert!(parse_relative_time("1m").is_some());
        assert!(parse_relative_time("1y").is_some());
        assert!(parse_relative_time("0d").is_some());
        // All results should be <= now
        for spec in &["1d", "1w", "1m", "1y"] {
            let ts = parse_relative_time(spec).unwrap();
            assert!(ts <= now, "{} yielded future timestamp", spec);
        }
    }

    #[test]
    fn test_parse_relative_time_unknown_unit() {
        assert!(parse_relative_time("1x").is_none());
        assert!(parse_relative_time("5h").is_none());
        assert!(parse_relative_time("").is_none());
    }

    #[test]
    fn test_parse_time_spec_ok() {
        let ts = parse_time_spec("1d").unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Should be close to now - 86400
        assert!(ts > 0 && ts <= now);
    }

    #[test]
    fn test_parse_time_spec_error() {
        assert!(parse_time_spec("not-a-time").is_err());
        assert!(parse_time_spec("").is_err());
    }
}
