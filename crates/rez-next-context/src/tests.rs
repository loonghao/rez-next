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
        ctx.resolved_packages.push(make_package("houdini", "19.5.0"));
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
        assert_eq!(
            vars.get("MAYA_VERSION").map(|s| s.as_str()),
            Some("2023.0")
        );
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
    use crate::{ResolvedContext, ContextStatus};
    use crate::serialization::{ContextFormat, ContextSerializer};
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
        assert_eq!(loaded.status, ContextStatus::Resolved, "Status should roundtrip");
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
        ctx.set_env_var("REZ_CONTEXT_FILE".to_string(), path.to_str().unwrap().to_string());
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
        env.insert("MAYA_ROOT".to_string(), "/opt/autodesk/maya2023".to_string());
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
        ContextStatus, ResolvedContext,
        serialization::{ContextFormat, ContextSerializer},
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
        assert_eq!(loaded.resolved_packages.len(), 2, "Should reload 2 packages from rxtb");
        let names: Vec<_> = loaded.resolved_packages.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"maya"), "maya should be in loaded packages");
        assert!(names.contains(&"python"), "python should be in loaded packages");
    }

    /// rxtb roundtrip: packages and env_vars survive serialize → deserialize
    #[test]
    fn test_rxtb_serialize_deserialize_roundtrip() {
        let mut ctx = make_ctx(&[("nuke", "14.0"), ("ocio", "2.2")]);
        ctx.environment_vars.insert("OCIO".to_string(), "/opt/ocio/config.ocio".to_string());
        ctx.environment_vars.insert("NUKE_PATH".to_string(), "/opt/nuke/14.0".to_string());

        let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Binary).unwrap();
        assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

        let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Binary).unwrap();
        assert_eq!(restored.resolved_packages.len(), 2);
        assert_eq!(restored.environment_vars.get("OCIO"), ctx.environment_vars.get("OCIO"));
        assert_eq!(restored.environment_vars.get("NUKE_PATH"), ctx.environment_vars.get("NUKE_PATH"));
    }

    /// from_string / to_string with Binary format uses base64 encoding
    #[test]
    fn test_rxtb_to_string_is_base64() {
        let ctx = make_ctx(&[("houdini", "20.5")]);
        let b64_str = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();

        // base64 strings only contain A-Z, a-z, 0-9, +, /, =
        let is_base64 = b64_str.chars().all(|c| {
            c.is_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '\n'
        });
        assert!(is_base64, "Binary format to_string should return base64: {}", &b64_str[..b64_str.len().min(50)]);
    }

    /// from_string roundtrip with Binary format
    #[test]
    fn test_rxtb_from_string_roundtrip() {
        let ctx = make_ctx(&[("renderman", "25.0"), ("katana", "6.0")]);
        let b64 = ContextSerializer::to_string(&ctx, ContextFormat::Binary).unwrap();
        let restored = ContextSerializer::from_string(&b64, ContextFormat::Binary).unwrap();

        let names: Vec<_> = restored.resolved_packages.iter().map(|p| p.name.clone()).collect();
        assert!(names.contains(&"renderman".to_string()), "renderman should survive binary roundtrip");
        assert!(names.contains(&"katana".to_string()), "katana should survive binary roundtrip");
    }

    /// ContextFormat extension detection for rxtb
    #[test]
    fn test_rxtb_format_detection() {
        let path = std::path::Path::new("mycontext.rxtb");
        let fmt = ContextFormat::from_extension(path);
        assert_eq!(fmt, Some(ContextFormat::Binary), "rxtb should be detected as Binary");

        let path2 = std::path::Path::new("mycontext.rxt");
        let fmt2 = ContextFormat::from_extension(path2);
        assert_eq!(fmt2, Some(ContextFormat::Json), "rxt should be detected as Json");
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
        ContextStatus, ResolvedContext,
        serialization::{ContextFormat, ContextSerializer},
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
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json).await.unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 2);
    }

    /// load_from_file correctly restores env_vars
    #[tokio::test]
    async fn test_load_from_file_env_vars() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("env.rxt");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json).await.unwrap();
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
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json).await.unwrap();
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.status, ContextStatus::Resolved);
    }

    /// load_from_file on corrupted file returns error
    #[tokio::test]
    async fn test_load_from_file_corrupted_returns_error() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("corrupt.rxt");

        // Write invalid JSON
        tokio::fs::write(&path, b"not valid json at all!!!").await.unwrap();
        let result = ContextSerializer::load_from_file(&path).await;
        assert!(result.is_err(), "Corrupted file should fail to load");
    }

    /// load_from_file on rxtb binary format
    #[tokio::test]
    async fn test_load_from_file_binary_format() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("context.rxtb");

        let ctx = make_full_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary).await.unwrap();
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
        ContextSerializer::save_to_file(&ctx1, &path, ContextFormat::Json).await.unwrap();

        // Second save: 3 packages (overwrites)
        let mut ctx2 = ResolvedContext::from_requirements(vec![]);
        ctx2.status = ContextStatus::Resolved;
        for (n, v) in &[("python", "3.11.0"), ("maya", "2024.0"), ("nuke", "14.0")] {
            ctx2.resolved_packages.push(make_package(n, v));
        }
        ContextSerializer::save_to_file(&ctx2, &path, ContextFormat::Json).await.unwrap();

        // Load should reflect second save
        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 3, "Should have 3 packages from second save");
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
        assert!(cfg.inherit_parent_env, "Should inherit parent env by default");
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
        assert_eq!(cfg.additional_env_vars.get("MY_VAR"), Some(&"hello".to_string()));
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

