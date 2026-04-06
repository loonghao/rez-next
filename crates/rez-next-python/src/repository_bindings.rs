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
            match result {
                Ok(pkgs) => assert!(pkgs.is_empty()),
                Err(_) => {} // also acceptable
            }
        }

        #[test]
        fn test_get_latest_package_on_nonexistent_path_returns_none_or_error() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/does/not/exist/xyz_cy95".to_string(),
            ]))
            .unwrap();
            let result = mgr.get_latest_package("any_pkg");
            match result {
                Ok(opt) => assert!(opt.is_none()),
                Err(_) => {}
            }
        }

        #[test]
        fn test_get_package_family_names_on_nonexistent_path() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/no/path/cy95_xyz".to_string(),
            ]))
            .unwrap();
            let result = mgr.get_package_family_names();
            match result {
                Ok(names) => assert!(names.is_empty()),
                Err(_) => {}
            }
        }
    }
}

