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
