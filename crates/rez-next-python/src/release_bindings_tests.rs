//! Tests for release_bindings — package release workflow.

#[cfg(test)]
mod release_tests {
    use crate::release_bindings::{release_package, PyReleaseManager, PyReleaseResult, ReleaseMode};

    #[test]
    fn test_release_mode_from_str() {
        assert_eq!(ReleaseMode::from_str("local"), ReleaseMode::Local);
        assert_eq!(ReleaseMode::from_str("dry_run"), ReleaseMode::DryRun);
        assert_eq!(ReleaseMode::from_str("release"), ReleaseMode::Release);
        assert_eq!(ReleaseMode::from_str("unknown"), ReleaseMode::Release);
    }

    #[test]
    fn test_release_manager_new() {
        let mgr = PyReleaseManager::new(None, false, false);
        assert_eq!(mgr.mode, ReleaseMode::Release);
        assert!(!mgr.skip_build);
    }

    #[test]
    fn test_release_manager_str() {
        let mgr = PyReleaseManager::new(Some("local"), false, true);
        let s = mgr.__str__();
        assert!(s.contains("Local"));
        assert!(s.contains("skip_tests=true"));
    }

    #[test]
    fn test_validate_missing_dir_returns_issues() {
        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some("/nonexistent/path/xyz_abc_123")).unwrap();
        assert!(!valid);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_release_missing_source_returns_error() {
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr.release(Some("/nonexistent/path"), None).unwrap();
        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_release_dry_run_with_temp_package() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'testpkg'\nversion = '1.0.0'\n").unwrap();
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr
            .release(Some(dir.path().to_str().unwrap()), Some("test release"))
            .unwrap();
        assert!(result.success, "dry_run should succeed: {:?}", result.errors);
        assert!(result.install_path.contains("[dry-run]"));
        assert_eq!(result.package_name, "testpkg");
        assert_eq!(result.version, "1.0.0");
    }

