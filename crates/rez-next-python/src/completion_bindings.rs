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
        "powershell" | "pwsh" => Ok(
            "~/.config/powershell/Microsoft.PowerShell_profile.ps1".to_string()
        ),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown shell type: '{}'", other
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
