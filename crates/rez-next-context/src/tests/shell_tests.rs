#[cfg(test)]
mod shell_behavior_tests {

    use crate::ShellType;

    #[test]
    fn test_shell_type_executable() {
        assert_eq!(ShellType::Bash.executable(), "bash");
        assert_eq!(ShellType::Zsh.executable(), "zsh");
        assert_eq!(ShellType::Fish.executable(), "fish");
        assert_eq!(ShellType::Cmd.executable(), "cmd");
        assert_eq!(ShellType::PowerShell.executable(), "powershell");
    }

    #[test]
    fn test_shell_type_script_extension() {
        let bash_ext = ShellType::Bash.script_extension();
        assert!(!bash_ext.is_empty());
        let ps_ext = ShellType::PowerShell.script_extension();
        assert!(!ps_ext.is_empty());
    }

    #[test]
    fn test_shell_type_equality() {
        assert_eq!(ShellType::Bash, ShellType::Bash);
        assert_ne!(ShellType::Bash, ShellType::Zsh);
        assert_ne!(ShellType::PowerShell, ShellType::Cmd);
    }
}
