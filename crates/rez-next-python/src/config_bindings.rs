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
    fn get(&self, field: &str, default: Option<Py<PyAny>>, py: Python) -> PyResult<Py<PyAny>> {
        if let Some(value) = self.inner.get_field(field) {
            match value {
                serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
                serde_json::Value::Bool(b) => Ok(pyo3::types::PyBool::new(py, b)
                    .to_owned()
                    .into_any()
                    .unbind()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(i.into_pyobject(py)?.into_any().unbind())
                    } else if let Some(f) = n.as_f64() {
                        Ok(f.into_pyobject(py)?.into_any().unbind())
                    } else {
                        Ok(py.None().into_any())
                    }
                }
                serde_json::Value::Array(arr) => {
                    let list: Vec<String> = arr
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    Ok(list.into_pyobject(py)?.into_any().unbind())
                }
                _ => Ok(py.None().into_any()),
            }
        } else {
            Ok(default.unwrap_or_else(|| py.None().into_any()))
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
            assert!(!cfg.default_shell.is_empty(), "default_shell should be set");
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

    mod test_config_getters {
        use super::*;

        #[test]
        fn test_packages_path_is_vec() {
            // packages_path returns a Vec<String>; default may be empty or have entries
            let _paths: Vec<String> = PyConfig::new().packages_path(); // must not panic
        }

        #[test]
        fn test_local_packages_path_getter_matches_inner() {
            let cfg = PyConfig::new();
            assert_eq!(cfg.local_packages_path(), cfg.inner.local_packages_path);
        }

        #[test]
        fn test_release_packages_path_getter_matches_inner() {
            let cfg = PyConfig::new();
            assert_eq!(cfg.release_packages_path(), cfg.inner.release_packages_path);
        }

        #[test]
        fn test_default_shell_getter_matches_inner() {
            let cfg = PyConfig::new();
            assert_eq!(cfg.default_shell(), cfg.inner.default_shell);
        }

        #[test]
        fn test_rez_version_getter_matches_inner() {
            let cfg = PyConfig::new();
            assert_eq!(cfg.rez_version(), cfg.inner.version);
        }
    }

    mod test_config_get_field {
        use super::*;

        #[test]
        fn test_get_known_string_field_local_packages_path() {
            // RezCoreConfig::get_field("local_packages_path") should return String value
            // We verify it's retrievable without checking exact value
            let inner = RezCoreConfig::load();
            let val = inner.get_field("local_packages_path");
            assert!(val.is_some(), "local_packages_path should be a known field");
        }

        #[test]
        fn test_get_unknown_field_returns_none_from_inner() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("__nonexistent_field_cycle90__");
            assert!(val.is_none(), "unknown field should return None");
        }

        #[test]
        fn test_new_and_default_same_packages_path() {
            let a = PyConfig::new();
            let b = PyConfig::default();
            assert_eq!(
                a.packages_path(),
                b.packages_path(),
                "new() and default() should yield identical packages_path"
            );
        }
    }
}
