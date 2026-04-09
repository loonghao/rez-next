//! Python bindings for `rez depends` — reverse dependency query
//!
//! Equivalent to `rez depends <package>`, which finds all packages in the
//! configured repositories that directly or transitively require a given package.

use crate::runtime::get_runtime;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

// ─── Public structs ──────────────────────────────────────────────────────────

/// A single "dependant" entry — a package that depends on the queried package.
#[pyclass(name = "DependsEntry", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyDependsEntry {
    /// Package name of the dependant
    #[pyo3(get)]
    pub name: String,
    /// Version of the dependant
    #[pyo3(get)]
    pub version: String,
    /// The requirement string that makes it depend on the queried package
    #[pyo3(get)]
    pub requirement: String,
    /// "direct" or "transitive"
    #[pyo3(get)]
    pub dependency_type: String,
}

#[pymethods]
impl PyDependsEntry {
    fn __repr__(&self) -> String {
        format!(
            "DependsEntry({}-{}, requires '{}', {})",
            self.name, self.version, self.requirement, self.dependency_type
        )
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        d.set_item("name", &self.name)?;
        d.set_item("version", &self.version)?;
        d.set_item("requirement", &self.requirement)?;
        d.set_item("dependency_type", &self.dependency_type)?;
        Ok(d.into_any().unbind())
    }
}

/// Result of a `rez depends` query.
#[pyclass(name = "DependsResult")]
pub struct PyDependsResult {
    /// The package that was queried
    #[pyo3(get)]
    pub queried_package: String,
    /// All packages that directly depend on the queried package
    #[pyo3(get)]
    pub direct_dependants: Vec<PyDependsEntry>,
    /// All packages that transitively depend on the queried package
    #[pyo3(get)]
    pub transitive_dependants: Vec<PyDependsEntry>,
}

#[pymethods]
impl PyDependsResult {
    /// Total count of all dependants (direct + transitive, deduplicated)
    fn total_count(&self) -> usize {
        let mut seen = std::collections::HashSet::new();
        for e in &self.direct_dependants {
            seen.insert(format!("{}-{}", e.name, e.version));
        }
        for e in &self.transitive_dependants {
            seen.insert(format!("{}-{}", e.name, e.version));
        }
        seen.len()
    }

    /// Return all dependants (direct + transitive) as a flat list.
    fn all_dependants(&self) -> Vec<PyDependsEntry> {
        let mut all = self.direct_dependants.clone();
        all.extend(self.transitive_dependants.clone());
        all
    }

    /// Format as human-readable text (like `rez depends` terminal output).
    fn format(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "Reverse dependencies for '{}':",
            self.queried_package
        ));
        if self.direct_dependants.is_empty() && self.transitive_dependants.is_empty() {
            lines.push("  (no dependants found)".to_string());
        } else {
            if !self.direct_dependants.is_empty() {
                lines.push("  Direct:".to_string());
                let mut sorted = self.direct_dependants.clone();
                sorted.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
                for e in &sorted {
                    lines.push(format!(
                        "    {}-{}  (requires '{}')",
                        e.name, e.version, e.requirement
                    ));
                }
            }
            if !self.transitive_dependants.is_empty() {
                lines.push("  Transitive:".to_string());
                let mut sorted = self.transitive_dependants.clone();
                sorted.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));
                for e in &sorted {
                    lines.push(format!(
                        "    {}-{}  (requires '{}')",
                        e.name, e.version, e.requirement
                    ));
                }
            }
        }
        lines.join("\n")
    }

    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        d.set_item("queried_package", &self.queried_package)?;
        let direct_list = PyList::empty(py);
        for e in &self.direct_dependants {
            direct_list.append(e.clone().into_pyobject(py)?)?;
        }
        d.set_item("direct_dependants", direct_list)?;
        let trans_list = PyList::empty(py);
        for e in &self.transitive_dependants {
            trans_list.append(e.clone().into_pyobject(py)?)?;
        }
        d.set_item("transitive_dependants", trans_list)?;
        Ok(d.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "DependsResult('{}', direct={}, transitive={})",
            self.queried_package,
            self.direct_dependants.len(),
            self.transitive_dependants.len()
        )
    }
}

// ─── Core logic ──────────────────────────────────────────────────────────────

