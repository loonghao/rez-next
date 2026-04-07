//! Python bindings for rez.plugins — plugin system compatibility layer
//!
//! Provides API compatible with:
//! - `rez.plugins.manager` — plugin discovery and registration
//! - `rez.plugins.shell` — shell plugin interface
//! - `rez.plugins.build_system` — build system plugin interface
//! - `rez.plugins.release_hook` — release hook interface

use pyo3::prelude::*;

/// Plugin type classification (mirrors rez.plugin_managers.PluginType)
#[pyclass(name = "PluginType", from_py_object)]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPluginType {
    #[pyo3(get)]
    pub name: String,
}

#[pymethods]
impl PyPluginType {
    #[new]
    fn new(name: &str) -> Self {
        PyPluginType {
            name: name.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("PluginType('{}')", self.name)
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }
}

/// Represents a registered plugin.
/// Compatible with rez's plugin_manager.Plugin interface.
#[pyclass(name = "Plugin", from_py_object)]
#[derive(Clone)]
pub struct PyPlugin {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub plugin_type: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub version: String,
}

#[pymethods]
impl PyPlugin {
    #[new]
    #[pyo3(signature = (name, plugin_type, description="", version="0.1.0"))]
    fn new(name: &str, plugin_type: &str, description: &str, version: &str) -> Self {
        PyPlugin {
            name: name.to_string(),
            plugin_type: plugin_type.to_string(),
            description: description.to_string(),
            version: version.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Plugin('{}', type='{}')", self.name, self.plugin_type)
    }

    fn __str__(&self) -> String {
        format!("{} ({})", self.name, self.plugin_type)
    }
}

/// Plugin manager — discovers and manages rez plugins.
/// Compatible with `rez.plugin_managers.RezPluginManager`.
#[pyclass(name = "RezPluginManager")]
pub struct PyRezPluginManager {
    plugins: Vec<PyPlugin>,
}

#[pymethods]
impl PyRezPluginManager {
    /// Create a new plugin manager with built-in rez-next plugins pre-registered.
    #[new]
    fn new() -> Self {
        // Register built-in rez-next plugins (mirrors rez's built-in plugin set)
        let mut manager = PyRezPluginManager {
            plugins: Vec::new(),
        };
        manager.register_builtin_plugins();
        manager
    }

    fn __repr__(&self) -> String {
        format!("RezPluginManager({} plugins)", self.plugins.len())
    }

    /// Get all registered plugins of a given type.
    /// Compatible with `plugin_manager.get_plugins(plugin_type)`
    fn get_plugins(&self, plugin_type: &str) -> Vec<PyPlugin> {
        self.plugins
            .iter()
            .filter(|p| p.plugin_type == plugin_type)
            .cloned()
            .collect()
    }

    /// Get a specific plugin by name and type.
    /// Compatible with `plugin_manager.get_plugin(plugin_type, name)`
    fn get_plugin(&self, plugin_type: &str, name: &str) -> Option<PyPlugin> {
        self.plugins
            .iter()
            .find(|p| p.plugin_type == plugin_type && p.name == name)
            .cloned()
    }

    /// List all plugin type names.
    fn plugin_types(&self) -> Vec<String> {
        let mut types: std::collections::HashSet<String> =
            self.plugins.iter().map(|p| p.plugin_type.clone()).collect();
        let mut result: Vec<String> = types.drain().collect();
        result.sort();
        result
    }

    /// Register a new plugin.
    fn register_plugin(&mut self, plugin: PyPlugin) {
        self.plugins.push(plugin);
    }

    /// Check if a plugin is registered.
    fn has_plugin(&self, plugin_type: &str, name: &str) -> bool {
        self.plugins
            .iter()
            .any(|p| p.plugin_type == plugin_type && p.name == name)
    }

