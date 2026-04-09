//! Shared shell utilities for rez-next Python bindings.
//!
//! Centralises the `string → ShellType` mapping so that all binding modules
//! (context, source, completion, shell, status) use a single, consistent
//! implementation.  Previously each module re-derived this match inline which
//! made the fallback behaviour easy to drift.

use rez_next_rex::ShellType;

/// Convert a shell name string to the corresponding [`ShellType`].
///
/// Recognises the same set of names as rez:
/// `"bash"`, `"zsh"`, `"fish"`, `"cmd"`, `"powershell"` / `"pwsh"`.
///
/// Any unrecognised value falls back to `ShellType::Bash` (POSIX default).
/// Call sites that need the auto-detection fallback should pass
/// [`crate::source_bindings::detect_current_shell()`] here.
pub(crate) fn shell_type_from_str(name: &str) -> ShellType {
    match name.to_lowercase().as_str() {
        "zsh" => ShellType::Zsh,
        "fish" => ShellType::Fish,
        "cmd" => ShellType::Cmd,
        "powershell" | "pwsh" | "ps1" => ShellType::PowerShell,
        _ => ShellType::Bash,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_type_from_str_known_shells() {
        assert!(matches!(shell_type_from_str("bash"), ShellType::Bash));
        assert!(matches!(shell_type_from_str("zsh"), ShellType::Zsh));
        assert!(matches!(shell_type_from_str("fish"), ShellType::Fish));
        assert!(matches!(shell_type_from_str("cmd"), ShellType::Cmd));
        assert!(matches!(shell_type_from_str("powershell"), ShellType::PowerShell));
        assert!(matches!(shell_type_from_str("pwsh"), ShellType::PowerShell));
        assert!(matches!(shell_type_from_str("ps1"), ShellType::PowerShell));
    }

    #[test]
    fn test_shell_type_from_str_case_insensitive() {
        assert!(matches!(shell_type_from_str("BASH"), ShellType::Bash));
        assert!(matches!(shell_type_from_str("ZSH"), ShellType::Zsh));
        assert!(matches!(shell_type_from_str("PowerShell"), ShellType::PowerShell));
    }

    #[test]
    fn test_shell_type_from_str_unknown_falls_back_to_bash() {
        assert!(matches!(shell_type_from_str("unknown"), ShellType::Bash));
        assert!(matches!(shell_type_from_str(""), ShellType::Bash));
        assert!(matches!(shell_type_from_str("sh"), ShellType::Bash));
    }
}
