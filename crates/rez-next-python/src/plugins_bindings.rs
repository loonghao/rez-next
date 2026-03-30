//! Python bindings for rez.plugins — plugin system compatibility layer
//!
//! Provides API compatible with:
//! - `rez.plugins.manager` — plugin discovery and registration
//! - `rez.plugins.shell` — shell plugin interface
//! - `rez.plugins.build_system` — build system plugin interface
//! - `rez.plugins.release_hook` — release hook interface

use pyo3::prelude::*;

/// Plugin type classification (mirrors rez.plugin_managers.PluginType)
#[pyclass(name = "PluginType")]
#[derive(Clone, Debug, PartialEq)]
pub struct PyPluginType {
    #[pyo3(get)]
    pub name: String,
}

#[pymethods]
impl PyPluginType {
    #[new]
    fn new(name: &str) -> Self {
        PyPluginType { name: name.to_string() }
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
#[pyclass(name = "Plugin")]
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
        let mut manager = PyRezPluginManager { plugins: Vec::new() };
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
        let mut types: std::collections::HashSet<String> = self.plugins
            .iter()
            .map(|p| p.plugin_type.clone())
            .collect();
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
        self.plugins.iter().any(|p| p.plugin_type == plugin_type && p.name == name)
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
        let repo_types = [
            ("filesystem", "Local filesystem package repository"),
        ];
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
    fn test_plugin_types() {
        let mgr = PyRezPluginManager::new();
        let types = mgr.plugin_types();
        assert!(types.contains(&"shell".to_string()));
        assert!(types.contains(&"build_system".to_string()));
        assert!(types.contains(&"release_hook".to_string()));
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
    fn test_get_shell_types_includes_standard_shells() {
        let mgr = PyRezPluginManager::new();
        let shell_types = mgr.get_shell_types();
        assert!(shell_types.contains(&"bash".to_string()));
        assert!(shell_types.contains(&"cmd".to_string()));
        assert!(shell_types.contains(&"powershell".to_string()));
    }

    #[test]
    fn test_is_shell_supported() {
        assert!(is_shell_supported("bash"));
        assert!(is_shell_supported("PowerShell")); // case-insensitive
        assert!(!is_shell_supported("nonexistent_xyz"));
    }
}
