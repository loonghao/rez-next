//! Python bindings for package_search module.
//!
//! This module provides Python-compatible interfaces for package search
//! functionality, aligning with Rez's `package_search.py` interface.

use pyo3::prelude::*;
use std::collections::HashMap;

use rez_next_repository::package_search::{
    get_plugins, get_reverse_dependency_tree, ResourceSearchResult,
};

/// Python wrapper for ResourceSearchResult.
#[pyclass(name = "ResourceSearchResult")]
#[derive(Clone)]
pub struct PyResourceSearchResult {
    inner: ResourceSearchResult,
}

#[pymethods]
impl PyResourceSearchResult {
    /// Create a new ResourceSearchResult.
    #[new]
    fn new(resource: String, resource_type: String) -> Self {
        Self {
            inner: ResourceSearchResult::new(resource, resource_type),
        }
    }

    /// Get the resource name.
    #[getter]
    fn resource(&self) -> String {
        self.inner.resource.clone()
    }

    /// Get the resource type.
    #[getter]
    fn resource_type(&self) -> String {
        self.inner.resource_type.clone()
    }

    /// Get the validation error.
    #[getter]
    fn validation_error(&self) -> Option<String> {
        self.inner.validation_error.clone()
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "ResourceSearchResult(resource='{}', type='{}')",
            self.inner.resource, self.inner.resource_type
        )
    }
}

/// Get plugins for a package.
///
/// Args:
///     package_name: Name of the package to find plugins for
///     paths: Optional list of repository paths to search
///
/// Returns:
///     List of plugin package names.
#[pyfunction]
#[pyo3(name = "get_plugins")]
pub fn py_get_plugins(
    package_name: String,
    paths: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let result = get_plugins(&package_name, paths);
    Ok(result)
}

/// Get reverse dependency tree for a package.
///
/// Args:
///     package_name: The package to find reverse dependencies for
///     depth: Maximum depth to search (None for unlimited)
///     paths: Optional list of repository paths to search
///     build_requires: Whether to include build_requires
///     private_build_requires: Whether to include private_build_requires
///
/// Returns:
///     Tuple of (layers, graph) where:
///     - layers: List of lists of package names grouped by depth
///     - graph: Dict mapping package names to list of dependent packages
#[pyfunction]
#[pyo3(name = "get_reverse_dependency_tree")]
pub fn py_get_reverse_dependency_tree(
    package_name: String,
    depth: Option<usize>,
    paths: Option<Vec<String>>,
    build_requires: bool,
    private_build_requires: bool,
) -> PyResult<(Vec<Vec<String>>, HashMap<String, Vec<String>>)> {
    let (layers, graph) = get_reverse_dependency_tree(
        &package_name,
        depth,
        paths,
        build_requires,
        private_build_requires,
    );
    Ok((layers, graph))
}

/// Setup the package_search submodule by adding classes and functions.
/// This should be called with the submodule (not the parent module).
pub fn setup_package_search_module(submodule: &Bound<'_, PyModule>) -> PyResult<()> {
    // Add classes
    submodule.add_class::<PyResourceSearchResult>()?;

    // Add functions
    submodule.add_function(wrap_pyfunction!(py_get_plugins, submodule)?)?;
    submodule.add_function(wrap_pyfunction!(
        py_get_reverse_dependency_tree,
        submodule
    )?)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_resource_search_result_creation() {
        let result = PyResourceSearchResult::new("python".to_string(), "family".to_string());
        assert_eq!(result.resource(), "python");
        assert_eq!(result.resource_type(), "family");
        assert!(result.validation_error().is_none());
    }
}
