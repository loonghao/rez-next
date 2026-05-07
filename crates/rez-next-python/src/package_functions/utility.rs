//! Utility functions for package operations exposed to Python.
//!
//! Covers: dump_package_data, get_developer_package, get_completions,
//! package_schema, variant_schema, package_family_schema, schema_keys,
//! package_release_keys, get_last_release_time, test_function.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PySet};

use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

use super::make_repo_manager;

/// Dump package data to a dictionary.
/// Equivalent to `rez.packages.dump_package_data(package)`.
#[pyfunction]
#[pyo3(signature = (package))]
pub fn dump_package_data<'py>(py: Python<'py>, package: &PyPackage) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    let pkg = &package.0;

    dict.set_item("name", pkg.name.clone())?;

    if let Some(ref v) = pkg.version {
        dict.set_item("version", v.as_str())?;
    }

    if let Some(ref desc) = pkg.description {
        dict.set_item("description", desc.clone())?;
    }

    dict.set_item("authors", pkg.authors.clone())?;
    dict.set_item("requires", pkg.requires.clone())?;
    dict.set_item("build_requires", pkg.build_requires.clone())?;
    dict.set_item("private_build_requires", pkg.private_build_requires.clone())?;
    dict.set_item("variants", pkg.variants.clone())?;
    dict.set_item("tools", pkg.tools.clone())?;

    if let Some(ref cmd) = pkg.commands {
        dict.set_item("commands", cmd.clone())?;
    }

    if let Some(ref cmd) = pkg.pre_commands {
        dict.set_item("pre_commands", cmd.clone())?;
    }

    if let Some(ref cmd) = pkg.post_commands {
        dict.set_item("post_commands", cmd.clone())?;
    }

    if let Some(ref cmd) = pkg.pre_test_commands {
        dict.set_item("pre_test_commands", cmd.clone())?;
    }

    if let Some(ref cmd) = pkg.pre_build_commands {
        dict.set_item("pre_build_commands", cmd.clone())?;
    }

    if let Some(ref cmd) = pkg.build_command {
        dict.set_item("build_command", cmd.clone())?;
    }

    if let Some(ref bs) = pkg.build_system {
        dict.set_item("build_system", bs.clone())?;
    }

    if let Some(ref uuid) = pkg.uuid {
        dict.set_item("uuid", uuid.clone())?;
    }

    if let Some(ts) = pkg.timestamp {
        dict.set_item("timestamp", ts)?;
    }

    if let Some(cachable) = pkg.cachable {
        dict.set_item("cachable", cachable)?;
    }

    if let Some(relocatable) = pkg.relocatable {
        dict.set_item("relocatable", relocatable)?;
    }

    if let Some(is_dev) = pkg.is_dev_package {
        dict.set_item("is_dev_package", is_dev)?;
    }

    if let Some(ref vcs) = pkg.vcs {
        dict.set_item("vcs", vcs.clone())?;
    }

    if let Some(ref changelog) = pkg.changelog {
        dict.set_item("changelog", changelog.clone())?;
    }

    if let Some(ref msg) = pkg.release_message {
        dict.set_item("release_message", msg.clone())?;
    }

    if let Some(ref rev) = pkg.revision {
        dict.set_item("revision", rev.clone())?;
    }

    if let Some(hv) = pkg.hashed_variants {
        dict.set_item("hashed_variants", hv)?;
    }

    if let Some(ref pre) = pkg.preprocess {
        dict.set_item("preprocess", pre.clone())?;
    }

    // plugin_for is Vec<String>
    if !pkg.plugin_for.is_empty() {
        dict.set_item("plugin_for", pkg.plugin_for.clone())?;
    }

    if let Some(ref rv) = pkg.requires_rez_version {
        dict.set_item("requires_rez_version", rv.clone())?;
    }

    if let Some(fv) = pkg.format_version {
        dict.set_item("format_version", fv)?;
    }

    // Add tests dict
    if !pkg.tests.is_empty() {
        let tests_dict = PyDict::new(py);
        for (k, v) in &pkg.tests {
            tests_dict.set_item(k.clone(), v.clone())?;
        }
        dict.set_item("tests", tests_dict)?;
    }

    // Add config dict
    if !pkg.config.is_empty() {
        let config_dict = PyDict::new(py);
        for (k, v) in &pkg.config {
            config_dict.set_item(k.clone(), v.clone())?;
        }
        dict.set_item("config", config_dict)?;
    }

    Ok(dict)
}

/// Load a package from a developer sandbox directory.
///
/// Equivalent to `rez.packages.get_developer_package(path, paths=None)`.
/// Looks for `package.py` or `package.yaml` in the given directory,
/// loads the package, and marks it as a developer package.
#[pyfunction]
#[pyo3(signature = (path, _paths=None))]
pub fn get_developer_package(
    path: &str,
    _paths: Option<Vec<String>>,
) -> PyResult<PyPackage> {
    use rez_next_package::serialization::PackageSerializer;
    use std::path::Path;

    let dir = Path::new(path);

    if !dir.is_dir() {
        return Err(pyo3::exceptions::PyNotADirectoryError::new_err(format!(
            "Not a directory: {}",
            path
        )));
    }

    // Look for package.py first, then package.yaml
    let package_py = dir.join("package.py");
    let package_yaml = dir.join("package.yaml");
    let package_json = dir.join("package.json");

    let file_path = if package_py.exists() {
        package_py
    } else if package_yaml.exists() {
        package_yaml
    } else if package_json.exists() {
        package_json
    } else {
        return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
            "No package.py, package.yaml, or package.json found in: {}",
            path
        )));
    };

    // Load the package from file
    let mut pkg = PackageSerializer::load_from_file(&file_path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    // Mark as developer package
    pkg.is_dev_package = Some(true);

    Ok(PyPackage(pkg))
}

