//! Python bindings for ResolvedContext

use crate::package_bindings::PyPackage;
use crate::package_functions::expand_home;
use crate::runtime::get_runtime;
use crate::shell_utils::shell_type_from_str;
use crate::source_bindings::detect_current_shell;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rez_next_context::{ContextStatus, ResolvedContext};
use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::path::PathBuf;
use std::sync::Arc;

/// Python-accessible ResolvedContext class, compatible with rez.resolved_context.ResolvedContext
#[pyclass(name = "ResolvedContext")]
pub struct PyResolvedContext {
    inner: ResolvedContext,
    /// Paths used for resolution
    paths: Vec<PathBuf>,
}

#[pymethods]
impl PyResolvedContext {
    /// Create a new ResolvedContext by resolving the given package requirements.
    /// Compatible with `rez.ResolvedContext(["python-3.9", "maya-2024"])`
    #[new]
    #[pyo3(signature = (packages, paths=None))]
    pub fn new(packages: Vec<String>, paths: Option<Vec<String>>) -> PyResult<Self> {
        use rez_next_common::config::RezCoreConfig;

        let config = RezCoreConfig::load();

        let pkg_paths: Vec<PathBuf> = paths
            .as_ref()
            .map(|p| p.iter().map(PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| PathBuf::from(expand_home(p)))
                    .collect()
            });

        // Parse requirements
        let requirements: Vec<PackageRequirement> = packages
            .iter()
            .map(|s| PackageRequirement::parse(s))
            .collect::<Result<_, _>>()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

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

        // Convert to Requirement for the resolver via string parsing
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
        let resolution = rt
            .block_on(resolver.resolve(resolver_reqs))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut context = ResolvedContext::from_requirements(requirements);
        context.resolved_packages = resolution
            .resolved_packages
            .into_iter()
            .map(|info| (*info.package).clone())
            .collect();
        context.status = ContextStatus::Resolved;

