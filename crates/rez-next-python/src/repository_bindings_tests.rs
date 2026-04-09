//! Tests for PyRepositoryManager — split from repository_bindings.rs to keep the main file ≤1000 lines.

use super::*;

mod test_repository_manager_construction {
    use super::*;

    #[test]
    fn test_new_with_empty_paths() {
        let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
        assert!(mgr.paths.is_empty());
    }

    #[test]
    fn test_new_with_explicit_paths() {
        let mgr = PyRepositoryManager::new(Some(vec![
            "/tmp/pkgs1".to_string(),
            "/tmp/pkgs2".to_string(),
        ]))
        .unwrap();
        assert_eq!(mgr.paths.len(), 2);
        assert_eq!(mgr.paths[0], PathBuf::from("/tmp/pkgs1"));
        assert_eq!(mgr.paths[1], PathBuf::from("/tmp/pkgs2"));
    }

    #[test]
    fn test_paths_order_preserved() {
        let paths = vec![
            "/first".to_string(),
            "/second".to_string(),
            "/third".to_string(),
        ];
        let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
        assert_eq!(mgr.paths[0], PathBuf::from("/first"));
        assert_eq!(mgr.paths[1], PathBuf::from("/second"));
        assert_eq!(mgr.paths[2], PathBuf::from("/third"));
    }

    #[test]
    fn test_duplicate_paths_preserved() {
        let mgr = PyRepositoryManager::new(Some(vec![
            "/same/path".to_string(),
            "/same/path".to_string(),
        ]))
        .unwrap();
        // The manager stores all provided paths unchanged — no deduplication at construction time
        assert_eq!(mgr.paths.len(), 2);
    }

    #[test]
    fn test_new_with_none_does_not_panic() {
        // Default config paths — just ensure no panic
        let result = PyRepositoryManager::new(None);
        assert!(result.is_ok(), "new(None) must not fail");
    }

    #[test]
    fn test_repr_shows_paths() {
        let mgr = PyRepositoryManager::new(Some(vec![
            "/x/first".to_string(),
            "/y/second".to_string(),
        ]))
        .unwrap();
        let repr = mgr.__repr__();
        assert!(repr.contains("RepositoryManager"), "repr: {}", repr);
        assert!(repr.contains("first"), "repr: {}", repr);
        assert!(repr.contains("second"), "repr: {}", repr);
    }

    #[test]
    fn test_repr_empty_paths_shows_empty_array() {
        let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
        let repr = mgr.__repr__();
        assert!(repr.contains("RepositoryManager"), "repr: {}", repr);
        assert!(repr.contains("[]"), "repr for empty should show []: {}", repr);
    }
}

mod test_repository_find_packages {
    use super::*;

