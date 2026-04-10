//! Extra unit tests for `package_functions` — Cycles 103, 122, 130, 132 additions.
//! Split from package_functions_tests.rs (Cycle 147).
//! Cycle 180: helpers and move_package tests moved to package_functions_move_tests.rs.
use crate::package_functions::{copy_package, expand_home, remove_package};
use std::fs;

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
