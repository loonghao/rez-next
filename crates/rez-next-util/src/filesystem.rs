//! File system utility functions

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rez_next_common::RezCoreError;
use crate::RezResult;

/// Expand a path that may start with `~` to the user's home directory
pub fn expand_user_path<P: AsRef<Path>>(path: P) -> RezResult<PathBuf> {
    let path = path.as_ref();
    
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            Ok(home.join(stripped.strip_prefix("/").unwrap_or(stripped)))
        } else {
            Err(RezCoreError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine home directory"
            )))
        }
    } else {
        Ok(path.to_path_buf())
    }
}

/// Ensure a directory exists, creating it and all parent directories if necessary
pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> RezResult<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(RezCoreError::Io)?;
    } else if !path.is_dir() {
        return Err(RezCoreError::Io(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path exists but is not a directory: {}", path.display())
        )));
    }
    
    Ok(())
}

/// Ensure a file's parent directory exists
pub fn ensure_parent_dir_exists<P: AsRef<Path>>(path: P) -> RezResult<()> {
    let path = path.as_ref();
    
    if let Some(parent) = path.parent() {
        ensure_dir_exists(parent)
    } else {
        Ok(())
    }
}

/// Check if a path is writable
pub fn is_writable<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref();
    
    if !path.exists() {
        // Check if parent directory is writable
        if let Some(parent) = path.parent() {
            is_writable(parent)
        } else {
            false
        }
    } else if path.is_dir() {
        // For directories, try to create a temp file to check writability
        let temp_file = path.join(".write_test_tmp");
        let result = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&temp_file)
            .is_ok();
        if result {
            let _ = fs::remove_file(temp_file);
        }
        result
    } else {
        // Try to open for writing
        fs::OpenOptions::new()
            .write(true)
            .open(path)
            .is_ok()
    }
}

/// Safely remove a file or directory (recursively for directories)
pub fn safe_remove<P: AsRef<Path>>(path: P) -> RezResult<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        return Ok(());
    }
    
    if path.is_dir() {
        fs::remove_dir_all(path)
            .map_err(RezCoreError::Io)?;
    } else {
        fs::remove_file(path)
            .map_err(RezCoreError::Io)?;
    }
    
    Ok(())
}

/// Copy a file, creating parent directories if necessary
pub fn copy_file<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> RezResult<u64> {
    let from = from.as_ref();
    let to = to.as_ref();
    
    ensure_parent_dir_exists(to)?;
    
    fs::copy(from, to)
        .map_err(RezCoreError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_expand_user_path() {
        let path = expand_user_path("~/test.txt").unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(path, home.join("test.txt"));
    }

    #[test]
    fn test_ensure_dir_exists() {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("a/b/c");
        
        ensure_dir_exists(&new_dir).unwrap();
        assert!(new_dir.exists());
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_is_writable() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create the file first
        fs::write(&file_path, "test").unwrap();
        
        // Now it should be writable
        assert!(is_writable(&file_path));
        
        // Non-existent file in writable directory should also be writable
        let file_path2 = temp_dir.path().join("test2.txt");
        assert!(is_writable(&file_path2));
    }

    #[test]
    fn test_safe_remove() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let dir_path = temp_dir.path().join("subdir");
        
        // Create file and directory
        fs::write(&file_path, "test").unwrap();
        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("nested.txt"), "test").unwrap();
        
        // Remove file
        safe_remove(&file_path).unwrap();
        assert!(!file_path.exists());
        
        // Remove directory recursively
        safe_remove(&dir_path).unwrap();
        assert!(!dir_path.exists());
    }

    #[test]
    fn test_copy_file() {
        let temp_dir = TempDir::new().unwrap();
        let src = temp_dir.path().join("src.txt");
        let dst = temp_dir.path().join("subdir/dst.txt");
        
        fs::write(&src, "hello").unwrap();
        
        copy_file(&src, &dst).unwrap();
        assert!(dst.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "hello");
    }
}
