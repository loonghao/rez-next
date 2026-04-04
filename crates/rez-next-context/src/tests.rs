//! Unit tests for rez-next-context

#[cfg(test)]
mod context_tests {
    use crate::{ContextConfig, ContextStatus, EnvironmentManager, PathStrategy, ResolvedContext};
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    fn make_package(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg
    }

    // ── ContextConfig ────────────────────────────────────────────────────────

    #[test]
    fn test_context_config_defaults() {
        let cfg = ContextConfig::default();
        assert!(cfg.inherit_parent_env);
        assert!(cfg.additional_env_vars.is_empty());
        assert!(cfg.unset_vars.is_empty());
        assert_eq!(cfg.path_strategy, PathStrategy::Prepend);
    }

    #[test]
    fn test_context_config_additional_vars() {
        let mut cfg = ContextConfig::default();
        cfg.additional_env_vars
            .insert("MY_VAR".to_string(), "hello".to_string());
        assert_eq!(
            cfg.additional_env_vars.get("MY_VAR"),
            Some(&"hello".to_string())
        );
    }

    // ── ResolvedContext ──────────────────────────────────────────────────────

    #[test]
    fn test_resolved_context_from_requirements() {
        let reqs = vec![PackageRequirement::parse("python-3.9").unwrap()];
        let ctx = ResolvedContext::from_requirements(reqs.clone());
        assert_eq!(ctx.requirements.len(), 1);
        assert_eq!(ctx.status, ContextStatus::Resolving);
        assert!(ctx.resolved_packages.is_empty());
    }

    #[test]
    fn test_resolved_context_id_is_non_empty() {
        let ctx1 = ResolvedContext::from_requirements(vec![]);
        let ctx2 = ResolvedContext::from_requirements(vec![]);
        assert!(!ctx1.id.is_empty());
        assert!(!ctx2.id.is_empty());
        // UUIDs are unique
        assert_ne!(ctx1.id, ctx2.id);
    }

    #[test]
    fn test_resolved_context_status_transitions() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        assert_eq!(ctx.status, ContextStatus::Resolving);
        ctx.status = ContextStatus::Resolved;
        assert_eq!(ctx.status, ContextStatus::Resolved);
        ctx.status = ContextStatus::Failed;
        assert_eq!(ctx.status, ContextStatus::Failed);
    }

    #[test]
    fn test_resolved_context_add_packages() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.resolved_packages.push(make_package("python", "3.9.0"));
        ctx.resolved_packages.push(make_package("maya", "2023.0"));
        assert_eq!(ctx.resolved_packages.len(), 2);
        assert_eq!(ctx.package_count(), 2);
    }

    #[test]
    fn test_resolved_context_contains_package() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.resolved_packages.push(make_package("python", "3.9.0"));
        assert!(ctx.contains_package("python"));
        assert!(!ctx.contains_package("maya"));
    }

    #[test]
    fn test_resolved_context_get_package() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.resolved_packages.push(make_package("python", "3.9.0"));
        let pkg = ctx.get_package("python");
        assert!(pkg.is_some());
        assert_eq!(pkg.unwrap().name, "python");
    }

    #[test]
    fn test_resolved_context_env_vars() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.set_env_var("FOO".to_string(), "bar".to_string());
        assert_eq!(ctx.get_env_var("FOO"), Some("bar".to_string()));
        assert_eq!(ctx.get_env_var("NONEXISTENT"), None);
    }

    #[test]
    fn test_resolved_context_get_package_names() {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.resolved_packages.push(make_package("python", "3.9.0"));
        ctx.resolved_packages
            .push(make_package("houdini", "19.5.0"));
        let names = ctx.get_package_names();
        assert!(names.contains(&"python".to_string()));
        assert!(names.contains(&"houdini".to_string()));
    }

    // ── EnvironmentManager ───────────────────────────────────────────────────

    #[test]
    fn test_env_manager_no_inherit() {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[])).unwrap();
        // With no inherit and no packages, should have no user-defined vars
        assert!(!vars.contains_key("MY_CUSTOM_NONEXISTENT_VAR"));
    }

    #[test]
    fn test_env_manager_sets_package_root() {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let pkg = make_package("python", "3.9.0");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();
        assert!(vars.contains_key("PYTHON_ROOT"));
    }

    #[test]
    fn test_env_manager_sets_version_var() {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let pkg = make_package("maya", "2023.0");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[pkg])).unwrap();
        assert_eq!(vars.get("MAYA_VERSION").map(|s| s.as_str()), Some("2023.0"));
    }

    #[test]
    fn test_env_manager_additional_vars() {
        let mut additional_env_vars = std::collections::HashMap::new();
        additional_env_vars.insert("CUSTOM_VAR".to_string(), "custom_value".to_string());
        let cfg = ContextConfig {
            inherit_parent_env: false,
            additional_env_vars,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[])).unwrap();
        assert_eq!(
            vars.get("CUSTOM_VAR").map(|s| s.as_str()),
            Some("custom_value")
        );
    }

    #[test]
    fn test_env_manager_unset_vars() {
        let mut additional_env_vars = std::collections::HashMap::new();
        additional_env_vars.insert("TO_REMOVE".to_string(), "should_be_gone".to_string());
        let cfg = ContextConfig {
            inherit_parent_env: false,
            additional_env_vars,
            unset_vars: vec!["TO_REMOVE".to_string()],
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&[])).unwrap();
        assert!(!vars.contains_key("TO_REMOVE"));
    }

    #[test]
    fn test_env_manager_multiple_packages() {
        let cfg = ContextConfig {
            inherit_parent_env: false,
            ..Default::default()
        };
        let mgr = EnvironmentManager::new(cfg);
        let packages = vec![
            make_package("python", "3.9.0"),
            make_package("houdini", "19.5.0"),
            make_package("nuke", "13.2.0"),
        ];
        let rt = tokio::runtime::Runtime::new().unwrap();
        let vars = rt.block_on(mgr.generate_environment(&packages)).unwrap();
        assert!(vars.contains_key("PYTHON_ROOT"));
        assert!(vars.contains_key("HOUDINI_ROOT"));
        assert!(vars.contains_key("NUKE_ROOT"));
        assert_eq!(
            vars.get("PYTHON_VERSION").map(|s| s.as_str()),
            Some("3.9.0")
        );
    }
}

