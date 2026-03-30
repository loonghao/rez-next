//! Tool detection utilities: find executables and read their versions.

use std::path::PathBuf;
use std::process::Command;

/// Find the first occurrence of a tool in PATH.
///
/// Returns the absolute path to the executable, or `None` if not found.
pub fn find_tool_executable(name: &str) -> Option<PathBuf> {
    let output = if cfg!(windows) {
        Command::new("where").arg(name).output().ok()?
    } else {
        Command::new("which").arg(name).output().ok()?
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        // `where` may return multiple lines; use the first
        let first_line = stdout.lines().next()?.trim();
        if first_line.is_empty() {
            return None;
        }
        Some(PathBuf::from(first_line))
    } else {
        None
    }
}

/// Attempt to detect the version string of a tool by running common version flags.
///
/// Tries `--version`, `-version`, `-V` in order. Returns the raw output string,
/// or an empty string if detection fails.
pub fn detect_tool_version(executable: &str) -> String {
    let flags = ["--version", "-version", "-V", "version"];

    for flag in &flags {
        if let Ok(output) = Command::new(executable).arg(flag).output() {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let trimmed = combined.trim().to_string();
            if !trimmed.is_empty() {
                return trimmed;
            }
        }
    }

    String::new()
}

/// Extract a semver-like version token from a raw version string.
///
/// Scans for the first token matching `N.N` or `N.N.N[.N...]` and returns it.
/// Falls back to returning the first whitespace-delimited token if no semver found.
pub fn extract_version_from_output(output: &str) -> Option<String> {
    // Match patterns like "3.11.4", "2.42.0", "1.8"
    let semver_re = regex::Regex::new(r"\b(\d+\.\d+(?:\.\d+)*)\b").ok()?;
    if let Some(cap) = semver_re.captures(output) {
        return Some(cap[1].to_string());
    }
    // Fallback: first word-like token that contains a digit
    for token in output.split_whitespace() {
        if token.chars().any(|c| c.is_ascii_digit()) {
            return Some(token.trim_matches(|c: char| !c.is_alphanumeric() && c != '.').to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_from_output_python() {
        let out = "Python 3.11.4";
        assert_eq!(extract_version_from_output(out), Some("3.11.4".to_string()));
    }

    #[test]
    fn test_extract_version_from_output_git() {
        let out = "git version 2.42.0.windows.1";
        assert_eq!(extract_version_from_output(out), Some("2.42.0".to_string()));
    }

    #[test]
    fn test_extract_version_from_output_cmake() {
        let out = "cmake version 3.26.0";
        assert_eq!(extract_version_from_output(out), Some("3.26.0".to_string()));
    }

    #[test]
    fn test_extract_version_short() {
        let out = "1.8";
        assert_eq!(extract_version_from_output(out), Some("1.8".to_string()));
    }

    #[test]
    fn test_extract_version_none() {
        let out = "no version information";
        // No digit-containing semver token — fallback returns something with digits if present
        // Here there are none, so None expected
        assert_eq!(extract_version_from_output(out), None);
    }
}
