//! Package management functions exposed to Python.
//!
//! Covers: create_package, copy_package, move_package, remove_package.
//!
//! ## Architecture (SOLID / Clean Architecture):
//! These bindings are thin wrappers that delegate to the domain crate
//! (`rez_next_package`). Domain logic lives in `package_copy.rs`,
//! `package_move.rs`, and `package_remove.rs` — the Python bindings
//! only handle type conversion and error mapping.
//!
//! ## Lessons from Rez Issues (avoided pitfalls):
//! - **#1438 (UNC paths)**: Path normalization delegated to domain layer
//!   via `dunce::canonicalize`.
//! - **#1302 (case sensitivity)**: Platform-aware path comparisons in domain.
//! - **#1374 (filtered packages)**: Explicit target-only removal with dry-run.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::path::PathBuf;

use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

use super::expand_home;

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

/// Resolve search paths from explicit paths or config.
fn resolve_search_paths(src_paths: &Option<Vec<String>>) -> Vec<PathBuf> {
    use rez_next_common::config::RezCoreConfig;

    match src_paths {
        Some(paths) if !paths.is_empty() => paths
            .iter()
            .map(|p| PathBuf::from(expand_home(p)))
            .collect(),
        _ => {
            let config = RezCoreConfig::load();
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        }
    }
}

/// Find the latest version of a package from the repository.
fn find_latest_version(
    pkg_name: &str,
    src_paths: &Option<Vec<String>>,
) -> PyResult<String> {
    use super::make_repo_manager;

    let repo_manager = make_repo_manager(src_paths.clone());
    let rt = get_runtime();
    let packages = rt
        .block_on(repo_manager.find_packages(pkg_name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    if packages.is_empty() {
        return Err(pyo3::exceptions::PyLookupError::new_err(format!(
            "Package '{}' not found in any search path",
            pkg_name
        )));
    }

    // Sort by version descending to find latest
    let mut sorted = packages;
    sorted.sort_by(|a, b| {
        b.version
            .as_ref()
            .and_then(|bv| a.version.as_ref().map(|av| bv.cmp(av)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    sorted
        .first()
        .and_then(|p| p.version.as_ref())
        .map(|v| v.to_string())
        .ok_or_else(|| {
            pyo3::exceptions::PyLookupError::new_err(format!(
                "Package '{}' found but has no version",
                pkg_name
            ))
        })
}

/// Copy a package to another location.
/// Equivalent to `rez cp <pkg> <dest>` / `rez.copy_package(pkg, dest_repo_path)`
///
/// Delegates to `rez_next_package::package_copy::copy_package()` domain function.
#[pyfunction]
#[pyo3(signature = (pkg_name, dest_path, version=None, src_paths=None, force=false))]
pub fn copy_package(
    pkg_name: &str,
    dest_path: &str,
    version: Option<&str>,
    src_paths: Option<Vec<String>>,
    force: bool,
) -> PyResult<String> {
    use rez_next_package::package_copy::{copy_package as domain_copy, PackageCopyConfig};

    let search_paths = resolve_search_paths(&src_paths);

    // Determine version: use provided or find latest
    let ver_str: String = match version {
        Some(v) => v.to_string(),
        None => find_latest_version(pkg_name, &src_paths)?,
    };

    let config = PackageCopyConfig {
        packages_path: search_paths,
        force,
        normalize_paths: true,
    };

    let result = domain_copy(pkg_name, &ver_str, PathBuf::from(dest_path).as_path(), &config)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(result.destination.to_string_lossy().to_string())
}

/// Move a package to another location.
/// Equivalent to `rez mv <pkg> <dest>`
///
/// Delegates to `rez_next_package::package_move::move_package()` domain function
/// (which itself delegates to copy + remove — SRP).
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
    use rez_next_package::package_move::{move_package as domain_move, PackageMoveConfig};

    let search_paths = resolve_search_paths(&src_paths);

    // Determine version: use provided or find latest
    let ver_str: String = match version {
        Some(v) => v.to_string(),
        None => find_latest_version(pkg_name, &src_paths)?,
    };

    let config = PackageMoveConfig {
        packages_path: search_paths,
        force,
        keep_source,
        normalize_paths: true,
    };

    let result = domain_move(pkg_name, &ver_str, PathBuf::from(dest_path).as_path(), &config)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(result.destination.to_string_lossy().to_string())
}

/// Remove a package version from repositories.
/// Equivalent to `rez rm <pkg>`
///
/// Delegates to `rez_next_package::package_remove` domain functions.
#[pyfunction]
#[pyo3(signature = (pkg_name, version=None, paths=None))]
pub fn remove_package(
    pkg_name: &str,
    version: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<usize> {
    use rez_next_package::package_remove::{
        remove_package_family, remove_package_version, PackageRemoveConfig,
    };

    let search_paths = resolve_search_paths(&paths);

    let config = PackageRemoveConfig {
        packages_path: search_paths,
        force: true,
        prune_empty_families: true,
    };

    if let Some(ver) = version {
        let result = remove_package_version(pkg_name, ver, &config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(result.versions_removed)
    } else {
        let result = remove_package_family(pkg_name, &config)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(result.versions_removed)
    }
}
