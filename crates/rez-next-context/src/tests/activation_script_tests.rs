/// Phase 67: Environment activation scripts integration tests
#[cfg(test)]
mod activation_script_behavior_tests {

    use crate::{ContextConfig, EnvironmentManager, ShellType};
    use std::collections::HashMap;

    fn make_test_env() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/local/bin".to_string());
        env.insert("PYTHON_ROOT".to_string(), "/opt/python/3.9".to_string());
        env.insert(
            "MAYA_ROOT".to_string(),
            "/opt/autodesk/maya2023".to_string(),
        );
        env.insert("MY_TOOL".to_string(), "my_value".to_string());
        env
    }

    fn make_mgr(shell_type: ShellType) -> EnvironmentManager {
        let cfg = ContextConfig {
            shell_type,
            ..Default::default()
        };
        EnvironmentManager::new(cfg)
    }

    #[test]
    fn test_generate_bash_activation_script() {
        let mgr = make_mgr(ShellType::Bash);
        let env = make_test_env();
        let script = mgr.generate_shell_script(&env).unwrap();

        assert!(!script.is_empty(), "bash script should not be empty");
        // Should contain export statements
        assert!(
            script.contains("export") || script.contains("PYTHON_ROOT"),
            "bash script should set env vars: {}",
            &script[..script.len().min(200)]
        );
    }

    #[test]
    fn test_generate_powershell_activation_script() {
        let mgr = make_mgr(ShellType::PowerShell);
        let env = make_test_env();
        let script = mgr.generate_shell_script(&env).unwrap();

        assert!(!script.is_empty(), "PowerShell script should not be empty");
        // PowerShell uses $env:VAR format
        assert!(
            script.contains("$env:") || script.contains("PYTHON_ROOT"),
            "PS script should set env vars: {}",
            &script[..script.len().min(200)]
        );
    }

    #[test]
    fn test_generate_fish_activation_script() {
        let mgr = make_mgr(ShellType::Fish);
        let env = make_test_env();
        let script = mgr.generate_shell_script(&env).unwrap();

        assert!(!script.is_empty(), "fish script should not be empty");
    }

    #[test]
    fn test_activation_script_contains_all_vars() {
        let mgr = make_mgr(ShellType::Bash);
        let mut env = HashMap::new();
        env.insert("VAR_A".to_string(), "value_a".to_string());
        env.insert("VAR_B".to_string(), "value_b".to_string());

        let script = mgr.generate_shell_script(&env).unwrap();
        // All variables should appear in script
        assert!(
            script.contains("VAR_A") || script.contains("value_a"),
            "VAR_A should be in script"
        );
        assert!(
            script.contains("VAR_B") || script.contains("value_b"),
            "VAR_B should be in script"
        );
    }

    #[test]
    fn test_cmd_activation_script() {
        let mgr = make_mgr(ShellType::Cmd);
        let env = make_test_env();
        let script = mgr.generate_shell_script(&env).unwrap();
        assert!(!script.is_empty(), "CMD script should not be empty");
        // CMD uses SET VAR=value
        assert!(
            script.contains("SET ") || script.contains("set ") || script.contains("PYTHON_ROOT"),
            "CMD script should have SET statements: {}",
            &script[..script.len().min(200)]
        );
    }
}
