//! Python bindings for shell script generation (rez.shell)

use pyo3::prelude::*;
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
use std::collections::HashMap;

/// Python wrapper for shell script generation.
/// Equivalent to `rez.shell` module.
#[pyclass(name = "Shell", from_py_object)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    mod test_shell_type_parse {
        use super::*;

        #[test]
        fn test_known_shell_types_parse() {
            for name in &["bash", "zsh", "fish", "cmd", "powershell"] {
                assert!(
                    ShellType::parse(name).is_some(),
                    "ShellType::parse('{}') should succeed",
                    name
                );
            }
        }

        #[test]
        fn test_unknown_shell_type_returns_none() {
            assert!(ShellType::parse("ksh").is_none());
            assert!(ShellType::parse("").is_none());
            assert!(ShellType::parse("tcsh").is_none());
        }
    }

    mod test_shell_script_generation {
        use super::*;
        use std::collections::HashMap;

        #[test]
        fn test_bash_script_sets_env_var() {
            let mut env = RexEnvironment::new();
            env.vars.insert("MY_VAR".to_string(), "hello".to_string());
            let script = generate_shell_script(&env, &ShellType::Bash);
            assert!(
                script.contains("MY_VAR") && script.contains("hello"),
                "bash script should contain MY_VAR=hello, got:\n{}",
                script
            );
        }

        #[test]
        fn test_powershell_script_sets_env_var() {
            let mut env = RexEnvironment::new();
            env.vars
                .insert("PS_VAR".to_string(), "ps_val".to_string());
            let script = generate_shell_script(&env, &ShellType::PowerShell);
            assert!(
                script.contains("PS_VAR"),
                "powershell script should reference PS_VAR, got:\n{}",
                script
            );
        }

        #[test]
        fn test_cmd_script_sets_env_var() {
            let mut env = RexEnvironment::new();
            env.vars
                .insert("CMD_VAR".to_string(), "cmd_val".to_string());
            let script = generate_shell_script(&env, &ShellType::Cmd);
            assert!(
                script.contains("CMD_VAR"),
                "cmd script should reference CMD_VAR, got:\n{}",
                script
            );
        }

        #[test]
        fn test_empty_env_generates_non_panic_script() {
            let env = RexEnvironment::new();
            // Must not panic regardless of shell type
            for st in &[
                ShellType::Bash,
                ShellType::Zsh,
                ShellType::Fish,
                ShellType::Cmd,
                ShellType::PowerShell,
            ] {
                let _ = generate_shell_script(&env, st);
            }
        }

        #[test]
        fn test_multiple_vars_all_appear_in_script() {
            let mut vars = HashMap::new();
            vars.insert("FOO".to_string(), "1".to_string());
            vars.insert("BAR".to_string(), "2".to_string());
            let mut env = RexEnvironment::new();
            env.vars = vars;
            let script = generate_shell_script(&env, &ShellType::Bash);
            assert!(script.contains("FOO"), "FOO missing from script");
            assert!(script.contains("BAR"), "BAR missing from script");
        }
    }

    mod test_get_available_shells {
        use super::*;

        #[test]
        fn test_available_shells_contains_all_types() {
            let shells = get_available_shells();
            for expected in &["bash", "zsh", "fish", "cmd", "powershell"] {
                assert!(
                    shells.contains(expected),
                    "get_available_shells() should contain '{}'",
                    expected
                );
            }
        }

        #[test]
        fn test_available_shells_count() {
            assert_eq!(get_available_shells().len(), 5);
        }
    }

    mod test_get_current_shell {
        use super::*;

        #[test]
        fn test_current_shell_returns_known_type() {
            let shell = get_current_shell();
            let known = ["bash", "zsh", "fish", "cmd", "powershell"];
            assert!(
                known.contains(&shell.as_str()),
                "get_current_shell() returned unknown shell: '{}'",
                shell
            );
        }
    }
}
