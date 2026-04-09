//! Regression tests for version selection in `package_functions`.
//!
//! Cycle 155 fixed a sort-direction bug: `av.cmp(bv)` (ascending) was used for "latest-first"
//! selection, so the **minimum** version was returned instead of the maximum.  These tests lock
//! that contract so future refactors can't silently regress.
use crate::package_functions::copy_package;
use std::fs;

/// Write a minimal `package.py` for the given package name and version.
fn write_pkg(base: &std::path::Path, name: &str, version: &str) {
    let dir = base.join(name).join(version);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("package.py"),
        format!("name = '{}'\nversion = '{}'\n", name, version).as_bytes(),
    )
    .unwrap();
}

mod test_copy_package_version_selection {
    use super::{copy_package, fs, write_pkg};

    /// When `version=None`, `copy_package` must copy the **latest** (highest) version.
    /// This is the regression test for the ascending-sort bug fixed in Cycle 155.
    #[test]
    fn test_copy_package_no_version_selects_latest() {
        let src = std::env::temp_dir().join("rez_cy155_vt_cp_src");
        let dest = std::env::temp_dir().join("rez_cy155_vt_cp_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        write_pkg(&src, "pkg", "1.0.0");
        write_pkg(&src, "pkg", "2.0.0");
        write_pkg(&src, "pkg", "3.0.0");

        let result = copy_package(
            "pkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy must succeed: {:?}", result);

        // Must have copied the latest version (3.0.0), not the first (1.0.0).
        assert!(
            dest.join("pkg").join("3.0.0").join("package.py").exists(),
            "copy_package(version=None) must copy the latest version 3.0.0, not 1.0.0 or 2.0.0"
        );
        assert!(
            !dest.join("pkg").join("1.0.0").exists(),
            "1.0.0 must NOT be at the destination"
        );
        assert!(
            !dest.join("pkg").join("2.0.0").exists(),
            "2.0.0 must NOT be at the destination"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// With two versions present and `version=None`, the returned dest path must
    /// contain the latest version string as its last component.
    #[test]
    fn test_copy_package_returned_path_contains_latest_version() {
        let src = std::env::temp_dir().join("rez_cy155_vt_path_src");
        let dest = std::env::temp_dir().join("rez_cy155_vt_path_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        write_pkg(&src, "apkg", "1.0.0");
        write_pkg(&src, "apkg", "5.0.0");

        let result = copy_package(
            "apkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy must succeed: {:?}", result);

        let dest_path = result.unwrap();
        assert!(
            dest_path.ends_with("5.0.0"),
            "returned dest path must end with the latest version '5.0.0', got: '{}'",
            dest_path
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// When `version` is explicitly specified, that exact version must be copied.
    #[test]
    fn test_copy_package_explicit_version_copies_that_version() {
        let src = std::env::temp_dir().join("rez_cy155_vt_explicit_src");
        let dest = std::env::temp_dir().join("rez_cy155_vt_explicit_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        write_pkg(&src, "bpkg", "1.0.0");
        write_pkg(&src, "bpkg", "2.0.0");
        write_pkg(&src, "bpkg", "3.0.0");

        let result = copy_package(
            "bpkg",
            dest.to_str().unwrap(),
            Some("2.0.0"),
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy must succeed: {:?}", result);
        assert!(
            dest.join("bpkg").join("2.0.0").join("package.py").exists(),
            "explicitly requested version 2.0.0 must be at destination"
        );
        assert!(
            !dest.join("bpkg").join("1.0.0").exists(),
            "1.0.0 must NOT be at destination when 2.0.0 was requested"
        );
        assert!(
            !dest.join("bpkg").join("3.0.0").exists(),
            "3.0.0 must NOT be at destination when 2.0.0 was requested"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }

    /// Single version present: must be selected regardless of sort order.
    #[test]
    fn test_copy_package_single_version_always_selected() {
        let src = std::env::temp_dir().join("rez_cy155_vt_single_src");
        let dest = std::env::temp_dir().join("rez_cy155_vt_single_dest");
        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);

        write_pkg(&src, "spkg", "7.3.2");

        let result = copy_package(
            "spkg",
            dest.to_str().unwrap(),
            None,
            Some(vec![src.to_string_lossy().to_string()]),
            false,
        );
        assert!(result.is_ok(), "copy of single-version package must succeed: {:?}", result);
        assert!(
            dest.join("spkg").join("7.3.2").join("package.py").exists(),
            "7.3.2 must be at destination"
        );

        let _ = fs::remove_dir_all(&src);
        let _ = fs::remove_dir_all(&dest);
    }
}