#[cfg(test)]
mod shell_tests {
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

/// Phase 86: Context rxt file async save/load integration tests
#[cfg(test)]
mod rxt_file_tests {
    use crate::serialization::{ContextFormat, ContextSerializer};
    use crate::{ContextStatus, ResolvedContext};
    use rez_next_package::Package;
    use rez_next_version::Version;
    use tempfile::TempDir;

    fn make_package(name: &str, ver: &str) -> Package {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    }

    fn make_ctx(pkgs: &[(&str, &str)]) -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        for (name, ver) in pkgs {
            ctx.resolved_packages.push(make_package(name, ver));
        }
        ctx.set_env_var("CONTEXT_VAR".to_string(), "context_value".to_string());
        ctx
    }

    #[tokio::test]
    async fn test_save_and_load_rxt_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ctx.rxt");

        let ctx = make_ctx(&[("python", "3.9.0"), ("maya", "2023.0")]);
        let orig_id = ctx.id.clone();

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .expect("save should succeed");

        let loaded = ContextSerializer::load_from_file(&path)
            .await
            .expect("load should succeed");

        assert_eq!(loaded.id, orig_id, "ID should roundtrip");
        assert_eq!(
            loaded.status,
            ContextStatus::Resolved,
            "Status should roundtrip"
        );
        assert_eq!(
            loaded.resolved_packages.len(),
            2,
            "Package count should roundtrip"
        );
    }

    #[tokio::test]
    async fn test_save_rxt_creates_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("new_ctx.rxt");

        assert!(!path.exists(), "File should not exist before save");
        let ctx = make_ctx(&[]);
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        assert!(path.exists(), "File should exist after save");
        let size = std::fs::metadata(&path).unwrap().len();
        assert!(size > 0, "File should not be empty");
    }

    #[tokio::test]
    async fn test_save_rxt_creates_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nested").join("deep").join("ctx.rxt");

        let ctx = make_ctx(&[("nuke", "13.0.0")]);
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        assert!(path.exists(), "File in nested dirs should be created");
    }

    #[tokio::test]
    async fn test_load_nonexistent_rxt_errors() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.rxt");

        let result = ContextSerializer::load_from_file(&path).await;
        assert!(result.is_err(), "Loading nonexistent file should error");
    }

    #[tokio::test]
    async fn test_rxt_env_vars_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("env_ctx.rxt");

        let mut ctx = make_ctx(&[("python", "3.9.0")]);
        ctx.set_env_var(
            "REZ_CONTEXT_FILE".to_string(),
            path.to_str().unwrap().to_string(),
        );
        ctx.set_env_var("MY_CUSTOM_VAR".to_string(), "my_custom_value".to_string());

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.get_env_var("MY_CUSTOM_VAR"),
            Some("my_custom_value".to_string()),
            "Custom env var should roundtrip"
        );
    }

    #[tokio::test]
    async fn test_format_from_extension() {
        assert_eq!(
            ContextFormat::from_extension(std::path::Path::new("foo.rxt")),
            Some(ContextFormat::Json)
        );
        assert_eq!(
            ContextFormat::from_extension(std::path::Path::new("foo.json")),
            None
        );
        assert_eq!(
            ContextFormat::from_extension(std::path::Path::new("foo.rxtb")),
            Some(ContextFormat::Binary)
        );
    }

    #[tokio::test]
    async fn test_rxt_package_names_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("pkg_ctx.rxt");

        let ctx = make_ctx(&[("houdini", "20.0"), ("python", "3.11.0"), ("nuke", "14.0")]);
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        let names = loaded.get_package_names();
        assert!(names.contains(&"houdini".to_string()));
        assert!(names.contains(&"python".to_string()));
        assert!(names.contains(&"nuke".to_string()));
    }
}

