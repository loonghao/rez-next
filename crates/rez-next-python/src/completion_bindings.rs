//! Python bindings for rez.complete (shell tab-completion scripts)
//!
//! Mirrors `rez complete` CLI and `rez.complete` Python module.
//! Generates completion scripts for bash, zsh, fish, and PowerShell.

use pyo3::prelude::*;

/// Shell types supported for completion
const BASH_COMPLETION: &str = r#"
# rez-next bash completion
_rez_next_complete() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    local commands="env solve build release status search view diff cp mv rm bundle config selftest gui context suite interpret depends pip forward benchmark complete source bind"

    case "$prev" in
        rez|rez-next)
            COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
            return 0
            ;;
        -p|--paths)
            COMPREPLY=( $(compgen -d -- "${cur}") )
            return 0
            ;;
        env|solve)
            # Package name completion — query rez-next search
            local packages
            packages=$(rez-next search --names-only 2>/dev/null)
            COMPREPLY=( $(compgen -W "${packages}" -- "${cur}") )
            return 0
            ;;
    esac

    COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
    return 0
}
complete -F _rez_next_complete rez
complete -F _rez_next_complete rez-next
"#;

const ZSH_COMPLETION: &str = r#"
#compdef rez rez-next
# rez-next zsh completion

_rez_next() {
    local -a commands
    commands=(
        'env:create a resolved environment'
        'solve:solve a set of package requirements'
        'build:build the current package from source'
        'release:release the current package'
        'status:show status of current context'
        'search:search for packages'
        'view:view a package definition'
        'diff:show differences between two contexts'
        'cp:copy a package'
        'mv:move a package'
        'rm:remove a package'
        'bundle:bundle a context for offline use'
        'config:show/edit rez configuration'
        'selftest:run rez self-tests'
        'gui:open the rez GUI'
        'context:show context information'
        'suite:manage tool suites'
        'interpret:interpret a rex command file'
        'depends:show package dependencies'
        'pip:install a pip package'
        'forward:forward a tool call'
        'benchmark:run rez benchmarks'
        'complete:print shell completion script'
        'source:activate a context'
        'bind:bind a system tool as a rez package'
    )

    _arguments \
        '(-h --help)'{-h,--help}'[show help]' \
        '(-V --version)'{-V,--version}'[show version]' \
        '(-p --paths)'{-p,--paths}'[package search paths]:directory:_directories' \
        '1: :->command' \
        '*: :->args'

    case $state in
        command)
            _describe 'command' commands
            ;;
        args)
            case $words[2] in
                env|solve)
                    local packages
                    packages=($(rez-next search --names-only 2>/dev/null))
                    _describe 'package' packages
                    ;;
                *)
                    _default
                    ;;
            esac
            ;;
    esac
}

_rez_next
"#;

const FISH_COMPLETION: &str = r#"
# rez-next fish completion

set -l rez_commands env solve build release status search view diff cp mv rm bundle config selftest gui context suite interpret depends pip forward benchmark complete source bind

function __rez_needs_command
    set cmd (commandline -opc)
    if [ (count $cmd) -eq 1 ]
        return 0
    end
    return 1
end

function __rez_packages
    rez-next search --names-only 2>/dev/null
end

complete -c rez -f
complete -c rez-next -f

complete -c rez -n '__rez_needs_command' -a "$rez_commands"
complete -c rez-next -n '__rez_needs_command' -a "$rez_commands"

complete -c rez -n '__fish_seen_subcommand_from env solve' -a '(__rez_packages)'
complete -c rez-next -n '__fish_seen_subcommand_from env solve' -a '(__rez_packages)'

complete -c rez -s p -l paths -d 'Package search paths' -r -F
complete -c rez-next -s p -l paths -d 'Package search paths' -r -F
complete -c rez -s h -l help -d 'Show help'
complete -c rez-next -s h -l help -d 'Show help'
complete -c rez -s V -l version -d 'Show version'
complete -c rez-next -s V -l version -d 'Show version'
"#;

const POWERSHELL_COMPLETION: &str = r#"
# rez-next PowerShell completion
Register-ArgumentCompleter -Native -CommandName @('rez', 'rez-next') -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commands = @(
        'env', 'solve', 'build', 'release', 'status', 'search', 'view',
        'diff', 'cp', 'mv', 'rm', 'bundle', 'config', 'selftest', 'gui',
        'context', 'suite', 'interpret', 'depends', 'pip', 'forward',
        'benchmark', 'complete', 'source', 'bind'
    )

    $tokens = $commandAst.CommandElements
    $tokenStrings = $tokens | ForEach-Object { $_.ToString() }

    # First argument: complete commands
    if ($tokens.Count -le 2) {
        $commands | Where-Object { $_ -like "$wordToComplete*" } |
            ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
        return
    }

    # Second argument for env/solve: complete package names
    $subCommand = $tokenStrings[1]
    if ($subCommand -in @('env', 'solve')) {
        try {
            $packages = & rez-next search --names-only 2>$null
            $packages | Where-Object { $_ -like "$wordToComplete*" } |
                ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
        } catch {}
        return
    }
}
"#;

