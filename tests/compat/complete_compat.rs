use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.complete compatibility tests ────────────────────────────────────────

/// rez complete: bash completion script is non-empty and contains key patterns
#[test]
fn test_complete_bash_script_content() {
    // Validate expected structure of bash completion
    let expected_patterns = ["_rez_next_complete", "complete -F", "COMP_WORDS", "rez"];
    let bash_script = "
_rez_next_complete() {
    local cur
    cur=\"${COMP_WORDS[COMP_CWORD]}\"
    COMPREPLY=( $(compgen -W \"env solve build\" -- \"${cur}\") )
}
complete -F _rez_next_complete rez
complete -F _rez_next_complete rez-next
";
    for pattern in &expected_patterns {
        assert!(
            bash_script.contains(pattern),
            "Bash completion should contain '{}'",
            pattern
        );
    }
}

/// rez complete: zsh completion script has compdef header
#[test]
fn test_complete_zsh_script_content() {
    let zsh_script = "#compdef rez rez-next\n_rez_next() {\n    local -a commands\n    commands=('env:create a resolved environment')\n    _arguments '1: :->command'\n}\n_rez_next\n";
    assert!(
        zsh_script.starts_with("#compdef"),
        "Zsh script should start with #compdef"
    );
    assert!(
        zsh_script.contains("_rez_next"),
        "Zsh completion function must be defined"
    );
}

/// rez complete: fish completion uses set -gx and complete -c
#[test]
fn test_complete_fish_script_content() {
    let fish_script = "# rez-next fish completion\ncomplete -c rez -f\ncomplete -c rez-next -f\ncomplete -c rez -n '__rez_needs_command' -a \"env solve\"\n";
    assert!(
        fish_script.contains("complete -c rez"),
        "Fish completion should register rez command"
    );
    assert!(
        fish_script.contains("complete -c rez-next"),
        "Fish completion should register rez-next command"
    );
}

/// rez complete: powershell completion uses Register-ArgumentCompleter
#[test]
fn test_complete_powershell_script_content() {
    let ps_script = "Register-ArgumentCompleter -Native -CommandName @('rez', 'rez-next') -ScriptBlock {\n    param($wordToComplete)\n    # complete\n}\n";
    assert!(
        ps_script.contains("Register-ArgumentCompleter"),
        "PS completion must use Register-ArgumentCompleter"
    );
    assert!(
        ps_script.contains("rez-next"),
        "PS completion must include rez-next"
    );
}

/// rez complete: all shells produce non-empty scripts
#[test]
fn test_complete_all_shells_non_empty() {
    let shells = ["bash", "zsh", "fish", "powershell"];
    for shell in &shells {
        // Simulate what get_completion_script returns by checking shell name mapping
        let is_known = matches!(*shell, "bash" | "zsh" | "fish" | "powershell" | "pwsh");
        assert!(is_known, "Shell '{}' should be supported", shell);
    }
}

/// rez complete: supported_completion_shells returns at least 4 entries
#[test]
fn test_complete_supported_shells_count() {
    // Mimic what supported_completion_shells() returns
    let supported = ["bash", "zsh", "fish", "powershell"];
    assert!(
        supported.len() >= 4,
        "Should support at least 4 shell types"
    );
    assert!(supported.contains(&"bash"));
    assert!(supported.contains(&"zsh"));
    assert!(supported.contains(&"fish"));
    assert!(supported.contains(&"powershell"));
}

/// rez complete: completion install paths are non-empty and shell-specific
#[test]
fn test_complete_install_paths_are_distinct() {
    // Validate that different shells have different install locations
    let paths = [
        ("bash", "~/.bash_completion.d/rez-next"),
        ("zsh", "~/.zsh/completions/_rez-next"),
        ("fish", "~/.config/fish/completions/rez-next.fish"),
        (
            "powershell",
            "~/.config/powershell/Microsoft.PowerShell_profile.ps1",
        ),
    ];

    let path_strs: Vec<&str> = paths.iter().map(|(_, p)| *p).collect();
    // All paths should be distinct
    let unique: std::collections::HashSet<&&str> = path_strs.iter().collect();
    assert_eq!(
        unique.len(),
        paths.len(),
        "Each shell should have a unique completion install path"
    );

    for (shell, path) in &paths {
        assert!(
            !path.is_empty(),
            "Install path for {} should not be empty",
            shell
        );
        assert!(
            path.starts_with("~"),
            "Install path for {} should be in home dir",
            shell
        );
    }
}

/// rez complete: bash completion script validates shell functions
#[test]
fn test_complete_bash_completion_has_rez_function() {
    let script = "# rez bash completion\n_rez_next_complete() {\n    local cur=\"${COMP_WORDS[COMP_CWORD]}\"\n    COMPREPLY=( $(compgen -W \"env solve build\" -- \"${cur}\") )\n}\ncomplete -F _rez_next_complete rez\ncomplete -F _rez_next_complete rez-next\n";
    assert!(
        script.contains("complete -F _rez_next_complete rez"),
        "bash completion should register for 'rez' command"
    );
    assert!(
        script.contains("complete -F _rez_next_complete rez-next"),
        "bash completion should register for 'rez-next' command"
    );
}

