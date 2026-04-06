//! Python bindings for ResolvedContext

use crate::package_functions::expand_home;
use crate::package_bindings::PyPackage;
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

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
        use rez_next_rex::{generate_shell_script, ShellType};

        let shell_type = match shell.unwrap_or("auto") {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            "cmd" => ShellType::Cmd,
            "powershell" | "pwsh" => ShellType::PowerShell,
            _ => {
                // Auto-detect shell
                if let Ok(sh) = std::env::var("SHELL") {
                    if sh.contains("zsh") {
                        ShellType::Zsh
                    } else if sh.contains("fish") {
                        ShellType::Fish
                    } else {
                        ShellType::Bash
                    }
                } else if cfg!(windows) {
                    if std::env::var("PSModulePath").is_ok() {
                        ShellType::PowerShell
                    } else {
                        ShellType::Cmd
                    }
                } else {
                    ShellType::Bash
                }
            }
        };

        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

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
    /// Compatible with `context.get_tools()`.
    fn get_tools(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for pkg in &self.inner.resolved_packages {
            for tool in &pkg.tools {
                // Tool path: {pkg_root}/bin/{tool}
                let pkg_root = format!("/packages/{}", pkg.name);
                let tool_path = format!("{}/bin/{}", pkg_root, tool);
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

#[cfg(test)]
mod context_bindings_tests {

    use rez_next_context::{ContextStatus, ResolvedContext};

    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    fn make_py_ctx_inner(pkgs: &[(&str, &str)]) -> ResolvedContext {
        let reqs: Vec<PackageRequirement> = pkgs
            .iter()
            .map(|(n, v)| PackageRequirement::parse(&format!("{}-{}", n, v)).unwrap())
            .collect();
        let mut ctx = ResolvedContext::from_requirements(reqs);
        for (name, ver) in pkgs {
            let mut pkg = Package::new(name.to_string());
            pkg.version = Some(Version::parse(ver).unwrap());
            ctx.resolved_packages.push(pkg);
        }
        ctx.status = ContextStatus::Resolved;
        ctx
    }

    // ── success / failure ────────────────────────────────────────────

    #[test]
    fn test_success_is_true_when_resolved() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        assert_eq!(ctx.status, ContextStatus::Resolved);
    }

    #[test]
    fn test_success_is_false_when_failed() {
        let mut ctx = make_py_ctx_inner(&[]);
        ctx.status = ContextStatus::Failed;
        assert_eq!(ctx.status, ContextStatus::Failed);
    }

    // ── resolved_packages ───────────────────────────────────────────

    #[test]
    fn test_resolved_packages_count() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0"), ("maya", "2024.1")]);
        assert_eq!(ctx.resolved_packages.len(), 2);
    }

    #[test]
    fn test_get_resolved_package_by_name() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0"), ("maya", "2024.1")]);
        let py_pkg = ctx.resolved_packages.iter().find(|p| p.name == "python");
        assert!(py_pkg.is_some());
        let ver = py_pkg.unwrap().version.as_ref().unwrap();
        assert_eq!(ver.as_str(), "3.11.0");
    }

    #[test]
    fn test_get_resolved_package_not_found() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        let not_found = ctx.resolved_packages.iter().find(|p| p.name == "houdini");
        assert!(not_found.is_none());
    }

    // ── requirements round-trip ─────────────────────────────────────

    #[test]
    fn test_requirements_stored_correctly() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0"), ("maya", "2024.1")]);
        assert_eq!(ctx.requirements.len(), 2);
        let req_names: Vec<&str> = ctx.requirements.iter().map(|r| r.name.as_str()).collect();
        assert!(req_names.contains(&"python"));
        assert!(req_names.contains(&"maya"));
    }

    // ── id uniqueness ────────────────────────────────────────────────

    #[test]
    fn test_context_ids_are_unique() {
        let ctx1 = make_py_ctx_inner(&[("python", "3.9.0")]);
        let ctx2 = make_py_ctx_inner(&[("python", "3.9.0")]);
        assert_ne!(ctx1.id, ctx2.id, "each context must have a unique ID");
    }

    // ── environment_vars injection ───────────────────────────────────

    #[test]
    fn test_environment_vars_can_be_set() {
        let mut ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        ctx.environment_vars
            .insert("MY_TOOL".to_string(), "active".to_string());
        assert_eq!(
            ctx.environment_vars.get("MY_TOOL"),
            Some(&"active".to_string())
        );
    }

    // ── get_summary ──────────────────────────────────────────────────

    #[test]
    fn test_get_summary_package_count() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0"), ("houdini", "20.0")]);
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 2);
    }

    #[test]
    fn test_get_summary_package_versions() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        let summary = ctx.get_summary();
        assert!(summary.package_versions.contains_key("python"));
        assert_eq!(
            summary.package_versions.get("python"),
            Some(&"3.11.0".to_string())
        );
    }

    // ── created_at timestamp ─────────────────────────────────────────

    #[test]
    fn test_created_at_is_positive() {
        let ctx = make_py_ctx_inner(&[]);
        assert!(
            ctx.created_at > 0,
            "created_at timestamp should be positive"
        );
    }
}
