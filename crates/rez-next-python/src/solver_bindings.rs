//! Python bindings for the dependency Solver

use crate::context_bindings::PyResolvedContext;
use pyo3::prelude::*;
use rez_next_solver::SolverConfig;
use std::path::PathBuf;

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

        Ok(PySolver {
            config: SolverConfig::default(),
            paths,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Solver(paths={}, max_attempts={}, prefer_latest={})",
            self.paths.len(),
            self.config.max_attempts,
            self.config.prefer_latest,
        )
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_solver::SolverConfig;

    mod test_solver_construction {
        use super::*;

        #[test]
        fn test_solver_with_empty_paths_repr() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            assert!(repr.starts_with("Solver("), "repr: {}", repr);
            assert!(repr.contains("paths=0"), "repr should show 0 paths: {}", repr);
        }

        #[test]
        fn test_solver_repr_contains_config_fields() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![PathBuf::from("/tmp/pkgs")],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("max_attempts"), "repr: {}", repr);
            assert!(repr.contains("prefer_latest"), "repr: {}", repr);
        }

        #[test]
        fn test_solver_config_defaults() {
            let config = SolverConfig::default();
            // max_attempts should be positive
            assert!(config.max_attempts > 0, "max_attempts must be > 0");
        }

        #[test]
        fn test_solver_with_multiple_paths() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![
                    PathBuf::from("/a"),
                    PathBuf::from("/b"),
                    PathBuf::from("/c"),
                ],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=3"), "repr: {}", repr);
        }
    }

    mod test_solver_config {
        use super::*;

        #[test]
        fn test_default_config_prefer_latest_is_bool() {
            let cfg = SolverConfig::default();
            // Just verify the field is accessible and has a definite value
            let _ = cfg.prefer_latest;
        }

        #[test]
        fn test_solver_paths_stored_correctly() {
            let paths = vec![PathBuf::from("/x/y"), PathBuf::from("/z")];
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: paths.clone(),
            };
            assert_eq!(solver.paths, paths);
        }
    }
}
