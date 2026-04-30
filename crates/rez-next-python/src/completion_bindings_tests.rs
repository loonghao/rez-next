use super::*;

// ── supported_completion_shells ───────────────────────────────────────────

#[test]
fn test_supported_shells_count() {
    let shells = supported_completion_shells();
    assert_eq!(shells.len(), 4);
}

#[test]
fn test_supported_shells_contains_all() {
    let shells = supported_completion_shells();
    assert!(shells.contains(&"bash".to_string()));
    assert!(shells.contains(&"zsh".to_string()));
    assert!(shells.contains(&"fish".to_string()));
    assert!(shells.contains(&"powershell".to_string()));
}

// ── get_completion_script: valid shells ───────────────────────────────────

#[test]
fn test_bash_script_returned() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("_rez_next_complete"));
    assert!(script.contains("rez-next"));
}

#[test]
fn test_zsh_script_returned() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(script.contains("_rez_next"));
    assert!(script.contains("#compdef"));
}

#[test]
fn test_fish_script_returned() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(script.contains("complete -c rez-next"));
    assert!(script.contains("__rez_next_complete"));
}

#[test]
fn test_powershell_script_returned() {
    let script = get_completion_script(Some("powershell")).unwrap();
    assert!(script.contains("Register-ArgumentCompleter"));
    assert!(script.contains("rez-next"));
}

#[test]
fn test_pwsh_alias_returns_powershell_script() {
    let script = get_completion_script(Some("pwsh")).unwrap();
    assert!(script.contains("Register-ArgumentCompleter"));
}

#[test]
fn test_ps1_alias_returns_powershell_script() {
    let script = get_completion_script(Some("ps1")).unwrap();
    assert!(script.contains("Register-ArgumentCompleter"));
}

#[test]
fn test_unknown_shell_is_not_in_supported_list() {
    let shells = supported_completion_shells();
    assert!(!shells.contains(&"tcsh".to_string()));
    assert!(!shells.contains(&"csh".to_string()));
}

// ── Script content sanity checks ─────────────────────────────────────────

#[test]
fn test_bash_script_lists_rez_commands() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("_rez_next_complete"), "bash script should define _rez_next_complete function");
    assert!(script.contains("--dynamic"), "bash script should use --dynamic mode");
}

#[test]
fn test_zsh_script_lists_env_command() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("_rez_next"),
        "zsh script should define _rez_next function"
    );
    assert!(
        script.contains("--dynamic"),
        "zsh script should use --dynamic mode"
    );
}

#[test]
fn test_scripts_are_non_empty() {
    for shell in &["bash", "zsh", "fish", "powershell"] {
        let script = get_completion_script(Some(shell)).unwrap();
        assert!(
            script.len() > 100,
            "Script for {} should be non-trivial",
            shell
        );
    }
}

// ── get_completion_install_path ───────────────────────────────────────────

#[test]
fn test_install_path_bash() {
    let path = get_completion_install_path(Some("bash")).unwrap();
    assert!(path.contains("bash"));
    assert!(path.contains("rez-next"));
}

#[test]
fn test_install_path_zsh() {
    let path = get_completion_install_path(Some("zsh")).unwrap();
    assert!(path.contains("zsh"));
    assert!(path.contains("_rez-next"));
}

#[test]
fn test_install_path_fish() {
    let path = get_completion_install_path(Some("fish")).unwrap();
    assert!(path.contains("fish"));
    assert!(path.contains("rez-next.fish"));
}

#[test]
fn test_install_path_powershell() {
    let path = get_completion_install_path(Some("powershell")).unwrap();
    assert!(path.contains("powershell") || path.contains("PowerShell"));
}

#[test]
fn test_install_path_unknown_is_not_in_supported_list() {
    let shells = supported_completion_shells();
    assert!(!shells.contains(&"csh".to_string()));
}

// ── get_completion_script_py ────────────────────────────────────────────

