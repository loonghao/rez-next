//! Tests for RepositoryScanner — split from scanner.rs to keep the main file ≤1000 lines.

use super::*;
use std::path::Path;
use std::sync::atomic::Ordering;
use tempfile::TempDir;

fn make_scanner_no_background() -> RepositoryScanner {
    let config = ScannerConfig {
        enable_background_refresh: false,
        enable_scan_cache: true,
        enable_prefix_matching: true,
        ..ScannerConfig::default()
    };
    RepositoryScanner::new(config)
}

// ---------------------------------------------------------------------------
// ScannerConfig::default()
// ---------------------------------------------------------------------------

#[test]
fn test_scanner_config_default_max_concurrent_scans() {
    let cfg = ScannerConfig::default();
    assert!(cfg.max_concurrent_scans > 0);
    assert_eq!(cfg.max_concurrent_scans, 20);
}

#[test]
fn test_scanner_config_default_includes_package_py() {
    let cfg = ScannerConfig::default();
    assert!(cfg.include_patterns.iter().any(|p| p == "package.py"));
}

#[test]
fn test_scanner_config_default_excludes_git() {
    let cfg = ScannerConfig::default();
    assert!(cfg.exclude_patterns.iter().any(|p| p.contains(".git")));
}

#[test]
fn test_scanner_config_default_timeout_is_300() {
    let cfg = ScannerConfig::default();
    assert_eq!(cfg.timeout_seconds, 300);
}

// ---------------------------------------------------------------------------
// matches_pattern
// ---------------------------------------------------------------------------

#[test]
fn test_matches_pattern_star_matches_everything() {
    let scanner = make_scanner_no_background();
    assert!(scanner.matches_pattern("anything", "*"));
}

#[test]
fn test_matches_pattern_exact_match() {
    let scanner = make_scanner_no_background();
    assert!(scanner.matches_pattern("package.py", "package.py"));
}

#[test]
fn test_matches_pattern_exact_mismatch() {
    let scanner = make_scanner_no_background();
    assert!(!scanner.matches_pattern("package.yaml", "package.py"));
}

#[test]
fn test_matches_pattern_single_star_wildcard() {
    let scanner = make_scanner_no_background();
    assert!(scanner.matches_pattern("package.yaml", "package.*"));
    assert!(scanner.matches_pattern("package.py", "package.*"));
}

#[test]
fn test_matches_pattern_double_star_wildcard() {
    let scanner = make_scanner_no_background();
    assert!(scanner.matches_pattern(".git/objects/foo", ".git/**"));
}

#[test]
fn test_matches_pattern_question_mark() {
    let scanner = make_scanner_no_background();
    assert!(scanner.matches_pattern("package.py", "package.p?"));
    assert!(scanner.matches_pattern("package.py", "package.??"));
    assert!(!scanner.matches_pattern("package.py", "package.?"));
}

#[test]
fn test_matches_pattern_no_match_empty_string() {
    let scanner = make_scanner_no_background();
    assert!(!scanner.matches_pattern("", "package.py"));
}

// ---------------------------------------------------------------------------
// is_package_file
// ---------------------------------------------------------------------------

#[test]
fn test_is_package_file_package_py() {
    let scanner = make_scanner_no_background();
    assert!(scanner.is_package_file(Path::new("/some/repo/package.py")));
}

#[test]
fn test_is_package_file_package_yaml() {
    let scanner = make_scanner_no_background();
    assert!(scanner.is_package_file(Path::new("/some/repo/package.yaml")));
}

#[test]
fn test_is_package_file_package_yml() {
    let scanner = make_scanner_no_background();
    assert!(scanner.is_package_file(Path::new("/some/repo/package.yml")));
}

#[test]
fn test_is_package_file_random_file_not_matched() {
    let scanner = make_scanner_no_background();
    assert!(!scanner.is_package_file(Path::new("/some/repo/readme.md")));
    assert!(!scanner.is_package_file(Path::new("/some/repo/main.py")));
}

#[test]
fn test_is_package_file_no_filename_returns_false() {
    let scanner = make_scanner_no_background();
    assert!(!scanner.is_package_file(Path::new("/")));
}

// ---------------------------------------------------------------------------
// should_exclude_path
// ---------------------------------------------------------------------------

#[test]
fn test_should_exclude_path_git_directory() {
    let scanner = make_scanner_no_background();
    assert!(scanner.should_exclude_path(Path::new("/repo/.git/objects")));
}

#[test]
fn test_should_exclude_path_node_modules() {
    let scanner = make_scanner_no_background();
    assert!(scanner.should_exclude_path(Path::new("/repo/node_modules/lodash")));
}

#[test]
fn test_should_exclude_path_normal_package_dir_not_excluded() {
    let scanner = make_scanner_no_background();
    assert!(!scanner.should_exclude_path(Path::new("/repo/mypackage/1.0.0")));
}

#[test]
fn test_should_exclude_path_pycache() {
    let scanner = make_scanner_no_background();
    assert!(scanner.should_exclude_path(Path::new("/repo/__pycache__/module.cpython")));
}

