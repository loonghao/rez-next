//! Unit tests for `package_functions` — split from the main module to keep file size ≤ 1000 lines.
//! Extra tests (expand_home_extra/copy_package_fs/remove_package_extra/helpers) → package_functions_extra_tests.rs (Cycle 147)
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
