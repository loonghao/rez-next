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
#[path = "repository_bindings_tests.rs"]
mod tests;
