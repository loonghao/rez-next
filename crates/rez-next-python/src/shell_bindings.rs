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
            env.vars.insert("PS_VAR".to_string(), "ps_val".to_string());
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

    mod test_py_shell {
        use super::*;

        #[test]
        fn test_pyshell_name_matches_input() {
            for name in &["bash", "zsh", "fish", "cmd", "powershell"] {
                let shell = PyShell::new(name).unwrap();
                assert_eq!(shell.name(), *name, "name() should return '{}'", name);
            }
        }

        #[test]
        fn test_pyshell_repr_format() {
            let shell = PyShell::new("bash").unwrap();
            let r = shell.__repr__();
            assert!(r.contains("Shell"), "repr must contain 'Shell', got {r}");
            assert!(r.contains("bash"), "repr must contain 'bash', got {r}");
        }

        #[test]
        fn test_pyshell_unknown_type_errors() {
            let result = PyShell::new("ksh");
            assert!(
                result.is_err(),
                "PyShell::new('ksh') should return Err"
            );
        }

        #[test]
        fn test_pyshell_generate_script_empty_env() {
            let shell = PyShell::new("bash").unwrap();
            let script = shell.generate_script(None, None, None);
            // Must not panic; result can be empty or not
            let _ = script;
        }

        #[test]
        fn test_pyshell_generate_script_with_vars() {
            let shell = PyShell::new("bash").unwrap();
            let mut vars = HashMap::new();
            vars.insert("MYVAR".to_string(), "myval".to_string());
            let script = shell.generate_script(Some(vars), None, None);
            assert!(
                script.contains("MYVAR"),
                "script should contain MYVAR, got: {script}"
            );
        }

        #[test]
        fn test_pyshell_generate_script_with_startup_commands() {
            let shell = PyShell::new("bash").unwrap();
            let cmds = vec!["echo hello".to_string()];
            let script = shell.generate_script(None, None, Some(cmds));
            // Should not panic; content depends on implementation
            let _ = script;
        }
    }

    mod test_create_shell_script {
        use super::*;

        #[test]
        fn test_create_shell_script_bash_no_vars() {
            let result = create_shell_script("bash", None, None, None);
            assert!(result.is_ok(), "create_shell_script bash should succeed");
        }

        #[test]
        fn test_create_shell_script_powershell_with_var() {
            let mut vars = HashMap::new();
            vars.insert("PWSH_VAR".to_string(), "pwsh_val".to_string());
            let result = create_shell_script("powershell", Some(vars), None, None);
            assert!(result.is_ok());
            let script = result.unwrap();
            assert!(
                script.contains("PWSH_VAR"),
                "powershell script should have PWSH_VAR, got: {script}"
            );
        }

        #[test]
        fn test_create_shell_script_unknown_shell_errors() {
            let result = create_shell_script("tcsh", None, None, None);
            assert!(
                result.is_err(),
                "unknown shell 'tcsh' should return Err"
            );
        }

        #[test]
        fn test_create_shell_script_all_known_shells_ok() {
            for name in &["bash", "zsh", "fish", "cmd", "powershell"] {
                let result = create_shell_script(name, None, None, None);
                assert!(
                    result.is_ok(),
                    "create_shell_script({}) should succeed",
                    name
                );
            }
        }
    }

    mod test_shell_extra_cy98 {
        use super::*;
        use std::collections::HashMap;

        /// aliases passed to generate_script appear in the bash script
        #[test]
        fn test_bash_script_includes_alias() {
            let shell = PyShell::new("bash").unwrap();
            let mut aliases = HashMap::new();
            aliases.insert("ll".to_string(), "ls -la".to_string());
            let script = shell.generate_script(None, Some(aliases), None);
            // alias definition should appear in bash script
            assert!(
                script.contains("ll") || script.contains("ls"),
                "bash script should reference alias 'll', got: {script}"
            );
        }

        /// startup commands appear in bash script
        #[test]
        fn test_bash_startup_commands_in_script() {
            let shell = PyShell::new("bash").unwrap();
            let cmds = vec!["export STARTUP=1".to_string()];
            let script = shell.generate_script(None, None, Some(cmds));
            assert!(
                script.contains("STARTUP"),
                "bash script should contain startup command content, got: {script}"
            );
        }

        /// fish shell script generation does not panic
        #[test]
        fn test_fish_script_with_var_no_panic() {
            let shell = PyShell::new("fish").unwrap();
            let mut vars = HashMap::new();
            vars.insert("FISH_VAR".to_string(), "fishval".to_string());
            let script = shell.generate_script(Some(vars), None, None);
            // must not panic; fish script should contain the var
            let _ = script;
        }

        /// zsh shell produces non-empty script with a var
        #[test]
        fn test_zsh_script_with_var_non_empty() {
            let mut env = RexEnvironment::new();
            env.vars.insert("ZSH_VAR".to_string(), "zshval".to_string());
            let script = generate_shell_script(&env, &ShellType::Zsh);
            assert!(
                script.contains("ZSH_VAR"),
                "zsh script should contain ZSH_VAR, got: {script}"
            );
        }

        /// PyShell clone has same name
        #[test]
        fn test_pyshell_clone_same_name() {
            let shell = PyShell::new("powershell").unwrap();
            let cloned = shell.clone();
            assert_eq!(cloned.name(), "powershell");
        }

        /// create_shell_script with aliases for bash succeeds
        #[test]
        fn test_create_shell_script_bash_with_aliases() {
            let mut aliases = HashMap::new();
            aliases.insert("gs".to_string(), "git status".to_string());
            let result = create_shell_script("bash", None, Some(aliases), None);
            assert!(result.is_ok(), "create_shell_script with aliases should succeed");
            let script = result.unwrap();
            assert!(
                script.contains("gs") || script.contains("git"),
                "script should contain alias, got: {script}"
            );
        }

        /// create_shell_script with startup_commands for zsh
        #[test]
        fn test_create_shell_script_zsh_with_startup_commands() {
            let cmds = vec!["echo rez-next".to_string()];
            let result = create_shell_script("zsh", None, None, Some(cmds));
            assert!(result.is_ok());
            let script = result.unwrap();
            assert!(
                script.contains("rez-next"),
                "zsh startup command should appear in script: {script}"
            );
        }
    }

    mod test_shell_extra_cy104 {
        use super::*;
        use std::collections::HashMap;

        /// cmd shell generate_script with var does not panic
        #[test]
        fn test_cmd_generate_script_with_var_no_panic() {
            let shell = PyShell::new("cmd").unwrap();
            let mut vars = HashMap::new();
            vars.insert("CMD_TEST".to_string(), "cmd_value".to_string());
            let script = shell.generate_script(Some(vars), None, None);
            assert!(
                script.contains("CMD_TEST"),
                "cmd script should reference CMD_TEST, got: {script}"
            );
        }

        /// get_available_shells() returns lowercase names only
        #[test]
        fn test_available_shells_all_lowercase() {
            let shells = get_available_shells();
            for s in &shells {
                assert_eq!(
                    *s,
                    s.to_lowercase(),
                    "shell name '{}' should be lowercase",
                    s
                );
            }
        }

        /// PyShell::new normalizes uppercase input via case-insensitive parsing
        #[test]
        fn test_pyshell_new_uppercase_normalizes() {
            let shell = PyShell::new("BASH").expect("uppercase shell names should parse");
            assert_eq!(shell.name(), "bash");
        }


        /// generate_script with both vars AND aliases AND commands simultaneously
        #[test]
        fn test_generate_script_all_params() {
            let shell = PyShell::new("bash").unwrap();
            let mut vars = HashMap::new();
            vars.insert("ALL_VAR".to_string(), "all_val".to_string());
            let mut aliases = HashMap::new();
            aliases.insert("la".to_string(), "ls -a".to_string());
            let cmds = vec!["echo combined".to_string()];
            let script = shell.generate_script(Some(vars), Some(aliases), Some(cmds));
            // Must not panic; all inputs processed
            assert!(
                script.contains("ALL_VAR") || script.contains("la") || script.contains("combined"),
                "script should contain at least one inserted value, got: {script}"
            );
        }

        /// create_shell_script for fish with vars succeeds
        #[test]
        fn test_create_shell_script_fish_with_var() {
            let mut vars = HashMap::new();
            vars.insert("FISH_KEY".to_string(), "fish_value".to_string());
            let result = create_shell_script("fish", Some(vars), None, None);
            assert!(
                result.is_ok(),
                "create_shell_script fish should succeed"
            );
        }
    }
}
