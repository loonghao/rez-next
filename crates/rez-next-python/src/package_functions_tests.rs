//! Unit tests for `package_functions` — split from the main module to keep file size ≤ 1000 lines.
use crate::package_functions::{copy_dir_recursive, copy_package, expand_home, remove_package};
use std::fs;

mod test_expand_home {
    use super::expand_home;

    #[test]
    fn test_expand_home_no_tilde() {
        let p = "/absolute/path";
        assert_eq!(expand_home(p), p);
    }

    #[test]
    fn test_expand_home_relative_no_tilde() {
        let p = "relative/path";
        assert_eq!(expand_home(p), p);
    }

    #[test]
    fn test_expand_home_tilde_slash_expands() {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let expanded = expand_home("~/packages");
            assert!(
                expanded.starts_with(&home),
                "expanded '{}' should start with home '{}'",
                expanded,
                home
            );
            assert!(
                expanded.ends_with("packages") || expanded.contains("packages"),
                "expanded path should retain the suffix"
            );
        }
    }

    #[test]
    fn test_expand_home_bare_tilde_expands() {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let expanded = expand_home("~");
            assert_eq!(expanded, home);
        }
    }

    #[test]
    fn test_expand_home_tilde_in_middle_is_unchanged() {
        let p = "/some/~/path";
        assert_eq!(expand_home(p), p);
    }
}

mod test_remove_package {
    use super::{fs, remove_package};

