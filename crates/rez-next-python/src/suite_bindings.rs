//! Python bindings for rez-next-suites
//!
//! Provides `rez_next.suite.Suite` — drop-in replacement for `rez.suite.Suite`.
//!
//! ## Usage
//! ```python
//! from rez_next.suite import Suite
//! # or as rez drop-in:
//! from rez_next import suite
//! s = suite.Suite()
//! s.add_context("maya", ["maya-2023", "python-3.9"])
//! s.save("/path/to/my_suite")
//! ```

use pyo3::prelude::*;
use rez_next_suites::{Suite, SuiteManager, ToolConflictMode};
use std::path::PathBuf;

/// Python wrapper for Suite
#[pyclass(name = "Suite")]
pub struct PySuite {
    inner: Suite,
}

#[pymethods]
impl PySuite {
    /// Create a new empty suite
    #[new]
    #[pyo3(signature = (description=None))]
    fn new(description: Option<&str>) -> Self {
        let mut suite = Suite::new();
        if let Some(d) = description {
            suite = suite.with_description(d);
        }
        PySuite { inner: suite }
    }

    /// Add a context to the suite
    ///
    /// Args:
    ///     name: Unique name for the context
    ///     requests: List of package requirement strings
    fn add_context(&mut self, name: &str, requests: Vec<String>) -> PyResult<()> {
        self.inner
            .add_context(name, requests)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Remove a context from the suite
    fn remove_context(&mut self, name: &str) -> PyResult<()> {
        self.inner
            .remove_context(name)
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
    }

    /// Get a list of context names
    fn context_names(&self) -> Vec<String> {
        self.inner
            .context_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Get the number of contexts
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Alias a tool in a context
    ///
    /// Args:
    ///     context_name: Name of the context
    ///     alias: New alias name
    ///     tool: Original tool name
    fn alias_tool(&mut self, context_name: &str, alias: &str, tool: &str) -> PyResult<()> {
        self.inner
            .alias_tool(context_name, alias, tool)
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
    }

    /// Hide a tool in a context
    fn hide_tool(&mut self, context_name: &str, tool: &str) -> PyResult<()> {
        self.inner
            .hide_tool(context_name, tool)
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
    }

    /// Save the suite to a directory
    fn save(&mut self, path: &str) -> PyResult<()> {
        self.inner
            .save(PathBuf::from(path))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Load a suite from a directory
    #[staticmethod]
    fn load(path: &str) -> PyResult<PySuite> {
        Suite::load(PathBuf::from(path))
            .map(|inner| PySuite { inner })
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Check if a path is a suite directory
    #[staticmethod]
    fn is_suite(path: &str) -> bool {
        Suite::is_suite(PathBuf::from(path))
    }

    /// Print suite information
    fn print_info(&self) {
        self.inner.print_info();
    }

    /// Get the suite description
    #[getter]
    fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// Set the suite description
    #[setter]
    fn set_description(&mut self, description: Option<String>) {
        self.inner.description = description;
    }

    /// Get the conflict mode
    #[getter]
    fn conflict_mode(&self) -> String {
        format!("{:?}", self.inner.conflict_mode).to_lowercase()
    }

    /// Set the conflict mode
    #[setter]
    fn set_conflict_mode(&mut self, mode: &str) -> PyResult<()> {
        self.inner.conflict_mode = mode
            .parse::<ToolConflictMode>()
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(())
    }

    /// Get suite path
    #[getter]
    fn path(&self) -> Option<String> {
        self.inner
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
    }

    /// Get tools exposed by the suite as a dict
    fn get_tools(&self, py: Python) -> PyResult<Py<PyAny>> {
        let tools = self
            .inner
            .get_tools()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let dict = pyo3::types::PyDict::new(py);
        for (name, tool) in tools {
            let tool_dict = pyo3::types::PyDict::new(py);
            tool_dict.set_item("name", &tool.name)?;
            tool_dict.set_item("original_name", &tool.original_name)?;
            tool_dict.set_item("context_name", &tool.context_name)?;
            tool_dict.set_item("package", &tool.package)?;
            tool_dict.set_item("is_alias", tool.is_alias)?;
            dict.set_item(name, tool_dict)?;
        }
        Ok(dict.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "Suite(contexts={}, path={:?})",
            self.inner.len(),
            self.inner.path
        )
    }
}

/// Python wrapper for SuiteManager
#[pyclass(name = "SuiteManager")]
pub struct PySuiteManager {
    inner: SuiteManager,
}

#[pymethods]
impl PySuiteManager {
    /// Create a new suite manager
    #[new]
    #[pyo3(signature = (paths=None))]
    fn new(paths: Option<Vec<String>>) -> Self {
        let manager = match paths {
            Some(p) => SuiteManager::with_paths(p.into_iter().map(PathBuf::from).collect()),
            None => SuiteManager::new(),
        };
        PySuiteManager { inner: manager }
    }

    /// Add a search path
    fn add_path(&mut self, path: &str) {
        self.inner.add_path(PathBuf::from(path));
    }

    /// List all suite names
    fn list_suite_names(&self) -> Vec<String> {
        self.inner.list_suite_names()
    }

    /// Find all suite paths
    fn find_suites(&self) -> Vec<String> {
        self.inner
            .find_suites()
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    }

    /// Load a suite by name
    fn load_suite(&self, name: &str) -> PyResult<PySuite> {
        self.inner
            .load_suite(name)
            .map(|inner| PySuite { inner })
            .map_err(|e| pyo3::exceptions::PyKeyError::new_err(e.to_string()))
    }
}

#[cfg(test)]
#[path = "suite_bindings_tests.rs"]
mod tests;