/// Compute reverse dependencies for a package name (and optional version range).
///
/// Scans all packages in `pkg_paths` for their `requires` fields, collecting
/// those that depend on `target_name`.
pub fn compute_depends(
    target_name: &str,
    target_version_range: Option<&str>,
    pkg_paths: &[std::path::PathBuf],
    transitive: bool,
) -> Result<PyDependsResult, String> {
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use rez_next_version::VersionRange;

    let rt = get_runtime();
    let mut repo_manager = RepositoryManager::new();
    for (i, path) in pkg_paths.iter().enumerate() {
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path.clone(),
                format!("repo_{}", i),
            )));
        }
    }

    // Parse the optional target version range for filtering dependants
    let target_range: Option<VersionRange> = target_version_range
        .filter(|s| !s.is_empty())
        .and_then(|s| VersionRange::parse(s).ok());

    // Collect all packages
    let all_packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| e.to_string())?;

    let mut direct_dependants = Vec::new();

    for pkg in &all_packages {
        // Skip the target package itself
        if pkg.name == target_name {
            continue;
        }
        for req_str in &pkg.requires {
            // Check if this requirement refers to target_name
            if let Ok(req) = rez_next_package::PackageRequirement::parse(req_str) {
                if req.name == target_name {
                    // Optionally filter by version range
                    let matches = match (&target_range, &req.version_spec) {
                        (Some(range), Some(spec)) => {
                            // Check if the requirement's version spec overlaps target range
                            VersionRange::parse(spec)
                                .map(|r| r.intersects(range))
                                .unwrap_or(true)
                        }
                        _ => true,
                    };
                    if matches {
                        let ver = pkg
                            .version
                            .as_ref()
                            .map(|v| v.as_str().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        direct_dependants.push(PyDependsEntry {
                            name: pkg.name.clone(),
                            version: ver,
                            requirement: req_str.clone(),
                            dependency_type: "direct".to_string(),
                        });
                        break; // Don't add the same package twice
                    }
                }
            }
        }
    }

    // For transitive: find packages that depend on any direct dependant
    let mut transitive_dependants = Vec::new();
    if transitive {
        let direct_names: std::collections::HashSet<&str> =
            direct_dependants.iter().map(|e| e.name.as_str()).collect();

        for pkg in &all_packages {
            if pkg.name == target_name || direct_names.contains(pkg.name.as_str()) {
                continue;
            }
            for req_str in &pkg.requires {
                if let Ok(req) = rez_next_package::PackageRequirement::parse(req_str) {
                    if direct_names.contains(req.name.as_str()) {
                        let ver = pkg
                            .version
                            .as_ref()
                            .map(|v| v.as_str().to_string())
                            .unwrap_or_else(|| "unknown".to_string());
                        transitive_dependants.push(PyDependsEntry {
                            name: pkg.name.clone(),
                            version: ver,
                            requirement: req_str.clone(),
                            dependency_type: "transitive".to_string(),
                        });
                        break;
                    }
                }
            }
        }
    }

    Ok(PyDependsResult {
        queried_package: target_name.to_string(),
        direct_dependants,
        transitive_dependants,
    })
}

// ─── Python functions ─────────────────────────────────────────────────────────

/// Find all packages that depend on a given package.
///
/// Parameters
/// ----------
/// package_name : str
///     Name of the package to query reverse dependencies for.
/// version_range : str, optional
///     Version range filter (e.g. ">=3.9"). Only packages requiring a matching
///     version are returned.
/// paths : list[str], optional
///     Repository paths. Defaults to configured packages_path.
/// transitive : bool, optional
///     Also return packages that transitively depend on the queried package.
///     Defaults to False.
///
/// Returns
/// -------
/// DependsResult
///     Object with direct_dependants and (if transitive=True) transitive_dependants.
///
/// This is the Python-API equivalent of `rez depends <package>`.
#[pyfunction]
#[pyo3(signature = (package_name, version_range=None, paths=None, transitive=false))]
pub fn get_reverse_dependencies(
    package_name: &str,
    version_range: Option<&str>,
    paths: Option<Vec<String>>,
    transitive: bool,
) -> PyResult<PyDependsResult> {
    use crate::package_functions::expand_home;
    use rez_next_common::config::RezCoreConfig;
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let pkg_paths: Vec<PathBuf> = paths
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    compute_depends(package_name, version_range, &pkg_paths, transitive)
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)
}

/// Return a flat list of package name strings that directly depend on the given package.
///
/// Convenience wrapper around `get_reverse_dependencies` for simple use cases.
#[pyfunction]
#[pyo3(signature = (package_name, version_range=None, paths=None))]
pub fn get_dependants(
    package_name: &str,
    version_range: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let result = get_reverse_dependencies(package_name, version_range, paths, false)?;
    let mut names: Vec<String> = result
        .direct_dependants
        .iter()
        .map(|e| format!("{}-{}", e.name, e.version))
        .collect();
    names.sort();
    names.dedup();
    Ok(names)
}

/// Print reverse dependency information to a string (like `rez depends` CLI output).
#[pyfunction]
#[pyo3(signature = (package_name, version_range=None, paths=None, transitive=false))]
pub fn print_depends(
    package_name: &str,
    version_range: Option<&str>,
    paths: Option<Vec<String>>,
    transitive: bool,
) -> PyResult<String> {
    let result = get_reverse_dependencies(package_name, version_range, paths, transitive)?;
    Ok(result.format())
}

// ─── Rust unit tests (in separate file to keep this file ≤ 1000 lines) ───────

#[cfg(test)]
#[path = "depends_bindings_tests.rs"]
mod depends_bindings_tests;

#[cfg(test)]
#[path = "depends_bindings_advanced_tests.rs"]
mod depends_bindings_advanced_tests;