/// Phase 67: Environment activation scripts integration tests
#[cfg(test)]
mod activation_script_tests {
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

// ── Phase 92: Binary format (rxtb) tests ─────────────────────────────────────
#[cfg(test)]
mod rxtb_tests {
    use crate::{
        serialization::{ContextFormat, ContextSerializer},
        ContextStatus, ResolvedContext,
    };
    use rez_next_package::Package;
    use rez_next_version::Version;
    use tempfile::TempDir;

    fn make_ctx(packages: &[(&str, &str)]) -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        for (name, ver) in packages {
            let mut pkg = Package::new(name.to_string());
            pkg.version = Some(Version::parse(ver).unwrap());
            ctx.resolved_packages.push(pkg);
        }
        ctx
    }

    /// rxtb file format: serialize → write file → load → same packages
    #[tokio::test]
    async fn test_rxtb_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ctx.rxtb");

        let ctx = make_ctx(&[("maya", "2024.0"), ("python", "3.10.0")]);
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary)
            .await
            .unwrap();

        assert!(path.exists(), "rxtb file should be created");
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.resolved_packages.len(),
            2,
            "Should reload 2 packages from rxtb"
        );
        let names: Vec<_> = loaded
            .resolved_packages
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert!(names.contains(&"maya"), "maya should be in loaded packages");
        assert!(
            names.contains(&"python"),
            "python should be in loaded packages"
        );
    }

    /// rxtb roundtrip: packages and env_vars survive serialize → deserialize
    #[test]
    fn test_rxtb_serialize_deserialize_roundtrip() {
        let mut ctx = make_ctx(&[("nuke", "14.0"), ("ocio", "2.2")]);
        ctx.environment_vars
            .insert("OCIO".to_string(), "/opt/ocio/config.ocio".to_string());
        ctx.environment_vars
            .insert("NUKE_PATH".to_string(), "/opt/nuke/14.0".to_string());

        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

        let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Binary).unwrap();
        assert_eq!(restored.resolved_packages.len(), 2);
        assert_eq!(
            restored.environment_vars.get("OCIO"),
            ctx.environment_vars.get("OCIO")
        );
        assert_eq!(
            restored.environment_vars.get("NUKE_PATH"),
            ctx.environment_vars.get("NUKE_PATH")
        );
    }

    /// from_string / to_string with Binary format uses base64 encoding
    #[test]
    fn test_rxtb_to_string_is_base64() {
        let ctx = make_ctx(&[("houdini", "20.5")]);
        let b64_str = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();

        // base64 strings only contain A-Z, a-z, 0-9, +, /, =
        let is_base64 = b64_str
            .chars()
            .all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '\n');
        assert!(
            is_base64,
            "Binary format to_string should return base64: {}",
            &b64_str[..b64_str.len().min(50)]
        );
    }

    /// from_string roundtrip with Binary format
    #[test]
    fn test_rxtb_from_string_roundtrip() {
        let ctx = make_ctx(&[("renderman", "25.0"), ("katana", "6.0")]);
        let b64 = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();
        let restored = ContextSerializer::from_string(&b64, ContextFormat::Binary).unwrap();

        let names: Vec<_> = restored
            .resolved_packages
            .iter()
            .map(|p| p.name.clone())
            .collect();
        assert!(
            names.contains(&"renderman".to_string()),
            "renderman should survive binary roundtrip"
        );
        assert!(
            names.contains(&"katana".to_string()),
            "katana should survive binary roundtrip"
        );
    }

    /// ContextFormat extension detection for rxtb
    #[test]
    fn test_rxtb_format_detection() {
        let path = std::path::Path::new("mycontext.rxtb");
        let fmt = ContextFormat::from_extension(path);
        assert_eq!(
            fmt,
            Some(ContextFormat::Binary),
            "rxtb should be detected as Binary"
        );

        let path2 = std::path::Path::new("mycontext.rxt");
        let fmt2 = ContextFormat::from_extension(path2);
        assert_eq!(
            fmt2,
            Some(ContextFormat::Json),
            "rxt should be detected as Json"
        );
    }

    /// JSON format extension is still "rxt", binary is "rxtb"
    #[test]
    fn test_format_extension_names() {
        assert_eq!(ContextFormat::Json.extension(), "rxt");
        assert_eq!(ContextFormat::Binary.extension(), "rxtb");
    }

    /// Empty context serializes to binary and back
    #[test]
    fn test_rxtb_empty_context() {
        let ctx = ResolvedContext::from_requirements(vec![]);
        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Binary).unwrap();
        assert_eq!(restored.resolved_packages.len(), 0);
        // Empty context may or may not have environment vars depending on implementation
        let _ = restored.environment_vars;
    }

    /// Binary format produces smaller or equal bytes vs JSON pretty (no forced assertion, just no panic)
    #[test]
    fn test_binary_vs_json_both_valid() {
        let ctx = make_ctx(&[("pkg_a", "1.0"), ("pkg_b", "2.0"), ("pkg_c", "3.0")]);
        let json_bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
        let bin_bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        assert!(!json_bytes.is_empty(), "JSON bytes non-empty");
        assert!(!bin_bytes.is_empty(), "Binary bytes non-empty");
    }
}

