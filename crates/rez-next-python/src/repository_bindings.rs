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

impl PyRepositoryManager {
    /// Build a `RepositoryManager` containing only the paths that currently exist on disk.
    fn build_repo_manager(&self) -> RepositoryManager {
        let mut manager = RepositoryManager::new();
        for (i, path) in self.paths.iter().enumerate() {
            if path.exists() {
                manager.add_repository(Box::new(SimpleRepository::new(
                    path.clone(),
                    format!("repo_{}", i),
                )));
            }
        }
        manager
    }
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

    /// Find all packages matching a name (exact family name lookup).
    fn find_packages(&self, name: &str) -> PyResult<Vec<PyPackage>> {
        let rt = get_runtime();
        let repo_manager = self.build_repo_manager();

        let packages = rt
            .block_on(repo_manager.find_packages(name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(packages
            .into_iter()
            .map(|p| PyPackage((*p).clone()))
            .collect())
    }

    /// Iterate all versions of a package family, sorted newest-first by semantic version.
    ///
    /// Equivalent to `rez.packages.iter_packages(name)`.
    /// Returns all discovered versions in descending semantic version order.
    fn iter_packages(&self, name: &str) -> PyResult<Vec<PyPackage>> {
        let rt = get_runtime();
        let repo_manager = self.build_repo_manager();

        let mut packages: Vec<_> = rt
            .block_on(repo_manager.find_packages(name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?
            .into_iter()
            .map(|p| PyPackage((*p).clone()))
            .collect();

        // Sort descending by semantic version (newest first), matching rez.packages.iter_packages
        packages.sort_by(|a, b| {
            let va = a.0.version.as_ref().and_then(|v| {
                rez_next_version::Version::parse(v.as_str()).ok()
            });
            let vb = b.0.version.as_ref().and_then(|v| {
                rez_next_version::Version::parse(v.as_str()).ok()
            });
            match (va, vb) {
                (Some(a), Some(b)) => b.cmp(&a), // descending: newest first
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        Ok(packages)
    }

    /// Get the latest version of a package.
    fn get_latest_package(&self, name: &str) -> PyResult<Option<PyPackage>> {
        let rt = get_runtime();
        let repo_manager = self.build_repo_manager();

        // Use get_package(name, None) which applies semantic Version comparison,
        // ensuring "3.11.0" beats "3.9.0" correctly (not lexicographic).
        let result = rt
            .block_on(repo_manager.get_package(name, None))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(result.map(|p| PyPackage((*p).clone())))
    }

    /// List all package family names across all repositories, sorted and deduplicated.
    fn get_package_family_names(&self) -> PyResult<Vec<String>> {
        let rt = get_runtime();
        let repo_manager = self.build_repo_manager();

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
