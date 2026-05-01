//! Unit tests for rez-next-build
//!
//! Tests cover: BuildSystem detection, BuildManager lifecycle,
//! BuildOptions, BuildRequest, BuildConfig defaults, etc.

#[cfg(test)]
mod build_tests {
    use crate::*;
    use rez_next_package::Package;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn make_package(name: &str, version: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(rez_next_version::Version::parse(version).unwrap());
        pkg
    }

    fn make_request(pkg: Package, source_dir: PathBuf) -> BuildRequest {
        BuildRequest::new(pkg, None, source_dir)
    }

    // ── BuildConfig tests ────────────────────────────────────────────────────

    #[test]
    fn test_build_config_defaults() {
        let config = BuildConfig::default();
        assert_eq!(config.max_concurrent_builds, 4);
        assert_eq!(config.build_timeout_seconds, 3600);
        assert!(!config.clean_before_build);
        assert!(config.keep_artifacts);
        assert_eq!(config.verbosity, BuildVerbosity::Normal);
    }

    #[test]
    fn test_build_verbosity_variants() {
        let _ = BuildVerbosity::Silent;
        let _ = BuildVerbosity::Verbose;
        let _ = BuildVerbosity::Debug;
        let _ = BuildVerbosity::Normal;
    }

    // ── BuildManager tests ───────────────────────────────────────────────────

    #[test]
    fn test_build_manager_new() {
        let manager = BuildManager::new();
        let stats = manager.get_stats();
        assert_eq!(stats.builds_started, 0);
        assert_eq!(stats.builds_successful, 0);
        assert_eq!(stats.builds_failed, 0);
    }

    #[test]
    fn test_build_manager_active_count_starts_zero() {
        let manager = BuildManager::new();
        assert_eq!(manager.get_active_builds().len(), 0);
    }

    #[tokio::test]
    async fn test_build_manager_start_build_returns_id() {
        let mut manager = BuildManager::new();
        let pkg = make_package("mypkg", "1.0.0");
        let tmp = TempDir::new().unwrap();

        // Write a minimal package.py to temp dir so detection can proceed
        let pkg_py = tmp.path().join("package.py");
        std::fs::write(&pkg_py, "name = 'mypkg'\nversion = '1.0.0'\n").unwrap();

        let request = make_request(pkg, tmp.path().to_path_buf());
        let build_ids = manager.start_build(request).await;
        assert!(
            build_ids.is_ok(),
            "start_build should succeed: {:?}",
            build_ids.err()
        );
        let ids = build_ids.unwrap();
        assert!(!ids.is_empty(), "build ids should not be empty");
        assert!(!ids[0].is_empty(), "first build id should not be empty");
    }

    #[tokio::test]
    async fn test_build_manager_wait_for_unknown_build_errors() {
        let mut manager = BuildManager::new();
        let result = manager.wait_for_build("nonexistent-build-id").await;
        assert!(result.is_err(), "waiting for unknown build id should fail");
    }

    // ── BuildOptions tests ───────────────────────────────────────────────────

    #[test]
    fn test_build_options_defaults() {
        let opts = BuildOptions::default();
        assert!(!opts.force_rebuild);
        assert!(!opts.skip_tests);
        assert!(!opts.release_mode);
        assert!(opts.build_args.is_empty());
        assert!(opts.env_vars.is_empty());
    }

    #[test]
    fn test_build_options_custom() {
        let opts = BuildOptions {
            force_rebuild: true,
            skip_tests: true,
            release_mode: true,
            build_args: vec!["-j4".to_string()],
            env_vars: {
                let mut m = HashMap::new();
                m.insert("VERBOSE".to_string(), "1".to_string());
                m
            },
        };
        assert!(opts.force_rebuild);
        assert!(opts.skip_tests);
        assert!(opts.release_mode);
        assert_eq!(opts.build_args.len(), 1);
        assert_eq!(opts.env_vars.get("VERBOSE"), Some(&"1".to_string()));
    }

    // ── BuildSystem detection tests ──────────────────────────────────────────