        Ok(PyResolvedContext {
            inner: context,
            paths: pkg_paths,
        })
    }

    fn __str__(&self) -> String {
        format!(
            "ResolvedContext({} packages)",
            self.inner.resolved_packages.len()
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "ResolvedContext(packages={:?}, paths={})",
            self.inner
                .resolved_packages
                .iter()
                .map(|p| p.name.clone())
                .collect::<Vec<_>>(),
            self.paths.len()
        )
    }

    /// Whether the context resolved successfully
    #[getter]
    fn success(&self) -> bool {
        self.inner.status == ContextStatus::Resolved
    }

    /// List of resolved packages
    #[getter]
    fn resolved_packages(&self) -> Vec<PyPackage> {
        self.inner
            .resolved_packages
            .iter()
            .map(|p| PyPackage(p.clone()))
            .collect()
    }

    /// Number of resolved packages
    #[getter]
    fn num_resolved_packages(&self) -> usize {
        self.inner.resolved_packages.len()
    }

    /// Get a resolved package by name (or None)
    fn get_resolved_package(&self, name: &str) -> Option<PyPackage> {
        self.inner
            .resolved_packages
            .iter()
            .find(|p| p.name == name)
            .map(|p| PyPackage(p.clone()))
    }

    /// Get environment variables for this context (as dict)
    fn get_environ(&self, py: Python) -> PyResult<Py<PyAny>> {
        let rt = get_runtime();

        let env_manager = rez_next_context::EnvironmentManager::new(self.inner.config.clone());
        let env_vars = rt
            .block_on(env_manager.generate_environment(&self.inner.resolved_packages))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let dict = PyDict::new(py);
        for (k, v) in env_vars {
            dict.set_item(k, v)?;
        }
        Ok(dict.into_any().unbind())
    }

    /// Apply environment to current process
    fn apply_to_os_environ(&self) -> PyResult<()> {
        let rt = get_runtime();

        let env_manager = rez_next_context::EnvironmentManager::new(self.inner.config.clone());
        let env_vars = rt
            .block_on(env_manager.generate_environment(&self.inner.resolved_packages))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        for (k, v) in env_vars {
            std::env::set_var(k, v);
        }
        Ok(())
    }

    /// Execute a command in the resolved context and return the result
    #[pyo3(signature = (command, stdout=None, stderr=None))]
    fn execute_command(
        &self,
        command: Vec<String>,
        stdout: Option<bool>,
        stderr: Option<bool>,
    ) -> PyResult<i32> {
        let _ = stdout;
        let _ = stderr;
        let rt = get_runtime();

        let env_manager = rez_next_context::EnvironmentManager::new(self.inner.config.clone());
        let env_vars = rt
            .block_on(env_manager.generate_environment(&self.inner.resolved_packages))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        if command.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Command cannot be empty",
            ));
        }

        let mut proc = std::process::Command::new(&command[0]);
        if command.len() > 1 {
            proc.args(&command[1..]);
        }

        for (k, v) in env_vars {
            proc.env(k, v);
        }

        let status = proc
            .status()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        Ok(status.code().unwrap_or(-1))
    }

    /// Context ID
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Creation timestamp
    #[getter]
    fn created_at(&self) -> i64 {
        self.inner.created_at
    }

    /// Save context to file (.rxt format)
    fn save(&self, path: &str) -> PyResult<()> {
        use rez_next_context::{ContextFormat, ContextSerializer};
        use std::path::Path;

        let rt = get_runtime();

        rt.block_on(ContextSerializer::save_to_file(
            &self.inner,
            Path::new(path),
            ContextFormat::Json,
        ))
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Load context from file
    #[staticmethod]
    fn load(path: &str) -> PyResult<PyResolvedContext> {
        use rez_next_context::ContextSerializer;
        use std::path::Path;

        let rt = get_runtime();

        let context = rt
            .block_on(ContextSerializer::load_from_file(Path::new(path)))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        Ok(PyResolvedContext {
            inner: context,
            paths: Vec::new(),
        })
    }

    /// Print the context summary (rez compat: context.print_info())
    fn print_info(&self) {
        let summary = self.inner.get_summary();
        println!("resolved packages ({}):", summary.package_count);
        for (name, version) in &summary.package_versions {
            println!("  {}-{}", name, version);
        }
    }

    /// Generate a shell activation script for the resolved context.
    /// Compatible with `context.get_shell_code(shell)`.
    /// shell: "bash" | "zsh" | "fish" | "cmd" | "powershell" (default: auto-detect)
    #[pyo3(signature = (shell=None))]
    fn to_shell_script(&self, shell: Option<&str>) -> PyResult<String> {
        use rez_next_rex::generate_shell_script;

        let shell_name = shell
            .map(|s| s.to_string())
            .unwrap_or_else(detect_current_shell);
        let shell_type = shell_type_from_str(&shell_name);

        let rt = get_runtime();

        let env_manager = rez_next_context::EnvironmentManager::new(self.inner.config.clone());
        let env_vars = rt
            .block_on(env_manager.generate_environment(&self.inner.resolved_packages))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        // Build RexEnvironment from env_vars
        let mut rex_env = rez_next_rex::RexEnvironment::new();
        rex_env.vars = env_vars;

        Ok(generate_shell_script(&rex_env, &shell_type))
    }

    /// Get the list of tools provided by packages in this context.
    ///
    /// Returns a `{tool_name: tool_path}` dict.  The tool path is built from
    /// `pkg.base` (the package installation root as recorded in `package.py`)
    /// plus a `bin/` sub-directory.  When `pkg.base` is `None` — which happens
    /// for in-memory packages that have not been installed — the path is an
    /// **estimated** `<pkg_name>-<version>/bin/<tool>` string; callers should
    /// treat `None`-base entries as advisory only.
    ///
    /// Compatible with `context.get_tools()`.
    fn get_tools(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for pkg in &self.inner.resolved_packages {
            for tool in &pkg.tools {
                let tool_path = if let Some(base) = &pkg.base {
                    // Use the real installation base recorded in package.py
                    format!("{}/bin/{}", base, tool)
                } else {
                    // Fallback: estimated path for uninstalled / in-memory packages.
                    // This is advisory; the actual path depends on the rez packages path.
                    let ver = pkg
                        .version
                        .as_ref()
                        .map(|v| v.as_str())
                        .unwrap_or("unknown");
                    format!("{}-{}/bin/{}", pkg.name, ver, tool)
                };
                dict.set_item(tool, tool_path)?;
            }
        }
        Ok(dict.into_any().unbind())
    }

    /// Get the context as a dict (rez compat: for serialization)
    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        dict.set_item("id", &self.inner.id)?;
        dict.set_item("status", format!("{:?}", self.inner.status))?;
        dict.set_item(
            "packages",
            self.inner
                .resolved_packages
                .iter()
                .map(|p| {
                    format!(
                        "{}-{}",
                        p.name,
                        p.version.as_ref().map(|v| v.as_str()).unwrap_or("unknown")
                    )
                })
                .collect::<Vec<_>>(),
        )?;
        dict.set_item("num_packages", self.inner.resolved_packages.len())?;
        Ok(dict.into_any().unbind())
    }

    /// Check if this context failed to resolve.
    /// Compatible with `context.failure_description` (returns None if success).
    #[getter]
    fn failure_description(&self) -> Option<String> {
        if self.inner.status == ContextStatus::Failed {
            Some("Resolution failed".to_string())
        } else {
            None
        }
    }

    /// Get the resolved packages as a list of (name, version) tuples.
    fn get_resolved_packages_info(&self, py: Python) -> PyResult<Py<PyAny>> {
        use pyo3::types::PyList;
        let list = PyList::empty(py);
        for pkg in &self.inner.resolved_packages {
            let ver = pkg
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");
            let tuple = pyo3::types::PyTuple::new(
                py,
                [
                    pkg.name.clone().into_pyobject(py)?.into_any().unbind(),
                    ver.to_string().into_pyobject(py)?.into_any().unbind(),
                ],
            )?;
            list.append(tuple)?;
        }
        Ok(list.into_any().unbind())
    }
}

// ─── Rust unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "context_bindings_tests.rs"]
mod context_bindings_tests;