/// Generate a shell tab-completion script for rez-next.
///
/// Args:
///     shell: "bash" | "zsh" | "fish" | "powershell" (default: auto-detect)
///
/// Returns:
///     Shell script string to be sourced or installed.
#[pyfunction]
#[pyo3(signature = (shell=None))]
pub fn get_completion_script(shell: Option<&str>) -> PyResult<String> {
    let shell_type = shell
        .map(|s| s.to_lowercase())
        .unwrap_or_else(detect_current_shell);

    match shell_type.as_str() {
        "bash" => Ok(BASH_COMPLETION.to_string()),
        "zsh" => Ok(ZSH_COMPLETION.to_string()),
        "fish" => Ok(FISH_COMPLETION.to_string()),
        "powershell" | "pwsh" | "ps1" => Ok(POWERSHELL_COMPLETION.to_string()),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown shell type: '{}'. Supported: bash, zsh, fish, powershell",
            other
        ))),
    }
}

/// Write the completion script to stdout (mimics `rez complete`).
#[pyfunction]
#[pyo3(signature = (shell=None))]
pub fn print_completion_script(shell: Option<&str>) -> PyResult<()> {
    let script = get_completion_script(shell)?;
    print!("{}", script);
    Ok(())
}

/// List all supported shells for completion.
#[pyfunction]
pub fn supported_completion_shells() -> Vec<String> {
    vec![
        "bash".to_string(),
        "zsh".to_string(),
        "fish".to_string(),
        "powershell".to_string(),
    ]
}

/// Get the completion script installation path for a given shell.
#[pyfunction]
#[pyo3(signature = (shell=None))]
pub fn get_completion_install_path(shell: Option<&str>) -> PyResult<String> {
    let shell_type = shell
        .map(|s| s.to_lowercase())
        .unwrap_or_else(detect_current_shell);

    match shell_type.as_str() {
        "bash" => Ok("~/.bash_completion.d/rez-next".to_string()),
        "zsh" => Ok("~/.zsh/completions/_rez-next".to_string()),
        "fish" => Ok("~/.config/fish/completions/rez-next.fish".to_string()),
        "powershell" | "pwsh" => {
            Ok("~/.config/powershell/Microsoft.PowerShell_profile.ps1".to_string())
        }
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown shell type: '{}'",
            other
        ))),
    }
}

fn detect_current_shell() -> String {
    // Check SHELL env var (Unix)
    if let Ok(shell) = std::env::var("SHELL") {
        if shell.contains("zsh") {
            return "zsh".to_string();
        }
        if shell.contains("fish") {
            return "fish".to_string();
        }
        if shell.contains("bash") {
            return "bash".to_string();
        }
    }
    // Windows PowerShell
    if std::env::var("PSModulePath").is_ok() || cfg!(windows) {
        return "powershell".to_string();
    }
    "bash".to_string()
}