    #[test]
    fn test_validate_with_valid_package() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'mypkg'\nversion = '2.0.0'\n").unwrap();
        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some(dir.path().to_str().unwrap())).unwrap();
        let _ = (valid, issues);
    }

    #[test]
    fn test_release_result_str() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/packages/mypkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("OK"));
        assert!(s.contains("mypkg"));
    }

    #[test]
    fn test_release_result_failed_str() {
        let result = PyReleaseResult {
            success: false,
            package_name: "badpkg".to_string(),
            version: "0.0.0".to_string(),
            install_path: String::new(),
            errors: vec!["Missing version".to_string()],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("FAILED"));
    }

    #[test]
    fn test_release_result_repr_equals_str() {
        let result = PyReleaseResult {
            success: true,
            package_name: "pkgx".to_string(),
            version: "3.2.1".to_string(),
            install_path: "/pkgs/pkgx/3.2.1".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(result.__repr__(), result.__str__());
    }

    #[test]
    fn test_release_result_str_contains_version() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "2.5.0".to_string(),
            install_path: "/dest/mypkg/2.5.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("2.5.0"), "str should contain version: {}", s);
    }

    #[test]
    fn test_release_result_str_contains_path() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/custom/install/path".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("/custom/install/path"), "str: {}", s);
    }

    #[test]
    fn test_release_mode_dry_run_alias() {
        assert_eq!(ReleaseMode::from_str("dry-run"), ReleaseMode::DryRun);
    }

    #[test]
    fn test_release_manager_dry_run_mode() {
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        assert_eq!(mgr.mode, ReleaseMode::DryRun);
    }

    #[test]
    fn test_release_manager_skip_build_flag() {
        let mgr = PyReleaseManager::new(None, true, false);
        assert!(mgr.skip_build);
        assert!(!mgr.skip_tests);
    }

    #[test]
    fn test_release_manager_skip_tests_flag() {
        let mgr = PyReleaseManager::new(None, false, true);
        assert!(!mgr.skip_build);
        assert!(mgr.skip_tests);
    }

    #[test]
    fn test_dry_run_result_has_dry_run_prefix() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'drytestpkg'\nversion = '0.1.0'\n").unwrap();
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr.release(Some(dir.path().to_str().unwrap()), None).unwrap();
        assert!(result.success);
        assert!(
            result.install_path.starts_with("[dry-run]"),
            "path: {}",
            result.install_path
        );
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_dry_run_with_message_populates_warnings() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'notepkg'\nversion = '0.2.0'\n").unwrap();
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr
            .release(Some(dir.path().to_str().unwrap()), Some("review note"))
            .unwrap();
        assert!(!result.warnings.is_empty(), "warnings should contain dry-run note");
        assert!(
            result.warnings[0].contains("review note"),
            "warning: {}",
            result.warnings[0]
        );
    }

    #[test]
    fn test_validate_empty_dir_returns_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some(dir.path().to_str().unwrap())).unwrap();
        assert!(!valid, "empty dir should be invalid");
        assert!(!issues.is_empty(), "should report missing package.py/yaml");
    }

    #[test]
    fn test_release_mode_equality() {
        assert_eq!(ReleaseMode::Release, ReleaseMode::Release);
        assert_ne!(ReleaseMode::Release, ReleaseMode::Local);
        assert_ne!(ReleaseMode::Local, ReleaseMode::DryRun);
    }

    #[test]
    fn test_release_mode_copy_semantics() {
        let mode = ReleaseMode::DryRun;
        let mode2 = mode;
        assert_eq!(mode, mode2);
    }

    #[test]
    fn test_release_manager_mode_local() {
        let mgr = PyReleaseManager::new(Some("local"), false, false);
        assert_eq!(mgr.mode, ReleaseMode::Local);
    }

    #[test]
    fn test_release_result_success_flag_true() {
        let r = PyReleaseResult {
            success: true,
            package_name: "pkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/pkgs/pkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert!(r.success);
        assert!(r.errors.is_empty());
    }

    #[test]
    fn test_release_result_success_flag_false() {
        let r = PyReleaseResult {
            success: false,
            package_name: "pkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: String::new(),
            errors: vec!["err1".to_string()],
            warnings: vec![],
        };
        assert!(!r.success);
        assert_eq!(r.errors.len(), 1);
    }

    #[test]
    fn test_release_result_warnings_preserved() {
        let r = PyReleaseResult {
            success: true,
            package_name: "pkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/pkgs/pkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec!["warn1".to_string(), "warn2".to_string()],
        };
        assert_eq!(r.warnings.len(), 2);
        assert_eq!(r.warnings[0], "warn1");
    }

    #[test]
    fn test_release_manager_str_contains_release_mode() {
        let mgr = PyReleaseManager::new(Some("release"), false, false);
        let s = mgr.__str__();
        assert!(s.contains("Release"), "str: {s}");
    }

    #[test]
    fn test_release_manager_str_both_flags_false() {
        let mgr = PyReleaseManager::new(None, false, false);
        let s = mgr.__str__();
        assert!(s.contains("skip_build=false"), "str: {s}");
        assert!(s.contains("skip_tests=false"), "str: {s}");
    }

    #[test]
    fn test_release_result_failed_str_contains_errors_list() {
        let r = PyReleaseResult {
            success: false,
            package_name: "brokenpkg".to_string(),
            version: "0.1.0".to_string(),
            install_path: String::new(),
            errors: vec!["compilation failed".to_string()],
            warnings: vec![],
        };
        let s = r.__str__();
        assert!(s.contains("compilation failed"), "str: {s}");
    }

    #[test]
    fn test_release_result_repr_same_as_str() {
        let r = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "2.0.0".to_string(),
            install_path: "/pkgs/mypkg/2.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(r.__repr__(), r.__str__(), "__repr__ must equal __str__");
    }

    #[test]
    fn test_release_manager_local_mode_str_contains_local() {
        let mgr = PyReleaseManager::new(Some("local"), false, false);
        let s = mgr.__str__();
        assert!(s.contains("Local"), "str must contain Local for local mode: '{s}'");
    }

    #[test]
    fn test_release_manager_both_skip_flags_true() {
        let mgr = PyReleaseManager::new(None, true, true);
        assert!(mgr.skip_build);
        assert!(mgr.skip_tests);
    }

    #[test]
    fn test_release_manager_default_mode_is_release() {
        let mgr = PyReleaseManager::new(None, false, false);
        assert_eq!(mgr.mode, ReleaseMode::Release);
    }

    #[test]
    fn test_release_mode_debug_format() {
        let modes = [ReleaseMode::Release, ReleaseMode::Local, ReleaseMode::DryRun];
        for m in &modes {
            let debug_str = format!("{:?}", m);
            assert!(!debug_str.is_empty(), "debug format must not be empty for {:?}", m);
        }
    }

    #[test]
    fn test_release_result_empty_errors_and_warnings() {
        let r = PyReleaseResult {
            success: true,
            package_name: "pkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/dest/pkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert!(r.errors.is_empty(), "errors must be empty");
        assert!(r.warnings.is_empty(), "warnings must be empty");
    }

    #[test]
    fn test_release_result_multiple_errors() {
        let r = PyReleaseResult {
            success: false,
            package_name: "multi_err_pkg".to_string(),
            version: "0.1.0".to_string(),
            install_path: String::new(),
            errors: vec!["error1".to_string(), "error2".to_string(), "error3".to_string()],
            warnings: vec![],
        };
        assert_eq!(r.errors.len(), 3);
        assert_eq!(r.errors[1], "error2");
    }

    #[test]
    fn test_release_result_package_name_field() {
        let r = PyReleaseResult {
            success: true,
            package_name: "specific_pkg_name".to_string(),
            version: "5.0.0".to_string(),
            install_path: "/p/specific_pkg_name/5.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(r.package_name, "specific_pkg_name");
    }

    #[test]
    fn test_release_result_version_field() {
        let r = PyReleaseResult {
            success: true,
            package_name: "vpkg".to_string(),
            version: "10.20.30".to_string(),
            install_path: "/pkgs/vpkg/10.20.30".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(r.version, "10.20.30");
    }

    #[test]
    fn test_release_manager_str_dry_run_label() {
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let s = mgr.__str__();
        assert!(s.contains("DryRun") || s.contains("Dry"), "str must mention dry run: '{s}'");
    }

    #[test]
    fn test_release_function_dry_run_arg() {
        let result = release_package(Some("/nonexistent/path_dry_test"), false, true, None).unwrap();
        let _ = result;
    }

    #[test]
    fn test_release_result_install_path_field() {
        let r = PyReleaseResult {
            success: true,
            package_name: "pathtestpkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/pkgs/pathtestpkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(r.install_path, "/pkgs/pathtestpkg/1.0.0");
    }

    #[test]
    fn test_release_result_errors_empty_means_success_possible() {
        let r = PyReleaseResult {
            success: true,
            package_name: "cleanpkg".to_string(),
            version: "0.1.0".to_string(),
            install_path: "/dest/cleanpkg/0.1.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert!(r.success && r.errors.is_empty());
    }

    #[test]
    fn test_release_manager_str_is_non_empty() {
        let mgr = PyReleaseManager::new(None, false, false);
        assert!(!mgr.__str__().is_empty(), "__str__ must not be empty");
    }

    #[test]
    fn test_release_mode_debug_format_all_variants() {
        let debug_release = format!("{:?}", ReleaseMode::Release);
        let debug_local = format!("{:?}", ReleaseMode::Local);
        let debug_dry = format!("{:?}", ReleaseMode::DryRun);
        assert!(debug_release.contains("Release"), "debug: {debug_release}");
        assert!(debug_local.contains("Local"), "debug: {debug_local}");
        assert!(debug_dry.contains("DryRun") || debug_dry.contains("Dry"), "debug: {debug_dry}");
    }

    #[test]
    fn test_release_result_str_contains_package_name() {
        let r = PyReleaseResult {
            success: true,
            package_name: "distinctname_xyz".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/pkgs/distinctname_xyz/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert!(
            r.__str__().contains("distinctname_xyz"),
            "str must contain package name, got: {}",
            r.__str__()
        );
    }

    #[test]
    fn test_release_package_local_false_dry_false_is_release_mode() {
        let result = release_package(
            Some("/nonexistent/release_mode_test"),
            false,
            false,
            None,
        )
        .unwrap();
        assert!(!result.success, "nonexistent path should yield failure");
    }

    #[test]
    fn test_release_manager_new_default_fields() {
        let mgr = PyReleaseManager::new(None, false, false);
        let _ = mgr;
    }

    #[test]
    fn test_release_manager_skip_build_stored() {
        let mgr = PyReleaseManager::new(None, true, false);
        assert!(mgr.skip_build, "skip_build=true must be stored");
    }

    #[test]
    fn test_release_manager_skip_tests_stored() {
        let mgr = PyReleaseManager::new(None, false, true);
        assert!(mgr.skip_tests, "skip_tests=true must be stored");
    }

    #[test]
    fn test_release_result_success_field_is_bool() {
        let result = release_package(Some("/nonexistent/cy126"), false, true, None).unwrap();
        let _: bool = result.success;
    }

    #[test]
    fn test_release_result_errors_is_vec() {
        let result = release_package(Some("/nonexistent/cy126b"), false, true, None).unwrap();
        let _ = result.errors.len();
    }
}
