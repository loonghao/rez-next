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
mod tests {
    use super::*;

    mod test_suite_basic {

        use super::*;

        #[test]
        fn test_new_suite_is_empty() {
            let s = PySuite::new(None);
            assert_eq!(s.inner.len(), 0);
            assert!(s.inner.context_names().is_empty());
        }

        #[test]
        fn test_new_suite_with_description() {
            let s = PySuite::new(Some("my suite"));
            assert_eq!(s.inner.description.as_deref(), Some("my suite"));
        }

        #[test]
        fn test_add_context_increases_count() {
            let mut s = PySuite::new(None);
            s.add_context("maya", vec!["maya-2023".to_string()])
                .unwrap();
            assert_eq!(s.inner.len(), 1);
        }

        #[test]
        fn test_add_multiple_contexts() {
            let mut s = PySuite::new(None);
            s.add_context("maya", vec!["maya-2023".to_string()])
                .unwrap();
            s.add_context("nuke", vec!["nuke-14".to_string()]).unwrap();
            assert_eq!(s.inner.len(), 2);
        }

        #[test]
        fn test_context_names_reflects_added_contexts() {
            let mut s = PySuite::new(None);
            s.add_context("ctx1", vec!["pkgA-1".to_string()]).unwrap();
            s.add_context("ctx2", vec!["pkgB-2".to_string()]).unwrap();
            let names = s.context_names();
            assert!(names.contains(&"ctx1".to_string()));
            assert!(names.contains(&"ctx2".to_string()));
        }

        #[test]
        fn test_remove_context_decreases_count() {
            let mut s = PySuite::new(None);
            s.add_context("ctx1", vec!["pkg-1".to_string()]).unwrap();
            s.remove_context("ctx1").unwrap();
            assert_eq!(s.inner.len(), 0);
        }

        #[test]
        fn test_remove_nonexistent_context_returns_err() {
            let mut s = PySuite::new(None);
            let result = s.remove_context("no_such_context");
            assert!(result.is_err(), "removing absent context should fail");
        }

        #[test]
        fn test_len_via_inner() {
            let mut s = PySuite::new(None);
            assert_eq!(s.inner.len(), 0);
            s.add_context("a", vec![]).unwrap();
            assert_eq!(s.inner.len(), 1);
        }
    }

    mod test_suite_description {
        use super::*;

        #[test]
        fn test_default_description_is_none() {
            let s = PySuite::new(None);
            assert!(s.inner.description.is_none());
        }

        #[test]
        fn test_set_description_updates_value() {
            let mut s = PySuite::new(Some("first"));
            s.set_description(Some("second".to_string()));
            assert_eq!(s.inner.description.as_deref(), Some("second"));
        }

        #[test]
        fn test_clear_description() {
            let mut s = PySuite::new(Some("has desc"));
            s.set_description(None);
            assert!(s.inner.description.is_none());
        }
    }

    mod test_suite_conflict_mode {
        use super::*;

        #[test]
        fn test_set_conflict_mode_error() {
            let mut s = PySuite::new(None);
            let result = s.set_conflict_mode("error");
            assert!(result.is_ok(), "set_conflict_mode('error') failed");
        }

        #[test]
        fn test_set_conflict_mode_first() {
            let mut s = PySuite::new(None);
            assert!(s.set_conflict_mode("first").is_ok());
        }

        #[test]
        fn test_set_conflict_mode_last() {
            let mut s = PySuite::new(None);
            assert!(s.set_conflict_mode("last").is_ok());
        }

        #[test]
        fn test_set_conflict_mode_prefix() {
            let mut s = PySuite::new(None);
            assert!(s.set_conflict_mode("prefix").is_ok());
        }

        #[test]
        fn test_set_conflict_mode_invalid_returns_err() {
            let mut s = PySuite::new(None);
            let result = s.set_conflict_mode("garbage_mode_xyz");
            assert!(result.is_err(), "invalid conflict mode should fail");
        }
    }

    mod test_suite_is_suite {
        use super::*;

        #[test]
        fn test_is_suite_nonexistent_path_returns_false() {
            assert!(!PySuite::is_suite("/nonexistent/path/xyz"));
        }

        #[test]
        fn test_is_suite_empty_temp_dir_returns_false() {
            let tmp = std::env::temp_dir().join("rez_suite_test_empty");
            std::fs::create_dir_all(&tmp).unwrap();
            assert!(!PySuite::is_suite(&tmp.to_string_lossy()));
            let _ = std::fs::remove_dir_all(&tmp);
        }
    }

    mod test_suite_manager {
        use super::*;

        #[test]
        fn test_suite_manager_new_empty_paths() {
            let mgr = PySuiteManager::new(Some(vec![]));
            let names = mgr.list_suite_names();
            assert!(names.is_empty(), "empty path manager should have no suites");
        }

        #[test]
        fn test_suite_manager_find_suites_nonexistent_returns_empty() {
            let mgr = PySuiteManager::new(Some(vec!["/no/such/path".to_string()]));
            let suites = mgr.find_suites();
            assert!(suites.is_empty());
        }

        #[test]
        fn test_suite_manager_load_nonexistent_returns_err() {
            let mgr = PySuiteManager::new(Some(vec![]));
            let result = mgr.load_suite("nonexistent_suite");
            assert!(result.is_err());
        }
    }

    mod test_suite_extra {
        use super::*;

        #[test]
        fn test_repr_contains_contexts_count() {
            let mut s = PySuite::new(None);
            s.add_context("a", vec!["pkg-1".to_string()]).unwrap();
            s.add_context("b", vec!["pkg-2".to_string()]).unwrap();
            let repr = s.__repr__();
            assert!(repr.contains("Suite("), "repr must start with Suite(, got {repr}");
            assert!(repr.contains('2') || repr.contains("contexts=2"),
                "repr must reflect 2 contexts, got {repr}");
        }

        #[test]
        fn test_add_same_context_twice_is_err_or_ok() {
            let mut s = PySuite::new(None);
            s.add_context("dup", vec!["pkg-1".to_string()]).unwrap();
            // Adding same name again: implementation may error or silently update.
            // We only verify the function doesn't panic.
            let _ = s.add_context("dup", vec!["pkg-2".to_string()]);
        }

        #[test]
        fn test_context_names_empty_after_remove_all() {
            let mut s = PySuite::new(None);
            s.add_context("x", vec![]).unwrap();
            s.add_context("y", vec![]).unwrap();
            s.remove_context("x").unwrap();
            s.remove_context("y").unwrap();
            assert!(s.context_names().is_empty(), "context_names must be empty after removing all");
        }

        #[test]
        fn test_alias_tool_nonexistent_context_returns_err() {
            let mut s = PySuite::new(None);
            let result = s.alias_tool("no_such_ctx", "new_alias", "original_tool");
            assert!(result.is_err(), "alias_tool on absent context should fail");
        }

        #[test]
        fn test_hide_tool_nonexistent_context_returns_err() {
            let mut s = PySuite::new(None);
            let result = s.hide_tool("no_such_ctx", "some_tool");
            assert!(result.is_err(), "hide_tool on absent context should fail");
        }

        #[test]
        fn test_path_none_before_save() {
            let s = PySuite::new(None);
            assert!(s.path().is_none(), "path must be None for unsaved suite");
        }

        #[test]
        fn test_suite_manager_no_paths_gives_none_suite() {
            let mgr = PySuiteManager::new(None);
            let names = mgr.list_suite_names();
            // With no paths (None), manager searches default dirs which may be empty
            // Just verify it doesn't panic and returns a Vec (may be empty or not)
            let _ = names.len();
        }
    }
}
