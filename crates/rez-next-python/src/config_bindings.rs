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
            let inner = RezCoreConfig::load();
            let val = inner.get_field("local_packages_path");
            assert!(val.is_some(), "local_packages_path should be a known field");
            if let Some(serde_json::Value::String(s)) = val {
                assert!(!s.is_empty());
            }
        }

        #[test]
        fn test_get_unknown_field_returns_none_from_inner() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("__nonexistent_field_cycle94__");
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

        #[test]
        fn test_get_field_version_is_string() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("version");
            assert!(val.is_some());
            if let Some(serde_json::Value::String(v)) = val {
                assert!(v.contains('.'), "version should be semver-like: {v}");
            }
        }

        #[test]
        fn test_get_field_default_shell_is_string() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("default_shell");
            assert!(val.is_some());
            if let Some(serde_json::Value::String(s)) = val {
                assert!(!s.is_empty());
            }
        }

        #[test]
        fn test_get_field_packages_path_is_array() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("packages_path");
            assert!(val.is_some());
            assert!(
                matches!(val, Some(serde_json::Value::Array(_))),
                "packages_path should be a JSON array"
            );
        }

        #[test]
        fn test_get_field_nested_cache_memory_cache_size() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("cache.memory_cache_size");
            assert!(val.is_some(), "cache.memory_cache_size should exist");
            if let Some(serde_json::Value::Number(n)) = val {
                let size = n.as_u64().unwrap_or(0);
                assert!(size > 0, "memory_cache_size must be > 0");
            }
        }

        #[test]
        fn test_get_field_nested_cache_enable_memory_cache_is_bool() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("cache.enable_memory_cache");
            assert!(val.is_some());
            assert!(
                matches!(val, Some(serde_json::Value::Bool(_))),
                "cache.enable_memory_cache should be a boolean"
            );
        }

        #[test]
        fn test_get_field_nested_cache_ttl_seconds_is_positive() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("cache.cache_ttl_seconds");
            assert!(val.is_some());
            if let Some(serde_json::Value::Number(n)) = val {
                let ttl = n.as_u64().unwrap_or(0);
                assert!(ttl > 0, "cache_ttl_seconds should be > 0");
            }
        }

        #[test]
        fn test_get_field_tmpdir_non_empty() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("tmpdir");
            assert!(val.is_some(), "tmpdir field should exist");
            if let Some(serde_json::Value::String(s)) = val {
                assert!(!s.is_empty(), "tmpdir should not be empty");
            }
        }

        #[test]
        fn test_get_field_editor_non_empty() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("editor");
            assert!(val.is_some(), "editor field should exist");
            if let Some(serde_json::Value::String(s)) = val {
                assert!(!s.is_empty(), "editor should not be empty");
            }
        }

        #[test]
        fn test_get_field_use_rust_solver_is_bool() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("use_rust_solver");
            assert!(val.is_some(), "use_rust_solver should exist");
            assert!(
                matches!(val, Some(serde_json::Value::Bool(_))),
                "use_rust_solver should be a boolean"
            );
        }
    }

    mod test_config_default_values {
        use super::*;

        #[test]
        fn test_default_packages_path_has_three_entries() {
            let cfg = RezCoreConfig::default();
            assert_eq!(
                cfg.packages_path.len(),
                3,
                "default packages_path should have 3 entries, got: {:?}",
                cfg.packages_path
            );
        }

        #[test]
        fn test_default_packages_path_contains_tilde() {
            let cfg = RezCoreConfig::default();
            let has_tilde = cfg.packages_path.iter().any(|p| p.starts_with('~'));
            assert!(has_tilde, "default packages_path should contain tilde paths");
        }

        #[test]
        fn test_default_shell_is_platform_appropriate() {
            let cfg = RezCoreConfig::default();
            #[cfg(windows)]
            assert_eq!(
                cfg.default_shell, "cmd",
                "Windows default shell should be cmd"
            );
            #[cfg(not(windows))]
            assert_eq!(
                cfg.default_shell, "bash",
                "Unix default shell should be bash"
            );
        }

        #[test]
        fn test_default_editor_is_platform_appropriate() {
            let cfg = RezCoreConfig::default();
            #[cfg(windows)]
            assert_eq!(cfg.editor, "notepad");
            #[cfg(not(windows))]
            assert_eq!(cfg.editor, "vi");
        }

        #[test]
        fn test_default_cache_memory_size_is_1000() {
            let cfg = RezCoreConfig::default();
            assert_eq!(cfg.cache.memory_cache_size, 1000);
        }

        #[test]
        fn test_default_cache_ttl_is_1_hour() {
            let cfg = RezCoreConfig::default();
            assert_eq!(cfg.cache.cache_ttl_seconds, 3600);
        }
    }

    // ── Cycle 106 additions ──────────────────────────────────────────────────

    mod test_config_field_types_extra {
        use super::*;

        #[test]
        fn test_get_field_release_packages_path_is_string() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("release_packages_path");
            assert!(val.is_some(), "release_packages_path should be a known field");
            if let Some(serde_json::Value::String(s)) = val {
                assert!(!s.is_empty(), "release_packages_path should not be empty");
            }
        }

        #[test]
        fn test_config_local_packages_path_is_nonempty_string() {
            let cfg = PyConfig::new();
            let s = cfg.local_packages_path();
            assert!(!s.is_empty(), "local_packages_path must be non-empty");
        }

        #[test]
        fn test_config_rez_version_contains_dot() {
            let cfg = PyConfig::new();
            let v = cfg.rez_version();
            assert!(v.contains('.'), "rez_version should be semver-like: {v}");
        }
    }

    mod test_config_cy114 {
        use super::*;

        /// default packages_path entries are non-empty strings
        #[test]
        fn test_default_packages_path_entries_nonempty() {
            let cfg = RezCoreConfig::default();
            for path in &cfg.packages_path {
                assert!(!path.is_empty(), "packages_path entry must be non-empty");
            }
        }

        /// default use_rust_solver is true
        #[test]
        fn test_default_use_rust_solver_is_true() {
            let cfg = RezCoreConfig::default();
            assert!(cfg.use_rust_solver, "default use_rust_solver should be true");
        }

        /// get_field for an unknown key returns None
        #[test]
        fn test_get_field_unknown_key_returns_none() {
            let cfg = RezCoreConfig::load();
            let val = cfg.get_field("__completely_unknown_key_xyz__");
            assert!(val.is_none(), "get_field for unknown key should return None");
        }

        /// RezCoreConfig::default() produces same packages_path len as expected
        #[test]
        fn test_default_packages_path_len_is_positive() {
            let cfg = RezCoreConfig::default();
            assert!(
                !cfg.packages_path.is_empty(),
                "default packages_path must not be empty"
            );
        }

        /// PyConfig::new() does not panic
        #[test]
        fn test_pyconfig_new_no_panic() {
            let _ = PyConfig::new();
        }

        /// default cache memory_cache_size is greater than 0
        #[test]
        fn test_default_cache_memory_size_gt_zero() {
            let cfg = RezCoreConfig::default();
            assert!(cfg.cache.memory_cache_size > 0, "cache memory size must be > 0");
        }

        /// default_shell is one of the known shell names
        #[test]
        fn test_default_shell_is_known_shell_name() {
            let cfg = RezCoreConfig::default();
            let known = ["bash", "zsh", "fish", "cmd", "powershell"];
            assert!(
                known.contains(&cfg.default_shell.as_str()),
                "default_shell '{}' should be a known shell",
                cfg.default_shell
            );
        }
    }
}
