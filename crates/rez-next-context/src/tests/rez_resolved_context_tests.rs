// ── Cycle 49: RezResolvedContext method tests ────────────────────────────────
#[cfg(test)]
mod rez_resolved_context_behavior_tests {

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
        let expected_env: std::collections::HashMap<String, String> =
            std::env::vars().collect();
        let mut ctx = RezResolvedContext::new(vec![]);
        ctx.resolved_packages
            .push(make_resolved_pkg("python", "3.9.0", "/pkgs/python/3.9.0"));

        let env_map = ctx.get_environ().expect("get_environ should succeed");

        assert_eq!(env_map, expected_env);
    }
}