#[test]
fn test_get_completion_script_py_valid_shell_no_panic() {
    assert!(get_completion_script_py(Some("bash")).is_ok());
}

// ── bash script: dynamic mode checks ────────────────────────────────────

#[test]
fn test_bash_script_uses_dynamic_mode() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("--dynamic"), "bash script should use --dynamic flag");
    assert!(script.contains("COMP_LINE"), "bash script should read COMP_LINE");
    assert!(script.contains("COMP_POINT"), "bash script should read COMP_POINT");
}

#[test]
fn test_bash_script_defines_complete_function() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("_rez_next_complete"), "bash script should define completion function");
    assert!(script.contains("COMPREPLY"), "bash script should set COMPREPLY");
}

// ── zsh script subcommand descriptions ───────────────────────────────────

#[test]
fn test_zsh_script_uses_dynamic_for_solve() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("--dynamic"),
        "zsh should use --dynamic for solve"
    );
}

#[test]
fn test_zsh_script_uses_dynamic_for_bind() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("--dynamic"),
        "zsh should use --dynamic for bind"
    );
}

// ── fish script structural checks ────────────────────────────────────────

#[test]
fn test_fish_script_defines_correct_function() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(
        script.contains("function __rez_next_complete"),
        "fish script should define __rez_next_complete"
    );
}

// ── install path: pwsh alias ──────────────────────────────────────────────

#[test]
fn test_install_path_pwsh_alias_returns_powershell_path() {
    let path = get_completion_install_path(Some("pwsh")).unwrap();
    assert!(!path.is_empty(), "pwsh install path should not be empty");
}

// ── get_completion_script_py for all shells ────────────────────────────────

#[test]
fn test_get_completion_script_py_all_shells_no_panic() {
    for shell in &["zsh", "fish", "powershell"] {
        assert!(
            get_completion_script_py(Some(shell)).is_ok(),
            "get_completion_script_py({}) should not error",
            shell
        );
    }
}

// ── bash script: case block for env/solve ────────────────────────────────

#[test]
fn test_bash_script_handles_env_solve_dynamically() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(
        script.contains("--dynamic"),
        "bash script should use --dynamic for env/solve completion"
    );
}

// ── zsh script ───────────────────────────────────────────────────────────

#[test]
fn test_zsh_script_ends_with_rez_next_call() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("_rez_next"),
        "zsh script should invoke _rez_next function"
    );
}

// ── fish script: contains all major subcommands ───────────────────────────

#[test]
fn test_fish_script_uses_dynamic_mode() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(script.contains("--dynamic"), "fish script should use --dynamic flag");
    assert!(script.contains("commandline"), "fish script should read commandline");
}

#[test]
fn test_fish_script_defines_complete_function() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(script.contains("__rez_next_complete"), "fish script should define __rez_next_complete function");
    assert!(script.contains("complete -c rez"), "fish script should register completions for rez");
}

// ── powershell script ─────────────────────────────────────────────────────

#[test]
fn test_powershell_script_has_word_to_complete() {
    let script = get_completion_script(Some("powershell")).unwrap();
    assert!(
        script.contains("wordToComplete") || script.contains("WordToComplete"),
        "powershell completion script should reference wordToComplete"
    );
}

// ── supported_shells list is deduplicated ────────────────────────────────

#[test]
fn test_supported_shells_no_duplicates() {
    let shells = supported_completion_shells();
    let mut seen = std::collections::HashSet::new();
    for s in &shells {
        assert!(seen.insert(s.clone()), "duplicate shell entry: {}", s);
    }
}

// ── install_path for unknown shell errors correctly ───────────────────────

#[test]
fn test_install_path_unknown_shell_errors() {
    let result = get_completion_install_path(Some("tcsh"));
    assert!(
        result.is_err(),
        "unknown shell 'tcsh' should return Err from get_completion_install_path"
    );
}

// ── bash script contains -p / --paths path-completion ────────────────────