// ---------------------------------------------------------------------------
// cache_size and clear_cache
// ---------------------------------------------------------------------------

#[test]
fn test_cache_size_initial_is_zero() {
    let scanner = make_scanner_no_background();
    assert_eq!(scanner.cache_size(), 0);
}

#[test]
fn test_clear_cache_resets_size() {
    let scanner = make_scanner_no_background();
    use std::time::SystemTime;
    let dummy_package = rez_next_package::Package::new("test".to_string());
    let entry = ScanCacheEntry {
        result: PackageScanResult {
            package: dummy_package,
            package_file: PathBuf::from("/fake/package.py"),
            package_dir: PathBuf::from("/fake"),
            file_size: 100,
            scan_duration_ms: 1,
        },
        mtime: SystemTime::UNIX_EPOCH,
        size: 100,
        access_count: 0,
        last_accessed: SystemTime::UNIX_EPOCH,
    };
    scanner
        .scan_cache
        .insert(PathBuf::from("/fake/package.py"), entry);
    assert_eq!(scanner.cache_size(), 1);

    scanner.clear_cache();
    assert_eq!(scanner.cache_size(), 0);
}

// ---------------------------------------------------------------------------
// get_cache_statistics
// ---------------------------------------------------------------------------

#[test]
fn test_cache_statistics_initial_all_zero() {
    let scanner = make_scanner_no_background();
    let stats = scanner.get_cache_statistics();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.prefix_hits, 0);
    assert_eq!(stats.hit_rate, 0.0);
    assert_eq!(stats.prefix_hit_rate, 0.0);
    assert_eq!(stats.cache_size, 0);
    assert_eq!(stats.total_entries, 0);
}

#[test]
fn test_cache_statistics_hit_rate_calculation() {
    let scanner = make_scanner_no_background();
    scanner.cache_hits.store(3, Ordering::Relaxed);
    scanner.cache_misses.store(1, Ordering::Relaxed);

    let stats = scanner.get_cache_statistics();
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 1);
    assert_eq!(stats.total_entries, 4);
    assert!((stats.hit_rate - 0.75).abs() < 1e-9);
}

#[test]
fn test_cache_statistics_prefix_hit_rate_calculation() {
    let scanner = make_scanner_no_background();
    scanner.cache_hits.store(1, Ordering::Relaxed);
    scanner.cache_misses.store(1, Ordering::Relaxed);
    scanner.prefix_hits.store(1, Ordering::Relaxed);

    let stats = scanner.get_cache_statistics();
    assert_eq!(stats.prefix_hits, 1);
    assert!((stats.prefix_hit_rate - 0.5).abs() < 1e-9);
}

// ---------------------------------------------------------------------------
// normalize_path
// ---------------------------------------------------------------------------

#[test]
fn test_normalize_path_removes_current_dir_components() {
    let scanner = make_scanner_no_background();
    let result = scanner.normalize_path(Path::new("foo/./bar"));
    let result_str = result.to_string_lossy();
    assert!(!result_str.contains("/./") && !result_str.contains("\\.\\"));
}

#[test]
fn test_normalize_path_removes_parent_dir_components() {
    let scanner = make_scanner_no_background();
    let result = scanner.normalize_path(Path::new("foo/bar/../baz"));
    let components: Vec<_> = result.components().collect();
    assert!(!components.contains(&std::path::Component::ParentDir));
}

// ---------------------------------------------------------------------------
// ScanErrorType
// ---------------------------------------------------------------------------

#[test]
fn test_scan_error_type_equality() {
    assert_eq!(
        ScanErrorType::FileSystemError,
        ScanErrorType::FileSystemError
    );
    assert_ne!(ScanErrorType::FileSystemError, ScanErrorType::Timeout);
}

// ---------------------------------------------------------------------------
// get_by_prefix with prefix matching disabled
// ---------------------------------------------------------------------------

#[test]
fn test_get_by_prefix_disabled_returns_none() {
    let config = ScannerConfig {
        enable_background_refresh: false,
        enable_prefix_matching: false,
        ..ScannerConfig::default()
    };
    let scanner = RepositoryScanner::new(config);
    let result = scanner.get_by_prefix(Path::new("/any/path"));
    assert!(result.is_none());
}

// ---------------------------------------------------------------------------
// Async tests for scan_repository, preload_common_paths, stop_background_refresh
// ---------------------------------------------------------------------------

mod test_async {
    use super::*;

    fn make_scanner_no_bg_no_cache() -> RepositoryScanner {
        let config = ScannerConfig {
            enable_background_refresh: false,
            enable_scan_cache: false,
            enable_prefix_matching: false,
            enable_cache_preload: false,
            ..ScannerConfig::default()
        };
        RepositoryScanner::new(config)
    }

    fn make_scanner_with_preload() -> RepositoryScanner {
        let config = ScannerConfig {
            enable_background_refresh: false,
            enable_scan_cache: true,
            enable_prefix_matching: true,
            enable_cache_preload: true,
            ..ScannerConfig::default()
        };
        RepositoryScanner::new(config)
    }

