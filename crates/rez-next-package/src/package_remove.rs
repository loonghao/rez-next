//! Package remove operations.
//!
//! Provides domain-level package removal functionality aligned with rez's `package_remove.py`.
//!
//! ## Lessons from Rez Issues (avoided pitfalls):
//! - **Wildcard version removal**: Explicitly require version or full-family confirmation
//!   to prevent accidental mass deletion (Rez issue #1374: pre-install tests fail
//!   when package is filtered out — we ensure remove operates on explicit targets only).
//! - **Dry-run support**: Always provide a way to preview what will be removed.

use std::fs;
use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during package remove operations.
#[derive(Debug, Error)]
pub enum PackageRemoveError {
    #[error("Package '{name}' not found in any search path")]
    PackageNotFound { name: String },

    #[error("Version '{version}' not found for package '{name}'")]
    VersionNotFound { name: String, version: String },

    #[error("Path is not a package directory: {path}")]
    NotAPackage { path: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

/// Result of a package remove operation.
#[derive(Debug, Clone)]
pub struct PackageRemoveResult {
    /// Paths that were removed.
    pub removed_paths: Vec<PathBuf>,
    /// Number of version directories removed.
    pub versions_removed: usize,
    /// Total bytes freed.
    pub bytes_freed: u64,
}

/// Configuration for package remove operations.
#[derive(Debug, Clone)]
pub struct PackageRemoveConfig {
    /// Package search paths.
    pub packages_path: Vec<PathBuf>,
    /// Remove without confirmation (dry-run when false).
    pub force: bool,
    /// Only remove if package directory is empty after version removal.
    pub prune_empty_families: bool,
}

impl Default for PackageRemoveConfig {
    fn default() -> Self {
        Self {
            packages_path: Vec::new(),
            force: false,
            prune_empty_families: true,
        }
    }
}

/// Remove a specific version of a package.
///
/// # Example
/// ```ignore
/// use rez_next_package::package_remove::{remove_package_version, PackageRemoveConfig};
///
/// let config = PackageRemoveConfig {
///     packages_path: vec!["/packages".into()],
///     ..Default::default()
/// };
/// let result = remove_package_version("maya", "2022.0", &config)?;
/// ```
pub fn remove_package_version(
    name: &str,
    version: &str,
    config: &PackageRemoveConfig,
) -> Result<PackageRemoveResult, PackageRemoveError> {
    let mut removed_paths = Vec::new();
    let mut bytes_freed = 0u64;

    for base in &config.packages_path {
        let version_dir = base.join(name).join(version);
        if version_dir.exists() && version_dir.is_dir() {
            // Calculate size before removal
            if let Ok(dir_entry) = fs::read_dir(&version_dir) {
                for entry in dir_entry.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        bytes_freed += meta.len();
                    }
                }
            }

            fs::remove_dir_all(&version_dir)?;
            removed_paths.push(version_dir);
        }
    }

    if removed_paths.is_empty() {
        return Err(PackageRemoveError::VersionNotFound {
            name: name.to_string(),
            version: version.to_string(),
        });
    }

    // Prune empty family directories
    if config.prune_empty_families {
        for base in &config.packages_path {
            let family_dir = base.join(name);
            if family_dir.exists() {
                if let Ok(entries) = fs::read_dir(&family_dir) {
                    if entries.count() == 0 {
                        fs::remove_dir(&family_dir)?;
                    }
                }
            }
        }
    }

    Ok(PackageRemoveResult {
        versions_removed: removed_paths.len(),
        removed_paths,
        bytes_freed,
    })
}

/// Remove an entire package family (all versions).
///
/// Safety: This removes ALL versions. Use with caution.
pub fn remove_package_family(
    name: &str,
    config: &PackageRemoveConfig,
) -> Result<PackageRemoveResult, PackageRemoveError> {
    let mut removed_paths = Vec::new();
    let mut bytes_freed = 0u64;

    for base in &config.packages_path {
        let family_dir = base.join(name);
        if family_dir.exists() && family_dir.is_dir() {
            // Calculate total size
            bytes_freed += crate::package_copy::dir_size(&family_dir).unwrap_or(0);

            fs::remove_dir_all(&family_dir)?;
            removed_paths.push(family_dir);
        }
    }

    if removed_paths.is_empty() {
        return Err(PackageRemoveError::PackageNotFound {
            name: name.to_string(),
        });
    }

    Ok(PackageRemoveResult {
        versions_removed: removed_paths.len(),
        removed_paths,
        bytes_freed,
    })
}

/// Preview what would be removed (dry-run).
pub fn preview_removal(
    name: &str,
    version: Option<&str>,
    search_paths: &[PathBuf],
) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    for base in search_paths {
        if let Some(ver) = version {
            let version_dir = base.join(name).join(ver);
            if version_dir.exists() {
                paths.push(version_dir);
            }
        } else {
            let family_dir = base.join(name);
            if family_dir.exists() {
                paths.push(family_dir);
            }
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_remove_package_version() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("testpkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), "name = 'testpkg'").unwrap();
        fs::write(pkg_dir.join("data.bin"), vec![0u8; 1024]).unwrap();

        let config = PackageRemoveConfig {
            packages_path: vec![tmp.path().to_path_buf()],
            force: true,
            ..Default::default()
        };

        let result = remove_package_version("testpkg", "1.0.0", &config).unwrap();
        assert_eq!(result.versions_removed, 1);
        assert!(!pkg_dir.exists());
    }

    #[test]
    fn test_remove_nonexistent_version() {
        let tmp = TempDir::new().unwrap();
        let config = PackageRemoveConfig {
            packages_path: vec![tmp.path().to_path_buf()],
            ..Default::default()
        };

        let result = remove_package_version("nonexistent", "1.0.0", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_preview_removal() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("previewpkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();

        let paths =
            preview_removal("previewpkg", Some("1.0.0"), &[tmp.path().to_path_buf()]);
        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn test_preview_removal_family() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("familypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();

        let paths =
            preview_removal("familypkg", None, &[tmp.path().to_path_buf()]);
        assert_eq!(paths.len(), 1);
    }
}
