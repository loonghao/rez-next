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
