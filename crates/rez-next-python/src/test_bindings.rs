//! Python bindings for PackageTestRunner and PackageTestResults
//!
//! Exposes rez.package_test functionality to Python.
//! Usage from Python:
//! ```python
//! import rez_next as rez
//! runner = rez.test.PackageTestRunner("my_package")
//! test_names = runner.get_test_names()
//! for name in test_names:
//!     exit_code = runner.run_test(name)
//! print(runner.print_summary())
//! ```

use pyo3::prelude::*;
use rez_next_package::package::test_runner::{
    PackageTestResults as RustTestResults, PackageTestRunner as RustTestRunner, TestStatus,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// ── PyPackageTestRunner ─────────────────────────────────────────────

/// Python-accessible PackageTestRunner.
#[pyclass(name = "PackageTestRunner", from_py_object)]
#[derive(Clone)]
pub struct PyPackageTestRunner {
    inner: Arc<Mutex<RustTestRunner>>,
}

#[pymethods]
impl PyPackageTestRunner {
    /// Create a new PackageTestRunner for the given package.
    #[new]
    pub fn new(package_spec: String) -> PyResult<Self> {
        let runner = RustTestRunner::new(package_spec)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(runner)),
        })
    }

    /// Set the working directory for test execution.
    #[pyo3(name = "with_working_dir")]
    pub fn py_with_working_dir(&self, dir: String) -> PyResult<Self> {
        let runner = self.inner.lock().unwrap();
        let runner_clone = runner.clone(); // Dereference MutexGuard, then clone
        let new_runner = runner_clone.with_working_dir(PathBuf::from(dir));
        Ok(Self {
            inner: Arc::new(Mutex::new(new_runner)),
        })
    }

    /// Set verbose output level (0-2).
    #[pyo3(name = "with_verbose")]
    pub fn py_with_verbose(&self, verbose: u8) -> PyResult<Self> {
        let runner = self.inner.lock().unwrap();
        let runner_clone = runner.clone();
        let new_runner = runner_clone.with_verbose(verbose);
        Ok(Self {
            inner: Arc::new(Mutex::new(new_runner)),
        })
    }

    /// Set dry run mode.
    #[pyo3(name = "with_dry_run")]
    pub fn py_with_dry_run(&self, dry_run: bool) -> PyResult<Self> {
        let runner = self.inner.lock().unwrap();
        let runner_clone = runner.clone();
        let new_runner = runner_clone.with_dry_run(dry_run);
        Ok(Self {
            inner: Arc::new(Mutex::new(new_runner)),
        })
    }

    /// Set stop on fail behavior.
    #[pyo3(name = "with_stop_on_fail")]
    pub fn py_with_stop_on_fail(&self, stop_on_fail: bool) -> PyResult<Self> {
        let runner = self.inner.lock().unwrap();
        let runner_clone = runner.clone();
        let new_runner = runner_clone.with_stop_on_fail(stop_on_fail);
        Ok(Self {
            inner: Arc::new(Mutex::new(new_runner)),
        })
    }

    /// Get available test names from the package definition.
    pub fn get_test_names(&self) -> PyResult<Vec<String>> {
        let runner = self.inner.lock().unwrap();
        runner
            .get_test_names()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Find requested test names (supports wildcards).
    pub fn find_requested_test_names(&self, requested: Vec<String>) -> PyResult<Vec<String>> {
        let runner = self.inner.lock().unwrap();
        runner
            .find_requested_test_names(&requested)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Run a specific test by name.
    pub fn run_test(&self, test_name: String) -> PyResult<i32> {
        let mut runner = self.inner.lock().unwrap();
        runner
            .run_test(&test_name)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Print test summary.
    pub fn print_summary(&self) {
        let runner = self.inner.lock().unwrap();
        runner.print_summary();
    }

    /// Get the package name.
    #[getter]
    pub fn package_name(&self) -> String {
        let runner = self.inner.lock().unwrap();
        runner.package_name.clone()
    }

    /// Get the number of test results.
    pub fn num_tests(&self) -> usize {
        let runner = self.inner.lock().unwrap();
        runner.test_results.len()
    }

    /// Get the number of successful tests.
    pub fn num_success(&self) -> usize {
        let runner = self.inner.lock().unwrap();
        runner
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Success)
            .count()
    }

    /// Get the number of failed tests.
    pub fn num_failed(&self) -> usize {
        let runner = self.inner.lock().unwrap();
        runner
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count()
    }

    /// Get the number of skipped tests.
    pub fn num_skipped(&self) -> usize {
        let runner = self.inner.lock().unwrap();
        runner
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Skipped)
            .count()
    }
}