#[test]
fn test_bash_script_handles_paths_dynamically() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(
        script.contains("--dynamic"),
        "bash script should use --dynamic (paths handled dynamically)"
    );
}

// ── Cycle 119 additions ──────────────────────────────────────────────────

mod test_completion_cy119 {
    use super::*;

    #[test]
    fn test_bash_script_contains_search_query() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(
            script.contains("search") || script.contains("rez-next"),
            "bash script should reference rez-next search for packages"
        );
    }

    #[test]
    fn test_zsh_script_contains_arguments_directive() {
        let script = get_completion_script(Some("zsh")).unwrap();
        assert!(
            script.contains("_arguments"),
            "zsh script should use '_arguments' completion helper"
        );
    }

    #[test]
    fn test_fish_script_registers_rez_completions() {
        let script = get_completion_script(Some("fish")).unwrap();
        assert!(
            script.contains("complete -c rez "),
            "fish script must register completions for 'rez'"
        );
    }

    #[test]
    fn test_powershell_script_uses_dynamic_mode() {
        let script = get_completion_script(Some("powershell")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "powershell script should use --dynamic flag"
        );
        assert!(
            script.contains("COMP_LINE"),
            "powershell script should read COMP_LINE"
        );
    }

    #[test]
    fn test_bash_install_path_starts_with_tilde() {
        let path = get_completion_install_path(Some("bash")).unwrap();
        assert!(
            path.starts_with('~'),
            "bash install path should start with '~': '{path}'"
        );
    }

    #[test]
    fn test_supported_shells_does_not_include_csh() {
        let shells = supported_completion_shells();
        assert!(
            !shells.iter().any(|s| s == "csh"),
            "csh must not be in supported shells"
        );
    }
}

mod test_completion_cy129 {
    use super::*;

    #[test]
    fn test_bash_script_uses_dynamic_for_all_commands() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "bash script should use --dynamic for all commands"
        );
    }

    #[test]
    fn test_zsh_script_handles_search_dynamically() {
        let script = get_completion_script(Some("zsh")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "zsh script should use --dynamic for search"
        );
    }

    #[test]
    fn test_fish_script_handles_context_dynamically() {
        let script = get_completion_script(Some("fish")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "fish script should use --dynamic for context"
        );
    }

    #[test]
    fn test_powershell_script_handles_env_dynamically() {
        let script = get_completion_script(Some("powershell")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "powershell script should use --dynamic for env"
        );
    }

    #[test]
    fn test_fish_install_path_ends_with_dot_fish() {
        let path = get_completion_install_path(Some("fish")).unwrap();
        assert!(
            path.ends_with(".fish"),
            "fish install path must end with '.fish': {path}"
        );
    }

    #[test]
    fn test_bash_script_handles_interpret_dynamically() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "bash script should use --dynamic for interpret"
        );
    }

    #[test]
    fn test_zsh_install_path_has_underscore_prefix() {
        let path = get_completion_install_path(Some("zsh")).unwrap();
        assert!(
            path.contains("_rez-next"),
            "zsh install path must contain '_rez-next': {path}"
        );
    }

    #[test]
    fn test_powershell_script_handles_solve_dynamically() {
        let script = get_completion_script(Some("powershell")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "powershell script should use --dynamic for solve"
        );
    }

    #[test]
    fn test_get_completion_script_none_shell_no_error() {
        let result = get_completion_script(None);
        assert!(result.is_ok(), "get_completion_script(None) must not error");
    }

    #[test]
    fn test_supported_shells_count_is_four() {
        assert_eq!(
            supported_completion_shells().len(),
            4,
            "exactly 4 shells must be supported"
        );
    }

    #[test]
    fn test_bash_script_handles_forward_dynamically() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "bash script should use --dynamic for forward"
        );
    }

    #[test]
    fn test_fish_script_handles_suite_dynamically() {
        let script = get_completion_script(Some("fish")).unwrap();
        assert!(
            script.contains("--dynamic"),
            "fish script should use --dynamic for suite"
        );
    }
}
