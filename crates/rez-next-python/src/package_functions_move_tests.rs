//! Unit tests for `copy_dir_recursive` helpers and `move_package` — split from
//! `package_functions_extra_tests.rs` in Cycle 180 to keep each file ≤500 lines.
use crate::package_functions::{copy_dir_recursive, copy_package, expand_home, move_package, remove_package};
use std::fs;

mod test_package_helpers_move {
    use super::{copy_dir_recursive, copy_package, expand_home, fs, remove_package};

    #[test]
    fn test_expand_home_tilde_slash_prefix_only() {
        let middle = "path/with~tilde/in/middle";
        assert_eq!(expand_home(middle), middle, "embedded tilde must not be expanded");
    }

    #[test]
    fn test_copy_dir_recursive_two_deep_subdir() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_cp_two_deep_src");
        let dest = tmp.join("rez_test_cp_two_deep_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let sub = src.join("level1").join("level2");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("deep.txt"), b"depth2").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();
        assert!(
            dest.join("level1").join("level2").join("deep.txt").exists(),
            "two-level nested file must be copied"
        );
        assert_eq!(
            fs::read(dest.join("level1").join("level2").join("deep.txt")).unwrap(),
            b"depth2"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_remove_package_empty_paths_list_returns_zero() {
        let result = remove_package("any_pkg", None, Some(vec![]));
        assert!(result.is_ok(), "empty paths list must not error");
        assert_eq!(result.unwrap(), 0, "no repos → 0 removals");
    }

    #[test]
    fn test_expand_home_returns_string_type() {
        for p in &["/abs", "rel", "~/home", "~", "", r"C:\win"] {
            let _ = expand_home(p);
        }
    }

    #[test]
    fn test_copy_dir_recursive_source_with_mixed_content() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_cp_mixed_src");
        let dest = tmp.join("rez_test_cp_mixed_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(src.join("subdir")).unwrap();
        fs::write(src.join("root.txt"), b"root-file").unwrap();
        fs::write(src.join("subdir").join("sub.txt"), b"sub-file").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.join("root.txt").exists(), "root file must be copied");
        assert!(
            dest.join("subdir").join("sub.txt").exists(),
            "sub file must be copied"
        );
        assert_eq!(fs::read(dest.join("root.txt")).unwrap(), b"root-file");
        assert_eq!(
            fs::read(dest.join("subdir").join("sub.txt")).unwrap(),
            b"sub-file"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    // ── Cycle 122 additions ──────────────────────────────────────────────

    #[test]
    fn test_expand_home_relative_path_unchanged() {
        let p = "relative/to/cwd";
        assert_eq!(expand_home(p), p, "relative path without tilde must not be modified");
    }

    #[test]
    fn test_copy_dir_recursive_single_file() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_cp_single_file_src");
        let dest = tmp.join("rez_test_cp_single_file_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("only_file.txt"), b"single").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.join("only_file.txt").exists(), "single file must be copied");
        assert_eq!(fs::read(dest.join("only_file.txt")).unwrap(), b"single");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_remove_package_with_specific_version_leaves_other_versions() {
        let tmp = std::env::temp_dir().join("rez_test_rm_specific_ver");
        let _ = fs::remove_dir_all(&tmp);

        fs::create_dir_all(tmp.join("mypkg").join("1.0.0")).unwrap();
        fs::create_dir_all(tmp.join("mypkg").join("2.0.0")).unwrap();

        let result = remove_package(
            "mypkg",
            Some("1.0.0"),
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "should remove exactly 1 version");
        assert!(!tmp.join("mypkg").join("1.0.0").exists(), "1.0.0 must be removed");
        assert!(tmp.join("mypkg").join("2.0.0").exists(), "2.0.0 must remain");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_expand_home_with_long_path_preserved() {
        let p = "/very/long/absolute/path/to/some/package/directory/1.0.0";
        assert_eq!(expand_home(p), p, "long absolute path must not be modified");
    }

    #[test]
    fn test_copy_dir_recursive_creates_dest_if_not_exists() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_cp_create_dest_src");
        let dest = tmp.join("rez_test_cp_create_dest_nonexistent");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("test.txt"), b"content").unwrap();

        assert!(!dest.exists(), "dest must not exist before copy");
        copy_dir_recursive(&src, &dest).unwrap();
        assert!(dest.exists(), "dest must be created by copy_dir_recursive");
        assert!(dest.join("test.txt").exists(), "file must be in created dest");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    // ─────── Cycle 130 additions ──────────────────────────────────────────

    #[test]
    fn test_expand_home_tilde_slash_with_deep_path() {
        let result = expand_home("~/a/b/c");
        assert!(!result.starts_with('~'), "result must not start with ~ after expansion: {}", result);
        assert!(
            result.ends_with("a/b/c") || result.ends_with("a\\b\\c"),
            "deep path suffix must be preserved: {}",
            result
        );
    }

    #[test]
    fn test_expand_home_with_env_var_path() {
        for input in &["~/pkg/v1", "~/", "~/a", "/absolute/path", "relative/path"] {
            let result = expand_home(input);
            assert!(!result.is_empty(), "expand_home result must not be empty for input '{}'", input);
        }
    }

    #[test]
    fn test_remove_package_nonexistent_path_returns_zero_cy130() {
        let result = remove_package(
            "ghost_pkg",
            None,
            Some(vec!["/nonexistent_rez_cy130_rm_path".to_string()]),
        );
        assert!(result.is_ok(), "nonexistent path must not error");
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_copy_dir_recursive_preserves_multiple_files() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_cy130_cp_multi_src");
        let dest = tmp.join("rez_cy130_cp_multi_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(&src).unwrap();
        for name in &["a.txt", "b.txt", "c.txt"] {
            fs::write(src.join(name), name.as_bytes()).unwrap();
        }
        copy_dir_recursive(&src, &dest).unwrap();
        for name in &["a.txt", "b.txt", "c.txt"] {
            assert!(dest.join(name).exists(), "{} must be copied", name);
            let content = fs::read_to_string(dest.join(name)).unwrap();
            assert_eq!(content, *name, "file content must be preserved for {}", name);
        }
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_expand_home_no_env_no_crash() {
        let result = expand_home("/absolute/no/tilde");
        assert_eq!(result, "/absolute/no/tilde", "absolute path must be returned unchanged");
    }

    // ─────── Cycle 132 additions ──────────────────────────────────────────

    #[test]
    fn test_copy_dir_recursive_binary_file_content_preserved() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_cy132_cp_binary_src");
        let dest = tmp.join("rez_cy132_cp_binary_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        let binary_data: Vec<u8> = (0u8..=255).collect();
        fs::write(src.join("data.bin"), &binary_data).unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        let copied = fs::read(dest.join("data.bin")).unwrap();
        assert_eq!(copied, binary_data, "binary file content must be preserved byte-for-byte");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_remove_package_family_dir_does_not_affect_other_families() {
        let tmp = std::env::temp_dir().join("rez_cy132_rm_family_isolate");
        let _ = fs::remove_dir_all(&tmp);

        fs::create_dir_all(tmp.join("pkgA").join("1.0.0")).unwrap();
        fs::create_dir_all(tmp.join("pkgB").join("2.0.0")).unwrap();

        let result = remove_package("pkgA", None, Some(vec![tmp.to_string_lossy().to_string()]));
        assert!(result.is_ok());
        assert!(!tmp.join("pkgA").exists(), "pkgA family must be removed");
        assert!(tmp.join("pkgB").exists(), "pkgB family must NOT be affected");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_expand_home_tilde_prefix_expands_to_nonempty() {
        let result = expand_home("~/packages");
        assert!(!result.is_empty(), "expanded tilde path must be non-empty");
        assert!(
            !result.starts_with('~'),
            "expanded result must not start with tilde: {}",
            result
        );
    }

    // ─────── Cycle 180: copy_package multi-version selection edge-case ───────

    /// When multiple versions exist and no version is specified, `copy_package`
    /// must copy the *latest* version (not the first/random one).
    #[test]
    fn test_copy_package_no_version_selects_latest() {
        let src = std::env::temp_dir().join("rez_cy180_cp_latest_src");
        let dest = std::env::temp_dir().join("rez_cy180_cp_latest_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        for v in &["1.0.0", "2.0.0", "3.0.0"] {
            let dir = src.join("multipkg").join(v);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("package.py"), format!("name = 'multipkg'\nversion = '{}'\n", v).as_bytes()).unwrap();
        }

        let result = copy_package(
            "multipkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy with no version must succeed: {:?}", result);
        // Must copy the latest (3.0.0), not an older version.
        assert!(
            dest.join("multipkg").join("3.0.0").join("package.py").exists(),
            "latest version 3.0.0 must be at destination; dest returned: {:?}",
            result
        );
        // Older versions must NOT be in dest.
        assert!(
            !dest.join("multipkg").join("1.0.0").exists(),
            "older version 1.0.0 must NOT be copied"
        );
        assert!(
            !dest.join("multipkg").join("2.0.0").exists(),
            "older version 2.0.0 must NOT be copied"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }
}

// ─────── Cycle 155: move_package tests ──────────────────────────────────────

mod test_move_package {
    use super::{fs, move_package};

    fn make_pkg(base: &std::path::Path, name: &str, version: &str) {
        let dir = base.join(name).join(version);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n", name, version).as_bytes(),
        )
        .unwrap();
    }

    /// `move_package` with explicit version: source directory must be deleted, dest must exist.
    #[test]
    fn test_move_package_explicit_version_removes_source() {
        let src = std::env::temp_dir().join("rez_cy155_mv_src_explicit");
        let dest = std::env::temp_dir().join("rez_cy155_mv_dest_explicit");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
        make_pkg(&src, "mypkg", "1.0.0");

        let result = move_package(
            "mypkg",
            dest.to_str().unwrap(),
            Some("1.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            false,
            false,
        );
        assert!(result.is_ok(), "move must succeed: {:?}", result);
        assert!(
            dest.join("mypkg").join("1.0.0").join("package.py").exists(),
            "package.py must be at destination"
        );
        assert!(
            !src.join("mypkg").join("1.0.0").exists(),
            "source version directory must be removed after move"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// `move_package` with version=None must copy the *latest* version and delete the
    /// correct source directory — not a directory named "unknown".
    #[test]
    fn test_move_package_no_version_selects_latest_and_removes_correct_source() {
        let src = std::env::temp_dir().join("rez_cy155_mv_src_auto_ver");
        let dest = std::env::temp_dir().join("rez_cy155_mv_dest_auto_ver");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        // Two versions present; 2.0.0 is the latest and should be selected.
        make_pkg(&src, "apkg", "1.0.0");
        make_pkg(&src, "apkg", "2.0.0");

        let result = move_package(
            "apkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
            false,
        );
        assert!(result.is_ok(), "move must succeed: {:?}", result);

        // Destination must have the latest version.
        assert!(
            dest.join("apkg").join("2.0.0").join("package.py").exists(),
            "latest version 2.0.0 must be at destination"
        );

        // The source 2.0.0 directory must be gone — this was the bug: previously "unknown" was
        // used as the version string so the source was left behind.
        assert!(
            !src.join("apkg").join("2.0.0").exists(),
            "source 2.0.0 directory must be deleted (not left behind due to 'unknown' bug)"
        );

        // The older version 1.0.0 must NOT be touched.
        assert!(
            src.join("apkg").join("1.0.0").exists(),
            "source 1.0.0 (older, not moved) must remain untouched"
        );

        // There must be no directory named "unknown" anywhere.
        assert!(
            !src.join("apkg").join("unknown").exists(),
            "no 'unknown' directory must exist in source"
        );
        assert!(
            !dest.join("apkg").join("unknown").exists(),
            "no 'unknown' directory must exist in dest"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// `keep_source=true` must copy but NOT delete the source.
    #[test]
    fn test_move_package_keep_source_does_not_remove_source() {
        let src = std::env::temp_dir().join("rez_cy155_mv_src_keep");
        let dest = std::env::temp_dir().join("rez_cy155_mv_dest_keep");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
        make_pkg(&src, "kpkg", "3.0.0");

        let result = move_package(
            "kpkg",
            dest.to_str().unwrap(),
            Some("3.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            false,
            true, // keep_source
        );
        assert!(result.is_ok(), "move with keep_source must succeed: {:?}", result);
        assert!(
            dest.join("kpkg").join("3.0.0").join("package.py").exists(),
            "package.py must be at destination"
        );
        assert!(
            src.join("kpkg").join("3.0.0").exists(),
            "source must be kept when keep_source=true"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// Moving a non-existent package must return an error.
    #[test]
    fn test_move_package_nonexistent_returns_error() {
        let src = std::env::temp_dir().join("rez_cy155_mv_src_missing");
        let dest = std::env::temp_dir().join("rez_cy155_mv_dest_missing");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(&src).unwrap();

        let result = move_package(
            "ghost_pkg_xyz",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
            false,
        );
        assert!(result.is_err(), "moving nonexistent package must return Err");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    // ─────── Cycle 180: move_package three-version scenario ─────────────────

    /// `move_package` with version=None, three versions present:
    /// must copy & delete the *latest* (3.0.0), leave 1.0.0 and 2.0.0 untouched.
    #[test]
    fn test_move_package_no_version_three_versions_picks_latest() {
        let src = std::env::temp_dir().join("rez_cy180_mv_src_three");
        let dest = std::env::temp_dir().join("rez_cy180_mv_dest_three");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        make_pkg(&src, "tpkg", "1.0.0");
        make_pkg(&src, "tpkg", "2.0.0");
        make_pkg(&src, "tpkg", "3.0.0");

        let result = move_package(
            "tpkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
            false,
        );
        assert!(result.is_ok(), "move with 3 versions must succeed: {:?}", result);
        assert!(
            dest.join("tpkg").join("3.0.0").join("package.py").exists(),
            "3.0.0 must be at destination"
        );
        assert!(
            !src.join("tpkg").join("3.0.0").exists(),
            "source 3.0.0 must be deleted"
        );
        assert!(
            src.join("tpkg").join("1.0.0").exists(),
            "source 1.0.0 must remain untouched"
        );
        assert!(
            src.join("tpkg").join("2.0.0").exists(),
            "source 2.0.0 must remain untouched"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }
}
