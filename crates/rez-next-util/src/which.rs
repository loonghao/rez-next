//! Implementation of `which` utility (find executable in PATH)
//!
//! This module provides functionality to find executable files in the system PATH,
//! compatible with Unix `which` and Windows `where` commands.

use std::env;
use std::path::{Path, PathBuf};

/// Find an executable in the system PATH.
///
/// This function searches for an executable file in the directories listed in the
/// `PATH` environment variable. On Windows, it also tries common executable
/// extensions (`.exe`, `.cmd`, `.bat`, `.ps1`) if the name doesn't
/// already have an extension.
///
/// # Arguments
///
/// * `command` - The command name to find (e.g., "python", "git")
///
/// # Returns
///
/// * `Some(PathBuf)` - Full path to the executable
/// * `None` - Command not found in PATH
///
/// # Examples
///
/// ```
/// use rez_next_util::which;
///
/// // Find python in PATH
/// if let Some(path) = which("python") {
///     println!("Found python at: {}", path.display());
/// }
/// ```
pub fn which(command: &str) -> Option<PathBuf> {
    // Get PATH environment variable
    let path_var = env::var("PATH").ok()?;

    // Determine path separator (cross-platform)
    #[cfg(windows)]
    let separator = ';';
    #[cfg(not(windows))]
    let separator = ':';

    // Get current directory for relative path check
    let current_dir = env::current_dir().ok()?;

    // Check if command contains path separators (relative/absolute path)
    let cmd_path = Path::new(command);
    if command.contains('/') || command.contains('\\') {
        // Command is a path - check if it's executable
        let full_path = if cmd_path.is_absolute() {
            cmd_path.to_path_buf()
        } else {
            current_dir.join(cmd_path)
        };

        if is_executable(&full_path) {
            return Some(full_path);
        }
        return None;
    }

    // Search in PATH directories
    for dir in path_var.split(separator) {
        let dir_path = Path::new(dir);

        // Skip empty directory entries
        if dir.is_empty() {
            continue;
        }

        // On Windows, also check common executable extensions
        #[cfg(windows)]
        {
            let extensions = ["", ".exe", ".cmd", ".bat", ".ps1"];
            for ext in &extensions {
                let candidate = dir_path.join(format!("{command}{ext}"));
                if is_executable(&candidate) {
                    return Some(candidate);
                }
            }
        }

        #[cfg(not(windows))]
        {
            let candidate = dir_path.join(command);
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

/// Find all executables with the given name in PATH.
///
/// Unlike `which()` which returns the first match, this function returns
/// all matching executables in PATH order.
///
/// # Arguments
///
/// * `command` - The command name to find
///
/// # Returns
///
/// * `Vec<PathBuf>` - All found executable paths
pub fn which_all(command: &str) -> Vec<PathBuf> {
    let mut results = Vec::new();

    let path_var = match env::var("PATH") {
        Ok(p) => p,
        Err(_) => return results,
    };

    #[cfg(windows)]
    let separator = ';';
    #[cfg(not(windows))]
    let separator = ':';

    for dir in path_var.split(separator) {
        if dir.is_empty() {
            continue;
        }

        let dir_path = Path::new(dir);

        #[cfg(windows)]
        {
            let extensions = ["", ".exe", ".cmd", ".bat", ".ps1"];
            for ext in &extensions {
                let candidate = dir_path.join(format!("{command}{ext}"));
                if is_executable(&candidate) {
                    results.push(candidate);
                }
            }
        }

        #[cfg(not(windows))]
        {
            let candidate = dir_path.join(command);
            if is_executable(&candidate) {
                results.push(candidate);
            }
        }
    }

    results
}

/// Check if a file is executable.
///
/// On Unix, this checks the executable permission bit.
/// On Windows, this checks if the file has an executable extension.
fn is_executable(path: &Path) -> bool {
    // File must exist
    if !path.is_file() {
        return false;
    }

    #[cfg(windows)]
    {
        // On Windows, check for executable extensions
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            matches!(ext_str.as_str(), "exe" | "cmd" | "bat" | "ps1" | "com")
        } else {
            // No extension - might still be executable (shebang scripts)
            // But for safety, require an extension on Windows
            false
        }
    }

    #[cfg(unix)]
    {
        use std::os::unix::prelude::*;
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return false,
        };
        // Check owner, group, or other execute permission
        metadata.permissions().mode() & 0o111 != 0
    }

    #[cfg(not(any(windows, unix)))]
    {
        // Fallback: just check if file exists
        path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_executable(dir: &Path, name: &str) -> PathBuf {
        let filename = if cfg!(windows) {
            format!("{}.cmd", name)
        } else {
            name.to_string()
        };
        let path = dir.join(filename);

        #[cfg(windows)]
        {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .attributes(0x80000000) // FILE_ATTRIBUTE_NORMAL
                .open(&path)
                .unwrap();
            writeln!(file, "@echo off\necho test").unwrap();
        }

        #[cfg(not(windows))]
        {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap();
            writeln!(file, "#!/bin/bash\necho test").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::prelude::*;
                let mut perms = fs::metadata(&path).unwrap().permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&path, perms).unwrap();
            }
        }

        path
    }

    fn create_nonexecutable(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        writeln!(file, "not executable").unwrap();
        path
    }

    #[test]
    fn test_which_nonexistent() {
        let result = which("this_command_definitely_does_not_exist_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_which_all_nonexistent() {
        let results = which_all("this_command_definitely_does_not_exist_12345");
        assert!(results.is_empty());
    }

    #[test]
    fn test_is_executable_on_nonexistent() {
        assert!(!is_executable(Path::new(
            "/this/path/does/not/exist/command"
        )));
    }

    #[test]
    fn test_which_finds_executable_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let exe_path = create_executable(temp_dir.path(), "test_cmd");

        // Add temp_dir to PATH
        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", temp_dir.path().display(), original_path);
        env::set_var("PATH", new_path);

        let result = which("test_cmd");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), exe_path);

        // Restore PATH
        env::set_var("PATH", original_path);
    }

    #[test]
    fn test_which_all_finds_multiple() {
        let temp_dir1 = TempDir::new().unwrap();
        let temp_dir2 = TempDir::new().unwrap();

        let exe1 = create_executable(temp_dir1.path(), "multi_cmd");
        let exe2 = create_executable(temp_dir2.path(), "multi_cmd");

        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{};{};{}",
            temp_dir1.path().display(),
            temp_dir2.path().display(),
            original_path
        );
        env::set_var("PATH", new_path);

        let results = which_all("multi_cmd");
        assert_eq!(results.len(), 2);
        assert!(results.contains(&exe1));
        assert!(results.contains(&exe2));

        env::set_var("PATH", original_path);
    }

    #[test]
    fn test_which_skips_nonexecutable() {
        let temp_dir = TempDir::new().unwrap();
        create_nonexecutable(temp_dir.path(), "not_exec");

        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{};{}", temp_dir.path().display(), original_path);
        env::set_var("PATH", new_path);

        let result = which("not_exec");
        assert!(result.is_none());

        env::set_var("PATH", original_path);
    }
}
