//! Python bindings for rez.env — rez env command environment generation
//!
//! Provides:
//! - `RezEnv`: complete environment activation (`rez env <PKG>...`)
//! - `env_activate()`: generate activation scripts
//! - `env_apply()`: apply environment to current process

use crate::package_bindings::PyPackage;
use crate::package_functions::expand_home;
use crate::runtime::get_runtime;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_rex::{generate_shell_script, RexExecutor, ShellType};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Complete rez environment: resolved packages + generated environment variables + shell scripts.
/// Equivalent to `rez env <packages>` output.
#[pyclass(name = "RezEnv")]
pub struct PyRezEnv {
    #[pyo3(get)]
    pub packages: Vec<String>,
    /// Shell-specific activation script content
    pub scripts: HashMap<String, String>,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
    /// Whether resolution succeeded
    #[pyo3(get)]
    pub success: bool,
    /// Failure message (if any)
    #[pyo3(get)]
    pub failure_reason: Option<String>,
    /// Resolved package list
    pub resolved_packages: Vec<PyPackage>,
}

#[pymethods]
impl PyRezEnv {
    /// Create a RezEnv by resolving packages and generating environment.
    /// Equivalent to: `rez env python-3.9 maya-2024`
    #[new]
    #[pyo3(signature = (packages, shell=None, paths=None))]
    pub fn new(
        packages: Vec<String>,
        shell: Option<&str>,
        paths: Option<Vec<String>>,
    ) -> PyResult<Self> {
        use rez_next_common::config::RezCoreConfig;

        let config = RezCoreConfig::load();
        let pkg_paths: Vec<PathBuf> = paths
            .map(|p| p.into_iter().map(PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| PathBuf::from(expand_home(p)))
                    .collect()
            });

        // Parse requirements
        let requirements: Result<Vec<PackageRequirement>, _> = packages
            .iter()
            .map(|s| PackageRequirement::parse(s))
            .collect();

        let requirements = match requirements {
            Ok(reqs) => reqs,
            Err(e) => {
                return Ok(PyRezEnv {
                    packages,
                    scripts: HashMap::new(),
                    env_vars: HashMap::new(),
                    success: false,
                    failure_reason: Some(e.to_string()),
                    resolved_packages: Vec::new(),
                });
            }
        };

        // Build repository manager
        let mut repo_manager = RepositoryManager::new();
        for (i, path) in pkg_paths.iter().enumerate() {
            if path.exists() {
                repo_manager.add_repository(Box::new(SimpleRepository::new(
                    path.clone(),
                    format!("repo_{}", i),
                )));
            }
        }

        // Convert to Requirement for resolver
        let resolver_reqs: Vec<Requirement> = requirements
            .iter()
            .map(|pr| {
                let req_str = pr.to_string();
                req_str
                    .parse::<Requirement>()
                    .unwrap_or_else(|_| Requirement::new(pr.name.clone()))
            })
            .collect();

        let rt = get_runtime();

        let repo_arc = Arc::new(repo_manager);
        let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), SolverConfig::default());

        let resolution = match rt.block_on(resolver.resolve(resolver_reqs)) {
            Ok(r) => r,
            Err(e) => {
                return Ok(PyRezEnv {
                    packages,
                    scripts: HashMap::new(),
                    env_vars: HashMap::new(),
                    success: false,
                    failure_reason: Some(e.to_string()),
                    resolved_packages: Vec::new(),
                });
            }
        };

        let resolved_packages: Vec<PyPackage> = resolution
            .resolved_packages
            .iter()
            .map(|info| PyPackage((*info.package).clone()))
            .collect();

        // Generate environment variables from resolved packages
        let mut env_vars: HashMap<String, String> = HashMap::new();
        for pkg_info in &resolution.resolved_packages {
            let pkg = &pkg_info.package;
            if let Some(ref cmds) = pkg.commands {
                let root = pkg
                    .version
                    .as_ref()
                    .map(|v| format!("/opt/{}/{}", pkg.name, v.as_str()))
                    .unwrap_or_else(|| format!("/opt/{}", pkg.name));

                let mut executor = RexExecutor::new();
                if let Ok(rex_env) = executor.execute_commands(
                    cmds,
                    &pkg.name,
                    Some(&root),
                    pkg.version.as_ref().map(|v| v.as_str()),
                ) {
                    for (k, v) in rex_env.vars {
                        env_vars.insert(k, v);
                    }
                }
            }
        }

        // Generate shell-specific activation scripts
        let shell_types_to_generate = if let Some(shell_name) = shell {
            vec![shell_name.to_string()]
        } else {
            vec![
                "bash".to_string(),
                "powershell".to_string(),
                "fish".to_string(),
                "cmd".to_string(),
            ]
        };

        let mut scripts: HashMap<String, String> = HashMap::new();
        for shell_name in &shell_types_to_generate {
            if let Some(shell_type) = ShellType::parse(shell_name) {
                let mut rex_env = rez_next_rex::RexEnvironment::new();
                rex_env.vars = env_vars.clone();
                let script = generate_shell_script(&rex_env, &shell_type);
                scripts.insert(shell_name.clone(), script);
            }
        }

        Ok(PyRezEnv {
            packages,
            scripts,
            env_vars,
            success: true,
            failure_reason: None,
            resolved_packages,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "RezEnv(packages={:?}, success={})",
            self.packages, self.success
        )
    }

    /// Get the activation script for a specific shell.
    /// Compatible with `context.get_shell_code(shell)`.
    fn get_shell_code(&self, shell: &str) -> Option<String> {
        self.scripts.get(shell).cloned()
    }

    /// Get the environment variables dict.
    /// Compatible with `context.get_environ()`.
    fn get_environ(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (k, v) in &self.env_vars {
            dict.set_item(k, v)?;
        }
        Ok(dict.into_any().unbind())
    }

    /// Get resolved package list.
    #[getter]
    fn resolved_packages(&self) -> Vec<PyPackage> {
        self.resolved_packages.clone()
    }

    /// Number of resolved packages.
    #[getter]
    fn num_resolved_packages(&self) -> usize {
        self.resolved_packages.len()
    }

    /// Apply environment variables to the current process.
    /// Equivalent to `context.apply_to_os_environ()`.
    fn apply_to_os_environ(&self) -> PyResult<()> {
        for (k, v) in &self.env_vars {
            std::env::set_var(k, v);
        }
        Ok(())
    }

    /// Print the activation script to stdout.
    /// Equivalent to `rez env --output-shell bash`.
    fn print_script(&self, shell: &str) {
        if let Some(script) = self.scripts.get(shell) {
            print!("{}", script);
        }
    }

    /// Get available shell scripts.
    fn available_shells(&self) -> Vec<String> {
        let mut shells: Vec<String> = self.scripts.keys().cloned().collect();
        shells.sort();
        shells
    }

    /// Write activation script to a file.
    /// Compatible with `context.write_context(path)`.
    #[pyo3(signature = (path, shell="bash"))]
    fn write_script(&self, path: &str, shell: &str) -> PyResult<()> {
        if let Some(script) = self.scripts.get(shell) {
            std::fs::write(path, script)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            Ok(())
        } else {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "No script generated for shell '{}'. Available: {:?}",
                shell,
                self.available_shells()
            )))
        }
    }
}

