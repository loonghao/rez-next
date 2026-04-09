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

        // Use the repository manager's get_package(name, None) which sorts by
        // rez_next_version::Version (semantic) rather than string comparison,
        // ensuring "3.11.0" beats "3.9.0" correctly.
        let result = rt
            .block_on(repo_manager.get_package(name, None))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(result.map(|p| PyPackage((*p).clone())))
    }

    /// List all package family names across all repositories
    fn get_package_family_names(&self) -> PyResult<Vec<String>> {
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

        // Use list_packages() which enumerates all cache keys after scanning,
        // instead of find_packages("") which looks for the empty-string key and
        // always returns [].
        let names = rt
            .block_on(repo_manager.list_packages())
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(names)
    }
}

#[cfg(test)]
#[path = "repository_bindings_tests.rs"]
mod tests;
