//! Python bindings for rez configuration

use pyo3::prelude::*;
use rez_next_common::config::RezCoreConfig;

/// Python-accessible Config class, compatible with rez.config
#[pyclass(name = "Config")]
pub struct PyConfig {
    inner: RezCoreConfig,
}

impl Default for PyConfig {
    fn default() -> Self {
        Self::new()
    }
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
    fn get(&self, field: &str, default: Option<PyObject>, py: Python) -> PyResult<PyObject> {
        if let Some(value) = self.inner.get_field(field) {
            match value {
                serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into()),
                serde_json::Value::Bool(b) => Ok(pyo3::types::PyBool::new(py, b).to_owned().into()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_common::config::RezCoreConfig;

    mod test_config_load {
        use super::*;

        #[test]
        fn test_config_loads_without_panic() {
            let _ = RezCoreConfig::load();
        }

        #[test]
        fn test_packages_path_is_list() {
            let cfg = RezCoreConfig::load();
            // Must be a valid Vec (possibly empty in CI)
            let _ = cfg.packages_path;
        }

        #[test]
        fn test_local_packages_path_non_empty() {
            let cfg = RezCoreConfig::load();
            assert!(
                !cfg.local_packages_path.is_empty(),
                "local_packages_path should have a default"
            );
        }

        #[test]
        fn test_release_packages_path_non_empty() {
            let cfg = RezCoreConfig::load();
            assert!(
                !cfg.release_packages_path.is_empty(),
                "release_packages_path should have a default"
            );
        }

        #[test]
        fn test_default_shell_non_empty() {
            let cfg = RezCoreConfig::load();
            assert!(
                !cfg.default_shell.is_empty(),
                "default_shell should be set"
            );
        }

        #[test]
        fn test_version_non_empty() {
            let cfg = RezCoreConfig::load();
            assert!(!cfg.version.is_empty(), "version must be non-empty");
            assert!(
                cfg.version.contains('.'),
                "version should be semver-like: {}",
                cfg.version
            );
        }
    }

    mod test_config_repr {
        use super::*;

        #[test]
        fn test_repr_is_config() {
            let cfg = PyConfig::new();
            assert_eq!(cfg.__repr__(), "Config()");
        }

        #[test]
        fn test_new_and_default_produce_same_repr() {
            let a = PyConfig::new();
            let b = PyConfig::default();
            assert_eq!(a.__repr__(), b.__repr__());
        }
    }
}
