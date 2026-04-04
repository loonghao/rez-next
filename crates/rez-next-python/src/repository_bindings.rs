//! Python bindings for repository management

use crate::package_bindings::PyPackage;
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
                    .map(|p| {
                        let expanded = if p.starts_with("~/") || p == "~" {
                            if let Ok(home) =
                                std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
                            {
                                p.replacen("~", &home, 1)
                            } else {
                                p.clone()
                            }
                        } else {
                            p.clone()
                        };
                        PathBuf::from(expanded)
                    })
                    .collect()
            });

        Ok(PyRepositoryManager { paths: repo_paths })
    }

    fn __repr__(&self) -> String {
        format!("RepositoryManager(paths={:?})", self.paths)
    }

    /// Find all packages matching a name pattern
    fn find_packages(&self, name: &str) -> PyResult<Vec<PyPackage>> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
    }

    mod test_repository_find_packages {
        use super::*;

        #[test]
        fn test_find_packages_in_nonexistent_dir_returns_empty() {
            let mgr = PyRepositoryManager::new(Some(vec![
                "/no/such/path/xyz_nonexistent".to_string(),
            ]))
            .unwrap();
            let result = mgr.find_packages("anything");
            // Either Ok([]) or Err; must not panic
            match result {
                Ok(pkgs) => assert!(pkgs.is_empty()),
                Err(_) => {} // acceptable — repo path doesn't exist
            }
        }

        #[test]
        fn test_find_packages_in_empty_temp_dir_returns_empty() {
            let tmp = std::env::temp_dir().join("rez_repo_test_empty");
            std::fs::create_dir_all(&tmp).unwrap();
            let mgr =
                PyRepositoryManager::new(Some(vec![tmp.to_string_lossy().to_string()])).unwrap();
            let result = mgr.find_packages("somepkg");
            match result {
                Ok(pkgs) => assert!(pkgs.is_empty()),
                Err(_) => {}
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
            match result {
                Ok(pkg) => assert!(pkg.is_none()),
                Err(_) => {}
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
            match result {
                Ok(names) => assert!(names.is_empty()),
                Err(_) => {}
            }
            let _ = std::fs::remove_dir_all(&tmp);
        }
    }
}
