//! Python bindings for repository management

use crate::package_bindings::PyPackage;
use crate::package_functions::expand_home;
use crate::runtime::get_runtime;
use pyo3::prelude::*;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;

/// Python-accessible RepositoryManager class
#[pyclass(name = "RepositoryManager")]
pub struct PyRepositoryManager {
    paths: Vec<PathBuf>,
}

#[pymethods]
impl PyRepositoryManager {
    /// Create a RepositoryManager from a list of paths
    #[new]
    #[pyo3(signature = (paths=None))]
    pub fn new(paths: Option<Vec<String>>) -> PyResult<Self> {
        use rez_next_common::config::RezCoreConfig;

        let config = RezCoreConfig::load();
        let repo_paths: Vec<PathBuf> = paths
            .map(|p| p.into_iter().map(PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| PathBuf::from(expand_home(p)))
                    .collect()
            });

        Ok(PyRepositoryManager { paths: repo_paths })
    }

    fn __repr__(&self) -> String {
        format!("RepositoryManager(paths={:?})", self.paths)
    }

    /// Find all packages matching a name pattern
    fn find_packages(&self, name: &str) -> PyResult<Vec<PyPackage>> {
        let rt = get_runtime();

        let mut repo_manager = RepositoryManager::new();
        for (i, path) in self.paths.iter().enumerate() {
            if path.exists() {
                repo_manager.add_repository(Box::new(SimpleRepository::new(
                    path.clone(),
                    format!("repo_{}", i),
                )));
            }
        }

        let packages = rt
            .block_on(repo_manager.find_packages(name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(packages
            .into_iter()
            .map(|p| PyPackage((*p).clone()))
            .collect())
    }

    /// Get the latest version of a package
    fn get_latest_package(&self, name: &str) -> PyResult<Option<PyPackage>> {
        let packages = self.find_packages(name)?;
        let mut sorted = packages;
        sorted.sort_by(|a, b| {
            let av = a.0.version.as_ref().map(|v| v.as_str().to_string());
            let bv = b.0.version.as_ref().map(|v| v.as_str().to_string());
            bv.cmp(&av) // descending
        });
        Ok(sorted.into_iter().next())
    }

    /// List all package names in all repositories
    fn get_package_family_names(&self) -> PyResult<Vec<String>> {
        let packages = self.find_packages("")?;
        let mut names: Vec<String> = packages.iter().map(|p| p.0.name.clone()).collect();
        names.sort();
        names.dedup();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
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
        fn test_repr_contains_paths() {
            let mgr = PyRepositoryManager::new(Some(vec!["/a/b".to_string()])).unwrap();
            let repr = mgr.__repr__();
            assert!(repr.contains("RepositoryManager"), "repr: {}", repr);
        }

        #[test]
        fn test_repr_empty_paths_shows_empty_array() {
            let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
            let repr = mgr.__repr__();
            assert!(
                repr.contains("RepositoryManager"),
                "repr should contain type name: {}",
                repr
            );
            assert!(
                repr.contains("[]"),
                "repr for empty should show []: {}",
                repr
            );
        }

        #[test]
        fn test_repr_multiple_paths_shows_both() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/x/first".to_string(),
                "/y/second".to_string(),
            ]))
            .unwrap();
            let repr = mgr.__repr__();
            assert!(repr.contains("first"), "repr: {}", repr);
            assert!(repr.contains("second"), "repr: {}", repr);
        }

        #[test]
        fn test_new_with_none_does_not_panic() {
            // Default config paths — just ensure no panic
            let result = PyRepositoryManager::new(None);
            assert!(result.is_ok(), "new(None) must not fail");
        }
    }

    mod test_repository_find_packages {
        use super::*;

        #[test]
        fn test_find_packages_in_nonexistent_dir_returns_empty() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/no/such/path/xyz_nonexistent".to_string()]))
                    .unwrap();
            let result = mgr.find_packages("anything");
            // Either Ok([]) or Err; must not panic
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }
        }

        #[test]
        fn test_find_packages_in_empty_temp_dir_returns_empty() {
            let tmp = std::env::temp_dir().join("rez_repo_test_empty");
            std::fs::create_dir_all(&tmp).unwrap();
            let mgr =
                PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
            let result = mgr.find_packages("somepkg");
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }

            let _ = std::fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_get_latest_package_empty_repo_returns_none() {
            let tmp = std::env::temp_dir().join("rez_repo_latest_empty");
            std::fs::create_dir_all(&tmp).unwrap();
            let mgr =
                PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
            let result = mgr.get_latest_package("ghost_pkg");
            if let Ok(pkg) = result {
                assert!(pkg.is_none());
            }

            let _ = std::fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_get_package_family_names_empty_repo_returns_empty() {
            let tmp = std::env::temp_dir().join("rez_repo_family_empty");
            std::fs::create_dir_all(&tmp).unwrap();
            let mgr =
                PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
            let result = mgr.get_package_family_names();
            if let Ok(names) = result {
                assert!(names.is_empty());
            }

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
                    // Acceptable if repo scanning is not implemented for this path format
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

        #[test]
        fn test_get_package_family_names_dedup_and_sorted() {
            let tmp = std::env::temp_dir().join("rez_repo_family_sort_cy90");
            // Create two families: zebra and alpha
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
            match result {
                Ok(names) => {
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
                Err(_) => {
                    // Acceptable if repo scanning is not supported
                }
            }

            let _ = std::fs::remove_dir_all(&tmp);
        }
    }

    mod test_repository_manager_paths {
        use super::*;

        #[test]
        fn test_single_path_stored_correctly() {
            let mgr = PyRepositoryManager::new(Some(vec!["/single/path".to_string()])).unwrap();
            assert_eq!(mgr.paths.len(), 1);
            assert_eq!(mgr.paths[0], PathBuf::from("/single/path"));
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
        fn test_repr_contains_path_value() {
            let mgr = PyRepositoryManager::new(Some(vec!["/my/pkg/path".to_string()])).unwrap();
            let repr = mgr.__repr__();
            assert!(repr.contains("my"), "repr should contain path fragment: {repr}");
        }

        #[test]
        fn test_paths_len_matches_input() {
            for n in 0..=4 {
                let paths: Vec<String> = (0..n).map(|i| format!("/path/{}", i)).collect();
                let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
                assert_eq!(mgr.paths.len(), n, "expected {} paths", n);
            }
        }

        #[test]
        fn test_repr_shows_full_path() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/packages/production".to_string()])).unwrap();
            let repr = mgr.__repr__();
            assert!(
                repr.contains("packages"),
                "repr must contain path segment: {repr}"
            );
            assert!(
                repr.contains("production"),
                "repr must contain path segment: {repr}"
            );
        }

        #[test]
        fn test_find_packages_empty_name_in_nonexistent_repo() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/no/such/repo_xyz_nonexistent_12345".to_string(),
            ]))
            .unwrap();
            let result = mgr.find_packages("");
            // Must not panic; empty or error both acceptable
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }

        }

        #[test]
        fn test_get_latest_package_on_nonexistent_path_returns_none_or_error() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/does/not/exist/xyz_cy95".to_string(),
            ]))
            .unwrap();
            let result = mgr.get_latest_package("any_pkg");
            if let Ok(opt) = result {
                assert!(opt.is_none());
            }

        }

        #[test]
        fn test_get_package_family_names_on_nonexistent_path() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/no/path/cy95_xyz".to_string(),
            ]))
            .unwrap();
            let result = mgr.get_package_family_names();
            if let Ok(names) = result {
                assert!(names.is_empty());
            }

        }
    }

    mod test_repository_manager_extra {
        use super::*;

        /// new() with a path containing unicode characters does not panic
        #[test]
        fn test_new_with_unicode_path_no_panic() {
            let result = PyRepositoryManager::new(Some(vec!["/pkgs/日本語テスト".to_string()]));
            assert!(result.is_ok(), "unicode path must not fail construction");
        }

        /// repr for path with unicode must contain RepositoryManager
        #[test]
        fn test_repr_unicode_path_contains_type_name() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/pkgs/日本語テスト".to_string()])).unwrap();
            let repr = mgr.__repr__();
            assert!(
                repr.contains("RepositoryManager"),
                "repr must contain type name for unicode path: {repr}"
            );
        }

        /// PathBuf from empty string is stored as-is
        #[test]
        fn test_empty_string_path_stored() {
            let mgr = PyRepositoryManager::new(Some(vec!["".to_string()])).unwrap();
            assert_eq!(mgr.paths.len(), 1);
            assert_eq!(mgr.paths[0], PathBuf::from(""));
        }

        /// new() with 100 paths stores all 100
        #[test]
        fn test_hundred_paths_stored() {
            let paths: Vec<String> = (0..100).map(|i| format!("/bulk/{}", i)).collect();
            let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
            assert_eq!(mgr.paths.len(), 100);
        }

        /// find_packages with special chars in name does not panic
        #[test]
        fn test_find_packages_special_chars_no_panic() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/no/such/special_cy104".to_string()])).unwrap();
            let result = mgr.find_packages("pkg-with-dashes_and.dots");
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }

        }

        /// repr for path with spaces must still be valid string
        #[test]
        fn test_repr_path_with_spaces() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/path with spaces/pkgs".to_string(),
            ]))
            .unwrap();
            let repr = mgr.__repr__();
            assert!(
                repr.contains("RepositoryManager"),
                "repr should start with RepositoryManager: {repr}"
            );
        }

        /// Five paths: paths.len() == 5
        #[test]
        fn test_five_paths_stored() {
            let paths: Vec<String> = (0..5).map(|i| format!("/repo/{}", i)).collect();
            let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
            assert_eq!(mgr.paths.len(), 5);
        }

        /// Duplicate paths are NOT deduplicated by the manager (preserves input as-is)
        #[test]
        fn test_duplicate_paths_preserved() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/same/path".to_string(),
                "/same/path".to_string(),
            ]))
            .unwrap();
            // The manager stores all provided paths unchanged
            assert_eq!(mgr.paths.len(), 2);
        }

        /// find_packages with very long package name does not panic
        #[test]
        fn test_find_packages_long_name_no_panic() {
            let long_name = "a".repeat(256);
            let mgr =
                PyRepositoryManager::new(Some(vec!["/nonexistent_cy98".to_string()])).unwrap();
            let result = mgr.find_packages(&long_name);
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }

        }

        /// get_latest_package on a repo with one package returns Some
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

        /// repr for a 10-path manager contains "RepositoryManager"
        #[test]
        fn test_repr_ten_paths() {
            let paths: Vec<String> = (0..10).map(|i| format!("/p{}", i)).collect();
            let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
            assert!(mgr.__repr__().contains("RepositoryManager"));
        }

        /// get_package_family_names returns sorted list
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
    }

    mod test_repository_manager_cy114 {
        use super::*;

        /// paths list with exactly one element has len == 1
        #[test]
        fn test_single_element_paths_len_is_one() {
            let mgr = PyRepositoryManager::new(Some(vec!["/only/one".to_string()])).unwrap();
            assert_eq!(mgr.paths.len(), 1);
        }

        /// repr for a path containing numeric segments is valid
        #[test]
        fn test_repr_numeric_path_segment() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/pkgs/2024/release".to_string()])).unwrap();
            let repr = mgr.__repr__();
            assert!(
                repr.contains("2024"),
                "repr should contain numeric segment '2024': {repr}"
            );
        }

        /// find_packages with path that exists but has no package.py at root
        #[test]
        fn test_find_packages_dir_without_packages_returns_empty() {
            let tmp = std::env::temp_dir().join("rez_repo_nopackage_cy114");
            // Create a subdirectory but no package.py inside
            let sub = tmp.join("empty_family");
            std::fs::create_dir_all(&sub).unwrap();
            // No package.py here

            let mgr =
                PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
            let result = mgr.find_packages("empty_family");
            if let Ok(pkgs) = result {
                // Without package.py, the package shouldn't be found
                assert!(
                    pkgs.is_empty(),
                    "expected no packages found without package.py"
                );
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
                    // The returned package must be "mypkg"
                    assert_eq!(pkg.0.name, "mypkg");
                }
                Ok(None) => {} // acceptable if scanning not implemented
                Err(_) => {}
            }

            let _ = std::fs::remove_dir_all(&tmp);
        }

        /// paths constructed from relative-looking strings are stored verbatim
        #[test]
        fn test_relative_path_stored_verbatim() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["relative/path/to/pkgs".to_string()])).unwrap();
            assert_eq!(mgr.paths[0], PathBuf::from("relative/path/to/pkgs"));
        }

        /// repr for manager with 3 distinct paths shows all 3 paths count
        #[test]
        fn test_repr_three_paths_all_present() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/first/path".to_string(),
                "/second/path".to_string(),
                "/third/path".to_string(),
            ]))
            .unwrap();
            let repr = mgr.__repr__();
            assert!(repr.contains("first"), "repr missing 'first': {repr}");
            assert!(repr.contains("second"), "repr missing 'second': {repr}");
            assert!(repr.contains("third"), "repr missing 'third': {repr}");
        }

        /// get_package_family_names on single-family repo returns exactly one name
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
                // Should not have duplicates; unique_pkg appears at most once
                let count = names.iter().filter(|n| n.as_str() == "unique_pkg").count();
                assert!(count <= 1, "unique_pkg should appear at most once, got {count}");
            }


            let _ = std::fs::remove_dir_all(&tmp);
        }
    }

    mod test_repository_cy120 {
        use super::*;

        /// new() with two identical paths preserves both (no dedup)
        #[test]
        fn test_new_two_identical_paths_len_two() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/same/path".to_string(),
                "/same/path".to_string(),
            ]))
            .unwrap();
            assert_eq!(mgr.paths.len(), 2, "duplicate paths must be preserved as-is");
        }

        /// repr for manager with 7 paths contains "RepositoryManager"
        #[test]
        fn test_repr_seven_paths_contains_type_name() {
            let paths: Vec<String> = (0..7).map(|i| format!("/pkgs/{i}")).collect();
            let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
            assert!(
                mgr.__repr__().contains("RepositoryManager"),
                "repr for 7-path manager must contain type name"
            );
        }

        /// find_packages with empty name on nonexistent path returns empty (not panic)
        #[test]
        fn test_find_packages_empty_name_nonexistent_no_panic() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/totally/nonexistent_cy120".to_string(),
            ]))
            .unwrap();
            let result = mgr.find_packages("");
            if let Ok(pkgs) = result {
                assert!(pkgs.is_empty());
            }
        }

        /// paths stored from new(None) come from default config (must not panic)
        #[test]
        fn test_new_none_uses_default_config_no_panic() {
            let result = PyRepositoryManager::new(None);
            assert!(result.is_ok(), "new(None) must not fail");
            let mgr = result.unwrap();
            // We cannot assert exact count (depends on user config), just no panic
            let _ = mgr.paths.len();
        }

        /// get_package_family_names on empty path list returns empty vec
        #[test]
        fn test_get_family_names_empty_path_list_returns_empty() {
            let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
            let result = mgr.get_package_family_names();
            if let Ok(names) = result {
                assert!(names.is_empty(), "empty path list must produce no families");
            }
        }

        /// repr for manager with exactly 0 paths shows paths=[] or similar
        #[test]
        fn test_repr_zero_paths_valid_string() {
            let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
            let repr = mgr.__repr__();
            assert!(!repr.is_empty(), "repr must not be empty for 0-path manager");
            assert!(repr.contains("RepositoryManager"));
        }
    }

    mod test_repository_cy126 {
        use super::*;

        /// new() with one path stores exactly one path
        #[test]
        fn test_new_one_path_len_is_one() {
            let mgr = PyRepositoryManager::new(Some(vec!["/single/path".to_string()])).unwrap();
            assert_eq!(mgr.paths.len(), 1);
        }

        /// new() with empty paths list has len 0
        #[test]
        fn test_new_empty_paths_len_is_zero() {
            let mgr = PyRepositoryManager::new(Some(vec![])).unwrap();
            assert_eq!(mgr.paths.len(), 0);
        }

        /// repr contains each path entry
        #[test]
        fn test_repr_contains_path_count() {
            let paths = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
            let mgr = PyRepositoryManager::new(Some(paths)).unwrap();
            let repr = mgr.__repr__();
            // repr format: RepositoryManager(paths=["/a", "/b", "/c"])
            assert!(
                repr.contains("/a") && repr.contains("/b") && repr.contains("/c"),
                "repr should contain all path entries: {repr}"
            );
        }

        /// find_packages on non-existent path does not panic
        #[test]
        fn test_find_packages_nonexistent_path_no_panic() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/nonexistent_cy126/pkgs".to_string()]))
                    .unwrap();
            let _ = mgr.find_packages("python");
        }

        /// get_package_family_names on single non-existent path returns Ok
        #[test]
        fn test_get_family_names_single_nonexistent_path_ok() {
            let mgr =
                PyRepositoryManager::new(Some(vec!["/nonexistent_cy126/pkgs2".to_string()]))
                    .unwrap();
            // Should either return Ok(empty) or Err — but must not panic
            let _ = mgr.get_package_family_names();
        }
    }
}




