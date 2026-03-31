//! Unit tests for rez-next-build
//!
//! Tests cover: BuildSystem detection, BuildManager lifecycle,
//! BuildOptions, BuildRequest, BuildConfig defaults, etc.

#[cfg(test)]
mod tests {
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
        BuildRequest {
            package: pkg,
            context: None,
            source_dir,
            variant: None,
            options: BuildOptions::default(),
            install_path: None,
        }
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
        let build_id = manager.start_build(request).await;
        assert!(
            build_id.is_ok(),
            "start_build should succeed: {:?}",
            build_id.err()
        );
        let id = build_id.unwrap();
        assert!(!id.is_empty(), "build id should not be empty");
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::CMake(_)));
    }

    #[test]
    fn test_detect_python_build_system_setup_py() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("setup.py"), "from setuptools import setup").unwrap();
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Cargo(_)));
    }

    #[test]
    fn test_detect_makefile_build_system() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("Makefile"), "all:\n\techo build").unwrap();
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Make(_)));
    }

    #[test]
    fn test_detect_custom_build_script() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("build.sh"), "#!/bin/bash\necho build").unwrap();
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
        assert!(system.is_ok());
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    #[test]
    fn test_detect_unknown_build_system_empty_dir() {
        let tmp = TempDir::new().unwrap();
        // Empty dir falls back to Custom("default")
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
        assert!(system.is_ok());
        // Empty directory should fall back to Custom with "default" script name
        assert!(matches!(system.unwrap(), BuildSystem::Custom(_)));
    }

    // ── BuildRequest tests ───────────────────────────────────────────────────

    #[test]
    fn test_build_request_no_install() {
        let pkg = make_package("mypkg", "1.0.0");
        let src = PathBuf::from("/tmp/mypkg");
        let req = BuildRequest {
            package: pkg.clone(),
            context: None,
            source_dir: src.clone(),
            variant: None,
            options: BuildOptions::default(),
            install_path: None,
        };
        assert_eq!(req.package.name, "mypkg");
        assert_eq!(req.source_dir, src);
        assert!(req.install_path.is_none());
    }

    #[test]
    fn test_build_request_with_install_path() {
        let pkg = make_package("mypkg", "1.0.0");
        let req = BuildRequest {
            package: pkg,
            context: None,
            source_dir: PathBuf::from("/tmp/mypkg"),
            variant: None,
            options: BuildOptions::default(),
            install_path: Some(PathBuf::from("/packages/local")),
        };
        assert!(req.install_path.is_some());
        assert_eq!(req.install_path.unwrap(), PathBuf::from("/packages/local"));
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf());
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
        let system = BuildSystem::detect(&tmp.path().to_path_buf()).unwrap();
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
        let _id = manager.start_build(req).await.unwrap();

        let stats = manager.get_stats();
        assert_eq!(
            stats.builds_started, 1,
            "builds_started should be incremented"
        );
    }
}
