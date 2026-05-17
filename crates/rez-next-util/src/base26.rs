//! Base26 encoding utilities.
//!
//! Provides functions for generating Base26 sequences (a, b, ..., z, aa, ab, ...)
//! and creating unique Base26 symlinks. This is commonly used in Rez for
//! generating short, unique identifiers.

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Error type for Base26 operations.
#[derive(Debug, Clone)]
pub enum Base26Error {
    /// Invalid Base26 string (contains non-lowercase-letter characters).
    InvalidBase26(String),
    /// IO error during symlink creation.
    IoError(String),
    /// Failed to create unique symlink after maximum retries.
    RetryExhausted(String),
}

impl fmt::Display for Base26Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base26Error::InvalidBase26(s) => {
                write!(
                    f,
                    "Invalid Base26 string: '{}' (must be lowercase a-z only)",
                    s
                )
            }
            Base26Error::IoError(s) => write!(f, "IO error: {}", s),
            Base26Error::RetryExhausted(s) => {
                write!(f, "Failed to create unique symlink after retries: {}", s)
            }
        }
    }
}

impl Error for Base26Error {}

/// Get the next Base26 string in sequence.
///
/// Sequence: a -> b -> ... -> z -> aa -> ab -> ...
///
/// # Arguments
///
/// * `prev` - Optional previous Base26 string. If None, returns "a".
///
/// # Returns
///
/// The next Base26 string in sequence.
///
/// # Errors
///
/// Returns `Base26Error::InvalidBase26` if `prev` is not a valid Base26 string
/// (must contain only lowercase a-z characters).
///
/// # Examples
///
/// ```
/// use rez_next_util::get_next_base26;
///
/// assert_eq!(get_next_base26(None).unwrap(), "a");
/// assert_eq!(get_next_base26(Some("a")).unwrap(), "b");
/// assert_eq!(get_next_base26(Some("z")).unwrap(), "aa");
/// assert_eq!(get_next_base26(Some("az")).unwrap(), "ba");
/// ```
pub fn get_next_base26(prev: Option<&str>) -> Result<String, Base26Error> {
    let prev = match prev {
        None => return Ok("a".to_string()),
        Some(s) => s,
    };

    // Validate: must be only lowercase a-z
    if !prev.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(Base26Error::InvalidBase26(prev.to_string()));
    }

    let mut chars: Vec<char> = prev.chars().collect();

    // Increment like a base26 number (a=0, z=25)
    let mut idx = chars.len() as isize - 1;
    let mut carry = true;

    while carry && idx >= 0 {
        let c = chars[idx as usize];
        if c == 'z' {
            chars[idx as usize] = 'a';
            idx -= 1;
        } else {
            chars[idx as usize] = ((c as u8) + 1) as char;
            carry = false;
        }
    }

    if carry {
        // All digits were 'z', prepend 'a'
        chars.insert(0, 'a');
    }

    Ok(chars.iter().collect())
}

/// Create a unique Base26-named symlink in the given directory.
///
/// If a symlink already exists in `path` that points to `source`, returns
/// that existing symlink path. Otherwise, creates a new symlink with a
/// unique Base26 name.
///
/// # Arguments
///
/// * `path` - Directory where the symlink will be created.
/// * `source` - Path that the symlink will point to.
///
/// # Returns
///
/// Path to the created (or existing) symlink.
///
/// # Errors
///
/// Returns `Base26Error` if symlink creation fails after maximum retries,
/// or if IO operations fail.
///
/// # Platform Support
///
/// This function is only available on Unix-like systems (Linux, macOS).
/// On Windows, it will return an error.
#[cfg(unix)]
pub fn create_unique_base26_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    path: P,
    source: Q,
) -> Result<PathBuf, Base26Error> {
    use std::os::unix::fs::symlink;

    let path = path.as_ref();
    let source = source.as_ref();

    // Ensure the directory exists
    if !path.exists() {
        std::fs::create_dir_all(path)
            .map_err(|e| Base26Error::IoError(format!("Failed to create directory: {}", e)))?;
    }

    // Check if a symlink already exists pointing to source
    if let Some(existing) = find_matching_symlink(path, source)? {
        return Ok(existing);
    }

    // Find the maximum existing Base26 name
    let max_name = get_max_base26_name(path)?;
    let mut next_name = get_next_base26(max_name.as_deref())?;

    // Retry up to 10 times (handle race conditions)
    for _ in 0..10 {
        let symlink_path = path.join(&next_name);

        // Check if this name already exists (race condition handling)
        if symlink_path.exists() || symlink_path.is_symlink() {
            next_name = get_next_base26(Some(&next_name))?;
            continue;
        }

        // Try to create symlink
        match symlink(source, &symlink_path) {
            Ok(()) => return Ok(symlink_path),
            Err(e) => {
                // If symlink creation failed because path already exists,
                // try the next name
                next_name = get_next_base26(Some(&next_name))?;
                continue;
            }
        }
    }

    Err(Base26Error::RetryExhausted(format!(
        "Failed to create unique symlink in {:?} after 10 retries",
        path
    )))
}

