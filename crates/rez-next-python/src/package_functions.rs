//! Package query and management functions exposed to Python.
//!
//! Covers: get_latest_package, get_package, resolve_packages, iter_packages,
//! get_package_family_names, walk_packages, copy_package, move_package, remove_package.

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::context_bindings::PyResolvedContext;
use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

/// Expand `~` in path strings.
pub(crate) fn expand_home(p: &str) -> String {
    if p.starts_with("~/") || p == "~" {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return p.replacen("~", &home, 1);
        }
    }
    p.to_string()
}

/// Build a `RepositoryManager` from the provided or configured package paths.
fn make_repo_manager(
    paths: Option<Vec<String>>,
) -> rez_next_repository::simple_repository::RepositoryManager {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let mut repo_manager = RepositoryManager::new();

    let pkg_paths: Vec<PathBuf> = paths
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    for (i, path) in pkg_paths.iter().enumerate() {
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path.clone(),
                format!("repo_{}", i),
            )));
        }
    }

    repo_manager
}

/// Get the latest version of a package from all configured repositories.
/// Equivalent to `rez.packages.get_latest_package(name, range_)`
#[pyfunction]
#[pyo3(signature = (name, range_=None, paths=None))]
pub fn get_latest_package(
    name: &str,
    range_: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Filter by range if specified
    let filtered: Vec<_> = packages
        .into_iter()
        .filter(|pkg| {
            if let Some(range_str) = range_ {
                if let Some(ref version) = pkg.version {
                    if let Ok(range) = rez_next_version::VersionRange::parse(range_str) {
                        return range.contains(version);
                    }
                }
                true
            } else {
                true
            }
        })
        .collect();

    // Return latest (first after sort by version descending)
    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        b.version
            .as_ref()
            .and_then(|bv| a.version.as_ref().map(|av| bv.cmp(av)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(sorted.into_iter().next().map(|p| PyPackage((*p).clone())))
}

/// Get a specific version of a package.
/// Equivalent to `rez.packages.get_package(name, version)`
#[pyfunction]
#[pyo3(signature = (name, version=None, paths=None))]
pub fn get_package(
    name: &str,
    version: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let result = packages.into_iter().find(|pkg| {
        if let Some(ver) = version {
            pkg.version.as_ref().is_some_and(|v| v.as_str() == ver)
        } else {
            true
        }
    });

    Ok(result.map(|p| PyPackage((*p).clone())))
}

/// Resolve a list of package requirements into a ResolvedContext.
/// Equivalent to `rez.resolved_context.ResolvedContext(packages)`
#[pyfunction]
#[pyo3(signature = (packages, paths=None))]
pub fn resolve_packages(
    packages: Vec<String>,
    paths: Option<Vec<String>>,
) -> PyResult<PyResolvedContext> {
    PyResolvedContext::new(packages, paths)
}

/// Iterate over all versions of a package.
/// Equivalent to `rez.packages_.iter_packages(name, range_=None, paths=None)`
/// Returns a Python list of Package objects.
#[pyfunction]
#[pyo3(signature = (name, range_=None, paths=None))]
pub fn iter_packages(
    py: Python,
    name: &str,
    range_: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Filter by range if specified
    let mut filtered: Vec<PyPackage> = packages
        .into_iter()
        .filter(|pkg| {
            if let Some(range_str) = range_ {
                if let Some(ref version) = pkg.version {
                    if let Ok(range) = rez_next_version::VersionRange::parse(range_str) {
                        return range.contains(version);
                    }
                }
                true
            } else {
                true
            }
        })
        .map(|p| PyPackage((*p).clone()))
        .collect();

    // Sort by version ascending (standard rez iter_packages order)
    filtered.sort_by(|a, b| {
        a.0.version
            .as_ref()
            .and_then(|av| b.0.version.as_ref().map(|bv| av.cmp(bv)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Return as Python list
    let list = PyList::new(py, filtered)?;
    Ok(list.into_any().unbind())
}

/// Get all package family names from configured repositories.
/// Equivalent to `rez.packages_.get_package_family_names(paths=None)`
#[pyfunction]
#[pyo3(signature = (paths=None))]
pub fn get_package_family_names(paths: Option<Vec<String>>) -> PyResult<Vec<String>> {
    use std::collections::HashSet;

    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    // Search with empty string to list all packages
    let packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let mut names: HashSet<String> = packages.iter().map(|p| p.name.clone()).collect();
    let mut result: Vec<String> = names.drain().collect();
    result.sort();
    Ok(result)
}

/// Walk all packages across all configured repositories.
/// Equivalent to `rez.packages_.walk_packages(paths=None)`
/// Returns a Python list of (family_name, version_list) tuples.
#[pyfunction]
#[pyo3(signature = (paths=None))]
pub fn walk_packages(py: Python, paths: Option<Vec<String>>) -> PyResult<Py<PyAny>> {
    use std::collections::HashMap;

    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    // Find all packages (empty string matches all)
    let packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Group by family name
    let mut families: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in &packages {
        let ver = pkg
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");
        families
            .entry(pkg.name.clone())
            .or_default()
            .push(ver.to_string());
    }

    // Sort versions within each family
    for versions in families.values_mut() {
        versions.sort();
    }

    // Build result list of (name, versions) tuples
    let result_list = pyo3::types::PyList::empty(py);
    let mut sorted_families: Vec<_> = families.into_iter().collect();
    sorted_families.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, versions) in sorted_families {
        let tuple = pyo3::types::PyTuple::new(
            py,
            [
                name.into_pyobject(py)?.into_any().unbind(),
                pyo3::types::PyList::new(py, versions)?.into_any().unbind(),
            ],
        )?;
        result_list.append(tuple)?;
    }

    Ok(result_list.into_any().unbind())
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

/// Recursively copy a directory tree.
pub(crate) fn copy_dir_recursive(
    src: &std::path::Path,
    dest: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

// ─── Rust unit tests (in separate file to keep this file ≤ 1000 lines) ───────

#[cfg(test)]
#[path = "package_functions_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "package_functions_extra_tests.rs"]
mod extra_tests;

#[cfg(test)]
#[path = "package_functions_version_tests.rs"]
mod version_tests;
