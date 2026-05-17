//! Package query functions exposed to Python.
//!
//! Covers: get_latest_package, get_package, resolve_packages, iter_packages,
//! get_package_family_names, walk_packages, iter_package_families,
//! get_package_from_string, get_latest_package_from_string.

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::context_bindings::PyResolvedContext;
use crate::env_bindings::PyPackageFamily;
use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

use super::make_repo_manager;

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
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    // Use list_packages to get all family names
    let names = rt
        .block_on(repo_manager.list_packages())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    Ok(names)
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

    // Get all family names first
    let family_names = rt
        .block_on(repo_manager.list_packages())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Group by family name with all versions
    let mut families: HashMap<String, Vec<String>> = HashMap::new();

    for name in &family_names {
        let packages = rt
            .block_on(repo_manager.find_packages(name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let versions: Vec<String> = packages
            .iter()
            .map(|p| {
                p.version
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            })
            .collect();
        families.insert(name.clone(), versions);
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

/// Iterate over package families.
/// Equivalent to `rez.packages.iter_package_families(paths=None)`
/// Returns a list of `PackageFamily` objects.
#[pyfunction]
#[pyo3(signature = (paths=None))]
pub fn iter_package_families(paths: Option<Vec<String>>) -> PyResult<Vec<PyPackageFamily>> {
    use std::collections::HashMap;

    let rt = get_runtime();
    let repo_manager = make_repo_manager(paths);

    // Get all family names
    let family_names = rt
        .block_on(repo_manager.list_packages())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Group packages by family name
    let mut families: HashMap<String, Vec<PyPackage>> = HashMap::new();

    for name in &family_names {
        let packages = rt
            .block_on(repo_manager.find_packages(name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let py_packages: Vec<PyPackage> =
            packages.iter().map(|p| PyPackage((**p).clone())).collect();

        families.insert(name.clone(), py_packages);
    }

    // Convert to PyPackageFamily objects
    let mut result: Vec<PyPackageFamily> = families
        .into_iter()
        .map(|(name, packages)| {
            let mut family = PyPackageFamily::new(name);
            for pkg in packages {
                family.add_package(pkg);
            }
            family
        })
        .collect();

    // Sort by family name
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(result)
}

/// Parse a string like "python-3.9.0" into a `Package` object.
///
/// Equivalent to `rez.packages_.get_package_from_string(s)`.
/// The heuristic splits on the **last** `'-'`; the suffix is parsed as a
/// `Version` and stored in `Package.version`.
#[pyfunction]
#[pyo3(signature = (package_string))]
pub fn get_package_from_string(package_string: &str) -> PyResult<PyPackage> {
    use rez_next_package::Package;

    // Split on last '-':  "python-3.9.0" → name="python", version_str="3.9.0"
    if let Some(pos) = package_string.rfind('-') {
        let (name, version_str) = package_string.split_at(pos);
        let version_str = &version_str[1..]; // drop the '-'

        let mut pkg = Package::new(name.to_string());

        // Attempt to parse the version suffix; leave None if it's not a valid version
        if let Ok(version) = rez_next_version::Version::parse(version_str) {
            pkg.version = Some(version);
        }

        Ok(PyPackage(pkg))
    } else {
        // No '-' found — treat the entire string as the package name
        let pkg = Package::new(package_string.to_string());
        Ok(PyPackage(pkg))
    }
}

/// Get the latest package matching a request string like "python-3.9".
///
/// Equivalent to `rez.packages.get_latest_package_from_string(request_string, paths=None)`.
#[pyfunction]
#[pyo3(signature = (request_string, paths=None))]
pub fn get_latest_package_from_string(
    request_string: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    // Parse request string: "python-3.9" → name="python", range_str="3.9"
    if let Some(pos) = request_string.rfind('-') {
        let (name, version_str) = request_string.split_at(pos);
        let version_str = &version_str[1..]; // drop the '-'

        // Use version string as range (e.g., "3.9" matches >=3.9,<3.10)
        let range_str = if version_str.is_empty() {
            None
        } else {
            Some(version_str)
        };

        get_latest_package(name, range_str, paths)
    } else {
        // No '-' found — treat entire string as package name
        get_latest_package(request_string, None, paths)
    }
}