/// Find an existing symlink in `dir` that points to `target`.
#[cfg(unix)]
fn find_matching_symlink(dir: &Path, target: &Path) -> Result<Option<PathBuf>, Base26Error> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Base26Error::IoError(format!("Failed to read directory: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_symlink() {
            match std::fs::read_link(&path) {
                Ok(link_target) => {
                    if link_target == target {
                        return Ok(Some(path));
                    }
                }
                Err(_) => continue,
            }
        }
    }

    Ok(None)
}

/// Get the maximum (lexicographically last) Base26 name in the directory.
/// Returns None if no valid Base26 names exist.
#[cfg(unix)]
fn get_max_base26_name(dir: &Path) -> Result<Option<String>, Base26Error> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| Base26Error::IoError(format!("Failed to read directory: {}", e)))?;

    let mut max_name: Option<String> = None;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Check if name is a valid Base26 string (lowercase a-z only)
        if !name.chars().all(|c| c.is_ascii_lowercase()) {
            continue;
        }

        match &max_name {
            None => max_name = Some(name),
            Some(current) => {
                if name > *current {
                    max_name = Some(name);
                }
            }
        }
    }

    Ok(max_name)
}

/// Stub for non-Unix platforms.
#[cfg(not(unix))]
pub fn create_unique_base26_symlink<P: AsRef<Path>, Q: AsRef<Path>>(
    _path: P,
    _source: Q,
) -> Result<PathBuf, Base26Error> {
    Err(Base26Error::IoError(
        "create_unique_base26_symlink is only supported on Unix-like systems".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_next_base26_none() {
        let result = get_next_base26(None).unwrap();
        assert_eq!(result, "a");
    }

    #[test]
    fn test_get_next_base26_a() {
        let result = get_next_base26(Some("a")).unwrap();
        assert_eq!(result, "b");
    }

    #[test]
    fn test_get_next_base26_z() {
        let result = get_next_base26(Some("z")).unwrap();
        assert_eq!(result, "aa");
    }

    #[test]
    fn test_get_next_base26_az() {
        let result = get_next_base26(Some("az")).unwrap();
        assert_eq!(result, "ba");
    }

    #[test]
    fn test_get_next_base26_zz() {
        let result = get_next_base26(Some("zz")).unwrap();
        assert_eq!(result, "aaa");
    }

    #[test]
    fn test_get_next_base26_azzz() {
        let result = get_next_base26(Some("azzz")).unwrap();
        assert_eq!(result, "baaa");
    }

    #[test]
    fn test_get_next_base26_invalid() {
        let result = get_next_base26(Some("a1b"));
        assert!(result.is_err());

        let result = get_next_base26(Some("A"));
        assert!(result.is_err());

        let result = get_next_base26(Some("aB"));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_next_base26_sequence() {
        let mut current = None;
        let mut results = Vec::new();

        for _ in 0..100 {
            current = Some(get_next_base26(current.as_deref()).unwrap());
            results.push(current.clone().unwrap());
        }

        assert_eq!(results[0], "a");
        assert_eq!(results[1], "b");
        assert_eq!(results[25], "z");
        assert_eq!(results[26], "aa");
        assert_eq!(results[27], "ab");
    }
}