    #[test]
    fn test_remove_package_nonexistent_returns_zero() {
        let tmp = std::env::temp_dir().join("rez_test_rm_nonexistent");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();

        let result = remove_package(
            "nonexistent_pkg_xyz",
            None,
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0, "nothing to remove → count must be 0");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_remove_package_specific_version() {
        let tmp = std::env::temp_dir().join("rez_test_rm_version");
        let _ = fs::remove_dir_all(&tmp);

        let pkg_dir = tmp.join("mypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), b"name = 'mypkg'\nversion = '1.0.0'\n").unwrap();

        let result = remove_package(
            "mypkg",
            Some("1.0.0"),
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok(), "remove must succeed: {:?}", result);
        assert_eq!(result.unwrap(), 1, "should have removed 1 version");
        assert!(!pkg_dir.exists(), "version directory must be deleted");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_remove_package_entire_family() {
        let tmp = std::env::temp_dir().join("rez_test_rm_family");
        let _ = fs::remove_dir_all(&tmp);

        let v1 = tmp.join("myfamily").join("1.0.0");
        let v2 = tmp.join("myfamily").join("2.0.0");
        fs::create_dir_all(&v1).unwrap();
        fs::create_dir_all(&v2).unwrap();

        let result = remove_package(
            "myfamily",
            None,
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1, "should have removed 1 family dir");
        assert!(!tmp.join("myfamily").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}

mod test_copy_dir_recursive {
    use super::{copy_dir_recursive, fs};

    #[test]
    fn test_copy_flat_directory() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_src_flat");
        let dest = tmp.join("rez_test_copy_dest_flat");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("file1.txt"), b"hello").unwrap();
        fs::write(src.join("file2.txt"), b"world").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.join("file1.txt").exists());
        assert!(dest.join("file2.txt").exists());
        assert_eq!(fs::read(dest.join("file1.txt")).unwrap(), b"hello");
        assert_eq!(fs::read(dest.join("file2.txt")).unwrap(), b"world");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_nested_directory() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_src_nested");
        let dest = tmp.join("rez_test_copy_dest_nested");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let sub = src.join("subdir");
        fs::create_dir_all(&sub).unwrap();
        fs::write(src.join("root.txt"), b"root").unwrap();
        fs::write(sub.join("child.txt"), b"child").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(dest.join("root.txt").exists());
        assert!(dest.join("subdir").join("child.txt").exists());
        assert_eq!(
            fs::read(dest.join("subdir").join("child.txt")).unwrap(),
            b"child"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_empty_directory() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_src_empty");
        let dest = tmp.join("rez_test_copy_dest_empty");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        copy_dir_recursive(&src, &dest).unwrap();
        assert!(dest.exists());

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_preserves_file_content() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_src_content");
        let dest = tmp.join("rez_test_copy_dest_content");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        let content = b"rez-next package.py content\nversion = '1.0.0'\n";
        fs::write(src.join("package.py"), content).unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        let copied = fs::read(dest.join("package.py")).unwrap();
        assert_eq!(copied, content);

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_over_existing_dest_overwrites() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_overwrite_src");
        let dest = tmp.join("rez_test_copy_overwrite_dest");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("package.py"), b"new content").unwrap();

        fs::create_dir_all(&dest).unwrap();
        fs::write(dest.join("package.py"), b"old content").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        let result = fs::read(dest.join("package.py")).unwrap();
        assert_eq!(result, b"new content", "copy must overwrite old file");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_multiple_files_all_transferred() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_copy_multi_src");
        let dest = tmp.join("rez_test_copy_multi_dest");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        fs::create_dir_all(&src).unwrap();
        for i in 0..5 {
            fs::write(src.join(format!("file{}.txt", i)), format!("content{}", i).as_bytes())
                .unwrap();
        }

        copy_dir_recursive(&src, &dest).unwrap();

        for i in 0..5 {
            let p = dest.join(format!("file{}.txt", i));
            assert!(p.exists(), "file{}.txt should exist in dest", i);
            let content = fs::read_to_string(&p).unwrap();
            assert_eq!(content, format!("content{}", i));
        }

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_deeply_nested_structure() {
        let tmp = std::env::temp_dir();
        let src = tmp.join("rez_test_deep_src");
        let dest = tmp.join("rez_test_deep_dest");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let deep = src.join("a").join("b").join("c");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join("leaf.txt"), b"deep file").unwrap();

        copy_dir_recursive(&src, &dest).unwrap();

        assert!(
            dest.join("a").join("b").join("c").join("leaf.txt").exists(),
            "deeply nested file must be copied"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }
}

mod test_expand_home_extra {
    use super::expand_home;

    #[test]
    fn test_expand_home_empty_string() {
        let result = expand_home("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_expand_home_only_slash() {
        let result = expand_home("/");
        assert_eq!(result, "/");
    }

    #[test]
    fn test_expand_home_tilde_not_at_start() {
        let result = expand_home("no/tilde/here");
        assert_eq!(result, "no/tilde/here");
    }

    #[test]
    fn test_expand_home_double_slash_path() {
        let result = expand_home("//some//path");
        assert_eq!(result, "//some//path");
    }

    #[test]
    fn test_expand_home_windows_absolute_path() {
        let result = expand_home(r"C:\Users\foo\packages");
        assert_eq!(result, r"C:\Users\foo\packages");
    }
}

mod test_copy_package_fs {
    use super::{copy_package, fs};

    #[test]
    fn test_copy_package_not_found_errors() {
        let src = std::env::temp_dir().join("rez_test_cp_src_empty");
        let dest = std::env::temp_dir().join("rez_test_cp_dest_empty");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
        fs::create_dir_all(&src).unwrap();

        let result = copy_package(
            "nonexistent_pkg_abc",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_err(), "missing package must return Err");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_package_succeeds_when_package_exists() {
        let src = std::env::temp_dir().join("rez_test_cp_src_has_pkg");
        let dest = std::env::temp_dir().join("rez_test_cp_dest_has_pkg");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let pkg_dir = src.join("mypkg").join("1.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            b"name = 'mypkg'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let result = copy_package(
            "mypkg",
            dest.to_str().unwrap(),
            Some("1.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy must succeed: {:?}", result);
        assert!(
            dest.join("mypkg").join("1.0.0").join("package.py").exists(),
            "package.py must be copied to dest"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_package_dest_exists_without_force_errors() {
        let src = std::env::temp_dir().join("rez_test_cp_force_src");
        let dest = std::env::temp_dir().join("rez_test_cp_force_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let pkg_dir = src.join("pkg2").join("2.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("package.py"), b"name = 'pkg2'\nversion = '2.0.0'\n").unwrap();

        let dest_pkg = dest.join("pkg2").join("2.0.0");
        fs::create_dir_all(&dest_pkg).unwrap();
        fs::write(dest_pkg.join("package.py"), b"old").unwrap();

        let result = copy_package(
            "pkg2",
            dest.to_str().unwrap(),
            Some("2.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_err(), "dest exists without force must return Err");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    #[test]
    fn test_copy_package_with_force_overwrites() {
        let src = std::env::temp_dir().join("rez_test_cp_force_ow_src");
        let dest = std::env::temp_dir().join("rez_test_cp_force_ow_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        let pkg_dir = src.join("pkg3").join("3.0.0");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            b"name = 'pkg3'\nversion = '3.0.0'\nnew_content = True\n",
        )
        .unwrap();

        let dest_pkg = dest.join("pkg3").join("3.0.0");
        fs::create_dir_all(&dest_pkg).unwrap();
        fs::write(dest_pkg.join("package.py"), b"old").unwrap();

        let result = copy_package(
            "pkg3",
            dest.to_str().unwrap(),
            Some("3.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            true,
        );
        assert!(result.is_ok(), "force copy must succeed: {:?}", result);
        let new_content = fs::read_to_string(dest_pkg.join("package.py")).unwrap();
        assert!(new_content.contains("new_content"), "overwrite must use new source");

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }
}

mod test_remove_package_extra {
    use super::{fs, remove_package};

    #[test]
    fn test_remove_package_version_not_present_returns_zero() {
        let tmp = std::env::temp_dir().join("rez_test_rm_ver_missing");
        let _ = fs::remove_dir_all(&tmp);
        let pkg_dir = tmp.join("mypkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::create_dir_all(pkg_dir.join("1.0.0")).unwrap();

        let result = remove_package(
            "mypkg",
            Some("9.9.9"),
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0, "missing version should return 0 removals");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_remove_package_family_removes_all_versions() {
        let tmp = std::env::temp_dir().join("rez_test_rm_family_all");
        let _ = fs::remove_dir_all(&tmp);

        for v in &["1.0.0", "2.0.0", "3.0.0"] {
            fs::create_dir_all(tmp.join("bigpkg").join(v)).unwrap();
        }

        let result = remove_package(
            "bigpkg",
            None,
            Some(vec![tmp.to_string_lossy().to_string()]),
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        assert!(!tmp.join("bigpkg").exists(), "entire family must be removed");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_remove_package_multiple_repos_removes_from_first_match() {
        let tmp1 = std::env::temp_dir().join("rez_test_rm_multi_repo1");
        let tmp2 = std::env::temp_dir().join("rez_test_rm_multi_repo2");
        let _ = fs::remove_dir_all(&tmp1);
        let _ = fs::remove_dir_all(&tmp2);

        fs::create_dir_all(tmp1.join("pkg").join("1.0.0")).unwrap();
        fs::create_dir_all(tmp2.join("pkg").join("1.0.0")).unwrap();

        let result = remove_package(
            "pkg",
            Some("1.0.0"),
            Some(vec![
                tmp1.to_string_lossy().to_string(),
                tmp2.to_string_lossy().to_string(),
            ]),
        );
        assert!(result.is_ok());
        assert!(result.unwrap() >= 1);

        let _ = fs::remove_dir_all(&tmp1);
        let _ = fs::remove_dir_all(&tmp2);
    }
}

mod test_package_helpers_extra {
    use super::{copy_dir_recursive, expand_home, fs, remove_package};

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
}
