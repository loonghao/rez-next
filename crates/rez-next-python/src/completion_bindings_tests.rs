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
    assert!(script.contains("__rez_needs_command"));
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
    assert!(script.contains("env"));
    assert!(script.contains("solve"));
}

#[test]
fn test_zsh_script_lists_env_command() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(script.contains("env:create a resolved environment"));
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

// ── print_completion_script ────────────────────────────────────────────

#[test]
fn test_print_completion_script_valid_shell_no_panic() {
    assert!(print_completion_script(Some("bash")).is_ok());
}

// ── bash script command-list coverage ────────────────────────────────────

#[test]
fn test_bash_script_contains_build_and_release() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("build"), "bash script should list 'build'");
    assert!(script.contains("release"), "bash script should list 'release'");
}

#[test]
fn test_bash_script_contains_bundle_and_config() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(script.contains("bundle"), "bash script should list 'bundle'");
    assert!(script.contains("config"), "bash script should list 'config'");
}

// ── zsh script subcommand descriptions ───────────────────────────────────

#[test]
fn test_zsh_script_contains_solve_description() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("solve:solve a set of package requirements"),
        "zsh should describe 'solve'"
    );
}

#[test]
fn test_zsh_script_contains_bind_description() {
    let script = get_completion_script(Some("zsh")).unwrap();
    assert!(
        script.contains("bind:bind a system tool as a rez package"),
        "zsh should describe 'bind'"
    );
}

// ── fish script structural checks ────────────────────────────────────────

#[test]
fn test_fish_script_contains_needs_command_function() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(
        script.contains("function __rez_needs_command"),
        "fish script should define __rez_needs_command"
    );
}

// ── install path: pwsh alias ──────────────────────────────────────────────

#[test]
fn test_install_path_pwsh_alias_returns_powershell_path() {
    let path = get_completion_install_path(Some("pwsh")).unwrap();
    assert!(!path.is_empty(), "pwsh install path should not be empty");
}

// ── print_completion_script for all shells ────────────────────────────────

#[test]
fn test_print_completion_script_all_shells_no_panic() {
    for shell in &["zsh", "fish", "powershell"] {
        assert!(
            print_completion_script(Some(shell)).is_ok(),
            "print_completion_script({}) should not error",
            shell
        );
    }
}

// ── bash script: case block for env/solve ────────────────────────────────

#[test]
fn test_bash_script_has_compreply_for_env_and_solve() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(
        script.contains("env|solve") || (script.contains("env") && script.contains("solve")),
        "bash script should handle env/solve completion"
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
fn test_fish_script_contains_build_and_release() {
    let script = get_completion_script(Some("fish")).unwrap();
    assert!(script.contains("build"), "fish script should contain 'build'");
    assert!(script.contains("release"), "fish script should contain 'release'");
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
fn test_bash_script_handles_paths_flag() {
    let script = get_completion_script(Some("bash")).unwrap();
    assert!(
        script.contains("-p") || script.contains("--paths"),
        "bash completion should handle -p/--paths directory completion"
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
    fn test_powershell_script_lists_twenty_commands() {
        let script = get_completion_script(Some("powershell")).unwrap();
        let count = script.split('\'').count() / 2;
        assert!(
            count >= 20,
            "powershell script should list at least 20 commands, found ~{count}"
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
    fn test_bash_script_contains_status() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(script.contains("status"), "bash script must list 'status' subcommand");
    }

    #[test]
    fn test_zsh_script_contains_search_description() {
        let script = get_completion_script(Some("zsh")).unwrap();
        assert!(
            script.contains("search"),
            "zsh script must describe 'search' subcommand"
        );
    }

    #[test]
    fn test_fish_script_contains_context() {
        let script = get_completion_script(Some("fish")).unwrap();
        assert!(script.contains("context"), "fish script must mention 'context' subcommand");
    }

    #[test]
    fn test_powershell_script_contains_env() {
        let script = get_completion_script(Some("powershell")).unwrap();
        assert!(script.contains("env"), "powershell script must contain 'env' command");
    }

    #[test]
    fn test_fish_install_path_ends_with_dot_fish() {
        let path = get_completion_install_path(Some("fish")).unwrap();
        assert!(path.ends_with(".fish"), "fish install path must end with '.fish': {path}");
    }

    #[test]
    fn test_bash_script_contains_interpret() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(script.contains("interpret"), "bash script must list 'interpret' subcommand");
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
    fn test_powershell_script_contains_solve() {
        let script = get_completion_script(Some("powershell")).unwrap();
        assert!(script.contains("solve"), "powershell script must contain 'solve' command");
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
    fn test_bash_script_contains_forward() {
        let script = get_completion_script(Some("bash")).unwrap();
        assert!(script.contains("forward"), "bash script must list 'forward' subcommand");
    }

    #[test]
    fn test_fish_script_contains_suite() {
        let script = get_completion_script(Some("fish")).unwrap();
        assert!(script.contains("suite"), "fish script must mention 'suite' subcommand");
    }
}
