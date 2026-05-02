//! Python bindings for rez.complete (shell tab-completion scripts)
//!
//! Mirrors `rez complete` CLI and `rez.complete` Python module.
//! Generates completion scripts for bash, zsh, fish, and PowerShell.

use pyo3::prelude::*;

use crate::source_bindings::detect_current_shell;

/// Shell types supported for completion
const BASH_COMPLETION: &str = r#"
# rez-next bash completion (dynamic mode)
_rez_next_complete() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    # Use dynamic completion (reads COMP_LINE/COMP_POINT)
    local completions
    completions=$(COMP_LINE="${COMP_LINE}" COMP_POINT="${COMP_POINT}" rez-next complete --dynamic 2>/dev/null)
    COMPREPLY=( $(compgen -W "${completions}" -- "${cur}") )
    return 0
}
complete -F _rez_next_complete rez
complete -F _rez_next_complete rez-next
"#;

const ZSH_COMPLETION: &str = r#"
#compdef rez rez-next
# rez-next zsh completion (dynamic mode)

_rez_next() {
    local -a commands
    local completions

    # Use dynamic completion (reads COMP_LINE/COMP_POINT)
    completions=($(COMP_LINE="${(j: :)words}" COMP_POINT="$((CURRENT - 1))" rez-next complete --dynamic 2>/dev/null))

    _arguments \
        '(-h --help)'{-h,--help}'[show help]' \
        '(-V --version)'{-V,--version}'[show version]' \
        '(-p --paths)'{-p,--paths}'[package search paths]:directory:_directories' \
        '1: :->command' \
        '*: :->args'

    case $state in
        command)
            _values 'commands' "${completions[@]}"
            ;;
        args)
            case $words[2] in
                env|solve|search|depends)
                    local pkg_completions
                    pkg_completions=($(COMP_LINE="${(j: :)words}" COMP_POINT="$((CURRENT - 1))" rez-next complete --dynamic 2>/dev/null))
                    _values 'packages' "${pkg_completions[@]}"
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
# rez-next fish completion (dynamic mode)

function __rez_next_complete
    set -l completions (commandline -c)[-1]
    set -l cursor (commandline -C)
    set -l output (COMP_LINE=(commandline) COMP_POINT=$cursor rez-next complete --dynamic 2>/dev/null)
    for c in $output
        echo $c
    end
end

complete -c rez -f -a '(__rez_next_complete)'
complete -c rez-next -f -a '(__rez_next_complete)'
"#;

const POWERSHELL_COMPLETION: &str = r#"
# rez-next PowerShell completion (dynamic mode)
Register-ArgumentCompleter -Native -CommandName @('rez', 'rez-next') -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    # Use dynamic completion (reads COMP_LINE/COMP_POINT)
    $completions = COMP_LINE=$commandAst.ToString() COMP_POINT=$cursorPosition rez-next complete --dynamic 2>$null
    $completions | Where-Object { $_ -like "$wordToComplete*" } |
        ForEach-Object { [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }
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

/// Get the completion script as a string (mimics `rez complete`).
#[pyfunction]
#[pyo3(signature = (shell=None))]
pub fn get_completion_script_py(shell: Option<&str>) -> PyResult<String> {
    get_completion_script(shell)
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

#[cfg(test)]
#[path = "completion_bindings_tests.rs"]
mod tests;
