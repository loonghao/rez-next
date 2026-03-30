//! Python bindings for the dependency Solver

use crate::context_bindings::PyResolvedContext;
use pyo3::prelude::*;
use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::path::PathBuf;
use std::sync::Arc;

/// Python-accessible Solver class, compatible with rez.solver.Solver
#[pyclass(name = "Solver")]
pub struct PySolver {
    config: SolverConfig,
    paths: Vec<PathBuf>,
}

#[pymethods]
impl PySolver {
    /// Create a new Solver.
    /// Compatible with `rez.Solver(packages_path=[...])`
    #[new]
    #[pyo3(signature = (packages_path=None))]
    pub fn new(packages_path: Option<Vec<String>>) -> PyResult<Self> {
        use rez_next_common::config::RezCoreConfig;

        let config = RezCoreConfig::load();
        let paths: Vec<PathBuf> = packages_path
            .map(|p| p.into_iter().map(PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| {
                        let expanded = if p.starts_with("~/") || p == "~" {
                            if let Ok(home) = std::env::var("USERPROFILE")
                                .or_else(|_| std::env::var("HOME"))
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

        Ok(PySolver {
            config: SolverConfig::default(),
            paths,
        })
    }

    fn __repr__(&self) -> String {
        "Solver()".to_string()
    }

    /// Resolve a list of package requirements into a ResolvedContext.
    /// Compatible with `solver.solve(packages)` -> `[ResolvedPackage, ...]`
    fn solve(&self, packages: Vec<String>) -> PyResult<PyResolvedContext> {
        PyResolvedContext::new(
            packages,
            Some(
                self.paths
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect(),
            ),
        )
    }
}
