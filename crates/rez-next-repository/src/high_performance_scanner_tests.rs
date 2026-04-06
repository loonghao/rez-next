//! Unit tests for high_performance_scanner module.
//!
//! Extracted from `high_performance_scanner.rs` to keep the implementation
//! file under the 1000-line limit.

use super::*;
use std::path::Path;
use tempfile::TempDir;

// ── REZ_PACKAGE_FILENAMES constant tests ────────────────────────────────────

mod test_rez_package_filenames {
    use crate::scanner_types::REZ_PACKAGE_FILENAMES;

    #[test]
    fn test_contains_package_py() {
        assert!(REZ_PACKAGE_FILENAMES.contains(&"package.py"));
    }

    #[test]
    fn test_contains_package_yaml() {
        assert!(REZ_PACKAGE_FILENAMES.contains(&"package.yaml"));
    }

    #[test]
    fn test_contains_package_yml() {
        assert!(REZ_PACKAGE_FILENAMES.contains(&"package.yml"));
    }

    #[test]
    fn test_contains_package_json() {
        assert!(REZ_PACKAGE_FILENAMES.contains(&"package.json"));
    }

    #[test]
    fn test_does_not_contain_build_py() {
        assert!(!REZ_PACKAGE_FILENAMES.contains(&"build.py"));
    }

    #[test]
    fn test_does_not_contain_setup_yaml() {
        assert!(!REZ_PACKAGE_FILENAMES.contains(&"setup.yaml"));
    }

    #[test]
    fn test_exactly_four_entries() {
        assert_eq!(REZ_PACKAGE_FILENAMES.len(), 4);
    }

    /// Validates that the constant stays in sync with ScannerConfig::default().
    #[test]
    fn test_matches_scanner_config_default_include_patterns() {
        use crate::scanner_types::ScannerConfig;
        let config = ScannerConfig::default();
        for name in REZ_PACKAGE_FILENAMES {
            assert!(
                config.include_patterns.iter().any(|p| p == name),
                "REZ_PACKAGE_FILENAMES entry '{}' missing from ScannerConfig::default().include_patterns",
                name
            );
        }
        assert_eq!(
            config.include_patterns.len(),
            REZ_PACKAGE_FILENAMES.len(),
            "ScannerConfig::default().include_patterns has different count from REZ_PACKAGE_FILENAMES"
        );
    }
}

// ── SIMDPatternMatcher tests ─────────────────────────────────────────────────

mod test_simd_pattern_matcher {
    use super::*;

    // --- exact rez package filenames must match ---

