//! Python bindings for rez configuration

use pyo3::prelude::*;
use rez_next_common::config::RezCoreConfig;

/// Python-accessible Config class, compatible with rez.config
#[pyclass(name = "Config")]
pub struct PyConfig {
    inner: RezCoreConfig,
}

#[pymethods]
impl PyConfig {
    /// Create a new Config (loads from files if available)
    #[new]
    pub fn new() -> Self {
        PyConfig {
            inner: RezCoreConfig::load(),
        }
    }

    fn __repr__(&self) -> String {
        "Config()".to_string()
    }

    /// Package search paths
    #[getter]
    fn packages_path(&self) -> Vec<String> {
        self.inner.packages_path.clone()
    }

    /// Local packages path
    #[getter]
    fn local_packages_path(&self) -> String {
        self.inner.local_packages_path.clone()
    }

    /// Release packages path
    #[getter]
    fn release_packages_path(&self) -> String {
        self.inner.release_packages_path.clone()
    }

    /// Default shell
    #[getter]
    fn default_shell(&self) -> String {
        self.inner.default_shell.clone()
    }

    /// rez-next version
    #[getter]
    fn rez_version(&self) -> String {
        self.inner.version.clone()
    }

    /// Get a config field by name
    fn get(
        &self,
        field: &str,
        default: Option<PyObject>,
        py: Python,
    ) -> PyResult<PyObject> {
        if let Some(value) = self.inner.get_field(field) {
            match value {
                serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into()),
                serde_json::Value::Bool(b) => {
                    Ok(pyo3::types::PyBool::new(py, b).to_owned().into())
                }
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(i.into_pyobject(py)?.into())
                    } else if let Some(f) = n.as_f64() {
                        Ok(f.into_pyobject(py)?.into())
                    } else {
                        Ok(py.None())
                    }
                }
                serde_json::Value::Array(arr) => {
                    let list: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    Ok(list.into_pyobject(py)?.into())
                }
                _ => Ok(py.None()),
            }
        } else {
            Ok(default.unwrap_or_else(|| py.None()))
        }
    }
}