    /// Get all shell plugin names (convenience method).
    /// Compatible with `rez.shells.get_shell_types()`
    fn get_shell_types(&self) -> Vec<String> {
        self.get_plugins("shell")
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    /// Get all build system plugin names.
    /// Compatible with `rez.build_systems.get_build_system_types()`
    fn get_build_system_types(&self) -> Vec<String> {
        self.get_plugins("build_system")
            .into_iter()
            .map(|p| p.name)
            .collect()
    }

    /// Total number of registered plugins
    #[getter]
    fn count(&self) -> usize {
        self.plugins.len()
    }
}

impl PyRezPluginManager {
    fn register_builtin_plugins(&mut self) {
        // Shell plugins (mirrors rez built-in shells)
        let shells = [
            ("bash", "Unix Bash shell"),
            ("zsh", "Unix Zsh shell"),
            ("fish", "Fish shell"),
            ("sh", "POSIX sh shell"),
            ("cmd", "Windows CMD shell"),
            ("powershell", "Windows PowerShell"),
            ("pwsh", "PowerShell Core (cross-platform)"),
            ("csh", "C shell"),
            ("tcsh", "TENEX C shell"),
        ];
        for (name, desc) in &shells {
            self.plugins.push(PyPlugin {
                name: name.to_string(),
                plugin_type: "shell".to_string(),
                description: desc.to_string(),
                version: "1.0.0".to_string(),
            });
        }

        // Build system plugins
        let build_systems = [
            ("cmake", "CMake build system"),
            ("make", "GNU Make build system"),
            ("python_rezbuild", "Python rezbuild.py build system"),
            ("python", "Python setuptools/pyproject build system"),
            ("cargo", "Rust Cargo build system"),
            ("nodejs", "Node.js npm/yarn build system"),
            ("custom_script", "Custom build script (build.sh/build.bat)"),
        ];
        for (name, desc) in &build_systems {
            self.plugins.push(PyPlugin {
                name: name.to_string(),
                plugin_type: "build_system".to_string(),
                description: desc.to_string(),
                version: "1.0.0".to_string(),
            });
        }

        // Release hook plugins
        let release_hooks = [
            ("emailer", "Send email notification on release"),
            ("command", "Run custom command on release"),
        ];
        for (name, desc) in &release_hooks {
            self.plugins.push(PyPlugin {
                name: name.to_string(),
                plugin_type: "release_hook".to_string(),
                description: desc.to_string(),
                version: "1.0.0".to_string(),
            });
        }

        // Package repository plugins
        let repo_types = [("filesystem", "Local filesystem package repository")];
        for (name, desc) in &repo_types {
            self.plugins.push(PyPlugin {
                name: name.to_string(),
                plugin_type: "package_repository".to_string(),
                description: desc.to_string(),
                version: "1.0.0".to_string(),
            });
        }
    }
}

/// Get the global plugin manager singleton.
/// Compatible with `rez.plugin_managers.plugin_manager`
#[pyfunction]
pub fn get_plugin_manager() -> PyRezPluginManager {
    PyRezPluginManager::new()
}

/// Get all available shell types.
/// Compatible with `rez.shells.get_shell_types()`
#[pyfunction]
pub fn get_shell_types() -> Vec<String> {
    PyRezPluginManager::new().get_shell_types()
}

/// Get all available build system types.
/// Compatible with `rez.build_systems.get_build_system_types()`
#[pyfunction]
pub fn get_build_system_types() -> Vec<String> {
    PyRezPluginManager::new().get_build_system_types()
}

/// Check if a shell is supported by rez-next.
/// Compatible with `rez.shells.get_shell_types()` membership check.
#[pyfunction]
pub fn is_shell_supported(shell_name: &str) -> bool {
    let manager = PyRezPluginManager::new();
    manager.has_plugin("shell", &shell_name.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod test_plugin_struct {
        use super::*;

        #[test]
        fn test_plugin_repr() {
            let p = PyPlugin {
                name: "bash".to_string(),
                plugin_type: "shell".to_string(),
                description: "Unix Bash shell".to_string(),
                version: "1.0.0".to_string(),
            };
            let r = p.__repr__();
            assert!(r.contains("bash"), "repr must contain plugin name");
            assert!(r.contains("shell"), "repr must contain plugin type");
        }

        #[test]
        fn test_plugin_str() {
            let p = PyPlugin {
                name: "cmake".to_string(),
                plugin_type: "build_system".to_string(),
                description: "CMake".to_string(),
                version: "1.0.0".to_string(),
            };
            let s = p.__str__();
            assert!(s.contains("cmake"));
            assert!(s.contains("build_system"));
        }

        #[test]
        fn test_plugin_type_repr() {
            let pt = PyPluginType::new("shell");
            assert!(pt.__repr__().contains("shell"));
        }

        #[test]
        fn test_plugin_type_str() {
            let pt = PyPluginType::new("release_hook");
            assert_eq!(pt.__str__(), "release_hook");
        }
    }

    mod test_plugin_manager {
        use super::*;

        #[test]
        fn test_plugin_manager_creates_with_builtins() {
            let mgr = PyRezPluginManager::new();
            assert!(mgr.count() > 0);
        }

        #[test]
        fn test_get_shell_plugins() {
            let mgr = PyRezPluginManager::new();
            let shells = mgr.get_plugins("shell");
            assert!(!shells.is_empty());
            let names: Vec<_> = shells.iter().map(|s| s.name.as_str()).collect();
            assert!(names.contains(&"bash"));
            assert!(names.contains(&"powershell"));
            assert!(names.contains(&"fish"));
        }

        #[test]
        fn test_get_build_system_plugins() {
            let mgr = PyRezPluginManager::new();
            let bsys = mgr.get_plugins("build_system");
            assert!(!bsys.is_empty());
            let names: Vec<_> = bsys.iter().map(|b| b.name.as_str()).collect();
            assert!(names.contains(&"cmake"));
            assert!(names.contains(&"python_rezbuild"));
        }

        #[test]
        fn test_has_plugin() {
            let mgr = PyRezPluginManager::new();
            assert!(mgr.has_plugin("shell", "bash"));
            assert!(mgr.has_plugin("shell", "powershell"));
            assert!(!mgr.has_plugin("shell", "nonexistent_shell"));
        }

        #[test]
        fn test_plugin_types_sorted() {
            let mgr = PyRezPluginManager::new();
            let types = mgr.plugin_types();
            // plugin_types() sorts alphabetically
            let mut sorted = types.clone();
            sorted.sort();
            assert_eq!(types, sorted, "plugin_types() should return sorted list");
        }

        #[test]
        fn test_plugin_types_contains_all_categories() {
            let mgr = PyRezPluginManager::new();
            let types = mgr.plugin_types();
            assert!(types.contains(&"shell".to_string()));
            assert!(types.contains(&"build_system".to_string()));
            assert!(types.contains(&"release_hook".to_string()));
            assert!(types.contains(&"package_repository".to_string()));
        }

        #[test]
        fn test_get_plugin_by_name_and_type() {
            let mgr = PyRezPluginManager::new();
            let found = mgr.get_plugin("shell", "bash");
            assert!(found.is_some(), "bash shell plugin must exist");
            let p = found.unwrap();
            assert_eq!(p.name, "bash");
            assert_eq!(p.plugin_type, "shell");
        }

        #[test]
        fn test_get_plugin_missing_returns_none() {
            let mgr = PyRezPluginManager::new();
            assert!(mgr.get_plugin("shell", "no_such_shell_xyz").is_none());
        }

        #[test]
        fn test_register_custom_plugin() {
            let mut mgr = PyRezPluginManager::new();
            let before = mgr.count();
            let plugin = PyPlugin {
                name: "my_custom_shell".to_string(),
                plugin_type: "shell".to_string(),
                description: "Custom test shell".to_string(),
                version: "0.1.0".to_string(),
            };
            mgr.register_plugin(plugin);
            assert_eq!(mgr.count(), before + 1);
            assert!(mgr.has_plugin("shell", "my_custom_shell"));
        }

        #[test]
        fn test_manager_repr_includes_count() {
            let mgr = PyRezPluginManager::new();
            let r = mgr.__repr__();
            let count_str = mgr.count().to_string();
            assert!(
                r.contains(&count_str),
                "repr '{}' should include count '{}'",
                r,
                count_str
            );
        }
    }

    mod test_free_functions {
        use super::*;

        #[test]
        fn test_get_shell_types_includes_standard_shells() {
            let mgr = PyRezPluginManager::new();
            let shell_types = mgr.get_shell_types();
            assert!(shell_types.contains(&"bash".to_string()));
            assert!(shell_types.contains(&"cmd".to_string()));
            assert!(shell_types.contains(&"powershell".to_string()));
        }

        #[test]
        fn test_get_build_system_types_includes_standard() {
            let mgr = PyRezPluginManager::new();
            let bs_types = mgr.get_build_system_types();
            assert!(bs_types.contains(&"cmake".to_string()));
            assert!(bs_types.contains(&"cargo".to_string()));
        }

        #[test]
        fn test_is_shell_supported() {
            assert!(is_shell_supported("bash"));
            assert!(is_shell_supported("PowerShell")); // case-insensitive
            assert!(!is_shell_supported("nonexistent_xyz"));
        }

        #[test]
        fn test_get_shell_types_free_fn_not_empty() {
            let types = get_shell_types();
            assert!(!types.is_empty(), "get_shell_types() must return non-empty list");
            assert!(types.contains(&"bash".to_string()));
        }

        #[test]
        fn test_get_build_system_types_free_fn_not_empty() {
            let types = get_build_system_types();
            assert!(!types.is_empty());
            assert!(types.contains(&"cmake".to_string()));
        }

        // ── New tests (Cycle 96) ─────────────────────────────────────────────

        #[test]
        fn test_is_shell_supported_zsh() {
            assert!(is_shell_supported("zsh"));
        }

        #[test]
        fn test_is_shell_supported_fish() {
            assert!(is_shell_supported("fish"));
        }

        #[test]
        fn test_is_shell_supported_cmd_windows() {
            assert!(is_shell_supported("cmd"), "cmd must be a supported shell");
        }

        #[test]
        fn test_get_shell_types_count_at_least_five() {
            let types = get_shell_types();
            assert!(
                types.len() >= 5,
                "expected at least 5 shell types, got {}",
                types.len()
            );
        }

        #[test]
        fn test_get_build_system_types_contains_make() {
            let types = get_build_system_types();
            assert!(types.contains(&"make".to_string()));
        }

        #[test]
        fn test_get_build_system_types_contains_python_rezbuild() {
            let types = get_build_system_types();
            assert!(types.contains(&"python_rezbuild".to_string()));
        }

        // ── Cycle 103 additions ──────────────────────────────────────────────

        #[test]
        fn test_get_shell_types_are_lowercase() {
            let shells = get_shell_types();
            for s in &shells {
                assert_eq!(s, &s.to_lowercase(), "shell type must be lowercase: {s}");
            }
        }

        #[test]
        fn test_get_build_system_types_are_lowercase() {
            let types = get_build_system_types();
            for t in &types {
                assert_eq!(t, &t.to_lowercase(), "build system type must be lowercase: {t}");
            }
        }

        #[test]
        fn test_plugin_description_field_accessible() {
            let p = PyPlugin {
                name: "bash".to_string(),
                plugin_type: "shell".to_string(),
                description: "Bash shell integration".to_string(),
                version: "2.0.0".to_string(),
            };
            assert!(!p.description.is_empty(), "description must not be empty");
            assert!(p.description.contains("Bash"));
        }

        #[test]
        fn test_plugin_version_field_accessible() {
            let p = PyPlugin {
                name: "cmake".to_string(),
                plugin_type: "build_system".to_string(),
                description: "CMake build".to_string(),
                version: "3.0.0".to_string(),
            };
            assert_eq!(p.version, "3.0.0");
        }

        #[test]
        fn test_plugin_manager_get_shell_types_non_empty() {
            let mgr = PyRezPluginManager::new();
            let shell_types = mgr.get_shell_types();
            assert!(
                !shell_types.is_empty(),
                "get_shell_types must return at least one shell type"
            );
            // Every shell type must be a non-empty lowercase string
            for s in &shell_types {
                assert!(!s.is_empty(), "shell type must not be empty");
                assert_eq!(s, &s.to_lowercase(), "shell type must be lowercase: {s}");
            }
        }

        #[test]
        fn test_plugin_manager_get_build_system_types_non_empty() {
            let mgr = PyRezPluginManager::new();
            let build_types = mgr.get_build_system_types();
            assert!(
                !build_types.is_empty(),
                "get_build_system_types must return at least one type"
            );
            for t in &build_types {
                assert!(!t.is_empty(), "build system type must not be empty");
                assert_eq!(t, &t.to_lowercase(), "build system type must be lowercase: {t}");
            }
        }

        #[test]
        fn test_get_shell_types_no_duplicates() {
            let shells = get_shell_types();
            let mut deduped = shells.clone();
            deduped.dedup();
            assert_eq!(
                shells, deduped,
                "get_shell_types must not return duplicates"
            );
        }
    }

    // ── Cycle 115 additions ─────────────────────────────────────────────────

    mod test_cycle_115 {
        use super::*;

        #[test]
        fn test_plugin_manager_has_filesystem_repo_plugin() {
            let mgr = PyRezPluginManager::new();
            assert!(
                mgr.has_plugin("package_repository", "filesystem"),
                "filesystem repo plugin must be registered"
            );
        }

        #[test]
        fn test_plugin_manager_release_hooks_exist() {
            let mgr = PyRezPluginManager::new();
            let hooks = mgr.get_plugins("release_hook");
            assert!(!hooks.is_empty(), "at least one release_hook plugin must exist");
            let names: Vec<_> = hooks.iter().map(|h| h.name.as_str()).collect();
            assert!(names.contains(&"emailer") || names.contains(&"command"),
                "known release hooks must be registered, got: {:?}", names);
        }

        #[test]
        fn test_plugin_manager_get_plugins_unknown_type_is_empty() {
            let mgr = PyRezPluginManager::new();
            let result = mgr.get_plugins("nonexistent_plugin_type_xyz");
            assert!(result.is_empty(), "unknown plugin type must return empty list");
        }

        #[test]
        fn test_plugin_type_repr_format() {
            let pt = PyPluginType::new("build_system");
            let repr = pt.__repr__();
            assert!(repr.starts_with("PluginType("), "repr: {repr}");
            assert!(repr.ends_with(')'), "repr: {repr}");
            assert!(repr.contains("build_system"), "repr: {repr}");
        }

        #[test]
        fn test_plugin_version_is_semver_like() {
            let mgr = PyRezPluginManager::new();
            let shells = mgr.get_plugins("shell");
            for plugin in &shells {
                assert!(
                    plugin.version.contains('.'),
                    "plugin version should be semver-like, got: '{}'", plugin.version
                );
            }
        }

        #[test]
        fn test_get_plugin_case_sensitive_type() {
            let mgr = PyRezPluginManager::new();
            // plugin type lookup is exact-case (lowercase)
            let found = mgr.get_plugin("Shell", "bash");
            // type "Shell" (capital S) does not exist → should return None
            assert!(found.is_none(), "uppercase type lookup should not match");
        }

        #[test]
        fn test_register_plugin_increases_count_by_one() {
            let mut mgr = PyRezPluginManager::new();
            let before = mgr.count();
            mgr.register_plugin(PyPlugin {
                name: "unique_test_plugin".to_string(),
                plugin_type: "shell".to_string(),
                description: "Test".to_string(),
                version: "0.0.1".to_string(),
            });
            assert_eq!(mgr.count(), before + 1);
        }
    }
}