/// Convenience function: create a RezEnv from package list.
/// Compatible with `rez.env.RezEnv(packages)`.
#[pyfunction]
#[pyo3(signature = (packages, shell=None, paths=None))]
pub fn create_env(
    packages: Vec<String>,
    shell: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<PyRezEnv> {
    PyRezEnv::new(packages, shell, paths)
}

/// Get the activation script for the given packages in the given shell.
/// Equivalent to `rez env <packages> -- printenv` + shell script generation.
#[pyfunction]
#[pyo3(signature = (packages, shell="bash", paths=None))]
pub fn get_activation_script(
    packages: Vec<String>,
    shell: &str,
    paths: Option<Vec<String>>,
) -> PyResult<String> {
    let env = PyRezEnv::new(packages, Some(shell), paths)?;
    if !env.success {
        return Err(pyo3::exceptions::PyRuntimeError::new_err(
            env.failure_reason
                .unwrap_or_else(|| "Unknown error".to_string()),
        ));
    }
    Ok(env.scripts.get(shell).cloned().unwrap_or_default())
}

/// Apply resolved packages to the current process environment.
/// Equivalent to `context.apply_to_os_environ()`.
#[pyfunction]
#[pyo3(signature = (packages, paths=None))]
pub fn apply_env(
    packages: Vec<String>,
    paths: Option<Vec<String>>,
) -> PyResult<HashMap<String, String>> {
    let env = PyRezEnv::new(packages, None, paths)?;
    if env.success {
        for (k, v) in &env.env_vars {
            std::env::set_var(k, v);
        }
    }
    Ok(env.env_vars)
}

/// PackageFamily: groups all versions of a package.
/// Compatible with `rez.packages.PackageFamily`.
#[pyclass(name = "PackageFamily", from_py_object)]
#[derive(Clone)]
pub struct PyPackageFamily {
    #[pyo3(get)]
    pub name: String,
    packages: Vec<PyPackage>,
}

#[pymethods]
impl PyPackageFamily {
    #[new]
    fn new(name: String) -> Self {
        PyPackageFamily {
            name,
            packages: Vec::new(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PackageFamily('{}', {} versions)",
            self.name,
            self.packages.len()
        )
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }

    /// Number of versions
    #[getter]
    fn num_versions(&self) -> usize {
        self.packages.len()
    }

    /// All versions (as strings)
    #[getter]
    fn versions(&self) -> Vec<String> {
        self.packages
            .iter()
            .filter_map(|p| p.0.version.as_ref().map(|v| v.as_str().to_string()))
            .collect()
    }

    /// Get the latest version package
    #[getter]
    fn latest_version(&self) -> Option<PyPackage> {
        let mut sorted = self.packages.clone();
        sorted.sort_by(|a, b| {
            b.0.version
                .as_ref()
                .and_then(|bv| a.0.version.as_ref().map(|av| av.cmp(bv)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().next()
    }

    /// Add a package version to this family
    fn add_package(&mut self, pkg: PyPackage) {
        self.packages.push(pkg);
    }

    /// Iterate packages (returns list)
    fn iter_packages(&self) -> Vec<PyPackage> {
        self.packages.clone()
    }
}


// ─── Rust unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "env_bindings_tests.rs"]
mod env_bindings_tests;