    #[test]
    fn test_matches_package_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new("/some/dir/package.py")));
    }

    #[test]
    fn test_matches_package_yaml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new("/some/dir/package.yaml")));
    }

    #[test]
    fn test_matches_package_yml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new("/some/dir/package.yml")));
    }

    #[test]
    fn test_matches_package_json() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new("/some/dir/package.json")));
    }

    // --- non-rez .py files must NOT match (regression guard) ---

    #[test]
    fn test_does_not_match_build_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(
            !matcher.matches_package_pattern(Path::new("/some/dir/build.py")),
            "build.py should NOT be treated as a rez package file"
        );
    }

    #[test]
    fn test_does_not_match_setup_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(
            !matcher.matches_package_pattern(Path::new("/some/dir/setup.py")),
            "setup.py should NOT be treated as a rez package file"
        );
    }

    #[test]
    fn test_does_not_match_rezbuild_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(
            !matcher.matches_package_pattern(Path::new("/some/dir/rezbuild.py")),
            "rezbuild.py is a build script, not a package definition"
        );
    }

    // --- non-rez .yaml / .json files must NOT match ---

    #[test]
    fn test_does_not_match_setup_yaml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(
            !matcher.matches_package_pattern(Path::new("/some/dir/setup.yaml")),
            "setup.yaml should NOT be treated as a rez package file"
        );
    }

    #[test]
    fn test_does_not_match_requirements_txt() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.matches_package_pattern(Path::new("/some/dir/requirements.txt")));
    }

    #[test]
    fn test_does_not_match_rs_extension() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.matches_package_pattern(Path::new("/some/dir/lib.rs")));
    }

    #[test]
    fn test_does_not_match_empty_path() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.matches_package_pattern(Path::new("")));
    }

    // --- filename-only paths (no parent dir) ---

    #[test]
    fn test_matches_bare_package_yaml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new("package.yaml")));
    }

    #[test]
    fn test_does_not_match_bare_non_package_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.matches_package_pattern(Path::new("build.py")));
    }

    // --- Windows-style paths ---

    #[test]
    fn test_matches_windows_path_package_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.matches_package_pattern(Path::new(r"C:\packages\maya\2024.1\package.py")));
    }

    #[test]
    fn test_does_not_match_windows_path_setup_py() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.matches_package_pattern(Path::new(r"C:\packages\maya\2024.1\setup.py")));
    }

    #[test]
    fn test_is_json_simd_detects_object() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.is_json_simd(r#"{"name": "mypkg"}"#));
    }

    #[test]
    fn test_is_json_simd_rejects_yaml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.is_json_simd("name: mypkg\nversion: 1.0"));
    }

    #[test]
    fn test_is_json_simd_rejects_empty() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.is_json_simd(""));
    }

    #[test]
    fn test_is_yaml_simd_detects_yaml() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.is_yaml_simd("name: mypkg\nversion: 1.0"));
    }

    #[test]
    fn test_is_yaml_simd_rejects_json() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.is_yaml_simd(r#"{"name": "mypkg"}"#));
    }

    #[test]
    fn test_is_python_simd_detects_python() {
        let matcher = SIMDPatternMatcher::new();
        assert!(matcher.is_python_simd("name = 'mypkg'\nversion = '1.0'"));
    }

    #[test]
    fn test_is_python_simd_rejects_plain_text() {
        let matcher = SIMDPatternMatcher::new();
        assert!(!matcher.is_python_simd("hello world"));
    }

    #[test]
    fn test_default_is_same_as_new() {
        let a = SIMDPatternMatcher::new();
        let b = SIMDPatternMatcher::default();
        assert!(a.matches_package_pattern(Path::new("package.yaml")));
        assert!(b.matches_package_pattern(Path::new("package.yaml")));
        assert!(!a.matches_package_pattern(Path::new("build.py")));
        assert!(!b.matches_package_pattern(Path::new("build.py")));
    }
}

// ── PrefetchPredictor smoke tests ────────────────────────────────────────────
//
// NOTE: PrefetchPredictor is a placeholder implementation.
// All methods return constant / empty values (0.5 / []).
// These tests only verify that the API compiles, does not panic, and returns
// values in the expected range.  When real prediction semantics are
// introduced, replace these smoke tests with contract tests that check
// actual prediction behavior against known inputs.

mod test_prefetch_predictor_smoke {
    use super::*;

    #[test]
    fn predict_directory_priority_smoke_returns_value_in_range() {
        // Placeholder: always returns 0.5 — just verify range contract holds.
        let predictor = PrefetchPredictor::new();
        let priority = predictor.predict_directory_priority(Path::new("/opt/packages/maya"));
        assert!(
            (0.0..=1.0).contains(&priority),
            "placeholder priority {priority} must be in [0.0, 1.0]"
        );
    }

    #[test]
    fn predict_file_access_smoke_empty_input_returns_empty() {
        // Placeholder: always returns [] for empty input.
        let predictor = PrefetchPredictor::new();
        let result = predictor.predict_file_access(&[]);
        assert!(
            result.is_empty(),
            "empty input must yield empty predictions"
        );
    }

    #[test]
    fn predict_file_access_smoke_non_empty_input_returns_subset_in_range() {
        // Placeholder: always returns [] — subset of input, scores in [0,1].
        let predictor = PrefetchPredictor::new();
        let files = vec![
            PathBuf::from("/opt/packages/maya/2024/package.yaml"),
            PathBuf::from("/opt/packages/houdini/20/package.yaml"),
        ];
        let result = predictor.predict_file_access(&files);
        assert!(
            result.len() <= files.len(),
            "predictions must not exceed input count"
        );
        for (path, score) in &result {
            assert!(
                files.contains(path),
                "predicted path {path:?} must be one of the input files"
            );
            assert!(
                (0.0..=1.0).contains(score),
                "prediction score {score} must be in [0.0, 1.0]"
            );
        }
    }

