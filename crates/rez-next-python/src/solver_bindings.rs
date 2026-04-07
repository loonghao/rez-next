//! Python bindings for the dependency Solver

use crate::context_bindings::PyResolvedContext;
use crate::package_functions::expand_home;
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
                    .map(|p| PathBuf::from(expand_home(p)))
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
            assert!(
                repr.contains("paths=0"),
                "repr should show 0 paths: {}",
                repr
            );
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
        fn test_solver_paths_stored_correctly() {
            let paths = vec![PathBuf::from("/x/y"), PathBuf::from("/z")];
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: paths.clone(),
            };
            assert_eq!(solver.paths, paths);
        }

        // ── New tests (Cycle 89) ─────────────────────────────────────────────

        #[test]
        fn test_solver_config_prefer_latest_default_true() {
            let config = SolverConfig::default();
            assert!(config.prefer_latest, "prefer_latest should default to true");
        }

        #[test]
        fn test_solver_config_allow_prerelease_default_false() {
            let config = SolverConfig::default();
            assert!(
                !config.allow_prerelease,
                "allow_prerelease should default to false"
            );
        }

        #[test]
        fn test_solver_config_strict_mode_default_false() {
            let config = SolverConfig::default();
            assert!(!config.strict_mode, "strict_mode should default to false");
        }

        #[test]
        fn test_solver_config_enable_caching_default_true() {
            let config = SolverConfig::default();
            assert!(
                config.enable_caching,
                "enable_caching should default to true"
            );
        }

        #[test]
        fn test_solver_config_max_attempts_1000() {
            let config = SolverConfig::default();
            assert_eq!(config.max_attempts, 1000);
        }

        #[test]
        fn test_solver_config_max_time_300s() {
            let config = SolverConfig::default();
            assert_eq!(config.max_time_seconds, 300);
        }

        #[test]
        fn test_solver_repr_shows_prefer_latest_true() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("prefer_latest=true"), "repr: {}", repr);
        }

        #[test]
        fn test_solver_repr_shows_max_attempts_1000() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("max_attempts=1000"), "repr: {}", repr);
        }

        #[test]
        fn test_solver_zero_paths_repr_paths_0() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=0"), "repr: {}", repr);
        }

        #[test]
        fn test_solver_single_path_repr_paths_1() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![PathBuf::from("/packages")],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=1"), "repr: {}", repr);
        }

        // ── New tests (Cycle 96) ─────────────────────────────────────────────

        #[test]
        fn test_solver_config_timeout_positive() {
            let config = SolverConfig::default();
            assert!(
                config.max_time_seconds > 0,
                "max_time_seconds must be positive"
            );
        }

        #[test]
        fn test_solver_paths_empty_vec() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            assert!(solver.paths.is_empty());
        }

        #[test]
        fn test_solver_paths_preserves_order() {
            let paths = vec![
                PathBuf::from("/a/pkgs"),
                PathBuf::from("/b/pkgs"),
                PathBuf::from("/c/pkgs"),
            ];
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: paths.clone(),
            };
            assert_eq!(solver.paths[0], PathBuf::from("/a/pkgs"));
            assert_eq!(solver.paths[1], PathBuf::from("/b/pkgs"));
            assert_eq!(solver.paths[2], PathBuf::from("/c/pkgs"));
        }

        #[test]
        fn test_solver_repr_format_is_valid_string() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            // repr must start and end with parentheses pattern
            assert!(repr.starts_with("Solver("), "repr: {repr}");
            assert!(repr.ends_with(')'), "repr: {repr}");
        }

        #[test]
        fn test_solver_config_allow_prerelease_can_be_set() {
            // Verify SolverConfig fields are accessible
            let mut config = SolverConfig {
                allow_prerelease: true,
                ..SolverConfig::default()
            };
            assert!(config.allow_prerelease);
            config.allow_prerelease = false;
            assert!(!config.allow_prerelease);
        }


        #[test]
        fn test_solver_config_strict_mode_can_be_set() {
            let config = SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            };
            assert!(config.strict_mode);
        }


        #[test]
        fn test_solver_repr_paths_count_four() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![
                    PathBuf::from("/a"),
                    PathBuf::from("/b"),
                    PathBuf::from("/c"),
                    PathBuf::from("/d"),
                ],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=4"), "repr: {repr}");
        }

        // ── Cycle 101 additions ───────────────────────────────────────────────

        #[test]
        fn test_solver_config_enable_caching_can_be_toggled() {
            let mut config = SolverConfig {
                enable_caching: false,
                ..SolverConfig::default()
            };
            assert!(!config.enable_caching);
            config.enable_caching = true;
            assert!(config.enable_caching);
        }


        #[test]
        fn test_solver_config_max_attempts_can_be_changed() {
            let config = SolverConfig {
                max_attempts: 500,
                ..SolverConfig::default()
            };
            assert_eq!(config.max_attempts, 500);
        }


        #[test]
        fn test_solver_config_prefer_latest_can_be_set_false() {
            let config = SolverConfig {
                prefer_latest: false,
                ..SolverConfig::default()
            };
            assert!(!config.prefer_latest);
        }


        #[test]
        fn test_solver_repr_contains_parentheses_balanced() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            let open = repr.chars().filter(|&c| c == '(').count();
            let close = repr.chars().filter(|&c| c == ')').count();
            assert_eq!(open, close, "repr parentheses must be balanced: {repr}");
        }

        #[test]
        fn test_solver_paths_five_elements_repr() {
            let paths: Vec<PathBuf> = (1..=5).map(|i| PathBuf::from(format!("/pkg{i}"))).collect();
            let solver = PySolver {
                config: SolverConfig::default(),
                paths,
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=5"), "repr: {repr}");
        }

        #[test]
        fn test_solver_paths_last_element_preserved() {
            let paths = vec![
                PathBuf::from("/first"),
                PathBuf::from("/middle"),
                PathBuf::from("/last"),
            ];
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: paths.clone(),
            };
            assert_eq!(solver.paths.last().unwrap(), &PathBuf::from("/last"));
        }

        #[test]
        fn test_solver_config_max_time_can_be_changed() {
            let config = SolverConfig {
                max_time_seconds: 60,
                ..SolverConfig::default()
            };
            assert_eq!(config.max_time_seconds, 60);
        }

        // ── Cycle 106 additions ──────────────────────────────────────────────

        #[test]
        fn test_solver_config_max_attempts_100_is_positive() {
            let config = SolverConfig {
                max_attempts: 100,
                ..SolverConfig::default()
            };
            assert!(config.max_attempts > 0);
            assert_eq!(config.max_attempts, 100);
        }

        #[test]
        fn test_solver_two_paths_repr_paths_2() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![PathBuf::from("/p1"), PathBuf::from("/p2")],
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=2"), "repr: {repr}");
        }

        #[test]
        fn test_solver_repr_does_not_contain_negative_paths() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            // paths=0 is valid; paths=-N is never valid
            assert!(!repr.contains("paths=-"), "repr must not show negative path count: {repr}");
        }

        // ── Cycle 115 additions ──────────────────────────────────────────────

        #[test]
        fn test_solver_config_all_defaults_consistent() {
            let c1 = SolverConfig::default();
            let c2 = SolverConfig::default();
            assert_eq!(c1.max_attempts, c2.max_attempts);
            assert_eq!(c1.max_time_seconds, c2.max_time_seconds);
            assert_eq!(c1.prefer_latest, c2.prefer_latest);
            assert_eq!(c1.allow_prerelease, c2.allow_prerelease);
            assert_eq!(c1.strict_mode, c2.strict_mode);
        }

        #[test]
        fn test_solver_config_enable_caching_set_then_unset() {
            let mut config = SolverConfig::default();
            config.enable_caching = false;
            assert!(!config.enable_caching);
            config.enable_caching = true;
            assert!(config.enable_caching);
        }

        #[test]
        fn test_solver_paths_count_zero_when_empty_vec() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: Vec::new(),
            };
            assert_eq!(solver.paths.len(), 0, "empty paths must have len 0");
        }

        #[test]
        fn test_solver_config_prefer_latest_toggle() {
            let mut config = SolverConfig::default();
            let original = config.prefer_latest;
            config.prefer_latest = !original;
            assert_ne!(config.prefer_latest, original);
            config.prefer_latest = original;
            assert_eq!(config.prefer_latest, original);
        }

        #[test]
        fn test_solver_repr_no_leading_whitespace() {
            let solver = PySolver {
                config: SolverConfig::default(),
                paths: vec![],
            };
            let repr = solver.__repr__();
            assert!(!repr.starts_with(' '), "repr must not start with whitespace: '{repr}'");
        }

        #[test]
        fn test_solver_config_strict_mode_false_by_default() {
            let config = SolverConfig::default();
            assert!(!config.strict_mode, "strict_mode default must be false");
        }

        #[test]
        fn test_solver_paths_six_elements() {
            let paths: Vec<PathBuf> = (1..=6).map(|i| PathBuf::from(format!("/p{i}"))).collect();
            let solver = PySolver {
                config: SolverConfig::default(),
                paths,
            };
            let repr = solver.__repr__();
            assert!(repr.contains("paths=6"), "repr: {repr}");
        }
    }
}