/// Get tab-completion candidates for a partial package name.
///
/// Equivalent to `rez.packages.get_completions(prefix, paths=None)`.
/// Returns a list of package family names that start with `prefix`.
#[pyfunction]
#[pyo3(signature = (prefix, paths=None))]
pub fn get_completions(prefix: &str, paths: Option<Vec<String>>) -> PyResult<Vec<String>> {
    let rt = get_runtime();
    let repo_manager = make_repo_manager(paths);

    // Get all family names
    let family_names = rt
        .block_on(repo_manager.list_packages())
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Filter by prefix (case-insensitive)
    let prefix_lower = prefix.to_lowercase();
    let mut matches: Vec<String> = family_names
        .into_iter()
        .filter(|name| name.to_lowercase().starts_with(&prefix_lower))
        .collect();

    // Sort alphabetically
    matches.sort();

    Ok(matches)
}

/// Return the set of allowed keys in a package definition.
///
/// Equivalent to `rez.packages.package_schema()` (simplified – returns the
/// key set directly, no voluptuous Schema object).
#[pyfunction]
pub fn package_schema(py: Python<'_>) -> PyResult<Py<PySet>> {
    let set = PySet::empty(py)?;

    let keys = [
        "name",
        "version",
        "description",
        "authors",
        "requires",
        "build_requires",
        "private_build_requires",
        "variants",
        "tools",
        "commands",
        "pre_commands",
        "post_commands",
        "pre_test_commands",
        "pre_build_commands",
        "tests",
        "requires_rez_version",
        "uuid",
        "config",
        "help",
        "relocatable",
        "cachable",
        "timestamp",
        "revision",
        "changelog",
        "release_message",
        "previous_version",
        "previous_revision",
        "vcs",
        "format_version",
        "base",
        "has_plugins",
        "plugin_for",
        "hashed_variants",
        "preprocess",
        "is_dev_package",
    ];

    for key in &keys {
        set.add(*key)?;
    }

    Ok(set.unbind())
}

/// Return the set of allowed keys in a variant definition.
///
/// Equivalent to `rez.packages.variant_schema()` (simplified).
#[pyfunction]
pub fn variant_schema(py: Python<'_>) -> PyResult<Py<PySet>> {
    // Variant schema is the same as package schema in rez
    package_schema(py)
}

/// Return the set of allowed keys in a package family.
///
/// Equivalent to `rez.packages.package_family_schema()` (simplified).
#[pyfunction]
pub fn package_family_schema(py: Python<'_>) -> PyResult<Py<PySet>> {
    let set = PySet::empty(py)?;
    set.add("name")?;
    Ok(set.unbind())
}

/// Extract string keys from a dict-based schema.
///
/// Equivalent to `rez.packages.schema_keys(schema)`.
/// Accepts a Python dict and returns a set of string keys.
#[pyfunction]
pub fn schema_keys(py: Python<'_>, schema: Bound<'_, PyDict>) -> PyResult<Py<PySet>> {
    let set = PySet::empty(py)?;

    for (key, _) in schema.iter() {
        if let Ok(key_str) = key.extract::<String>() {
            set.add(key_str)?;
        }
    }

    Ok(set.unbind())
}

/// Return the set of keys that trigger a new release when changed.
///
/// Equivalent to `rez.packages.package_release_keys()`.
#[pyfunction]
pub fn package_release_keys(py: Python<'_>) -> PyResult<Py<PySet>> {
    let set = PySet::empty(py)?;

    // These are the keys that, when changed, trigger a new release in rez
    let keys = [
        "version",
        "requires",
        "build_requires",
        "private_build_requires",
        "variants",
        "tools",
        "commands",
        "pre_commands",
        "post_commands",
        "tests",
        "uuid",
        "config",
        "help",
        "relocatable",
        "cachable",
        "vcs",
        "format_version",
        "base",
        "has_plugins",
        "plugin_for",
        "hashed_variants",
        "preprocess",
    ];

    for key in &keys {
        set.add(*key)?;
    }

    Ok(set.unbind())
}

/// Get the timestamp of the most recent release of a package.
///
/// Equivalent to `rez.packages_.get_last_release_time(name, paths=None)`.
/// Returns a Python `datetime` object, or `None` if no release has a timestamp.
#[pyfunction]
#[pyo3(signature = (name, paths=None))]
pub fn get_last_release_time<'py>(
    py: Python<'py>,
    name: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let rt = get_runtime();
    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Find the package with the latest timestamp
    let latest: Option<i64> = packages
        .iter()
        .filter_map(|p| p.timestamp)
        .max();

    match latest {
        Some(ts) => {
            // Convert Unix timestamp to Python datetime
            let datetime = py.import("datetime")?
                .getattr("datetime")?
                .call_method("fromtimestamp", (ts,), None)?;
            // Convert Bound<'py, PyDateTime> to Py<PyAny>
            let obj: Py<PyAny> = datetime.unbind();
            Ok(obj)
        }
        None => {
            // Return Python None (Py<PyNone> converts to Py<PyAny> via into_any)
            let none: Py<PyAny> = py.None().into_any();
            Ok(none)
        }
    }
}

/// Test function to verify registration mechanism.
#[pyfunction]
#[pyo3(signature = ())]
pub fn test_function() -> PyResult<String> {
    Ok("test_function works!".to_string())
}