    #[test]
    fn test_detect_cmake_build_system() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.0)",
        )
        .unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::CMake(_)));
    }

    #[test]
    fn test_detect_python_build_system_setup_py() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("setup.py"), "from setuptools import setup").unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Python(_)));
    }

    #[test]
    fn test_detect_python_build_system_pyproject() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[build-system]\nrequires = ['setuptools']",
        )
        .unwrap();
        // No CMakeLists.txt or Makefile, so Python should be detected
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Python(_)));
    }

    #[test]
    fn test_detect_nodejs_build_system() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"mypkg","version":"1.0.0"}"#,
        )
        .unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::NodeJs(_)));
    }

    #[test]
    fn test_detect_cargo_build_system() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"mypkg\"\nversion = \"0.1.0\"",
        )
        .unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Cargo(_)));
    }

    #[test]
    fn test_detect_makefile_build_system() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n\techo build").unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Make(_)));
    }

    #[test]
    fn test_detect_custom_build_script() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.sh"), "#!/bin/bash\necho build").unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    #[test]
    fn test_detect_unknown_build_system_empty_dir() {
        let tmp = TempDir::new().unwrap();
        // Empty dir falls back to Custom("default")
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        // Empty directory should fall back to Custom with "default" script name
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    // ── BuildRequest tests ───────────────────────────────────────────────────

    #[test]
    fn test_build_request_no_install() {
        let pkg = make_package("mypkg", "1.0.0");
        let src = PathBuf::from("/tmp/mypkg");
        let req = BuildRequest::new(pkg.clone(), None, src.clone());
        assert_eq!(req.package.name, "mypkg");
        assert_eq!(req.source_dir, src);
        assert!(!req.is_variant());
        assert!(req.install_path.is_none());
    }

    #[test]
    fn test_build_request_with_install_path() {
        let pkg = make_package("mypkg", "1.0.0");
        let install_path = Some(PathBuf::from("/packages/local"));
        let req = BuildRequest {
            package: pkg,
            context: None,
            source_dir: PathBuf::from("/tmp/mypkg"),
            variant_index: None,
            variant_requires: None,
            options: BuildOptions::default(),
            install_path,
        };
        assert!(req.install_path.is_some());
        assert_eq!(req.install_path.unwrap(), PathBuf::from("/packages/local"));
    }

    #[test]
    fn test_build_request_for_variant() {
        let pkg = make_package("mypkg", "1.0.0");
        let variant_reqs = vec!["python-3.9".to_string(), "platform-linux".to_string()];
        let req = BuildRequest::for_variant(
            pkg.clone(),
            None,
            PathBuf::from("/tmp/mypkg"),
            0,
            variant_reqs.clone(),
        );
        assert!(req.is_variant());
        assert_eq!(req.variant_index, Some(0));
        assert_eq!(req.variant_requires.as_ref().unwrap().len(), 2);

        // Test variant hash computation
        let hash = req.variant_hash();
        assert!(hash.is_some());
        assert!(!hash.unwrap().is_empty());
    }

    // ── BuildStats tests ─────────────────────────────────────────────────────

    #[test]
    fn test_build_stats_initial_values() {
        let stats = BuildStats {
            builds_started: 0,
            builds_successful: 0,
            builds_failed: 0,
            builds_running: 0,
            total_build_time_ms: 0,
            avg_build_time_ms: 0.0,
        };
        assert_eq!(stats.builds_started, 0);
        assert_eq!(stats.builds_successful, 0);
        assert_eq!(stats.builds_failed, 0);
        assert_eq!(stats.total_build_time_ms, 0);
    }

    // ── Phase 79: BuildEnvironment, BuildSystem name, priority detection ─────

    #[test]
    fn test_build_environment_new() {
        let pkg = make_package("mylib", "2.0.0");
        let tmp = TempDir::new().unwrap();
        let env = BuildEnvironment::new(&pkg, &tmp.path().to_path_buf(), None);
        assert!(
            env.is_ok(),
            "BuildEnvironment::new should succeed: {:?}",
            env.err()
        );
        let env = env.unwrap();
        // Build dir should be inside tmp
        assert!(env.get_build_dir().starts_with(tmp.path()));
    }

    #[test]
    fn test_build_environment_install_dir_has_version() {
        let pkg = make_package("mylib", "2.0.0");
        let tmp = TempDir::new().unwrap();
        let install_path = tmp.path().join("packages");
        let env = BuildEnvironment::with_install_path(
            &pkg,
            &tmp.path().to_path_buf(),
            None,
            Some(&install_path),
        );
        assert!(env.is_ok());
        let env = env.unwrap();
        // Install dir should include version
        let install_str = env.get_install_dir().to_string_lossy();
        assert!(
            install_str.contains("2.0.0"),
            "install_dir should include version: {}",
            install_str
        );
    }

    #[test]
    fn test_build_environment_get_env_var() {
        let pkg = make_package("mylib", "2.0.0");
        let tmp = TempDir::new().unwrap();
        let mut env = BuildEnvironment::new(&pkg, &tmp.path().to_path_buf(), None).unwrap();
        env.add_env_var("MY_BUILD_FLAG".to_string(), "1".to_string());
        assert_eq!(
            env.get_env_vars().get("MY_BUILD_FLAG"),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn test_build_environment_rez_build_vars_set() {
        let pkg = make_package("mylib", "2.0.0");
        let tmp = TempDir::new().unwrap();
        let env = BuildEnvironment::new(&pkg, &tmp.path().to_path_buf(), None).unwrap();
        // REZ_BUILD_PACKAGE_NAME should always be set
        let vars = env.get_env_vars();
        assert_eq!(
            vars.get("REZ_BUILD_PACKAGE_NAME"),
            Some(&"mylib".to_string())
        );
        assert_eq!(
            vars.get("REZ_BUILD_PACKAGE_VERSION"),
            Some(&"2.0.0".to_string())
        );
    }

    #[test]
    fn test_detect_build_script_bat() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.bat"), "@echo off\necho build").unwrap();
        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    #[test]
    fn test_detect_priority_custom_over_cmake() {
        // build.sh should take priority over CMakeLists.txt
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.sh"), "#!/bin/bash").unwrap();
        std::fs::write(
            tmp.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.0)",
        )
        .unwrap();
        let system = BuildSystem::detect(tmp.path()).unwrap();
        assert!(
            matches!(system, BuildSystem::Custom(_)),
            "build.sh should take priority over CMakeLists.txt"
        );
    }

    #[test]
    fn test_build_system_type_variants() {
        // All variants should be constructable
        let _ = BuildSystemType::CMake;
        let _ = BuildSystemType::Make;
        let _ = BuildSystemType::Python;
        let _ = BuildSystemType::NodeJs;
        let _ = BuildSystemType::Cargo;
        let _ = BuildSystemType::Custom;
        let _ = BuildSystemType::Unknown;
    }

    #[test]
    fn test_build_options_with_env_vars() {
        let mut opts = BuildOptions::default();
        opts.env_vars
            .insert("REZ_BUILD_INSTALL".to_string(), "1".to_string());
        opts.env_vars
            .insert("REZ_BUILD_PROJECT_NAME".to_string(), "maya".to_string());
        assert_eq!(opts.env_vars.len(), 2);
        assert_eq!(
            opts.env_vars.get("REZ_BUILD_INSTALL"),
            Some(&"1".to_string())
        );
    }

    #[tokio::test]
    async fn test_build_manager_builds_started_tracked() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();
        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'testpkg'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("testpkg", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());
        let ids = manager.start_build(req).await.unwrap();

        let stats = manager.get_stats();
        assert_eq!(
            stats.builds_started, 1,
            "builds_started should be incremented"
        );
        assert_eq!(
            ids.len(),
            1,
            "should return one build id for non-variant build"
        );
    }

    #[test]
    fn test_build_system_detect_with_ambiguous_files() {
        let tmp = TempDir::new().unwrap();
        // Write multiple build files - custom script (build.sh) has priority over CMake
        std::fs::write(
            tmp.path().join("CMakeLists.txt"),
            "cmake_minimum_required(VERSION 3.0)",
        )
        .unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\necho build").unwrap();
        std::fs::write(tmp.path().join("build.sh"), "#!/bin/bash").unwrap();

        let system = BuildSystem::detect(tmp.path());
        assert!(system.is_ok());
        // build.sh has priority over CMakeLists.txt and Makefile
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    #[test]
    fn test_build_options_with_all_fields() {
        let opts = BuildOptions {
            force_rebuild: true,
            skip_tests: true,
            release_mode: true,
            build_args: vec!["-j4".to_string(), "--verbose".to_string()],
            env_vars: {
                let mut m = std::collections::HashMap::new();
                m.insert("MY_VAR".to_string(), "value".to_string());
                m
            },
        };

        assert!(opts.force_rebuild);
        assert!(opts.skip_tests);
        assert!(opts.release_mode);
        assert_eq!(opts.build_args.len(), 2);
        assert_eq!(opts.env_vars.len(), 1);
    }

    #[test]
    fn test_build_stats_default() {
        let stats = BuildStats::default();
        assert_eq!(stats.builds_started, 0);
        assert_eq!(stats.builds_successful, 0);
        assert_eq!(stats.builds_failed, 0);
        assert_eq!(stats.builds_running, 0);
        assert_eq!(stats.total_build_time_ms, 0);
        assert_eq!(stats.avg_build_time_ms, 0.0);
    }

    #[test]
    fn test_build_system_type_equality() {
        assert_eq!(BuildSystemType::CMake, BuildSystemType::CMake);
        assert_eq!(BuildSystemType::Make, BuildSystemType::Make);
        assert_ne!(BuildSystemType::CMake, BuildSystemType::Make);
    }

    #[tokio::test]
    async fn test_build_manager_multiple_builds() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();

        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'multi'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("multi", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());

        // Start multiple builds
        let ids1 = manager.start_build(req.clone()).await.unwrap();
        let ids2 = manager.start_build(req).await.unwrap();

        // Each non-variant build returns a Vec with 1 ID
        assert_eq!(ids1.len(), 1);
        assert_eq!(ids2.len(), 1);
        assert_ne!(ids1[0], ids2[0], "Build IDs should be unique");
    }

    // ── BuildManager additional API tests ─────────────────────────────────

    #[tokio::test]
    async fn test_cancel_build_success() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();

        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'cancel_test'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("cancel_test", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());

        // Start a build
        let build_ids = manager.start_build(req).await.unwrap();
        let build_id = &build_ids[0];

        // Cancel the build
        let result = manager.cancel_build(build_id).await;
        assert!(
            result.is_ok(),
            "cancel_build should succeed for active build"
        );
    }

    #[tokio::test]
    async fn test_cancel_build_nonexistent() {
        let mut manager = BuildManager::new();

        // Try to cancel a non-existent build
        let result = manager.cancel_build("nonexistent-id").await;
        assert!(
            result.is_err(),
            "cancel_build should fail for non-existent build"
        );
    }

    #[tokio::test]
    async fn test_get_build_status() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();

        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'status_test'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("status_test", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());

        // Start a build
        let build_ids = manager.start_build(req).await.unwrap();
        let build_id = &build_ids[0];

        // Get build status
        let status = manager.get_build_status(build_id);
        // Status should be Some (either Running or Completed)
        assert!(status.is_some(), "active build should have a status");
    }

    #[test]
    fn test_get_active_builds_empty() {
        let manager = BuildManager::new();
        let active = manager.get_active_builds();
        assert_eq!(active.len(), 0, "new manager should have no active builds");
    }

    #[tokio::test]
    async fn test_get_active_builds_after_start() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();

        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'active_test'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("active_test", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());

        // Start a build
        let build_ids = manager.start_build(req).await.unwrap();
        let build_id = &build_ids[0];

        // Should have one active build
        let active = manager.get_active_builds();
        assert_eq!(active.len(), 1, "should have one active build");
        assert_eq!(&active[0], build_id, "active build ID should match");
    }

    #[tokio::test]
    async fn test_clean_build_dir() {
        let manager = BuildManager::with_config({
            let mut config = BuildConfig::default();
            let temp_dir = TempDir::new().unwrap();
            config.build_dir = temp_dir.path().to_path_buf();
            config
        });

        let build_dir = &manager.get_config().build_dir;

        // Create a build directory with some content
        std::fs::create_dir_all(build_dir).unwrap();
        std::fs::write(build_dir.join("test_file.txt"), "test").unwrap();

        // Clean the build directory
        let result = manager.clean_build_dir().await;
        assert!(result.is_ok(), "clean_build_dir should succeed");

        // Directory should exist but be empty
        assert!(
            build_dir.exists(),
            "build directory should exist after clean"
        );
        let entries: Vec<_> = std::fs::read_dir(build_dir).unwrap().collect();
        assert_eq!(
            entries.len(),
            0,
            "build directory should be empty after clean"
        );
    }

    #[test]
    fn test_get_config() {
        let manager = BuildManager::new();
        let config = manager.get_config();

        // Config should have default values
        assert_eq!(config.max_concurrent_builds, 4);
        assert_eq!(config.build_timeout_seconds, 3600);
        assert!(!config.clean_before_build);
        assert!(config.keep_artifacts);
    }

    #[tokio::test]
    async fn test_wait_for_build_success() {
        let mut manager = BuildManager::new();
        let tmp = TempDir::new().unwrap();

        // Write a package.py so detection doesn't error
        std::fs::write(
            tmp.path().join("package.py"),
            "name = 'wait_test'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let pkg = make_package("wait_test", "1.0.0");
        let req = make_request(pkg, tmp.path().to_path_buf());

        // Start a build
        let build_ids = manager.start_build(req).await.unwrap();
        let build_id = &build_ids[0];

        // Wait for build to complete
        let result = manager.wait_for_build(build_id).await;
        assert!(result.is_ok(), "wait_for_build should succeed");

        let build_result = result.unwrap();
        // Build may succeed or fail depending on the test environment
        // Just ensure we got a valid result
        assert!(!build_result.build_id.is_empty());
    }
}

