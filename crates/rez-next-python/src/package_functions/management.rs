//! Package management functions exposed to Python.
//!
//! Covers: create_package, copy_package, move_package, remove_package.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

use super::{expand_home, make_repo_manager, copy_dir_recursive};

/// Create a Package object from a dictionary.
/// Equivalent to `rez.packages.create_package(data)`.
///
/// The dict must contain at least a "name" key.
/// Optional keys: version, description, authors, requires,
/// build_requires, variants, tools, commands, uuid, timestamp, etc.
#[pyfunction]
pub fn create_package(py: Python<'_>, data: Bound<'_, PyDict>) -> PyResult<PyPackage> {
    PyPackage::from_dict(py, data)
}

/// Copy a package to another location.
/// Equivalent to `rez cp <pkg> <dest>` / `rez.copy_package(pkg, dest_repo_path)`
#[pyfunction]
#[pyo3(signature = (pkg_name, dest_path, version=None, src_paths=None, force=false))]
pub fn copy_package(
    pkg_name: &str,
    dest_path: &str,
    version: Option<&str>,
    src_paths: Option<Vec<String>>,
    force: bool,
) -> PyResult<String> {
    use rez_next_common::config::RezCoreConfig;
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let rt = get_runtime();

    let search_paths: Vec<PathBuf> = src_paths
        .clone()
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    let repo_manager = make_repo_manager(src_paths);
    let packages = rt
        .block_on(repo_manager.find_packages(pkg_name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let pkg = if let Some(ver) = version {
        packages
            .into_iter()
            .find(|p| p.version.as_ref().is_some_and(|v| v.as_str() == ver))
    } else {
        let mut sorted = packages;
        // Descending: b > a  →  bv.cmp(av) so that the latest version is first.
        sorted.sort_by(|a, b| {
            b.version
                .as_ref()
                .and_then(|bv| a.version.as_ref().map(|av| bv.cmp(av)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().next()
    };

    let pkg = pkg.ok_or_else(|| {
        pyo3::exceptions::PyLookupError::new_err(format!("Package '{}' not found", pkg_name))
    })?;

    let ver_str = pkg
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");

    // Find source path
    let mut src_root: Option<PathBuf> = None;
    for base in &search_paths {
        let candidate = base.join(&pkg.name).join(ver_str);
        if candidate.exists() {
            src_root = Some(candidate);
            break;
        }
    }

    let src_root = src_root.ok_or_else(|| {
        pyo3::exceptions::PyFileNotFoundError::new_err(format!(
            "Package directory for '{}-{}' not found",
            pkg_name, ver_str
        ))
    })?;

    let dest_root = PathBuf::from(dest_path).join(&pkg.name).join(ver_str);
    if !force && dest_root.exists() {
        return Err(pyo3::exceptions::PyFileExistsError::new_err(format!(
            "Destination already exists: {}. Use force=True to overwrite.",
            dest_root.display()
        )));
    }

    if force && dest_root.exists() {
        std::fs::remove_dir_all(&dest_root)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    }

    copy_dir_recursive(&src_root, &dest_root)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    Ok(dest_root.to_string_lossy().to_string())
}

/// Move a package to another location.
/// Equivalent to `rez mv <pkg> <dest>`
#[pyfunction]
#[pyo3(signature = (pkg_name, dest_path, version=None, src_paths=None, force=false, keep_source=false))]
pub fn move_package(
    pkg_name: &str,
    dest_path: &str,
    version: Option<&str>,
    src_paths: Option<Vec<String>>,
    force: bool,
    keep_source: bool,
) -> PyResult<String> {
    // Copy first, then remove source
    let dest = copy_package(pkg_name, dest_path, version, src_paths.clone(), force)?;

    if !keep_source {
        use rez_next_common::config::RezCoreConfig;
        let config = RezCoreConfig::load();

        let search_paths: Vec<std::path::PathBuf> = src_paths
            .map(|p| p.into_iter().map(std::path::PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| std::path::PathBuf::from(expand_home(p)))
                    .collect()
            });

        // Extract the actual version from the dest path returned by copy_package.
        // dest is always <dest_path>/<pkg_name>/<actual_version>, so the last
        // component is the version that was actually copied — even when version=None
        // (in which case copy_package picks the latest automatically).
        let dest_path_buf = std::path::PathBuf::from(&dest);
        let ver_display_owned: String = dest_path_buf
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| version.unwrap_or("unknown").to_string());
        let ver_display = ver_display_owned.as_str();

        for base in &search_paths {
            let candidate = base.join(pkg_name).join(ver_display);
            if candidate.exists() {
                std::fs::remove_dir_all(&candidate)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                break;
            }
        }
    }

    Ok(dest)
}

/// Remove a package version from repositories.
/// Equivalent to `rez rm <pkg>`
#[pyfunction]
#[pyo3(signature = (pkg_name, version=None, paths=None))]
pub fn remove_package(
    pkg_name: &str,
    version: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<usize> {
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::load();

    let search_paths: Vec<std::path::PathBuf> = paths
        .map(|p| p.into_iter().map(std::path::PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| std::path::PathBuf::from(expand_home(p)))
                .collect()
        });

    let mut removed = 0usize;
    for base in &search_paths {
        let pkg_base = base.join(pkg_name);
        if !pkg_base.exists() {
            continue;
        }

        if let Some(ver) = version {
            let candidate = pkg_base.join(ver);
            if candidate.exists() {
                std::fs::remove_dir_all(&candidate)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                removed += 1;
            }
        } else {
            // Remove entire package family
            std::fs::remove_dir_all(&pkg_base)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            removed += 1;
        }
    }

    Ok(removed)
}