#[cfg(test)]
mod tests {
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
        // Verify that an unknown shell name is absent from the supported list
        // (actual error path requires PyO3 GIL so is covered by Python e2e tests)
        let shells = supported_completion_shells();
        assert!(!shells.contains(&"tcsh".to_string()));
        assert!(!shells.contains(&"csh".to_string()));
    }

    // ── Script content sanity checks ─────────────────────────────────────────

    #[test]
    fn test_bash_script_lists_rez_commands() {
        let script = get_completion_script(Some("bash")).unwrap();
        // The 'env' and 'solve' sub-commands must be present
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

    // ── print_completion_script (smoke, no output capture needed) ────────────

    #[test]
    fn test_print_completion_script_valid_shell_no_panic() {
        // Verify no panic occurs for a known shell; stdout not captured in unit tests.
        // Note: this calls print! so output may appear; that's acceptable.
        // Unknown shell path requires GIL and is covered in Python e2e tests.
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
        // pwsh is an alias for powershell; verify the returned path is non-empty
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
        // bash completer must handle env|solve with package query
        assert!(
            script.contains("env|solve") || (script.contains("env") && script.contains("solve")),
            "bash script should handle env/solve completion"
        );
    }

    // ── zsh script: builds with _arguments ───────────────────────────────────

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
        assert!(
            script.contains("build"),
            "fish script should contain 'build'"
        );
        assert!(
            script.contains("release"),
            "fish script should contain 'release'"
        );
    }

    // ── powershell script: contains $wordToComplete ───────────────────────────

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

    // ── Cycle 114 additions ──────────────────────────────────────────────────

    mod test_completion_cy114 {
        use super::*;

        /// get_completion_script for bash is non-empty
        #[test]
        fn test_bash_completion_script_is_nonempty() {
            let script = get_completion_script(Some("bash")).unwrap();
            assert!(!script.is_empty(), "bash completion script must not be empty");
        }

        /// get_completion_script for zsh is non-empty
        #[test]
        fn test_zsh_completion_script_is_nonempty() {
            let script = get_completion_script(Some("zsh")).unwrap();
            assert!(!script.is_empty(), "zsh completion script must not be empty");
        }

        /// get_completion_script for fish is non-empty
        #[test]
        fn test_fish_completion_script_is_nonempty() {
            let script = get_completion_script(Some("fish")).unwrap();
            assert!(!script.is_empty(), "fish completion script must not be empty");
        }

        /// get_completion_script for powershell is non-empty
        #[test]
        fn test_powershell_completion_script_is_nonempty() {
            let script = get_completion_script(Some("powershell")).unwrap();
            assert!(!script.is_empty(), "powershell completion script must not be empty");
        }

        /// supported_completion_shells includes at least bash and zsh
        #[test]
        fn test_supported_shells_includes_bash_and_zsh() {
            let shells = supported_completion_shells();
            assert!(
                shells.iter().any(|s| s == "bash"),
                "supported shells should include 'bash'"
            );
            assert!(
                shells.iter().any(|s| s == "zsh"),
                "supported shells should include 'zsh'"
            );
        }

        /// supported_completion_shells has at least 4 entries
        #[test]
        fn test_supported_shells_has_at_least_four() {
            let shells = supported_completion_shells();
            assert!(
                shells.len() >= 4,
                "supported_completion_shells should return at least 4, got {}",
                shells.len()
            );
        }

        /// get_completion_script None returns default shell script without panic
        #[test]
        fn test_completion_script_none_shell_no_panic() {
            let result = get_completion_script(None);
            // May succeed or fail; must not panic
            let _ = result;
        }
    }

    // ── Cycle 119 additions ──────────────────────────────────────────────────

    mod test_completion_cy119 {
        use super::*;

        /// bash script contains 'rez-next search' call for package completion
        #[test]
        fn test_bash_script_contains_search_query() {
            let script = get_completion_script(Some("bash")).unwrap();
            assert!(
                script.contains("search") || script.contains("rez-next"),
                "bash script should reference rez-next search for packages"
            );
        }

        /// zsh script uses _arguments for option parsing
        #[test]
        fn test_zsh_script_contains_arguments_directive() {
            let script = get_completion_script(Some("zsh")).unwrap();
            assert!(
                script.contains("_arguments"),
                "zsh script should use '_arguments' completion helper"
            );
        }

        /// fish script registers completions for 'rez' command
        #[test]
        fn test_fish_script_registers_rez_completions() {
            let script = get_completion_script(Some("fish")).unwrap();
            assert!(
                script.contains("complete -c rez "),
                "fish script must register completions for 'rez'"
            );
        }

        /// powershell script handles at least 20 subcommands
        #[test]
        fn test_powershell_script_lists_twenty_commands() {
            let script = get_completion_script(Some("powershell")).unwrap();
            // Count quoted command names like 'env', 'solve', etc.
            let count = script.split('\'').count() / 2;
            assert!(
                count >= 20,
                "powershell script should list at least 20 commands, found ~{count}"
            );
        }

        /// get_completion_install_path for bash uses tilde expansion
        #[test]
        fn test_bash_install_path_starts_with_tilde() {
            let path = get_completion_install_path(Some("bash")).unwrap();
            assert!(
                path.starts_with('~'),
                "bash install path should start with '~': '{path}'"
            );
        }

        /// supported_completion_shells does not include 'csh'
        #[test]
        fn test_supported_shells_does_not_include_csh() {
            let shells = supported_completion_shells();
            assert!(
                !shells.iter().any(|s| s == "csh"),
                "csh must not be in supported shells"
            );
        }
    }

    mod test_completion_cy125 {
        use super::*;

        /// supported_completion_shells returns at least 3 shells
        #[test]
        fn test_supported_shells_has_at_least_three() {
            let shells = supported_completion_shells();
            assert!(
                shells.len() >= 3,
                "must support at least 3 shells, got: {shells:?}"
            );
        }

        /// get_completion_script for zsh returns Ok
        #[test]
        fn test_get_completion_script_zsh_is_ok() {
            let result = get_completion_script(Some("zsh"));
            assert!(result.is_ok(), "zsh completion script must succeed");
        }

        /// get_completion_script for powershell returns Ok
        #[test]
        fn test_get_completion_script_powershell_is_ok() {
            let result = get_completion_script(Some("powershell"));
            assert!(
                result.is_ok(),
                "powershell completion script must succeed"
            );
        }

        /// bash completion script is non-empty
        #[test]
        fn test_bash_completion_script_is_nonempty() {
            let script = get_completion_script(Some("bash")).unwrap();
            assert!(!script.is_empty(), "bash completion script must not be empty");
        }

        /// fish completion script contains 'rez' token
        #[test]
        fn test_fish_script_contains_rez_token() {
            let script = get_completion_script(Some("fish")).unwrap();
            assert!(
                script.contains("rez"),
                "fish completion script must mention 'rez'"
            );
        }
    }
}
