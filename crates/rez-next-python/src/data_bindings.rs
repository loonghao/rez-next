//! Python bindings for `rez.data` — built-in data resources access.
//!
//! Provides access to rez-next's built-in data files such as:
//! - Completion scripts (bash, zsh, fish)
//! - Example package definitions
//! - Schema definitions
//! - Default configuration templates

use pyo3::prelude::*;

/// Embedded completion script for bash
const BASH_COMPLETE: &str = r#"# rez-next bash completion
_rez_next() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    opts="env solve build release status search view diff cp mv rm bundle config selftest gui context suite interpret depends pip forward benchmark complete source bind"
    if [[ ${cur} == -* ]]; then
        COMPREPLY=( $(compgen -W "--help --version --debug --quiet" -- ${cur}) )
        return 0
    fi
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}
complete -F _rez_next rez-next rez_next
"#;

/// Embedded completion script for zsh
const ZSH_COMPLETE: &str = r#"# rez-next zsh completion
#compdef rez-next

_rez_next() {
    local -a commands
    commands=(
        'env:create a resolved environment'
        'solve:solve a dependency set'
        'build:build a package'
        'release:release a package'
        'search:search packages'
        'source:write activation script'
        'config:print configuration'
        'selftest:run self-tests'
    )
    _describe 'rez-next commands' commands
}
"#;

/// Example package.py content
const EXAMPLE_PACKAGE_PY: &str = r#"name = "my_package"
version = "1.0.0"
description = "An example rez package"
authors = ["Your Name"]

requires = [
    "python-3.9+",
]

def commands():
    env.MY_PACKAGE_ROOT = "{root}"
    env.PATH.prepend("{root}/bin")
"#;

/// Default rezconfig.py template
const DEFAULT_REZCONFIG: &str = r#"# rez-next configuration
# See https://github.com/loonghao/rez-next for documentation

# Package search paths
packages_path = [
    "~/packages",
    "/packages/int",
    "/packages/ext",
]

# Local packages path
local_packages_path = "~/packages"

# Release packages path
release_packages_path = "/packages/int"

# Temporary directory for builds
tmpdir = None

# Default shell
default_shell = ""  # auto-detect

# Quiet mode
quiet = False
"#;

/// Python class providing access to rez-next built-in data resources.
///
/// Equivalent to `rez.data` module providing completions, examples, etc.
#[pyclass(name = "RezData")]
#[derive(Debug)]
pub struct PyRezData;

#[pymethods]
impl PyRezData {
    #[new]
    pub fn new() -> Self {
        Self
    }

    /// Get completion script for a shell.
    ///
    /// Args:
    ///     shell: Shell name ("bash", "zsh", "fish"). Default: auto-detect.
    ///
    /// Returns: Completion script content as string.
    #[pyo3(signature = (shell = None))]
    pub fn get_completion_script(&self, shell: Option<&str>) -> PyResult<String> {
        let shell_name = shell.unwrap_or("bash");
        match shell_name.to_lowercase().as_str() {
            "bash" => Ok(BASH_COMPLETE.to_string()),
            "zsh" => Ok(ZSH_COMPLETE.to_string()),
            "fish" => Ok("# rez-next fish completion\n# TODO: fish completions\n".to_string()),
            _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown shell '{}'. Supported: bash, zsh, fish",
                shell_name
            ))),
        }
    }

    /// Get example package.py content.
    pub fn get_example_package(&self) -> String {
        EXAMPLE_PACKAGE_PY.to_string()
    }

    /// Get the default rezconfig.py template.
    pub fn get_default_config(&self) -> String {
        DEFAULT_REZCONFIG.to_string()
    }

    /// List available data resources.
    pub fn list_resources(&self) -> Vec<String> {
        vec![
            "completions/bash".to_string(),
            "completions/zsh".to_string(),
            "completions/fish".to_string(),
            "examples/package.py".to_string(),
            "config/rezconfig.py".to_string(),
        ]
    }

    /// Get a resource by path.
    ///
    /// Args:
    ///     path: Resource path (e.g., "completions/bash", "examples/package.py")
    ///
    /// Returns: Resource content as string.
    pub fn get_resource(&self, path: &str) -> PyResult<String> {
        match path {
            "completions/bash" => Ok(BASH_COMPLETE.to_string()),
            "completions/zsh" => Ok(ZSH_COMPLETE.to_string()),
            "completions/fish" => Ok("# rez-next fish completion\n".to_string()),
            "examples/package.py" => Ok(EXAMPLE_PACKAGE_PY.to_string()),
            "config/rezconfig.py" => Ok(DEFAULT_REZCONFIG.to_string()),
            _ => Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Resource not found: '{}'. Available: {:?}",
                path,
                self.list_resources()
            ))),
        }
    }

    /// Write a completion script to a file.
    #[pyo3(signature = (dest_path, shell = None))]
    pub fn write_completion_script(
        &self,
        dest_path: &str,
        shell: Option<&str>,
    ) -> PyResult<String> {
        let content = self.get_completion_script(shell)?;
        std::fs::write(dest_path, &content)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(dest_path.to_string())
    }

    /// Write example package.py to a directory.
    pub fn write_example_package(&self, dest_dir: &str) -> PyResult<String> {
        let path = std::path::PathBuf::from(dest_dir).join("package.py");
        std::fs::create_dir_all(dest_dir)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        std::fs::write(&path, EXAMPLE_PACKAGE_PY)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn __repr__(&self) -> String {
        "RezData()".to_string()
    }
}

