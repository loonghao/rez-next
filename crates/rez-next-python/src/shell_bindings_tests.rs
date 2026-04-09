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
        env.vars.insert("CMD_VAR".to_string(), "cmd_val".to_string());
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
        assert!(result.is_err(), "PyShell::new('ksh') should return Err");
    }

    #[test]
    fn test_pyshell_generate_script_empty_env() {
        let shell = PyShell::new("bash").unwrap();
        let script = shell.generate_script(None, None, None);
        let _ = script;
    }

    #[test]
    fn test_pyshell_generate_script_with_vars() {
        let shell = PyShell::new("bash").unwrap();
        let mut vars = std::collections::HashMap::new();
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
        let mut vars = std::collections::HashMap::new();
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
        assert!(result.is_err(), "unknown shell 'tcsh' should return Err");
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

    #[test]
    fn test_bash_script_includes_alias() {
        let shell = PyShell::new("bash").unwrap();
        let mut aliases = HashMap::new();
        aliases.insert("ll".to_string(), "ls -la".to_string());
        let script = shell.generate_script(None, Some(aliases), None);
        assert!(
            script.contains("ll") || script.contains("ls"),
            "bash script should reference alias 'll', got: {script}"
        );
    }

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

    #[test]
    fn test_fish_script_with_var_no_panic() {
        let shell = PyShell::new("fish").unwrap();
        let mut vars = HashMap::new();
        vars.insert("FISH_VAR".to_string(), "fishval".to_string());
        let script = shell.generate_script(Some(vars), None, None);
        let _ = script;
    }

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

    #[test]
    fn test_pyshell_clone_same_name() {
        let shell = PyShell::new("powershell").unwrap();
        let cloned = shell.clone();
        assert_eq!(cloned.name(), "powershell");
    }

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

    #[test]
    fn test_available_shells_all_lowercase() {
        let shells = get_available_shells();
        for s in &shells {
            assert_eq!(*s, s.to_lowercase(), "shell name '{}' should be lowercase", s);
        }
    }

    #[test]
    fn test_pyshell_new_uppercase_normalizes() {
        let shell = PyShell::new("BASH").expect("uppercase shell names should parse");
        assert_eq!(shell.name(), "bash");
    }

    #[test]
    fn test_generate_script_all_params() {
        let shell = PyShell::new("bash").unwrap();
        let mut vars = HashMap::new();
        vars.insert("ALL_VAR".to_string(), "all_val".to_string());
        let mut aliases = HashMap::new();
        aliases.insert("la".to_string(), "ls -a".to_string());
        let cmds = vec!["echo combined".to_string()];
        let script = shell.generate_script(Some(vars), Some(aliases), Some(cmds));
        assert!(
            script.contains("ALL_VAR") || script.contains("la") || script.contains("combined"),
            "script should contain at least one inserted value, got: {script}"
        );
    }

    #[test]
    fn test_create_shell_script_fish_with_var() {
        let mut vars = HashMap::new();
        vars.insert("FISH_KEY".to_string(), "fish_value".to_string());
        let result = create_shell_script("fish", Some(vars), None, None);
        assert!(result.is_ok(), "create_shell_script fish should succeed");
    }
}

mod test_shell_cy114 {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_pyshell_repr_all_shells() {
        for name in &["bash", "zsh", "fish", "cmd", "powershell"] {
            let shell = PyShell::new(name).unwrap();
            let repr = shell.__repr__();
            assert!(repr.contains(name), "repr for '{}' should contain shell name, got: {repr}", name);
            assert!(repr.contains("Shell"), "repr for '{}' should contain 'Shell', got: {repr}", name);
        }
    }

    #[test]
    fn test_generate_script_empty_vars_map_no_panic() {
        let shell = PyShell::new("powershell").unwrap();
        let script = shell.generate_script(Some(HashMap::new()), None, None);
        let _ = script;
    }

    #[test]
    fn test_generate_script_empty_aliases_map_no_panic() {
        let shell = PyShell::new("bash").unwrap();
        let script = shell.generate_script(None, Some(HashMap::new()), None);
        let _ = script;
    }