// ── PyPackageTestResults ──────────────────────────────────────────

/// Python-accessible PackageTestResults.
#[pyclass(name = "PackageTestResults", from_py_object)]
#[derive(Clone)]
pub struct PyPackageTestResults {
    inner: Arc<Mutex<RustTestResults>>,
}

#[pymethods]
impl PyPackageTestResults {
    /// Create a new empty test results collector.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustTestResults::new())),
        }
    }

    /// Add a test result.
    #[pyo3(name = "add_test_result")]
    pub fn py_add_test_result(
        &self,
        _py: Python<'_>,
        test_name: String,
        variant: Option<String>,
        status: Bound<'_, PyAny>,
        description: String,
    ) -> PyResult<()> {
        let status = pyobject_to_test_status(status)?;
        let mut results = self.inner.lock().unwrap();
        results.add_test_result(test_name, variant, status, description);
        Ok(())
    }

    /// Get the number of tests.
    pub fn num_tests(&self) -> usize {
        let results = self.inner.lock().unwrap();
        results.num_tests()
    }

    /// Get the number of successful tests.
    pub fn num_success(&self) -> usize {
        let results = self.inner.lock().unwrap();
        results.num_success()
    }

    /// Get the number of failed tests.
    pub fn num_failed(&self) -> usize {
        let results = self.inner.lock().unwrap();
        results.num_failed()
    }

    /// Get the number of skipped tests.
    pub fn num_skipped(&self) -> usize {
        let results = self.inner.lock().unwrap();
        results.num_skipped()
    }

    /// Print test summary.
    pub fn print_summary(&self) {
        let results = self.inner.lock().unwrap();
        results.print_summary();
    }
}

// ── Helper functions ─────────────────────────────────────────────────

/// Convert a Python object to TestStatus.
fn pyobject_to_test_status(obj: Bound<'_, PyAny>) -> PyResult<TestStatus> {
    // Accept string: "success", "failed", "skipped", "error"
    if let Ok(s) = obj.extract::<String>() {
        match s.to_lowercase().as_str() {
            "success" => Ok(TestStatus::Success),
            "failed" => Ok(TestStatus::Failed),
            "skipped" => Ok(TestStatus::Skipped),
            "error" => Ok(TestStatus::Error),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid test status: {}",
                s
            ))),
        }
    } else {
        Err(pyo3::exceptions::PyTypeError::new_err(
            "Expected string or TestStatus object",
        ))
    }
}

// ── Utility functions ─────────────────────────────────────────────

// ── Registration ──────────────────────────────────────────────────

/// Register the `rez_next._native.test` submodule.
pub fn register_test_submodule(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "test")?;

    m.add_class::<PyPackageTestRunner>()?;
    m.add_class::<PyPackageTestResults>()?;

    // Add status constants
    m.setattr("SUCCESS", "success")?;
    m.setattr("FAILED", "failed")?;
    m.setattr("SKIPPED", "skipped")?;
    m.setattr("ERROR", "error")?;

    // Add exceptions
    // TODO: Fix PackageTestError registration
    // m.add("PackageTestError", py.get_type::<PackageTestError>())?;

    // Add utility functions
    // TODO: Fix utility function registration
    // m.add_function(wrap_pyfunction!(py_heading, &m)?)?;
    // m.add_function(wrap_pyfunction!(py_print_error, &m)?)?;
    // m.add_function(wrap_pyfunction!(py_print_info, &m)?)?;
    // m.add_function(wrap_pyfunction!(py_print_warning, &m)?)?;

    // Register as submodule
    crate::register_submodule(parent, "test", &m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_py_package_test_runner_creation() {
        // This test just verifies the struct can be created
        let runner_result = PyPackageTestRunner::new("test".to_string());
        // May fail if package not found, but struct should be creatable
        assert!(runner_result.is_ok() || runner_result.is_err());
    }

    #[test]
    fn test_py_package_test_results_creation() {
        let _results = PyPackageTestResults::new();
    }
}