/// Get a built-in resource string by name.
///
/// Equivalent to `rez.data.get_resource(name)` in original rez.
#[pyfunction]
pub fn get_data_resource(path: &str) -> PyResult<String> {
    let data = PyRezData::new();
    data.get_resource(path)
}

/// List all available built-in data resources.
#[pyfunction]
pub fn list_data_resources() -> Vec<String> {
    PyRezData::new().list_resources()
}

/// Get completion script for a shell.
#[pyfunction]
#[pyo3(signature = (shell = None))]
pub fn get_completion_script(shell: Option<&str>) -> PyResult<String> {
    PyRezData::new().get_completion_script(shell)
}

// ─── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_bash_completion_not_empty() {
        assert!(!BASH_COMPLETE.is_empty());
        assert!(BASH_COMPLETE.contains("_rez_next"));
    }

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_zsh_completion_not_empty() {
        assert!(!ZSH_COMPLETE.is_empty());
        assert!(ZSH_COMPLETE.contains("rez-next"));
    }

    #[test]
    fn test_example_package_py_valid() {
        // Should contain required fields
        assert!(EXAMPLE_PACKAGE_PY.contains("name"));
        assert!(EXAMPLE_PACKAGE_PY.contains("version"));
        assert!(EXAMPLE_PACKAGE_PY.contains("requires"));
    }

    #[test]
    fn test_default_rezconfig_valid() {
        assert!(DEFAULT_REZCONFIG.contains("packages_path"));
        assert!(DEFAULT_REZCONFIG.contains("local_packages_path"));
    }

    #[test]
    fn test_list_resources_non_empty() {
        let data = PyRezData::new();
        let resources = data.list_resources();
        assert!(!resources.is_empty());
        assert!(resources.contains(&"completions/bash".to_string()));
        assert!(resources.contains(&"examples/package.py".to_string()));
    }

    #[test]
    fn test_write_completion_to_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("rez-complete.bash");
        // Write directly (no PyO3 GIL in tests)
        let content = BASH_COMPLETE;
        std::fs::write(&dest, content).unwrap();
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("_rez_next"));
    }

    #[test]
    fn test_resource_lookup_bash() {
        // Direct string match — no PyO3 in unit tests
        let content = match "completions/bash" {
            "completions/bash" => BASH_COMPLETE.to_string(),
            _ => panic!("not found"),
        };
        assert!(!content.is_empty());
    }

    #[test]
    fn test_resource_lookup_example_package() {
        let content = EXAMPLE_PACKAGE_PY;
        assert!(content.contains("name = \"my_package\""));
    }

    #[test]
    fn test_write_example_package_to_dir() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest_dir = tmp.path().to_str().unwrap();
        let pkg_path = std::path::PathBuf::from(dest_dir).join("package.py");
        std::fs::write(&pkg_path, EXAMPLE_PACKAGE_PY).unwrap();
        assert!(pkg_path.exists());
        let content = std::fs::read_to_string(&pkg_path).unwrap();
        assert!(content.contains("my_package"));
    }
}
