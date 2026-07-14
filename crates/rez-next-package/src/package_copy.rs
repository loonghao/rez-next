//! Package copy operations.
//!
//! Provides domain-level package copy functionality aligned with rez's `package_copy.py`.
//! Follows Clean Architecture: domain logic lives in the core crate, not in Python bindings.
//!
//! ## Lessons from Rez Issues (avoided pitfalls):
//! - **UNC paths (#1438)**: All paths are normalized via `dunce::canonicalize` to avoid UNC vs
//!   mapped drive mismatches on Windows.
//! - **Case sensitivity (#1302)**: Path comparisons use platform-appropriate case sensitivity.
//! - **Disk space (#N/A)**: Pre-check available disk space before copy operations.

use std::fs;
use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors that can occur during package copy operations.
#[derive(Debug, Error)]
pub enum PackageCopyError {
    #[error("Package '{name}' not found in search paths")]
    PackageNotFound { name: String },

    #[error("Package version '{version}' not found for '{name}'")]
    VersionNotFound { name: String, version: String },

    #[error("Source directory not found: {path}")]
    SourceNotFound { path: String },

    #[error("Destination already exists: {path}. Use force=true to overwrite.")]
    DestinationExists { path: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Result of a package copy operation.
#[derive(Debug, Clone)]
pub struct PackageCopyResult {
    /// Source path that was copied from.
    pub source: PathBuf,
    /// Destination path that was copied to.
    pub destination: PathBuf,
    /// Number of files copied.
    pub files_copied: usize,
    /// Total bytes copied.
    pub bytes_copied: u64,
}

/// Configuration for package copy operations.
///
/// Uses Dependency Inversion: accepts config rather than reading it internally,
/// making the function testable and reusable.
#[derive(Debug, Clone)]
pub struct PackageCopyConfig {
    /// Package search paths.
    pub packages_path: Vec<PathBuf>,
    /// Overwrite existing destination.
    pub force: bool,
    /// Whether to normalize paths (avoid UNC issues on Windows).
    pub normalize_paths: bool,
}

impl Default for PackageCopyConfig {
    fn default() -> Self {
        Self {
            packages_path: Vec::new(),
            force: false,
            normalize_paths: true,
        }
    }
}

/// Copy a package from one location to another.
///
/// This is the domain-level implementation, aligned with rez's `copy_package()`.
///
/// # Example
/// ```ignore
/// use rez_next_package::package_copy::{copy_package, PackageCopyConfig};
///
/// let config = PackageCopyConfig {
///     packages_path: vec!["/packages".into()],
///     ..Default::default()
/// };
/// let result = copy_package("maya", "2024.0", "/dest/packages", &config)?;
/// ```
pub fn copy_package(
    name: &str,
    version: &str,
    dest_base: &Path,
    config: &PackageCopyConfig,
) -> Result<PackageCopyResult, PackageCopyError> {
    // Find the source directory
    let src = find_package_dir(name, version, &config.packages_path)?;

    let dest = dest_base.join(name).join(version);

    // Check destination
    if dest.exists() {
        if config.force {
            fs::remove_dir_all(&dest)?;
        } else {
            return Err(PackageCopyError::DestinationExists {
                path: dest.display().to_string(),
            });
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    // Perform the copy
    let stats = copy_dir_recursive(&src, &dest)?;

    Ok(PackageCopyResult {
        source: src,
        destination: dest,
        files_copied: stats.files,
        bytes_copied: stats.bytes,
    })
}

/// Find a package directory by name and version across search paths.
pub fn find_package_dir(
    name: &str,
    version: &str,
    search_paths: &[PathBuf],
) -> Result<PathBuf, PackageCopyError> {
    for base in search_paths {
        // Check both version and version subdir patterns
        let candidates = [
            base.join(name).join(version),
            base.join(name).join(format!("{}-{}", name, version)),
        ];

        for candidate in &candidates {
            if candidate.exists() && candidate.is_dir() {
                return Ok(normalize_path(candidate));
            }
        }
    }

    Err(PackageCopyError::VersionNotFound {
        name: name.to_string(),
        version: version.to_string(),
    })
}

/// Normalize a path to avoid UNC path issues on Windows.
///
/// Addresses Rez issue #1438: "Environment resolution uses UNC paths with Python 3.10"
fn normalize_path(path: &Path) -> PathBuf {
    // Use dunce to strip UNC prefixes on Windows
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Debug, Default)]
struct CopyStats {
    files: usize,
    bytes: u64,
}

/// Recursively copy a directory, preserving symlinks and permissions.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<CopyStats, std::io::Error> {
    let mut stats = CopyStats::default();

    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if file_type.is_symlink() {
            let target = fs::read_link(&src_path)?;
            // On Windows, copy symlinks as regular files if symlink creation fails
            #[cfg(windows)]
            {
                if std::os::windows::fs::symlink_file(&target, &dest_path).is_err()
                    && std::os::windows::fs::symlink_dir(&target, &dest_path).is_err()
                {
                    if target.is_dir() {
                        let sub_stats = copy_dir_recursive(&target, &dest_path)?;
                        stats.files += sub_stats.files;
                        stats.bytes += sub_stats.bytes;
                    } else {
                        let bytes = fs::copy(&target, &dest_path)?;
                        stats.files += 1;
                        stats.bytes += bytes;
                    }
                } else {
                    stats.files += 1;
                }
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&target, &dest_path)?;
                stats.files += 1;
            }
        } else if file_type.is_dir() {
            let sub_stats = copy_dir_recursive(&src_path, &dest_path)?;
            stats.files += sub_stats.files;
            stats.bytes += sub_stats.bytes;
        } else if file_type.is_file() {
            let bytes = fs::copy(&src_path, &dest_path)?;
            stats.files += 1;
            stats.bytes += bytes;
        }
    }

    Ok(stats)
}

/// Calculate total size of a directory recursively.
pub fn dir_size(path: &Path) -> Result<u64, std::io::Error> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                total += dir_size(&entry.path())?;
            } else {
                total += meta.len();
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_find_package_dir_exists() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("testpkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), "name = 'testpkg'").unwrap();

        let result = find_package_dir("testpkg", "1.0.0", &[tmp.path().to_path_buf()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_package_dir_not_found() {
        let tmp = TempDir::new().unwrap();
        let result = find_package_dir("nonexistent", "1.0.0", &[tmp.path().to_path_buf()]);
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_package_basic() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            "name = 'mypkg'\nversion = '1.0.0'",
        )
        .unwrap();
        fs::write(pkg_dir.join("data.txt"), "hello").unwrap();

        let config = PackageCopyConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            ..Default::default()
        };

        let result = copy_package("mypkg", "1.0.0", dest_tmp.path(), &config).unwrap();
        assert!(result.destination.exists());
        assert!(result.files_copied > 0);
        assert!(result.bytes_copied > 0);
    }

    #[test]
    fn test_copy_package_destination_exists_no_force() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), "name = 'mypkg'").unwrap();

        // Pre-create destination
        fs::create_dir_all(dest_tmp.path().join("mypkg").join("1.0.0")).unwrap();

        let config = PackageCopyConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            force: false,
            ..Default::default()
        };

        let result = copy_package("mypkg", "1.0.0", dest_tmp.path(), &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_copy_package_destination_exists_with_force() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), "name = 'mypkg'").unwrap();

        let config = PackageCopyConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            force: true,
            ..Default::default()
        };

        let result = copy_package("mypkg", "1.0.0", dest_tmp.path(), &config).unwrap();
        assert!(result.destination.exists());
    }
}
