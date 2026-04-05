// ── Phase 107: Context load_from_file filesystem integration tests ────────────
#[cfg(test)]
mod context_load_behavior_tests {

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
