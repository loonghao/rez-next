//! Package move operations.
//!
//! Provides domain-level package move functionality aligned with rez's `package_move.py`.
//! Follows Clean Architecture: delegates to copy + remove, not duplicating logic.
//!
//! ## Lessons from Rez Issues (avoided pitfalls):
//! - **#1438 (UNC paths)**: Path normalization via `dunce::canonicalize` from `package_copy`.
//! - **#1374 (filtered packages)**: Atomic copy-remove with verification — if copy fails,
//!   source is never touched; remove only happens after successful copy verification.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::package_copy::{self, PackageCopyConfig, PackageCopyError};
use crate::package_remove::{self, PackageRemoveConfig, PackageRemoveError};

/// Errors that can occur during package move operations.
#[derive(Debug, Error)]
pub enum PackageMoveError {
    #[error("{0}")]
    Copy(#[from] PackageCopyError),

    #[error("{0}")]
    Remove(#[from] PackageRemoveError),

    #[error("Cannot move a package to the same location: {path}")]
    SameLocation { path: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Result of a package move operation.
#[derive(Debug, Clone)]
pub struct PackageMoveResult {
    /// Source path that was moved from.
    pub source: PathBuf,
    /// Destination path that was moved to.
    pub destination: PathBuf,
    /// Number of files moved.
    pub files_copied: usize,
    /// Total bytes moved.
    pub bytes_copied: u64,
    /// Whether the source was removed after copy.
    pub source_removed: bool,
}

/// Configuration for package move operations.
///
/// Uses Dependency Inversion: accepts config rather than reading it internally.
#[derive(Debug, Clone)]
pub struct PackageMoveConfig {
    /// Package search paths.
    pub packages_path: Vec<PathBuf>,
    /// Overwrite existing destination.
    pub force: bool,
    /// Keep source after move (makes it a copy + no removal).
    pub keep_source: bool,
    /// Normalize paths to avoid UNC issues on Windows.
    pub normalize_paths: bool,
}

impl Default for PackageMoveConfig {
    fn default() -> Self {
        Self {
            packages_path: Vec::new(),
            force: false,
            keep_source: false,
            normalize_paths: true,
        }
    }
}

impl PackageMoveConfig {
    /// Create a PackageCopyConfig from this move config.
    fn to_copy_config(&self) -> PackageCopyConfig {
        PackageCopyConfig {
            packages_path: self.packages_path.clone(),
            force: self.force,
            normalize_paths: self.normalize_paths,
        }
    }

    /// Create a PackageRemoveConfig from this move config.
    fn to_remove_config(&self) -> PackageRemoveConfig {
        PackageRemoveConfig {
            packages_path: self.packages_path.clone(),
            force: true, // We always force remove during move
            prune_empty_families: true,
        }
    }
}

/// Move a package from one location to another.
///
/// This is the domain-level implementation, aligned with rez's `move_package()`.
/// It delegates to `copy_package` and `remove_package_version` — following
/// the Single Responsibility Principle: each operation does one thing.
///
/// # Safety
/// - Copy first, verify copy success, then remove source (atomic-like behavior).
/// - If `keep_source` is true, the source is not removed (behaves like a copy).
///
/// # Example
/// ```ignore
/// use rez_next_package::package_move::{move_package, PackageMoveConfig};
///
/// let config = PackageMoveConfig {
///     packages_path: vec!["/packages".into()],
///     ..Default::default()
/// };
/// let result = move_package("maya", "2024.0", "/dest/packages", &config)?;
/// ```
pub fn move_package(
    name: &str,
    version: &str,
    dest_base: &Path,
    config: &PackageMoveConfig,
) -> Result<PackageMoveResult, PackageMoveError> {
    // Prevent moving to the same location
    let dest = dest_base.join(name).join(version);
    let src = package_copy::find_package_dir(name, version, &config.packages_path)?;

    // Normalize both paths
    let src_normalized = dunce::canonicalize(&src).unwrap_or_else(|_| src.clone());
    let dest_normalized = dunce::canonicalize(&dest).unwrap_or_else(|_| dest.clone());

    if src_normalized == dest_normalized {
        return Err(PackageMoveError::SameLocation {
            path: src.display().to_string(),
        });
    }

    // Phase 1: Copy
    let copy_result =
        package_copy::copy_package(name, version, dest_base, &config.to_copy_config())?;

    // Phase 2: Remove source (unless keep_source)
    let source_removed = if !config.keep_source {
        package_remove::remove_package_version(name, version, &config.to_remove_config())?;
        true
    } else {
        false
    };

    Ok(PackageMoveResult {
        source: copy_result.source,
        destination: copy_result.destination,
        files_copied: copy_result.files_copied,
        bytes_copied: copy_result.bytes_copied,
        source_removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_move_package_basic() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.py"), "name = 'mypkg'\nversion = '1.0.0'").unwrap();
        std::fs::write(pkg_dir.join("data.txt"), "hello").unwrap();

        let config = PackageMoveConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            ..Default::default()
        };

        let result = move_package("mypkg", "1.0.0", dest_tmp.path(), &config).unwrap();
        assert!(result.destination.exists());
        assert!(result.source_removed);
        assert!(!pkg_dir.exists(), "Source should be removed after move");
    }

    #[test]
    fn test_move_package_keep_source() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.py"), "name = 'mypkg'").unwrap();

        let config = PackageMoveConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            keep_source: true,
            ..Default::default()
        };

        let result = move_package("mypkg", "1.0.0", dest_tmp.path(), &config).unwrap();
        assert!(result.destination.exists());
        assert!(!result.source_removed);
        assert!(pkg_dir.exists(), "Source should remain when keep_source=true");
    }

    #[test]
    fn test_move_package_same_location() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let config = PackageMoveConfig {
            packages_path: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let result = move_package("mypkg", "1.0.0", tmp.path(), &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("same location"));
    }

    #[test]
    fn test_move_package_not_found() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let config = PackageMoveConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            ..Default::default()
        };

        let result = move_package("nonexistent", "1.0.0", dest_tmp.path(), &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_move_package_overwrite_with_force() {
        let src_tmp = TempDir::new().unwrap();
        let dest_tmp = TempDir::new().unwrap();

        let pkg_dir = src_tmp.path().join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.py"), "name = 'mypkg'").unwrap();

        // Pre-create destination
        std::fs::create_dir_all(dest_tmp.path().join("mypkg").join("1.0.0")).unwrap();

        let config = PackageMoveConfig {
            packages_path: vec![src_tmp.path().to_path_buf()],
            force: true,
            ..Default::default()
        };

        let result = move_package("mypkg", "1.0.0", dest_tmp.path(), &config).unwrap();
        assert!(result.source_removed);
    }
}
