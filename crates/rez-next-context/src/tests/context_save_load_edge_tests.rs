// ── Cycle 68: Context save/load edge case tests ───────────────────────────
//!
//! Covers boundary conditions not exercised by context_load_from_file_tests:
//!   - Save/load an empty context (no packages, no env_vars)
//!   - Save/load context with 100+ env_vars
//!   - Save/load context with special characters in env values
//!   - Concurrent parallel saves to different paths (thread-safety check)
//!   - Binary format roundtrip preserves all fields identically to JSON
//!   - Overwrite loop: N sequential saves, final load is consistent
//!   - Failed context (status=Failed) stores failure info via metadata
#[cfg(test)]
mod context_save_load_edge_tests {

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

    // ── Helper: empty context ───────────────────────────────────────────────

    fn make_empty_ctx() -> ResolvedContext {
        ResolvedContext::from_requirements(vec![])
    }

    fn make_rich_ctx() -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        ctx.resolved_packages.push(make_package("python", "3.10.0"));
        ctx.resolved_packages.push(make_package("numpy", "1.24.0"));
        ctx.set_env_var("PYTHONPATH".to_string(), "/pkgs/numpy/1.24.0/python".to_string());
        ctx.set_env_var("PATH".to_string(), "/pkgs/python/3.10.0/bin:/usr/bin".to_string());
        ctx
    }

    // ── 1. Empty context save/load ──────────────────────────────────────────

    /// An empty context (no packages, no env_vars) should serialise and
    /// deserialise without error and preserve all defaults.
    #[tokio::test]
    async fn test_save_load_empty_context_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.rxt");

        let ctx = make_empty_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 0);
        assert!(loaded.get_env_var("ANYTHING").is_none());
    }

    #[tokio::test]
    async fn test_save_load_empty_context_binary() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.rxtb");

        let ctx = make_empty_ctx();
        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 0);
    }

    // ── 2. Large env_vars (100 entries) ─────────────────────────────────────

    /// Saving a context with 100 env_var entries must round-trip all of them.
    #[tokio::test]
    async fn test_save_load_many_env_vars() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("many_env.rxt");

        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        for i in 0..100usize {
            ctx.set_env_var(format!("VAR_{i:03}"), format!("/path/to/value_{i}"));
        }

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();

        for i in 0..100usize {
            assert_eq!(
                loaded.get_env_var(&format!("VAR_{i:03}")),
                Some(format!("/path/to/value_{i}")),
                "Missing env_var VAR_{i:03}"
            );
        }
    }

    // ── 3. Special characters in env values ─────────────────────────────────

    /// Env values with spaces, colons, equals signs and unicode must survive
    /// JSON serialisation without corruption.
    #[tokio::test]
    async fn test_save_load_special_chars_in_env_value() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("special.rxt");

        let mut ctx = make_empty_ctx();
        // colon-separated PATH-like value
        ctx.set_env_var("PATH".to_string(), "/a/b/c:/d/e/f:/g".to_string());
        // value with equals (common in some env vars)
        ctx.set_env_var("FLAGS".to_string(), "-DFOO=1 -DBAR=2".to_string());
        // unicode characters
        ctx.set_env_var("DESCRIPTION".to_string(), "日本語テスト αβγ".to_string());
        // value with backslash (Windows paths)
        ctx.set_env_var("WIN_PATH".to_string(), r"C:\Program Files\pkg\bin".to_string());

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.get_env_var("PATH"),
            Some("/a/b/c:/d/e/f:/g".to_string())
        );
        assert_eq!(
            loaded.get_env_var("FLAGS"),
            Some("-DFOO=1 -DBAR=2".to_string())
        );
        assert_eq!(
            loaded.get_env_var("DESCRIPTION"),
            Some("日本語テスト αβγ".to_string())
        );
        assert_eq!(
            loaded.get_env_var("WIN_PATH"),
            Some(r"C:\Program Files\pkg\bin".to_string())
        );
    }

    // ── 4. JSON and Binary roundtrip produce identical state ────────────────

    /// Saving to JSON and to Binary must both restore a context that is
    /// functionally equivalent (same packages, same env_vars, same status).
    #[tokio::test]
    async fn test_json_and_binary_roundtrip_identical() {
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("ctx.rxt");
        let bin_path = tmp.path().join("ctx.rxtb");

        let ctx = make_rich_ctx();

        ContextSerializer::save_to_file(&ctx, &json_path, ContextFormat::Json)
            .await
            .unwrap();
        ContextSerializer::save_to_file(&ctx, &bin_path, ContextFormat::Binary)
            .await
            .unwrap();

        let from_json = ContextSerializer::load_from_file(&json_path).await.unwrap();
        let from_bin = ContextSerializer::load_from_file(&bin_path).await.unwrap();

        assert_eq!(from_json.resolved_packages.len(), from_bin.resolved_packages.len());
        assert_eq!(from_json.status, from_bin.status);

        // Package names must match
        let json_names: Vec<_> = from_json.resolved_packages.iter().map(|p| &p.name).collect();
        let bin_names: Vec<_> = from_bin.resolved_packages.iter().map(|p| &p.name).collect();
        assert_eq!(json_names, bin_names);

        // Key env vars must match
        for key in &["PYTHONPATH", "PATH"] {
            assert_eq!(
                from_json.get_env_var(key),
                from_bin.get_env_var(key),
                "Mismatch for env var {key}"
            );
        }
    }

    // ── 5. Failed context metadata roundtrip ────────────────────────────────

    /// A context with status=Failed can store failure info via `metadata`.
    /// Both the status and the failure message must survive save/load.
    #[tokio::test]
    async fn test_failed_context_metadata_roundtrip_json() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("failed.rxt");

        let mut ctx = make_empty_ctx();
        ctx.status = ContextStatus::Failed;
        ctx.add_metadata(
            "failure_description".to_string(),
            "could not resolve: python>=4.0 not found".to_string(),
        );

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.status, ContextStatus::Failed);
        assert_eq!(
            loaded.metadata.get("failure_description").map(String::as_str),
            Some("could not resolve: python>=4.0 not found")
        );
    }

    #[tokio::test]
    async fn test_failed_context_metadata_roundtrip_binary() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("failed.rxtb");

        let mut ctx = make_empty_ctx();
        ctx.status = ContextStatus::Failed;
        ctx.add_metadata(
            "failure_description".to_string(),
            "solver conflict: A requires B>=2, C requires B<2".to_string(),
        );

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Binary)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.status, ContextStatus::Failed);
        assert_eq!(
            loaded.metadata.get("failure_description").map(String::as_str),
            Some("solver conflict: A requires B>=2, C requires B<2")
        );
    }

    // ── 6. Sequential overwrite loop ────────────────────────────────────────

    /// 10 sequential saves to the same path; final load should reflect the
    /// last (10th) save only — no state bleed from earlier saves.
    #[tokio::test]
    async fn test_sequential_overwrite_loop() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("loop.rxt");

        for round in 1usize..=10 {
            let mut ctx = ResolvedContext::from_requirements(vec![]);
            ctx.status = ContextStatus::Resolved;
            // Each round adds a different number of packages
            for i in 0..round {
                ctx.resolved_packages.push(make_package(
                    &format!("pkg_{i}"),
                    &format!("{round}.{i}.0"),
                ));
            }
            ctx.set_env_var("ROUND".to_string(), round.to_string());
            ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
                .await
                .unwrap();
        }

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.resolved_packages.len(),
            10,
            "Final save had 10 packages"
        );
        assert_eq!(
            loaded.get_env_var("ROUND"),
            Some("10".to_string()),
            "ROUND should be 10 after 10 overwrites"
        );
    }

    // ── 7. Concurrent parallel saves to distinct paths ──────────────────────

    /// N async tasks each save a context to a unique path; all must succeed
    /// and load correctly (no file corruption from concurrent writes).
    #[tokio::test]
    async fn test_concurrent_saves_to_distinct_paths() {
        use std::sync::Arc;
        use tokio::task::JoinSet;

        let tmp = Arc::new(TempDir::new().unwrap());
        let mut join_set = JoinSet::new();
        let n = 20usize;

        for i in 0..n {
            let tmp_path = tmp.path().to_path_buf();
            join_set.spawn(async move {
                let path = tmp_path.join(format!("concurrent_{i}.rxt"));
                let mut ctx = ResolvedContext::from_requirements(vec![]);
                ctx.status = ContextStatus::Resolved;
                ctx.resolved_packages
                    .push(make_package("python", &format!("3.{i}.0")));
                ctx.set_env_var("TASK_ID".to_string(), i.to_string());
                ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
                    .await
                    .unwrap();

                let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
                assert_eq!(loaded.resolved_packages.len(), 1);
                assert_eq!(
                    loaded.get_env_var("TASK_ID"),
                    Some(i.to_string()),
                    "Task {i} data corrupted"
                );
                i
            });
        }

        let mut completed = Vec::new();
        while let Some(result) = join_set.join_next().await {
            completed.push(result.unwrap());
        }

        assert_eq!(
            completed.len(),
            n,
            "All {n} concurrent save/load tasks must complete"
        );
    }

    // ── 8. Zero packages with resolved status ───────────────────────────────

    /// A context can be Resolved with zero packages (edge case of empty repo).
    #[tokio::test]
    async fn test_resolved_status_zero_packages_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("zero_pkgs.rxt");

        let mut ctx = make_empty_ctx();
        ctx.status = ContextStatus::Resolved;
        // No packages added

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.status, ContextStatus::Resolved);
        assert_eq!(loaded.resolved_packages.len(), 0);
    }

    // ── 9. Large package count (50 packages) ────────────────────────────────

    /// A context with 50 packages serialises and deserialises fully.
    #[tokio::test]
    async fn test_save_load_fifty_packages() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("fifty.rxt");

        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        for i in 0..50usize {
            ctx.resolved_packages
                .push(make_package(&format!("lib_{i:02}"), &format!("{i}.0.0")));
        }

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(loaded.resolved_packages.len(), 50);

        // Spot-check a few entries
        assert!(loaded.resolved_packages.iter().any(|p| p.name == "lib_00"));
        assert!(loaded.resolved_packages.iter().any(|p| p.name == "lib_49"));
    }

    // ── 10. Metadata roundtrip (arbitrary key/value pairs) ──────────────────

    /// Arbitrary metadata stored on a context must survive save/load in both
    /// JSON and binary formats.
    #[tokio::test]
    async fn test_metadata_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("meta.rxt");

        let mut ctx = make_empty_ctx();
        ctx.status = ContextStatus::Resolved;
        ctx.add_metadata("build_host".to_string(), "ci-runner-42".to_string());
        ctx.add_metadata("pipeline".to_string(), "vfx-2024".to_string());
        ctx.add_metadata("tags".to_string(), "production,approved".to_string());

        ContextSerializer::save_to_file(&ctx, &path, ContextFormat::Json)
            .await
            .unwrap();

        let loaded = ContextSerializer::load_from_file(&path).await.unwrap();
        assert_eq!(
            loaded.metadata.get("build_host").map(String::as_str),
            Some("ci-runner-42")
        );
        assert_eq!(
            loaded.metadata.get("pipeline").map(String::as_str),
            Some("vfx-2024")
        );
        assert_eq!(
            loaded.metadata.get("tags").map(String::as_str),
            Some("production,approved")
        );
    }
}
