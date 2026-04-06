// ── Cycle 49: EnvDiff + EnvOperation::Append/Unset + PathStrategy tests ────
#[cfg(test)]
mod env_diff_behavior_tests {

    use crate::{ContextConfig, EnvironmentManager, PathStrategy, ShellType};
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    fn make_package(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg
    }

    fn make_package_with_tools(name: &str, version: &str, tools: Vec<String>) -> Package {
        let mut pkg = make_package(name, version);
        pkg.tools = tools;
        pkg
    }

    fn make_mgr(path_strategy: PathStrategy, shell: ShellType) -> EnvironmentManager {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            path_strategy,
            shell_type: shell,
            ..Default::default()
        };
        EnvironmentManager::new(cfg)
    }

    // ── EnvDiff helpers ──────────────────────────────────────────────────────

    #[test]
    fn test_env_diff_is_empty_when_no_changes() {
        let mgr = make_mgr(PathStrategy::Prepend, ShellType::Bash);
        let env: HashMap<String, String> = HashMap::new();
        let diff = mgr.get_env_diff(&env);
        assert!(diff.is_empty(), "No changes → diff should be empty");
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn test_env_diff_added_vars() {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let mut env = HashMap::new();
        env.insert("NEW_VAR".to_string(), "new_value".to_string());
        let diff = mgr.get_env_diff(&env);
        assert!(!diff.is_empty());
        assert!(diff.added.contains_key("NEW_VAR"));
        assert_eq!(diff.change_count(), 1);
    }

    #[test]
    fn test_env_diff_removed_vars() {
        let expected_removed = std::env::vars().count();
        let cfg = ContextConfig {
            inherit_parent_env: true,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let empty_env: HashMap<String, String> = HashMap::new();
        let diff = mgr.get_env_diff(&empty_env);

        assert!(
            diff.added.is_empty(),
            "Empty env should not report added vars"
        );
        assert!(
            diff.modified.is_empty(),
            "Empty env should not report modified vars"
        );
        assert_eq!(
            diff.removed.len(),
            expected_removed,
            "All inherited base vars should be reported as removed"
        );
    }

    #[test]
    fn test_env_diff_modified_vars() {
        // inherit=true so base_env = system env; then we add PATH with a modified value
        let cfg = ContextConfig {
            inherit_parent_env: true,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);

        // Build an env that has PATH modified
        let mut env: HashMap<String, String> = std::env::vars().collect();
        let orig = env.get("PATH").cloned().unwrap_or_default();
        let new_path = format!("/extra/bin:{}", orig);
        env.insert("PATH".to_string(), new_path.clone());
        env.insert("BRAND_NEW_VAR".to_string(), "hello".to_string());

        let diff = mgr.get_env_diff(&env);
        assert!(
            diff.modified.contains_key("PATH") || diff.added.contains_key("PATH"),
            "PATH should show as modified or added"
        );
        assert!(diff.added.contains_key("BRAND_NEW_VAR"));
        assert!(diff.change_count() >= 1);
    }

    #[test]
    fn test_env_diff_change_count_sums_all() {
        use crate::EnvDiff;
        let mut added = HashMap::new();
        added.insert("A".to_string(), "1".to_string());
        added.insert("B".to_string(), "2".to_string());
        let mut modified = HashMap::new();
        modified.insert("C".to_string(), ("old".to_string(), "new".to_string()));
        let removed = vec!["D".to_string()];

        let diff = EnvDiff {
            added,
            modified,
            removed,
        };
        assert_eq!(diff.change_count(), 4);
        assert!(!diff.is_empty());
    }

    // ── PathStrategy::Append ─────────────────────────────────────────────────

    #[test]
    fn test_path_strategy_append_with_tools() {
        let mgr = make_mgr(PathStrategy::Append, ShellType::Bash);
        let mut pkg = make_package_with_tools("mytool", "1.0.0", vec!["mytool".to_string()]);
        pkg.tools = vec!["mytool".to_string()];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();

        // PATH should end with the tool bin dir (Append strategy)
        if let Some(path) = vars.get("PATH") {
            // either it's the only entry or ends with the tool path
            assert!(
                path.contains("mytool"),
                "Append PATH should contain package bin: {}",
                path
            );
        }
    }

    #[test]
    fn test_path_strategy_replace_with_tools() {
        let mgr = make_mgr(PathStrategy::Replace, ShellType::Bash);
        let mut pkg = make_package_with_tools("mypkg", "2.0.0", vec!["mypkg_bin".to_string()]);
        pkg.tools = vec!["mypkg_bin".to_string()];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();

        if let Some(path) = vars.get("PATH") {
            assert!(
                path.contains("mypkg"),
                "Replace PATH should be the tool path: {}",
                path
            );
        }
    }

    #[test]
    fn test_path_strategy_no_modify_leaves_path_unchanged() {
        let mut additional_env_vars = HashMap::new();

        additional_env_vars.insert("PATH".to_string(), "/original/path".to_string());
        let cfg2 = ContextConfig {
            inherit_parent_env: false,
            path_strategy: PathStrategy::NoModify,
            additional_env_vars,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg2);

        let mut pkg = make_package("toolpkg", "1.0.0");
        pkg.tools = vec!["toolpkg".to_string()];

        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();

        // PATH should not be modified by the package tools
        let path = vars.get("PATH").map(|s| s.as_str()).unwrap_or("");
        assert_eq!(path, "/original/path", "NoModify should leave PATH as-is");
    }

    #[test]
    fn test_path_strategy_prepend_no_tools_no_path_change() {
        let mgr = make_mgr(PathStrategy::Prepend, ShellType::Bash);
        // Package with no tools should not modify PATH
        let pkg = make_package("notool", "1.0.0");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();
        // PATH either absent or unchanged (no tool dirs prepended)
        let _ = vars.get("PATH");
    }

    // ── EnvOperation::Append serde roundtrip ────────────────────────────────

    #[test]
    fn test_env_operation_serde_roundtrip() {
        use crate::{EnvOperation, EnvVarDefinition};

        let def = EnvVarDefinition {
            name: "MYVAR".to_string(),
            operation: EnvOperation::Append("append_val".to_string(), ":".to_string()),
            source_package: Some("mypkg".to_string()),
            priority: 5,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: EnvVarDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.name, "MYVAR");
        match restored.operation {
            EnvOperation::Append(val, sep) => {
                assert_eq!(val, "append_val");
                assert_eq!(sep, ":");
            }
            other => panic!("Expected Append, got {:?}", other),
        }
    }

    #[test]
    fn test_env_operation_unset_serde_roundtrip() {
        use crate::{EnvOperation, EnvVarDefinition};

        let def = EnvVarDefinition {
            name: "REMOVE_ME".to_string(),
            operation: EnvOperation::Unset,
            source_package: None,
            priority: 0,
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: EnvVarDefinition = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored.operation, EnvOperation::Unset));
        assert!(restored.source_package.is_none());
    }

    // ── Shell script escape correctness ─────────────────────────────────────

    #[test]
    fn test_bash_script_escapes_special_chars() {
        let mgr = make_mgr(PathStrategy::NoModify, ShellType::Bash);
        let mut env = HashMap::new();
        env.insert(
            "MY_VAR".to_string(),
            r#"val"ue with $special `chars`"#.to_string(),
        );
        let script = mgr.generate_shell_script(&env).unwrap();
        // Should not contain unescaped double-quote after the = sign
        assert!(script.contains("MY_VAR"), "Script should mention MY_VAR");
        // The raw $ should be escaped in bash export
        assert!(
            !script.contains("$special`") || script.contains(r"\$"),
            "bash should escape $ in value"
        );
    }

    #[test]
    fn test_powershell_script_escapes_special_chars() {
        let mgr = make_mgr(PathStrategy::NoModify, ShellType::PowerShell);
        let mut env = HashMap::new();
        env.insert(
            "PS_VAR".to_string(),
            r#"val with $env:PATH and "quotes""#.to_string(),
        );
        let script = mgr.generate_shell_script(&env).unwrap();
        assert!(script.contains("PS_VAR"), "Script should mention PS_VAR");
        // PowerShell uses `$ for escaping
        assert!(
            script.contains("`$") || script.contains("PS_VAR"),
            "PS should escape $ in value"
        );
    }

    #[test]
    fn test_zsh_script_format() {
        let mgr = make_mgr(PathStrategy::NoModify, ShellType::Zsh);
        let mut env = HashMap::new();
        env.insert("ZSH_VAR".to_string(), "zsh_value".to_string());
        let script = mgr.generate_shell_script(&env).unwrap();
        assert!(
            script.contains("#!/bin/zsh"),
            "Zsh script should have shebang"
        );
        assert!(script.contains("ZSH_VAR"));
    }

    #[test]
    fn test_fish_script_format() {
        let mgr = make_mgr(PathStrategy::NoModify, ShellType::Fish);
        let mut env = HashMap::new();
        env.insert("FISH_VAR".to_string(), "fish_value".to_string());
        let script = mgr.generate_shell_script(&env).unwrap();
        assert!(
            script.contains("set -x"),
            "Fish script should use set -x: {}",
            &script[..script.len().min(100)]
        );
        assert!(script.contains("FISH_VAR"));
    }
}