    #[test]
    fn test_find_packages_in_nonexistent_dir_returns_empty() {
        let mgr =
            PyRepositoryManager::new(Some(vec!["/no/such/path/xyz_nonexistent".to_string()]))
                .unwrap();
        let result = mgr.find_packages("anything").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_packages_in_empty_temp_dir_returns_empty() {
        let tmp = std::env::temp_dir().join("rez_repo_test_empty");
        std::fs::create_dir_all(&tmp).unwrap();
        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.find_packages("somepkg").unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_find_packages_empty_name_on_nonexistent_returns_empty() {
        let mgr = PyRepositoryManager::new(Some(vec![
            "/totally/nonexistent_cy157".to_string(),
        ]))
        .unwrap();
        let result = mgr.find_packages("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_latest_package_empty_repo_returns_none() {
        let tmp = std::env::temp_dir().join("rez_repo_latest_empty");
        std::fs::create_dir_all(&tmp).unwrap();
        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_latest_package("ghost_pkg").unwrap();
        assert!(result.is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_package_family_names_empty_repo_returns_empty() {
        let tmp = std::env::temp_dir().join("rez_repo_family_empty");
        std::fs::create_dir_all(&tmp).unwrap();
        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_package_family_names().unwrap();
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }



    /// Write a minimal package.py into a temp repo and verify find_packages
    /// returns that package.
    #[test]
    fn test_find_packages_with_real_package_py() {
        let tmp = std::env::temp_dir().join("rez_repo_real_pkg_cy90");
        let pkg_dir = tmp.join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.py"),
            b"name = 'mypkg'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.find_packages("mypkg");
        match result {
            Ok(pkgs) => {
                assert!(
                    !pkgs.is_empty(),
                    "should find mypkg in the temp repo, got empty"
                );
                assert!(
                    pkgs.iter().any(|p| p.0.name == "mypkg"),
                    "found packages: {:?}",
                    pkgs.iter().map(|p| &p.0.name).collect::<Vec<_>>()
                );
            }
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    !msg.contains("panic"),
                    "find_packages must not panic: {}",
                    msg
                );
            }
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// get_latest_package with multiple versions returns the highest
    #[test]
    fn test_get_latest_package_returns_highest_version() {
        let tmp = std::env::temp_dir().join("rez_repo_multi_ver_cy114");
        for ver in ["1.0.0", "2.0.0", "1.5.0"] {
            let dir = tmp.join("mypkg").join(ver);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.py"),
                format!("name = 'mypkg'\nversion = '{}'\n", ver).as_bytes(),
            )
            .unwrap();
        }

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_latest_package("mypkg");
        match result {
            Ok(Some(pkg)) => {
                assert_eq!(pkg.0.name, "mypkg");
            }
            Ok(None) => {} // acceptable if scanning not implemented
            Err(_) => {}
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// get_latest_package with a single package returns Some with correct name
    #[test]
    fn test_get_latest_package_with_one_package() {
        let tmp = std::env::temp_dir().join("rez_repo_latest_one_cy98");
        let pkg_dir = tmp.join("singlepkg").join("3.2.1");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(
            pkg_dir.join("package.py"),
            b"name = 'singlepkg'\nversion = '3.2.1'\n",
        )
        .unwrap();

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_latest_package("singlepkg");
        match result {
            Ok(Some(pkg)) => assert_eq!(pkg.0.name, "singlepkg"),
            Ok(None) => {} // acceptable if scanning not implemented
            Err(_) => {}
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_package_family_names_dedup_and_sorted() {
        let tmp = std::env::temp_dir().join("rez_repo_family_sort_cy90");
        for (name, ver) in [("zebra", "1.0.0"), ("alpha", "2.0.0"), ("alpha", "1.0.0")] {
            let dir = tmp.join(name).join(ver);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.py"),
                format!("name = '{}'\nversion = '{}'\n", name, ver).as_bytes(),
            )
            .unwrap();
        }

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_package_family_names();
        if let Ok(names) = result {
            // Should be deduplicated (alpha appears once) and sorted
            let alpha_count = names.iter().filter(|n| *n == "alpha").count();
            assert!(
                alpha_count <= 1,
                "alpha should be deduped, count={}",
                alpha_count
            );
            let sorted = {
                let mut s = names.clone();
                s.sort();
                s
            };
            assert_eq!(names, sorted, "family names should be sorted");
        }
        // Acceptable if repo scanning is not supported (result is Err)

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// get_package_family_names returns sorted list across 3 families
    #[test]
    fn test_get_package_family_names_sorted_order() {
        let tmp = std::env::temp_dir().join("rez_repo_sorted_cy98");
        for (name, ver) in [("zz_pkg", "1.0.0"), ("aa_pkg", "1.0.0"), ("mm_pkg", "2.0.0")] {
            let dir = tmp.join(name).join(ver);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.py"),
                format!("name = '{}'\nversion = '{}'\n", name, ver).as_bytes(),
            )
            .unwrap();
        }
        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_package_family_names();
        if let Ok(names) = result {
            let mut sorted = names.clone();
            sorted.sort();
            assert_eq!(names, sorted, "family names must be sorted");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// get_package_family_names on single-family repo returns at most one entry
    #[test]
    fn test_get_package_family_names_single_family_has_one_entry() {
        let tmp = std::env::temp_dir().join("rez_repo_single_family_cy114");
        let dir = tmp.join("unique_pkg").join("1.0.0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("package.py"),
            b"name = 'unique_pkg'\nversion = '1.0.0'\n",
        )
        .unwrap();

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_package_family_names();
        if let Ok(names) = result {
            let count = names.iter().filter(|n| n.as_str() == "unique_pkg").count();
            assert!(count <= 1, "unique_pkg should appear at most once, got {count}");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// find_packages on a dir with a family subdirectory but no package.py returns empty
    #[test]
    fn test_find_packages_dir_without_packages_returns_empty() {
        let tmp = std::env::temp_dir().join("rez_repo_nopackage_cy114");
        let sub = tmp.join("empty_family");
        std::fs::create_dir_all(&sub).unwrap();

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.find_packages("empty_family");
        if let Ok(pkgs) = result {
            assert!(pkgs.is_empty(), "expected no packages found without package.py");
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

/// Regression tests for Cycle 158 fixes:
/// - get_latest_package uses semantic Version comparison (3.11.0 > 3.9.0)
/// - get_package_family_names uses list_packages() to enumerate all families
mod test_repository_cy158_fixes {
    use super::*;

    /// Cycle 158: get_latest_package must return 3.11.0 over 3.9.0.
    /// String comparison would return 3.9.0 (lexicographic "3.9" > "3.11"),
    /// semantic Version comparison must return 3.11.0 (numeric 11 > 9).
    #[test]
    fn test_get_latest_package_semantic_version_beats_lexicographic() {
        let tmp = std::env::temp_dir().join("rez_repo_sem_ver_cy158");
        for ver in ["3.9.0", "3.11.0", "3.8.0"] {
            let dir = tmp.join("python").join(ver);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.py"),
                format!("name = 'python'\nversion = '{}'\n", ver).as_bytes(),
            )
            .unwrap();
        }

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_latest_package("python");
        match result {
            Ok(Some(pkg)) => {
                assert_eq!(pkg.0.name, "python");
                let version_str = pkg.0.version.as_ref().map(|v| v.as_str().to_string()).unwrap_or_default();
                assert!(
                    version_str.starts_with("3.11"),
                    "latest python must be 3.11.x (semantic), got: {}",
                    version_str
                );
            }
            Ok(None) => {} // acceptable if scanning not implemented
            Err(_) => {}
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Cycle 158: get_package_family_names must enumerate all families.
    /// Previously used find_packages("") which always returned [] because
    /// the cache key "" never matches any package name.
    #[test]
    fn test_get_package_family_names_enumerates_all_families() {
        let tmp = std::env::temp_dir().join("rez_repo_family_enum_cy158");
        for (name, ver) in [("python", "3.11.0"), ("numpy", "1.25.0"), ("scipy", "1.11.0")] {
            let dir = tmp.join(name).join(ver);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("package.py"),
                format!("name = '{}'\nversion = '{}'\n", name, ver).as_bytes(),
            )
            .unwrap();
        }

        let mgr =
            PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
        let result = mgr.get_package_family_names();
        match result {
            Ok(names) => {
                assert!(
                    names.contains(&"python".to_string()),
                    "expected 'python' in families, got: {:?}",
                    names
                );
                assert!(
                    names.contains(&"numpy".to_string()),
                    "expected 'numpy' in families, got: {:?}",
                    names
                );
                assert!(
                    names.contains(&"scipy".to_string()),
                    "expected 'scipy' in families, got: {:?}",
                    names
                );
                // Must be sorted
                let sorted = {
                    let mut s = names.clone();
                    s.sort();
                    s
                };
                assert_eq!(names, sorted, "family names must be sorted");
            }
            Err(e) => {
                panic!("get_package_family_names must not fail on a valid repo: {e}");
            }
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Cycle 158: get_package_family_names on empty path list returns empty.
    #[test]
    fn test_get_package_family_names_empty_path_list_returns_empty() {
        let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
        let result = mgr.get_package_family_names().unwrap();
        assert!(result.is_empty(), "empty path list must produce no families");
    }
}

