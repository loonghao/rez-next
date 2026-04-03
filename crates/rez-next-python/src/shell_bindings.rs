//! Python bindings for shell script generation (rez.shell)

use pyo3::prelude::*;
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
use std::collections::HashMap;

/// Python wrapper for shell script generation.
/// Equivalent to `rez.shell` module.
#[pyclass(name = "Shell")]
#[derive(Clone)]
pub struct PyShell {
    shell_type: ShellType,
}

#[pymethods]
impl PyShell {
    /// Create a Shell instance for the given shell type.
    /// shell_type: "bash", "zsh", "fish", "cmd", "powershell"
    #[new]
    pub fn new(shell_type: &str) -> PyResult<Self> {
        let st = ShellType::parse(shell_type).ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown shell type '{}'. Use: bash, zsh, fish, cmd, powershell",
                shell_type
            ))
        })?;
        Ok(Self { shell_type: st })
    }

    /// Generate activation script from env vars and aliases.
    /// Returns the script as a string.
    #[pyo3(signature = (vars=None, aliases=None, commands=None))]
    pub fn generate_script(
        &self,
        vars: Option<HashMap<String, String>>,
        aliases: Option<HashMap<String, String>>,
        commands: Option<Vec<String>>,
    ) -> String {
        let mut env = RexEnvironment::new();
        if let Some(v) = vars {
            env.vars = v;
        }
        if let Some(a) = aliases {
            env.aliases = a;
        }
        if let Some(c) = commands {
            env.startup_commands = c;
        }
        generate_shell_script(&env, &self.shell_type)
    }

    /// Shell type name
    #[getter]
    pub fn name(&self) -> &str {
        match self.shell_type {
            ShellType::Bash => "bash",
            ShellType::Zsh => "zsh",
            ShellType::Fish => "fish",
            ShellType::Cmd => "cmd",
            ShellType::PowerShell => "powershell",
        }
    }

    fn __repr__(&self) -> String {
        format!("Shell('{}')", self.name())
    }
}

/// Generate a shell activation script string.
/// Equivalent to `rez.shell.create_shell(shell).activate_code(context)`
#[pyfunction]
#[pyo3(signature = (shell_type, vars=None, aliases=None, startup_commands=None))]
pub fn create_shell_script(
    shell_type: &str,
    vars: Option<HashMap<String, String>>,
    aliases: Option<HashMap<String, String>>,
    startup_commands: Option<Vec<String>>,
) -> PyResult<String> {
    let st = ShellType::parse(shell_type).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown shell type '{}'. Use: bash, zsh, fish, cmd, powershell",
            shell_type
        ))
    })?;

    let mut env = RexEnvironment::new();
    if let Some(v) = vars {
        env.vars = v;
    }
    if let Some(a) = aliases {
        env.aliases = a;
    }
    if let Some(c) = startup_commands {
        env.startup_commands = c;
    }

    Ok(generate_shell_script(&env, &st))
}

/// List available shell types
#[pyfunction]
pub fn get_available_shells() -> Vec<&'static str> {
    vec!["bash", "zsh", "fish", "cmd", "powershell"]
}

/// Detect current shell type from environment
#[pyfunction]
pub fn get_current_shell() -> String {
    // Check SHELL env var (Unix)
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("bash") {
            return "bash".to_string();
        } else if shell.contains("zsh") {
            return "zsh".to_string();
        } else if shell.contains("fish") {
            return "fish".to_string();
        }
    }
    // Check Windows shell
    if std::env::var("PSModulePath").is_ok() {
        return "powershell".to_string();
    }
    if cfg!(windows) {
        return "cmd".to_string();
    }
    "bash".to_string()
}