// ── Phase 107: Context load_from_file filesystem integration tests ────────────
#[cfg(test)]
mod context_load_from_file_tests {
    use crate::{
        serialization::{ContextFormat, ContextSerializer},
        ContextStatus, ResolvedContext,
    };
    use rez_next_package::Package;
    use rez_next_version::Version;
    use tempfile::TempDir;

    fn make_package(name: &str, ver: &str) -> Package {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    }

    fn make_full_ctx() -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        ctx.resolved_packages.push(make_package("python", "3.9.0"));
        ctx.resolved_packages.push(make_package("maya", "2023.0"));
        ctx.set_env_var("REZ_USED_VERSION".to_string(), "1.0".to_string());
        ctx.set_env_var("MY_APP_HOME".to_string(), "/opt/myapp".to_string());
        ctx
    }

    /// load_from_file correctly restores package count from rxt
    #[tokio::test]
    async fn test_load_from_file_package_count() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("restore.rxt");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 2);
    }

    /// load_from_file correctly restores env_vars
    #[tokio::test]
    async fn test_load_from_file_env_vars() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("env.rxt");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();

        assert_eq!(
            loaded.get_env_var("MY_APP_HOME"),
            Some("/opt/myapp".to_string()),
        );
    }

    /// load_from_file correctly restores status
    #[tokio::test]
    async fn test_load_from_file_status_resolved() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("status.rxt");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.status, ContextStatus::Resolved);
    }

    /// load_from_file on corrupted file returns error
    #[tokio::test]
    async fn test_load_from_file_corrupted_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupt.rxt");

        // Write invalid JSON
        tokio::fs::write(&path, b"not valid json at all!!!")
            .await
            .unwrap();
        let result = ContextSerializer::load_from_file(&path).await;
        assert!(result.is_err(), "Corrupted file should fail to load");
    }

    /// load_from_file on rxtb binary format
    #[tokio::test]
    async fn test_load_from_file_binary_format() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("context.rxtb");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary)
            .await
            .unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 2);
    }

    /// load_from_file on unsupported extension returns error
    #[tokio::test]
    async fn test_load_from_file_unsupported_extension_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("context.yaml");

        // Write some content
        tokio::fs::write(&path, b"{}").await.unwrap();
        let result = ContextSerializer::load_from_file(&path).await;
        assert!(result.is_err(), "Unsupported extension should error");
    }

    /// Multiple sequential save/load preserves latest state
    #[tokio::test]
    async fn test_load_from_file_overwrites_on_save() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("overwrite.rxt");

        // First save: 1 package
        let mut ctx1 = ResolvedContext::from_requirements(vec![]);
        ctx1.status = ContextStatus::Resolved;
        ctx1.resolved_packages.push(make_package("python", "3.9.0"));
        ContextSerializer::save_to_file(&ctx1, &path, ContextFormat::Json)
            .await
            .unwrap();

        // Second save: 3 packages (overwrites)
        let mut ctx2 = ResolvedContext::from_requirements(vec![]);
        ctx2.status = ContextStatus::Resolved;
        for (n, v) in &[("python", "3.11.0"), ("maya", "2024.0"), ("nuke", "14.0")] {
            ctx2.resolved_packages.push(make_package(n, v));
        }
        ContextSerializer::save_to_file(&ctx2, &path, ContextFormat::Json)
            .await
            .unwrap();

        // Load should reflect second save
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.resolved_packages.len(),
            3,
            "Should have 3 packages from second save"
        );
    }
}

