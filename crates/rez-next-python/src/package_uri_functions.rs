//! Package URI functions exposed to Python.
//!
//! Implements: get_package_from_uri, get_variant_from_uri

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use std::path::Path;

use crate::package_bindings::PyPackage;
use rez_next_package::serialization::PackageSerializer;

/// Get a package given its URI.
///
/// Equivalent to `rez.packages.get_package_from_uri(uri, paths=None)`.
///
/// The URI is typically a path to a package file:
/// - `/{pkg-repo-path}/{pkg-name}/{pkg-version}/package.py`
/// - `/{pkg-repo-path}/{pkg-name}/package.py` (unversioned)
/// - `/{pkg-repo-path}/{pkg-name}/package.py<{version}>` (combined-type)
///
/// Returns a `Package` object, or `None` if not found.
#[pyfunction]
#[pyo3(signature = (uri, paths=None))]
#[allow(unused_variables)]
pub fn get_package_from_uri(
    py: Python<'_>,
    uri: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    use crate::package_functions::make_repo_manager;

    let rt = crate::runtime::get_runtime();
    let repo_manager = make_repo_manager(paths.clone());

    // Strategy 1: Try to find package by parsing URI as a file path
    let path = Path::new(uri);

    // Check if URI points to a package file
    if path.is_file() || uri.ends_with(".py") || uri.ends_with(".yaml") || uri.ends_with(".json") {
        // Direct file path - load package from file
        if let Ok(pkg) = PackageSerializer::load_from_file(path) {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    // Strategy 2: Try to extract package name and version from URI
    // Typical format: /{repo-path}/{pkg-name}/{version}/package.py
    let uri_path = uri.trim_start_matches('/');
    let parts: Vec<&str> = uri_path.split('/').collect();

    if parts.len() >= 3 {
        // Assume format: {repo-path}/{pkg-name}/{version}/package.py
        let pkg_name = parts[parts.len() - 3];
        let version_str = parts[parts.len() - 2];

        // Try to get package by name and version
        if let Ok(Some(pkg)) =
            get_package_by_name_and_version(pkg_name, version_str, &repo_manager, rt)
        {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    if parts.len() >= 2 {
        // Assume unversioned: {repo-path}/{pkg-name}/package.py
        let pkg_name = parts[parts.len() - 2];

        // Try to get latest package
        if let Ok(Some(pkg)) = get_latest_package_by_name(pkg_name, &repo_manager, rt) {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    // Strategy 3: If paths is provided, search in those paths
    if paths.is_some() {
        return Ok(None);
    }

    // Strategy 4: Try to infer repository path from URI
    // This mimics rez's behavior of trying to find the package
    if parts.len() >= 3 && !uri.contains('<') {
        // Try to extract repo path (everything except last 3 components)
        let repo_path = parts[..parts.len() - 3].join("/");
        if let Ok(Some(pkg)) = find_package_in_path(&repo_path, &parts[parts.len() - 3..]) {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    // Try unversioned or combined-type package
    if parts.len() >= 2 {
        let repo_path = parts[..parts.len() - 2].join("/");
        if let Ok(Some(pkg)) = find_package_in_path(&repo_path, &parts[parts.len() - 2..]) {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    Ok(None)
}

/// Helper: Get package by name and version from repository manager.
fn get_package_by_name_and_version(
    name: &str,
    version: &str,
    repo_manager: &rez_next_repository::simple_repository::RepositoryManager,
    rt: &tokio::runtime::Runtime,
) -> Result<Option<rez_next_package::Package>, String> {
    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| e.to_string())?;

    let pkg = packages
        .into_iter()
        .find(|p| p.version.as_ref().is_some_and(|v| v.as_str() == version));

    Ok(pkg.map(|p| (*p).clone()))
}

/// Helper: Get latest package by name from repository manager.
fn get_latest_package_by_name(
    name: &str,
    repo_manager: &rez_next_repository::simple_repository::RepositoryManager,
    rt: &tokio::runtime::Runtime,
) -> Result<Option<rez_next_package::Package>, String> {
    let mut packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| e.to_string())?;

    // Sort by version descending
    packages.sort_by(|a, b| match (&a.version, &b.version) {
        (Some(v1), Some(v2)) => v2.cmp(v1),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    Ok(packages.into_iter().next().map(|p| (*p).clone()))
}

/// Helper: Find package in a specific repository path.
fn find_package_in_path(
    repo_path: &str,
    remaining_parts: &[&str],
) -> Result<Option<rez_next_package::Package>, String> {
    use std::path::PathBuf;

    let mut path = PathBuf::from(repo_path);
    for part in remaining_parts {
        path.push(part);
    }

    // Try to find package file
    for filename in &["package.py", "package.yaml", "package.json"] {
        let file_path = path.join(filename);
        if file_path.exists() {
            if let Ok(pkg) = PackageSerializer::load_from_file(&file_path) {
                return Ok(Some(pkg));
            }
        }
    }

    // If path itself is a package file
    if path.exists()
        && (path.ends_with("package.py")
            || path.ends_with("package.yaml")
            || path.ends_with("package.json"))
    {
        if let Ok(pkg) = PackageSerializer::load_from_file(&path) {
            return Ok(Some(pkg));
        }
    }

    Ok(None)
}

/// Get a variant given its URI.
///
/// Equivalent to `rez.packages.get_variant_from_uri(uri, paths=None)`.
///
/// Returns a `Variant` object, or `None` if not found.
///
/// Note: This is a simplified implementation. Full variant support
/// requires additional work on the variant system.
#[pyfunction]
#[pyo3(signature = (uri, paths=None))]
#[allow(unused_variables)]
pub fn get_variant_from_uri(
    py: Python<'_>,
    uri: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Option<Py<PyAny>>> {
    // For now, return None as variant support is not fully implemented
    // TODO: Implement full variant support
    Ok(None)
}

/// Get a variant given its package URI and variant index.
///
/// Equivalent to `rez.packages.get_variant(uri, index=None, paths=None)`.
#[pyfunction]
#[pyo3(signature = (uri, index=None, paths=None))]
#[allow(unused_variables)]
pub fn get_variant(
    py: Python<'_>,
    uri: &str,
    index: Option<usize>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<Py<PyAny>>> {
    // For now, return None as variant support is not fully implemented
    // TODO: Implement full variant support
    Ok(None)
}

/// Get a package from a repository handle.
///
/// Equivalent to `rez.packages.get_package_from_handle(handle, paths=None)`.
///
/// The handle is repository-specific:
/// - For file-based repositories, it's typically the path to the package file.
/// - For other repositories, it may be a tuple or other identifier.
///
/// This implementation supports:
/// - String handles (interpreted as file paths)
/// - Tuple handles (repo_name, relative_path) - extracts path and loads package
#[pyfunction]
#[pyo3(signature = (handle, paths=None))]
pub fn get_package_from_handle(
    py: Python<'_>,
    handle: Py<PyAny>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    use std::path::Path;

    // Try to extract a file path from the handle
    let path_str = if let Ok(s) = handle.extract::<String>(py) {
        // Handle is a string - treat as file path
        s
    } else if let Ok(tuple) = handle.bind(py).cast::<pyo3::types::PyTuple>() {
        // Handle is a tuple - try to extract path from it
        // Format: (repo_name, relative_path) or similar
        if tuple.len() > 1 {
            if let Ok(path) = tuple.get_item(1)?.extract::<String>() {
                path
            } else {
                return Ok(None);
            }
        } else if tuple.len() == 1 {
            if let Ok(path) = tuple.get_item(0)?.extract::<String>() {
                path
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }
    } else {
        // Unsupported handle type
        return Ok(None);
    };

    // Try to load package from the path
    let path = Path::new(&path_str);

    // Check if it's a direct package file
    if path.exists()
        && (path.ends_with("package.py")
            || path.ends_with("package.yaml")
            || path.ends_with("package.json"))
    {
        if let Ok(pkg) = PackageSerializer::load_from_file(path) {
            return Ok(Some(PyPackage(pkg)));
        }
    }

    // If not a direct file, try to find package files in the directory
    if path.is_dir() {
        for filename in &["package.py", "package.yaml", "package.json"] {
            let file_path = path.join(filename);
            if file_path.exists() {
                if let Ok(pkg) = PackageSerializer::load_from_file(&file_path) {
                    return Ok(Some(PyPackage(pkg)));
                }
            }
        }
    }

    // If paths is provided, also search in those repositories
    if let Some(ref search_paths) = paths {
        for search_path in search_paths {
            let full_path = Path::new(search_path).join(&path_str);
            if full_path.exists() {
                if let Ok(pkg) = PackageSerializer::load_from_file(&full_path) {
                    return Ok(Some(PyPackage(pkg)));
                }
            }

            // Also check for package files in directory
            if full_path.is_dir() {
                for filename in &["package.py", "package.yaml", "package.json"] {
                    let file_path = full_path.join(filename);
                    if file_path.exists() {
                        if let Ok(pkg) = PackageSerializer::load_from_file(&file_path) {
                            return Ok(Some(PyPackage(pkg)));
                        }
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Get a package from a specific repository.
///
/// Equivalent to `rez.packages.get_package_from_repository(uri, repo_path)`.
#[pyfunction]
#[pyo3(signature = (_uri, repo_path))]
pub fn get_package_from_repository(
    _py: Python<'_>,
    _uri: &str,
    repo_path: &str,
) -> PyResult<Option<PyPackage>> {
    use std::path::Path;

    // Try to load package from the specific repository path
    let pkg_path = Path::new(repo_path);
    if !pkg_path.exists() {
        return Ok(None);
    }

    // Try to find package file in repo path
    for filename in &["package.py", "package.yaml", "package.json"] {
        let file_path = pkg_path.join(filename);
        if file_path.exists() {
            if let Ok(pkg) = PackageSerializer::load_from_file(&file_path) {
                return Ok(Some(PyPackage(pkg)));
            }
        }
    }

    // Search recursively
    if let Ok(entries) = std::fs::read_dir(pkg_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recursively search
                if let Ok(Some(pkg)) = find_package_in_path(&path.to_string_lossy(), &[]) {
                    return Ok(Some(PyPackage(pkg)));
                }
            }
        }
    }

    Ok(None)
}

/// Get package family from repository.
///
/// Equivalent to `rez.packages.get_package_family_from_repository(name, repo_path)`.
#[pyfunction]
#[pyo3(signature = (name, repo_path))]
pub fn get_package_family_from_repository(
    py: Python<'_>,
    name: &str,
    repo_path: &str,
) -> PyResult<Option<Py<PyAny>>> {
    use std::path::Path;

    let family_path = Path::new(repo_path).join(name);
    if !family_path.exists() {
        return Ok(None);
    }

    // Return a dict-like object representing the family
    let dict = PyDict::new(py);
    dict.set_item("name", name)?;

    // List versions
    if let Ok(entries) = std::fs::read_dir(&family_path) {
        let mut versions = Vec::new();
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(dir_name) = entry.file_name().to_str() {
                    versions.push(dir_name.to_string());
                }
            }
        }
        dict.set_item("versions", versions)?;
    }

    Ok(Some(dict.into_any().unbind()))
}

#[cfg(test)]
#[path = "package_uri_functions_tests.rs"]
mod tests;