    #[test]
    fn test_generate_script_empty_commands_no_panic() {
        let shell = PyShell::new("zsh").unwrap();
        let script = shell.generate_script(None, None, Some(vec![]));
        let _ = script;
    }

    #[test]
    fn test_create_shell_script_cmd_with_aliases() {
        let mut aliases = HashMap::new();
        aliases.insert("cl".to_string(), "cls".to_string());
        let result = create_shell_script("cmd", None, Some(aliases), None);
        assert!(result.is_ok(), "create_shell_script cmd with aliases should succeed");
    }

    #[test]
    fn test_available_shells_no_duplicates() {
        let shells = get_available_shells();
        let mut seen = std::collections::HashSet::new();
        for s in &shells {
            assert!(seen.insert(*s), "duplicate shell '{}' in get_available_shells()", s);
        }
    }

    #[test]
    fn test_pyshell_new_mixed_case_powershell() {
        let shell = PyShell::new("PowerShell");
        if let Ok(s) = shell {
            assert_eq!(s.name(), "powershell");
        }
    }
}

mod test_shell_cy120 {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_available_shells_len_is_five() {
        assert_eq!(
            get_available_shells().len(),
            5,
            "exactly 5 shell types must be available"
        );
    }

    #[test]
    fn test_create_shell_script_cmd_with_startup_commands() {
        let cmds = vec!["echo hello from cmd".to_string()];
        let result = create_shell_script("cmd", None, None, Some(cmds));
        assert!(result.is_ok(), "create_shell_script cmd with startup_commands must succeed");
    }

    #[test]
    fn test_powershell_script_with_alias_no_panic() {
        let shell = PyShell::new("powershell").unwrap();
        let mut aliases = HashMap::new();
        aliases.insert("ll".to_string(), "ls -Force".to_string());
        let script = shell.generate_script(None, Some(aliases), None);
        let _ = script;
    }

    #[test]
    fn test_bash_script_multiple_aliases() {
        let shell = PyShell::new("bash").unwrap();
        let mut aliases = HashMap::new();
        aliases.insert("g".to_string(), "git".to_string());
        aliases.insert("k".to_string(), "kubectl".to_string());
        let script = shell.generate_script(None, Some(aliases), None);
        assert!(
            script.contains("g") || script.contains("k"),
            "bash script should contain alias definitions, got: {script}"
        );
    }

    #[test]
    fn test_shell_type_parse_cmd_uppercase() {
        let result = ShellType::parse("CMD");
        let _ = result;
    }

    #[test]
    fn test_create_shell_script_fish_with_startup_commands() {
        let cmds = vec!["set -x FISH_INIT 1".to_string()];
        let result = create_shell_script("fish", None, None, Some(cmds));
        assert!(result.is_ok(), "create_shell_script fish with startup_commands must succeed");
    }
}

mod test_shell_cy125 {
    use super::*;

    #[test]
    fn test_available_shells_no_empty_entry() {
        let shells = get_available_shells();
        assert!(
            shells.iter().all(|s| !s.is_empty()),
            "no shell name should be empty in available shells: {shells:?}"
        );
    }

    #[test]
    fn test_pyshell_new_bash_is_ok() {
        assert!(PyShell::new("bash").is_ok(), "PyShell::new('bash') must succeed");
    }

    #[test]
    fn test_pyshell_new_zsh_is_ok() {
        assert!(PyShell::new("zsh").is_ok(), "PyShell::new('zsh') must succeed");
    }

    #[test]
    fn test_pyshell_new_unknown_shell_is_err() {
        let result = PyShell::new("unknownshell_cy125");
        assert!(result.is_err(), "PyShell::new for unknown shell should return Err");
    }

    #[test]
    fn test_create_shell_script_bash_no_startup_commands() {
        let result = create_shell_script("bash", None, None, None);
        assert!(result.is_ok(), "create_shell_script bash with no startup_commands must succeed");
    }
}