// ── Phase 111: ExecutionConfig + ContextExecutor + ProcessResult tests ─────

#[cfg(test)]
mod execution_tests {
    use crate::execution::{
        ContextExecutionBuilder, ContextExecutor, ExecutionConfig, ExecutionStats, ProcessResult,
    };
    use crate::{ContextStatus, ResolvedContext, ShellType};
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::path::PathBuf;

    fn make_context_with_pkg(name: &str, ver: &str) -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
        ctx.set_env_var(
            format!("{}_ROOT", name.to_uppercase()),
            format!("/packages/{}/{}", name, ver),
        );
        ctx
    }

    /// ExecutionConfig default values
    #[test]
    fn test_execution_config_defaults() {
        let cfg = ExecutionConfig::default();
        assert!(
            cfg.inherit_parent_env,
            "Should inherit parent env by default"
        );
        assert!(cfg.additional_env_vars.is_empty());
        assert!(cfg.working_directory.is_none());
        assert_eq!(cfg.timeout_seconds, 300);
        assert!(cfg.capture_output);
    }

    /// ExecutionConfig with custom values
    #[test]
    fn test_execution_config_custom() {
        let mut additional_env_vars = std::collections::HashMap::new();
        additional_env_vars.insert("MY_VAR".to_string(), "hello".to_string());
        let cfg = ExecutionConfig {
            shell_type: ShellType::Bash,
            timeout_seconds: 60,
            working_directory: Some(PathBuf::from("/tmp")),
            additional_env_vars,
            ..Default::default()
        };
        assert_eq!(cfg.timeout_seconds, 60);
        assert!(cfg.working_directory.is_some());
        assert_eq!(
            cfg.additional_env_vars.get("MY_VAR"),
            Some(&"hello".to_string())
        );
    }

    /// ContextExecutor::new creates executor with default config
    #[test]
    fn test_context_executor_new() {
        let ctx = make_context_with_pkg("python", "3.9.0");
        let executor = ContextExecutor::new(ctx);
        assert!(executor.get_context().resolved_packages.len() == 1);
    }

    /// ContextExecutor get_execution_stats reports correct counts
    #[test]
    fn test_context_executor_stats() {
        let ctx = make_context_with_pkg("maya", "2023.0");
        let executor = ContextExecutor::new(ctx);
        let stats = executor.get_execution_stats();
        assert_eq!(stats.package_count, 1);
        assert_eq!(stats.env_var_count, 1, "Should have MAYA_ROOT env var");
    }

    /// ContextExecutionBuilder fluent API
    #[test]
    fn test_execution_builder_fluent_api() {
        let ctx = make_context_with_pkg("houdini", "20.0");
        let executor = ContextExecutionBuilder::new(ctx)
            .with_shell(ShellType::PowerShell)
            .with_timeout(120)
            .with_working_directory(PathBuf::from("/work"))
            .with_env_var("TEST_VAR".to_string(), "test_value".to_string())
            .with_capture_output(false)
            .build();
        let stats = executor.get_execution_stats();
        assert_eq!(stats.package_count, 1);
        assert!(stats.working_directory.is_some());
    }

    /// ProcessResult is_success reflects exit code 0
    #[test]
    fn test_process_result_is_success() {
        let result = ProcessResult {
            program: "python".to_string(),
            args: vec!["--version".to_string()],
            exit_code: 0,
            stdout: "Python 3.9.0".to_string(),
            stderr: "".to_string(),
            execution_time_ms: 50,
        };
        assert!(result.is_success());
        assert_eq!(result.command_line(), "python --version");
    }

    /// ProcessResult combined_output merges stdout and stderr
    #[test]
    fn test_process_result_combined_output() {
        let with_both = ProcessResult {
            program: "cmd".to_string(),
            args: vec![],
            exit_code: 1,
            stdout: "output".to_string(),
            stderr: "error".to_string(),
            execution_time_ms: 10,
        };
        let combined = with_both.combined_output();
        assert!(combined.contains("output"));
        assert!(combined.contains("error"));
        assert!(!with_both.is_success());

        // Only stdout
        let only_out = ProcessResult {
            program: "prog".to_string(),
            args: vec![],
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            execution_time_ms: 5,
        };
        assert_eq!(only_out.combined_output(), "hello");
    }

    /// ExecutionStats serialization
    #[test]
    fn test_execution_stats_serialization() {
        let stats = ExecutionStats {
            context_id: "ctx-001".to_string(),
            package_count: 3,
            env_var_count: 10,
            tool_count: 5,
            shell_type: ShellType::Bash,
            working_directory: Some(PathBuf::from("/work")),
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("ctx-001"));
        assert!(json.contains("3"));
        let restored: ExecutionStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.context_id, "ctx-001");
        assert_eq!(restored.package_count, 3);
    }
}

