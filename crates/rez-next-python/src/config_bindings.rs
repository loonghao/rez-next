//! Python bindings for rez-next-config.
//!
//! This module provides Python access to the configuration management
//! functionality of rez-next.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;
use rez_next_config::Config;

/// Python wrapper for Config.
#[pyclass(name = "Config", from_py_object)]
#[derive(Clone)]
pub struct PyConfig {
    inner: Config,
}

#[pymethods]
impl PyConfig {
    /// Create a new empty configuration.
    #[new]
    pub fn new() -> Self {
        Self {
            inner: Config::new(),
        }
    }

    /// Load configuration from all standard sources.
    #[classmethod]
    pub fn load(_cls: &Bound<'_, PyType>) -> PyResult<Self> {
        match Config::load() {
            Ok(config) => Ok(Self { inner: config }),
            Err(e) => Err(PyValueError::new_err(format!(
                "Failed to load configuration: {}",
                e
            ))),
        }
    }

    /// Get a string value by key (dot-separated path).
    pub fn get_string(&self, key: &str) -> PyResult<Option<String>> {
        Ok(self.inner.get_string(key))
    }

    /// Get a boolean value by key (dot-separated path).
    pub fn get_bool(&self, key: &str) -> PyResult<Option<bool>> {
        Ok(self.inner.get_bool(key))
    }

    /// Get an integer value by key (dot-separated path).
    pub fn get_int(&self, key: &str) -> PyResult<Option<i64>> {
        Ok(self.inner.get_i64(key))
    }

    /// Get a float value by key (dot-separated path).
    pub fn get_float(&self, key: &str) -> PyResult<Option<f64>> {
        Ok(self.inner.get_f64(key))
    }

    /// Check if a configuration key exists.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// String representation.
    pub fn __repr__(&self) -> String {
        format!("Config(sources={:?})", self.inner.sources())
    }
}

/// Load configuration from all standard sources.
#[pyfunction]
pub fn load_config(py: Python<'_>) -> PyResult<Py<PyConfig>> {
    let config = PyConfig::load(&PyType::new::<PyConfig>(py))?;
    Py::new(py, config)
}

/// Register the config module
pub fn register_config_module(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent_module.py(), "config")?;

    m.add_class::<PyConfig>()?;
    m.add_function(wrap_pyfunction!(load_config, &m)?)?;

    // Register as submodule
    parent_module.add_submodule(&m)?;

    // Register in sys.modules
    let sys = parent_module.py().import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("rez_next._native.config", &m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pyconfig_creation() {
        let config = PyConfig::new();
        assert!(!config.inner.is_locked());
    }
}