    #[test]
    fn calculate_cache_score_smoke_returns_value_in_range() {
        // Placeholder: always returns 0.5 — verify range contract holds.
        let predictor = PrefetchPredictor::new();
        let score = predictor.calculate_cache_score(Path::new("/opt/packages/maya/package.yaml"));
        assert!(
            (0.0..=1.0).contains(&score),
            "placeholder cache score {score} must be in [0.0, 1.0]"
        );
    }

    #[test]
    fn default_and_new_are_equivalent_smoke() {
        // Verify that Default and new() produce identical placeholder behavior.
        let a = PrefetchPredictor::new();
        let b = PrefetchPredictor::default();
        let dir = Path::new("/tmp");
        let file = Path::new("/tmp/package.yaml");

        let a_priority = a.predict_directory_priority(dir);
        let b_priority = b.predict_directory_priority(dir);
        assert_eq!(
            a_priority, b_priority,
            "new() and default() must agree on directory priority"
        );
        assert!((0.0..=1.0).contains(&a_priority));

        let a_score = a.calculate_cache_score(file);
        let b_score = b.calculate_cache_score(file);
        assert_eq!(
            a_score, b_score,
            "new() and default() must agree on cache score"
        );
        assert!((0.0..=1.0).contains(&a_score));
    }
}

// ── HighPerformanceConfig tests ──────────────────────────────────────────────

mod test_high_performance_config {
    use super::*;

    #[test]
    fn test_default_config_max_concurrency_is_positive() {
        let config = HighPerformanceConfig::default();
        assert!(config.max_concurrency > 0);
    }

    #[test]
    fn test_default_config_cache_size_is_positive() {
        let config = HighPerformanceConfig::default();
        assert!(config.cache_size > 0);
    }

    #[test]
    fn test_default_config_mmap_threshold_is_64kb() {
        let config = HighPerformanceConfig::default();
        assert_eq!(config.mmap_threshold, 64 * 1024);
    }

    #[test]
    fn test_default_config_batch_size_is_100() {
        let config = HighPerformanceConfig::default();
        assert_eq!(config.batch_size, 100);
    }

    #[test]
    fn test_default_config_simd_enabled() {
        let config = HighPerformanceConfig::default();
        assert!(config.enable_simd);
    }

    #[test]
    fn test_default_config_prefetch_enabled() {
        let config = HighPerformanceConfig::default();
        assert!(config.enable_prefetch);
    }

    #[test]
    fn test_default_config_work_stealing_enabled() {
        let config = HighPerformanceConfig::default();
        assert!(config.enable_work_stealing);
    }

    #[test]
    fn test_clone_config() {
        let config = HighPerformanceConfig::default();
        let cloned = config.clone();
        assert_eq!(config.cache_size, cloned.cache_size);
        assert_eq!(config.mmap_threshold, cloned.mmap_threshold);
    }

    #[test]
    fn test_custom_config_values() {
        let config = HighPerformanceConfig {
            max_concurrency: 4,
            enable_simd: false,
            enable_prefetch: false,
            cache_size: 500,
            mmap_threshold: 1024,
            batch_size: 50,
            enable_work_stealing: false,
        };
        assert_eq!(config.max_concurrency, 4);
        assert!(!config.enable_simd);
        assert_eq!(config.cache_size, 500);
    }

    #[test]
    fn test_debug_format_contains_field_names() {
        let config = HighPerformanceConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("max_concurrency"));
        assert!(debug_str.contains("cache_size"));
    }
}

// ── HighPerformanceScanner construction tests ────────────────────────────────

mod test_scanner_construction {
    use super::*;

    #[test]
    fn test_new_scanner_has_zero_stats() {
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let stats = scanner.get_performance_stats();
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.simd_operations, 0);
        assert_eq!(stats.mmap_operations, 0);
        assert_eq!(stats.prefetch_hits, 0);
        assert_eq!(stats.total_scan_time, 0);
        assert_eq!(stats.cache_size, 0);
    }

    #[test]
    fn test_new_scanner_with_small_cache() {
        let config = HighPerformanceConfig {
            cache_size: 1,
            ..HighPerformanceConfig::default()
        };
        let scanner = HighPerformanceScanner::new(config);
        let stats = scanner.get_performance_stats();
        assert_eq!(stats.cache_size, 0); // empty at start
    }

    #[test]
    fn test_performance_stats_clone() {
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let stats = scanner.get_performance_stats();
        let cloned = stats.clone();
        assert_eq!(stats.cache_hits, cloned.cache_hits);
        assert_eq!(stats.total_scan_time, cloned.total_scan_time);
    }

    #[test]
    fn test_performance_stats_debug_format() {
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let stats = scanner.get_performance_stats();
        let debug_str = format!("{:?}", stats);
        assert!(debug_str.contains("cache_hits"));
        assert!(debug_str.contains("cache_misses"));
        assert!(debug_str.contains("simd_operations"));
    }
}

