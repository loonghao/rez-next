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

/// Embedded completion script for fish
const FISH_COMPLETE: &str = r#"# rez-next fish completion
# Place in ~/.config/fish/completions/rez-next.fish

set -l commands env solve build release status search view diff cp mv rm bundle config selftest gui context suite interpret depends pip forward benchmark complete source bind

complete -c rez-next -f
complete -c rez-next -n "__fish_use_subcommand" -a "$commands"

# Global flags
complete -c rez-next -s h -l help    -d "Show help"
complete -c rez-next -s V -l version -d "Show version"
complete -c rez-next -l debug        -d "Enable debug mode"
complete -c rez-next -l quiet        -d "Suppress non-critical output"

# env subcommand
complete -c rez-next -n "__fish_seen_subcommand_from env" -s i -l interactive -d "Launch interactive shell"
complete -c rez-next -n "__fish_seen_subcommand_from env" -l nl -d "Ignore noerase variables"
complete -c rez-next -n "__fish_seen_subcommand_from env" -l si -d "Print resolved context info"

# search subcommand
complete -c rez-next -n "__fish_seen_subcommand_from search" -l type -d "Package type filter"
complete -c rez-next -n "__fish_seen_subcommand_from search" -l format -d "Output format (table/json/yaml)"
complete -c rez-next -n "__fish_seen_subcommand_from search" -l latest -d "Only latest versions"

# build subcommand
complete -c rez-next -n "__fish_seen_subcommand_from build" -s i -l install -d "Install after build"
complete -c rez-next -n "__fish_seen_subcommand_from build" -s c -l clean   -d "Clean build"
complete -c rez-next -n "__fish_seen_subcommand_from build" -l variants -d "Build specific variants"

# solve subcommand
complete -c rez-next -n "__fish_seen_subcommand_from solve" -l json -d "Output as JSON"
complete -c rez-next -n "__fish_seen_subcommand_from solve" -l verbose -d "Verbose solver output"
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