// ── New tests for BuildType, get_build_process_types, create_build_system ─────
#[cfg(test)]
mod build_type_tests {
    use crate::{BuildType, create_build_system, get_build_process_types};

    #[test]
    fn build_type_name() {
        assert_eq!(BuildType::Local.name(), "local");
        assert_eq!(BuildType::Central.name(), "central");
    }

    #[test]
    fn build_type_from_str() {
        assert_eq!(BuildType::from_str("local"), Some(BuildType::Local));
        assert_eq!(BuildType::from_str("central"), Some(BuildType::Central));
        assert_eq!(BuildType::from_str("invalid"), None);
    }

    #[test]
    fn build_type_clone() {
        let bt = BuildType::Local;
        let bt2 = bt.clone();
        assert_eq!(bt, bt2);
    }

    #[test]
    fn build_type_eq() {
        assert_eq!(BuildType::Local, BuildType::Local);
        assert_ne!(BuildType::Local, BuildType::Central);
    }

    #[test]
    fn get_build_process_types_returns_local_and_central() {
        let types = get_build_process_types();
        assert!(types.contains(&"local"));
        assert!(types.contains(&"central"));
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn create_build_system_valid_types() {
        assert!(create_build_system("cmake").is_some());
        assert!(create_build_system("make").is_some());
        assert!(create_build_system("python").is_some());
        assert!(create_build_system("nodejs").is_some());
        assert!(create_build_system("cargo").is_some());
        assert!(create_build_system("custom").is_some());
    }

    #[test]
    fn create_build_system_invalid_type() {
        assert!(create_build_system("invalid").is_none());
    }

    #[test]
    fn build_system_clone() {
        let bs = create_build_system("cmake").unwrap();
        let _bs2 = bs.clone(); // Should compile if Clone is derived
    }
}