    // --- scan_repository: error paths ---

    #[tokio::test]
    async fn test_scan_repository_nonexistent_path_returns_err() {
        let scanner = make_scanner_no_bg_no_cache();
        let result = scanner
            .scan_repository(Path::new("/nonexistent_path_xyz_12345"))
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("does not exist") || msg.contains("Repository"),
            "unexpected error: {}",
            msg
        );
    }

    #[tokio::test]
    async fn test_scan_repository_file_path_returns_err() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("not_a_dir.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let scanner = make_scanner_no_bg_no_cache();
        let result = scanner.scan_repository(&file_path).await;
        assert!(result.is_err());
    }

    // --- scan_repository: empty directory ---

    #[tokio::test]
    async fn test_scan_repository_empty_dir_returns_zero_packages() {
        let tmp = TempDir::new().unwrap();
        let scanner = make_scanner_no_bg_no_cache();

        let result = scanner.scan_repository(tmp.path()).await.unwrap();
        assert_eq!(result.packages.len(), 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[tokio::test]
    async fn test_scan_repository_empty_dir_metrics_are_valid() {
        let tmp = TempDir::new().unwrap();
        let scanner = make_scanner_no_bg_no_cache();

        let result = scanner.scan_repository(tmp.path()).await.unwrap();
        // directories_scanned is at least 1 (the root itself)
        assert!(result.directories_scanned >= 1);
    }

    // --- scan_repository: directory with a valid package.py ---

    #[tokio::test]
    async fn test_scan_repository_finds_package_yaml() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.yaml"),
            "name: mypkg\nversion: '1.0.0'\n",
        )
        .unwrap();

        let scanner = make_scanner_no_bg_no_cache();
        let result = scanner.scan_repository(tmp.path()).await.unwrap();

        assert_eq!(
            result.errors.len(),
            0,
            "valid YAML fixture should parse cleanly"
        );
        assert_eq!(
            result.packages.len(),
            1,
            "expected exactly one parsed package"
        );
        assert_eq!(result.packages[0].package.name, "mypkg");
        assert_eq!(
            result.packages[0]
                .package
                .version
                .as_ref()
                .map(|v| v.as_str()),
            Some("1.0.0")
        );
        assert!(result.files_examined >= 1);
    }

    #[tokio::test]
    async fn test_scan_repository_multiple_packages() {
        let tmp = TempDir::new().unwrap();
        for name in &["pkga", "pkgb", "pkgc"] {
            let pkg_dir = tmp.path().join(name).join("1.0.0");
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(
                pkg_dir.join("package.yaml"),
                format!("name: {}\nversion: '1.0.0'\n", name),
            )
            .unwrap();
        }

        let scanner = make_scanner_no_bg_no_cache();
        let result = scanner.scan_repository(tmp.path()).await.unwrap();
        let mut names: Vec<_> = result
            .packages
            .iter()
            .map(|pkg| pkg.package.name.as_str())
            .collect();
        names.sort_unstable();

        assert_eq!(
            result.errors.len(),
            0,
            "valid fixtures should not produce scan errors"
        );
        assert_eq!(
            result.packages.len(),
            3,
            "expected all package files to parse"
        );
        assert_eq!(names, vec!["pkga", "pkgb", "pkgc"]);
        assert!(result.files_examined >= 3);
    }

    // --- preload_common_paths ---

    #[tokio::test]
    async fn test_preload_common_paths_disabled_returns_zero() {
        let config = ScannerConfig {
            enable_background_refresh: false,
            enable_cache_preload: false,
            ..ScannerConfig::default()
        };
        let scanner = RepositoryScanner::new(config);
        let result = scanner
            .preload_common_paths(&[PathBuf::from("/some/path")])
            .await;
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_preload_common_paths_nonexistent_paths_skipped() {
        let scanner = make_scanner_with_preload();
        // Non-existent paths should be silently skipped and return 0
        let result = scanner
            .preload_common_paths(&[PathBuf::from("/nonexistent_xyz_987654")])
            .await;
        assert_eq!(result.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_preload_common_paths_with_package_updates_prefix_cache() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("toolpkg").join("2.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.py"),
            "name = 'toolpkg'\nversion = '2.0.0'\n",
        )
        .unwrap();

        let scanner = make_scanner_with_preload();
        scanner
            .preload_common_paths(&[tmp.path().to_path_buf()])
            .await
            .unwrap();

        assert!(scanner.prefix_cache.contains_key(tmp.path()));
    }

    // --- stop_background_refresh ---

    #[tokio::test]
    async fn test_stop_background_refresh_with_no_task_is_noop() {
        let scanner = make_scanner_no_bg_no_cache();
        // No background task started; stop should be a noop
        scanner.stop_background_refresh().await;
        // Verify the scanner is still usable
        assert_eq!(scanner.cache_size(), 0);
    }
}