impl Default for PyRezData {
    fn default() -> Self {
        Self::new()
    }
}

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
            "fish" => Ok(FISH_COMPLETE.to_string()),
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
            "completions/fish" => Ok(FISH_COMPLETE.to_string()),
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
    fn test_fish_completion_not_empty() {
        assert!(
            FISH_COMPLETE.len() > 10,
            "fish completion should have meaningful content"
        );
        assert!(FISH_COMPLETE.contains("rez-next"));
        assert!(FISH_COMPLETE.contains("complete -c rez-next"));
    }

    #[test]
    fn test_resource_lookup_fish() {
        let content = match "completions/fish" {
            "completions/fish" => FISH_COMPLETE.to_string(),
            _ => panic!("not found"),
        };
        assert!(!content.is_empty());
        assert!(content.contains("fish completion"));
    }

    #[test]
    fn test_bash_completion_not_empty() {
        assert!(
            BASH_COMPLETE.len() > 10,
            "bash completion should have meaningful content"
        );
        assert!(BASH_COMPLETE.contains("_rez_next"));
    }

    #[test]
    fn test_zsh_completion_not_empty() {
        assert!(
            ZSH_COMPLETE.len() > 10,
            "zsh completion should have meaningful content"
        );
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

    // ─── PyRezData method tests (pure Rust calls without GIL) ────────────────

    #[test]
    fn test_rez_data_new_no_panic() {
        let _d = PyRezData::new();
    }

    #[test]
    fn test_rez_data_default_no_panic() {
        let _d = PyRezData::new();
    }


    #[test]
    fn test_rez_data_repr() {
        let d = PyRezData::new();
        assert_eq!(d.__repr__(), "RezData()");
    }

    #[test]
    fn test_rez_data_list_resources_contains_completions() {
        let d = PyRezData::new();
        let resources = d.list_resources();
        assert!(
            resources.iter().any(|r| r.starts_with("completions/")),
            "list_resources must include completions/*, got: {:?}",
            resources
        );
    }

    #[test]
    fn test_rez_data_list_resources_count() {
        let d = PyRezData::new();
        let r = d.list_resources();
        // 5 resources: bash, zsh, fish completions + example + config
        assert_eq!(r.len(), 5, "expected 5 resources, got {}", r.len());
    }

    #[test]
    fn test_rez_data_get_resource_bash_ok() {
        let d = PyRezData::new();
        let r = d.get_resource("completions/bash");
        assert!(r.is_ok());
        assert!(r.unwrap().contains("_rez_next"));
    }

    #[test]
    fn test_rez_data_get_resource_zsh_ok() {
        let d = PyRezData::new();
        let r = d.get_resource("completions/zsh");
        assert!(r.is_ok());
        assert!(r.unwrap().contains("rez-next"));
    }

    #[test]
    fn test_rez_data_get_resource_fish_ok() {
        let d = PyRezData::new();
        let r = d.get_resource("completions/fish");
        assert!(r.is_ok());
        let content = r.unwrap();
        assert!(content.contains("fish completion") || content.contains("rez-next"));
    }

    #[test]
    fn test_rez_data_get_resource_example_package_ok() {
        let d = PyRezData::new();
        let r = d.get_resource("examples/package.py");
        assert!(r.is_ok());
        assert!(r.unwrap().contains("name = \"my_package\""));
    }

    #[test]
    fn test_rez_data_get_resource_config_ok() {
        let d = PyRezData::new();
        let r = d.get_resource("config/rezconfig.py");
        assert!(r.is_ok());
        assert!(r.unwrap().contains("packages_path"));
    }

    #[test]
    fn test_rez_data_get_resource_unknown_errors() {
        let d = PyRezData::new();
        let r = d.get_resource("unknown/path.txt");
        assert!(r.is_err(), "unknown resource should return Err");
    }

    #[test]
    fn test_rez_data_get_example_package() {
        let d = PyRezData::new();
        let content = d.get_example_package();
        assert!(content.contains("name"));
        assert!(content.contains("version"));
    }

    #[test]
    fn test_rez_data_get_default_config() {
        let d = PyRezData::new();
        let content = d.get_default_config();
        assert!(content.contains("packages_path"));
        assert!(content.contains("local_packages_path"));
    }

    // ─── Module-level function tests ─────────────────────────────────────────

    #[test]
    fn test_list_data_resources_non_empty() {
        let resources = list_data_resources();
        assert!(!resources.is_empty());
    }

    #[test]
    fn test_get_data_resource_bash() {
        let r = get_data_resource("completions/bash");
        assert!(r.is_ok());
    }

    #[test]
    fn test_get_data_resource_unknown_errors() {
        let r = get_data_resource("no/such/resource");
        assert!(r.is_err());
    }

    // ─── write_completion_script fs test ─────────────────────────────────────

    #[test]
    fn test_write_completion_script_to_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("rez-complete.bash").to_str().unwrap().to_string();
        let d = PyRezData::new();
        let result = d.write_completion_script(&dest, Some("bash"));
        assert!(result.is_ok(), "write_completion_script should succeed: {:?}", result);
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("_rez_next"));
    }

    #[test]
    fn test_write_completion_script_unknown_shell_errors() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("bad.sh").to_str().unwrap().to_string();
        let d = PyRezData::new();
        let result = d.write_completion_script(&dest, Some("ksh"));
        assert!(result.is_err(), "unknown shell should return Err");
    }

    // ─── Additional data_bindings boundary/edge tests ─────────────────────────

    #[test]
    fn test_bash_complete_contains_compgen() {
        // bash completion must use compgen for option expansion
        assert!(BASH_COMPLETE.contains("compgen"), "bash completion must use compgen");
    }

    #[test]
    fn test_zsh_complete_contains_compdef() {
        // zsh completion must use #compdef directive
        assert!(ZSH_COMPLETE.contains("#compdef"), "zsh must have #compdef: {ZSH_COMPLETE}");
    }

    #[test]
    fn test_fish_complete_contains_complete_c() {
        // fish completion uses `complete -c rez-next`
        assert!(FISH_COMPLETE.contains("complete -c rez-next"), "fish completion must have `complete -c rez-next`");
    }

    #[test]
    fn test_example_package_py_has_commands_fn() {
        assert!(EXAMPLE_PACKAGE_PY.contains("def commands():"), "package.py must define commands()");
    }

    #[test]
    fn test_default_rezconfig_has_local_packages_path() {
        assert!(DEFAULT_REZCONFIG.contains("local_packages_path"), "rezconfig must define local_packages_path");
        assert!(DEFAULT_REZCONFIG.contains("default_shell"), "rezconfig must define default_shell");
    }

    #[test]
    fn test_rez_data_get_completion_script_bash_no_panic() {
        let d = PyRezData::new();
        let r = d.get_completion_script(Some("bash"));
        assert!(r.is_ok());
        let script = r.unwrap();
        assert!(!script.is_empty(), "bash completion must not be empty");
    }

    #[test]
    fn test_rez_data_get_completion_script_none_defaults_to_bash() {
        let d = PyRezData::new();
        let r = d.get_completion_script(None);
        assert!(r.is_ok(), "None shell should default to bash: {:?}", r);
        let content = r.unwrap();
        assert!(content.contains("_rez_next"), "default completion should be bash");
    }
}