// ── scan_repository_optimized async tests ───────────────────────────────────

mod test_scan_optimized_async {
    use super::*;

    #[tokio::test]
    async fn test_scan_nonexistent_path_returns_ok_with_no_packages() {
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner
            .scan_repository_optimized(Path::new("/nonexistent/path/xyz"))
            .await;
        assert!(result.is_ok());
        let scan = result.unwrap();
        assert_eq!(scan.packages.len(), 0);
    }

    #[tokio::test]
    async fn test_scan_empty_dir_returns_zero_packages() {
        let tmp = TempDir::new().unwrap();
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert_eq!(result.packages.len(), 0);
    }

    #[tokio::test]
    async fn test_scan_updates_dirs_scanned_counter() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("subdir")).unwrap();

        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert!(result.directories_scanned >= 1);
    }

    #[tokio::test]
    async fn test_scan_performance_metrics_are_populated() {
        let tmp = TempDir::new().unwrap();
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert!(result.performance_metrics.peak_concurrency > 0);
    }

    #[tokio::test]
    async fn test_scan_valid_yaml_package_file_is_parsed() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("maya").join("2024.1");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let yaml = "name: maya\nversion: '2024.1'\ndescription: Maya DCC\n";
        std::fs::write(pkg_dir.join("package.yaml"), yaml).unwrap();

        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert_eq!(result.packages.len(), 1);
        assert_eq!(result.packages[0].package.name, "maya");
    }

    #[tokio::test]
    async fn test_scan_multiple_packages_are_all_found() {
        let tmp = TempDir::new().unwrap();
        let pkgs = [
            ("maya", "2024.1"),
            ("houdini", "20.0"),
            ("python", "3.11.0"),
        ];
        for (name, ver) in &pkgs {
            let pkg_dir = tmp.path().join(name).join(ver);
            std::fs::create_dir_all(&pkg_dir).unwrap();
            let yaml = format!("name: {}\nversion: '{}'\n", name, ver);
            std::fs::write(pkg_dir.join("package.yaml"), &yaml).unwrap();
        }

        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert_eq!(result.packages.len(), 3);
    }

    #[tokio::test]
    async fn test_scan_with_prefetch_disabled() {
        let tmp = TempDir::new().unwrap();
        let config = HighPerformanceConfig {
            enable_prefetch: false,
            ..HighPerformanceConfig::default()
        };
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        assert_eq!(result.packages.len(), 0);
    }

    #[tokio::test]
    async fn test_total_scan_time_updated_after_scan() {
        let tmp = TempDir::new().unwrap();
        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        let stats = scanner.get_performance_stats();
        assert_eq!(stats.total_scan_time, result.total_duration_ms);
    }

    #[tokio::test]
    async fn test_scan_ignores_non_package_files() {
        let tmp = TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("mypkg").join("1.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        // Write a valid package file
        let yaml = "name: mypkg\nversion: '1.0'\n";
        std::fs::write(pkg_dir.join("package.yaml"), yaml).unwrap();
        // Write non-package files that should be ignored
        std::fs::write(pkg_dir.join("build.py"), "# build script").unwrap();
        std::fs::write(pkg_dir.join("setup.yaml"), "key: value").unwrap();
        std::fs::write(pkg_dir.join("README.md"), "# readme").unwrap();

        let config = HighPerformanceConfig::default();
        let scanner = HighPerformanceScanner::new(config);
        let result = scanner.scan_repository_optimized(tmp.path()).await.unwrap();
        // Only package.yaml should be discovered; non-package files are filtered
        assert_eq!(
            result.packages.len(),
            1,
            "only package.yaml should be scanned"
        );
    }
}