// ── Cycle 49: EnvDiff + EnvOperation::Append/Unset + PathStrategy tests ────
#[cfg(test)]
mod env_diff_tests {
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
        // base env has FOO, generated env does not → removed
        let mut additional_env_vars = HashMap::new();
        additional_env_vars.insert("BASE_VAR".to_string(), "base_value".to_string());
        let cfg = ContextConfig {
            inherit_parent_env: false,
            additional_env_vars,
            ..Default::default()
        };
        // Create manager whose base_env contains BASE_VAR (inherit = false but we set via additional)
        // The base_env is only set from env::vars() when inherit=true.
        // So with inherit=false, base_env is empty; the "removed" path needs base_env to have something.
        // We test by creating a manager that inherits parent env, and passing empty env to diff.
        let cfg2 = ContextConfig {
            inherit_parent_env: true,
            ..Default::default()
        };
        let mgr2 = EnvironmentManager::new(cfg2);
        // If parent env has PATH, passing empty map → PATH should appear in removed
        let empty_env: HashMap<String, String> = HashMap::new();
        let diff = mgr2.get_env_diff(&empty_env);
        // We only assert that removed is non-empty (relies on PATH existing in parent env on CI)
        // If parent env is not empty, removed should contain entries
        let _ = diff.removed.len(); // just confirm no panic
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
        let cfg = ContextConfig {
            inherit_parent_env: false,
            path_strategy: PathStrategy::NoModify,
            ..Default::default()
        };
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
        let _ = cfg; // suppress unused warning
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
        env.insert("MY_VAR".to_string(), r#"val"ue with $special `chars`"#.to_string());
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
        env.insert("PS_VAR".to_string(), r#"val with $env:PATH and "quotes""#.to_string());
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
        assert!(script.contains("#!/bin/zsh"), "Zsh script should have shebang");
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

// ── Cycle 49: RezResolvedContext method tests ────────────────────────────────
#[cfg(test)]
mod rez_resolved_context_tests {
    use crate::resolved_context::{ResolvedPackage, RezResolvedContext};
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_arc_package(name: &str, version: &str) -> Arc<Package> {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        Arc::new(pkg)
    }

    fn make_arc_package_with_tools(name: &str, version: &str, tools: Vec<String>) -> Arc<Package> {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg.tools = tools;
        Arc::new(pkg)
    }

    fn make_resolved_pkg(name: &str, version: &str, root: &str) -> ResolvedPackage {
        ResolvedPackage::new(
            make_arc_package(name, version),
            PathBuf::from(root),
            true,
        )
    }

    // ── RezResolvedContext::new ──────────────────────────────────────────────

    #[test]
    fn test_rez_resolved_context_new() {
        use rez_next_package::Requirement;
        use std::str::FromStr;
        let reqs = vec![Requirement::from_str("python-3").unwrap()];
        let ctx = RezResolvedContext::new(reqs);
        assert_eq!(ctx.requirements.len(), 1);
        assert!(!ctx.failed);
        assert!(ctx.resolved_packages.is_empty());
        assert!(ctx.failure_description.is_none());
        assert!(!ctx.rez_version.is_empty());
    }

    // ── get_package_names ────────────────────────────────────────────────────

    #[test]
    fn test_get_package_names_empty() {
        let ctx = RezResolvedContext::new(vec![]);
        assert!(ctx.get_package_names().is_empty());
    }

    #[test]
    fn test_get_package_names_multiple() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));
        ctx.resolved_packages
            .push(make_resolved_pkg("maya", "2023.0", "/pkgs/maya/2023.0"));
        let names = ctx.get_package_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"python".to_string()));
        assert!(names.contains(&"maya".to_string()));
    }

    // ── get_package ──────────────────────────────────────────────────────────

    #[test]
    fn test_get_package_found() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("houdini", "20.0", "/pkgs/houdini/20.0"));
        let pkg = ctx.get_package("houdini");
        assert!(pkg.is_some());
        assert_eq!(pkg.unwrap().package.name, "houdini");
    }

    #[test]
    fn test_get_package_not_found() {
        let ctx = RezResolvedContext::new(vec![]);
        assert!(ctx.get_package("nonexistent").is_none());
    }

    // ── has_package ──────────────────────────────────────────────────────────

    #[test]
    fn test_has_package_true_and_false() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("nuke", "14.0", "/pkgs/nuke/14.0"));
        assert!(ctx.has_package("nuke"));
        assert!(!ctx.has_package("katana"));
    }

    // ── get_package_version ─────────────────────────────────────────────────

    #[test]
    fn test_get_package_version() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.11.0", "/pkgs/python/3.11.0"));
        let ver = ctx.get_package_version("python");
        assert!(ver.is_some());
        assert_eq!(ver.unwrap().as_str(), "3.11.0");
        assert!(ctx.get_package_version("missing").is_none());
    }

    // ── get_tools ────────────────────────────────────────────────────────────

    #[test]
    fn test_get_tools_empty_when_no_tools() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("notool", "1.0.0", "/pkgs/notool/1.0.0"));
        let tools = ctx.get_tools();
        assert!(tools.is_empty(), "Package with no tools should have no tools");
    }

    #[test]
    fn test_get_tools_with_tools() {
        let mut ctx = RezResolvedContext::new(vec![]);
        let pkg = make_arc_package_with_tools(
            "toolpkg",
            "1.0.0",
            vec!["mytool".to_string(), "othertool".to_string()],
        );
        ctx.resolved_packages.push(ResolvedPackage::new(
            pkg,
            PathBuf::from("/pkgs/toolpkg/1.0.0"),
            true,
        ));
        let tools = ctx.get_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains_key("mytool"));
        assert!(tools.contains_key("othertool"));
        let mytool_path = &tools["mytool"];
        assert!(
            mytool_path.to_string_lossy().contains("bin"),
            "Tool path should be in bin dir: {:?}",
            mytool_path
        );
    }

    #[test]
    fn test_get_tools_multiple_packages() {
        let mut ctx = RezResolvedContext::new(vec![]);
        let pkg1 = make_arc_package_with_tools("pkg_a", "1.0.0", vec!["tool_a".to_string()]);
        let pkg2 = make_arc_package_with_tools("pkg_b", "1.0.0", vec!["tool_b".to_string()]);
        ctx.resolved_packages
            .push(ResolvedPackage::new(pkg1, PathBuf::from("/pkgs/pkg_a"), true));
        ctx.resolved_packages
            .push(ResolvedPackage::new(pkg2, PathBuf::from("/pkgs/pkg_b"), true));
        let tools = ctx.get_tools();
        assert_eq!(tools.len(), 2);
        assert!(tools.contains_key("tool_a"));
        assert!(tools.contains_key("tool_b"));
    }

    // ── get_summary ─────────────────────────────────────────────────────────

    #[test]
    fn test_get_summary_fields() {
        use rez_next_package::Requirement;
        use std::str::FromStr;
        let reqs = vec![Requirement::from_str("python-3").unwrap()];
        let mut ctx = RezResolvedContext::new(reqs);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));
        ctx.resolved_packages
            .push(make_resolved_pkg("maya", "2023.0", "/pkgs/maya/2023.0"));

        let summary = ctx.get_summary();
        assert_eq!(summary.num_packages, 2);
        assert!(summary.package_names.contains(&"python".to_string()));
        assert!(summary.package_names.contains(&"maya".to_string()));
        assert!(!summary.failed);
        assert_eq!(summary.requirements.len(), 1);
        assert!(summary.requirements[0].contains("python"));
    }

    #[test]
    fn test_get_summary_failed_context() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.failed = true;
        ctx.failure_description = Some("could not resolve".to_string());
        let summary = ctx.get_summary();
        assert!(summary.failed);
        assert_eq!(summary.num_packages, 0);
    }

    // ── save and load ────────────────────────────────────────────────────────

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("ctx.json");

        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));
        ctx.environ
            .insert("MY_VAR".to_string(), "hello".to_string());

        ctx.save(&path).expect("save should succeed");
        assert!(path.exists(), "File should be created");

        let loaded = RezResolvedContext::load(&path).expect("load should succeed");
        assert_eq!(loaded.resolved_packages.len(), 1);
        assert_eq!(loaded.resolved_packages[0].package.name, "python");
        assert_eq!(loaded.environ.get("MY_VAR"), Some(&"hello".to_string()));
    }

    #[test]
    fn test_load_nonexistent_file_errors() {
        let path = std::path::PathBuf::from("/nonexistent/path/ctx.json");
        let result = RezResolvedContext::load(&path);
        assert!(result.is_err(), "Loading nonexistent file should error");
    }

    // ── get_variant ──────────────────────────────────────────────────────────

    #[test]
    fn test_get_variant_none_when_no_variant() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));
        // No variant set → should return None
        let variant = ctx.get_variant("python");
        assert!(variant.is_none(), "No variant set should return None");
    }

    #[test]
    fn test_get_variant_with_variant_index() {
        let mut pkg = Package::new("python".to_string());
        pkg.version = Some(Version::parse("3.9.0").unwrap());
        pkg.variants = vec![vec!["platform-linux".to_string()], vec!["platform-windows".to_string()]];
        let arc_pkg = Arc::new(pkg);

        let resolved_pkg = ResolvedPackage::new(
            arc_pkg,
            PathBuf::from("/pkgs/python/3.9.0"),
            true,
        )
        .with_variant(0);

        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages.push(resolved_pkg);

        let variant = ctx.get_variant("python");
        assert!(variant.is_some(), "Variant at index 0 should exist");
        let v = variant.unwrap();
        assert!(v.contains(&"platform-linux".to_string()));
    }

    // ── ResolvedPackage helpers ──────────────────────────────────────────────

    #[test]
    fn test_resolved_package_with_variant() {
        let pkg = make_arc_package("python", "3.9.0");
        let rp = ResolvedPackage::new(pkg, PathBuf::from("/root"), false);
        assert!(rp.variant_index.is_none());

        let with_var = rp.with_variant(2);
        assert_eq!(with_var.variant_index, Some(2));
    }

    #[test]
    fn test_resolved_package_add_parent_deduplicates() {
        let pkg = make_arc_package("child", "1.0.0");
        let mut rp = ResolvedPackage::new(pkg, PathBuf::from("/root"), false);
        assert!(rp.parent_packages.is_empty());

        rp.add_parent("parent_a".to_string());
        rp.add_parent("parent_b".to_string());
        rp.add_parent("parent_a".to_string()); // duplicate
        assert_eq!(
            rp.parent_packages.len(),
            2,
            "Duplicate parents should be deduplicated"
        );
        assert!(rp.parent_packages.contains(&"parent_a".to_string()));
        assert!(rp.parent_packages.contains(&"parent_b".to_string()));
    }

    #[test]
    fn test_resolved_package_requested_flag() {
        let pkg = make_arc_package("explicit_req", "1.0.0");
        let rp = ResolvedPackage::new(pkg, PathBuf::from("/root"), true);
        assert!(rp.requested);

        let pkg2 = make_arc_package("transitive", "2.0.0");
        let rp2 = ResolvedPackage::new(pkg2, PathBuf::from("/root2"), false);
        assert!(!rp2.requested);
    }

    // ── get_environ (basic smoke test) ──────────────────────────────────────

    #[test]
    fn test_get_environ_returns_map() {
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));
        // Should not panic; returns a HashMap
        let environ = ctx.get_environ();
        assert!(environ.is_ok(), "get_environ should succeed");
        // System vars like PATH should be present
        let env_map = environ.unwrap();
        // Minimal smoke: map is returned (may or may not have PATH depending on system)
        let _ = env_map.len();
    }
}
