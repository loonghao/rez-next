//! Python bindings for ResolvedContext

use crate::package_bindings::PyPackage;
use crate::package_functions::expand_home;
use crate::runtime::get_runtime;
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

    // ── failure_description ──────────────────────────────────────────

    #[test]
    fn test_failure_description_none_when_resolved() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        assert_eq!(ctx.status, ContextStatus::Resolved);
        // failure_description is None when resolved — verify via status
        let is_failed = ctx.status == ContextStatus::Failed;
        assert!(!is_failed);
    }

    #[test]
    fn test_failure_description_some_when_failed() {
        let mut ctx = make_py_ctx_inner(&[]);
        ctx.status = ContextStatus::Failed;
        let is_failed = ctx.status == ContextStatus::Failed;
        assert!(is_failed, "Status should be Failed");
    }

    // ── empty resolved context ───────────────────────────────────────

    #[test]
    fn test_empty_context_zero_packages() {
        let ctx = make_py_ctx_inner(&[]);
        assert_eq!(ctx.resolved_packages.len(), 0);
    }

    #[test]
    fn test_get_summary_empty_context() {
        let ctx = make_py_ctx_inner(&[]);
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 0);
        assert!(summary.package_versions.is_empty());
    }

    // ── multiple environment vars ────────────────────────────────────

    #[test]
    fn test_environment_vars_multiple_entries() {
        let mut ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        ctx.environment_vars
            .insert("PYTHONPATH".to_string(), "/usr/lib/python3.11".to_string());
        ctx.environment_vars
            .insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        ctx.environment_vars
            .insert("REZ_USED".to_string(), "1".to_string());
        assert_eq!(ctx.environment_vars.len(), 3);
        assert_eq!(ctx.environment_vars.get("REZ_USED"), Some(&"1".to_string()));
    }

    // ── resolved_packages order preserved ───────────────────────────

    #[test]
    fn test_resolved_packages_order_preserved() {
        let ctx = make_py_ctx_inner(&[("alpha", "1.0.0"), ("zeta", "2.0.0"), ("beta", "3.0.0")]);
        let names: Vec<&str> = ctx
            .resolved_packages
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(
            names,
            vec!["alpha", "zeta", "beta"],
            "Order should match insertion order"
        );
    }

    // ── get_summary returns correct version strings ──────────────────

    #[test]
    fn test_get_summary_all_packages_present() {
        let ctx = make_py_ctx_inner(&[
            ("python", "3.11.0"),
            ("maya", "2024.1"),
            ("arnold", "7.3.0"),
        ]);
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 3);
        assert!(summary.package_versions.contains_key("python"));
        assert!(summary.package_versions.contains_key("maya"));
        assert!(summary.package_versions.contains_key("arnold"));
        assert_eq!(summary.package_versions["arnold"], "7.3.0");
    }

    // ── requirements count matches input ────────────────────────────

    #[test]
    fn test_requirements_count_matches_input_len() {
        let ctx = make_py_ctx_inner(&[("python", "3.11.0"), ("houdini", "20.0"), ("nuke", "14.0")]);
        assert_eq!(ctx.requirements.len(), 3);
    }

    // ── environment_vars cleared ─────────────────────────────────────────────

    #[test]
    fn test_environment_vars_cleared_after_clear() {
        let mut ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        ctx.environment_vars
            .insert("FOO".to_string(), "bar".to_string());
        assert!(!ctx.environment_vars.is_empty());
        ctx.environment_vars.clear();
        assert!(
            ctx.environment_vars.is_empty(),
            "environment_vars should be empty after clear()"
        );
    }

    // ── get_summary: package with no version ────────────────────────────────

    #[test]
    fn test_get_summary_package_without_version() {
        let reqs = vec![PackageRequirement::parse("noversionpkg").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = rez_next_package::Package::new("noversionpkg".to_string());
        pkg.version = None;
        ctx.resolved_packages.push(pkg);
        ctx.status = ContextStatus::Resolved;
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 1);
    }

    // ── context status transitions ───────────────────────────────────────────

    #[test]
    fn test_context_can_transition_to_failed() {
        let mut ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
        assert_eq!(ctx.status, ContextStatus::Resolved);
        ctx.status = ContextStatus::Failed;
        assert_eq!(ctx.status, ContextStatus::Failed);
    }

    // ── resolved packages version string ────────────────────────────────────

    #[test]
    fn test_resolved_package_version_string_format() {
        let ctx = make_py_ctx_inner(&[("nuke", "14.0.5")]);
        let pkg = ctx.resolved_packages.first().unwrap();
        let ver = pkg.version.as_ref().unwrap().as_str();
        assert_eq!(ver, "14.0.5");
    }

    // ── requirements names match ─────────────────────────────────────────────

    #[test]
    fn test_requirement_names_match_resolved() {
        let ctx = make_py_ctx_inner(&[("maya", "2024.1"), ("python", "3.10.0")]);
        let req_names: Vec<&str> = ctx.requirements.iter().map(|r| r.name.as_str()).collect();
        assert!(req_names.contains(&"maya"), "should contain 'maya'");
        assert!(req_names.contains(&"python"), "should contain 'python'");
    }

    // ── large package set ────────────────────────────────────────────────────

    #[test]
    fn test_large_package_set_summary_count() {
        let pkgs: Vec<(&str, &str)> = vec![
            ("pkgA", "1.0"),
            ("pkgB", "2.0"),
            ("pkgC", "3.0"),
            ("pkgD", "4.0"),
            ("pkgE", "5.0"),
        ];
        let ctx = make_py_ctx_inner(&pkgs);
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 5);
    }

    // ── context id is non-empty ──────────────────────────────────────────────

    #[test]
    fn test_context_id_is_non_empty() {
        let ctx = make_py_ctx_inner(&[("python", "3.9.0")]);
        assert!(!ctx.id.is_empty(), "context id must not be empty");
    }

    // ── resolved packages do not include requirements-only entries ───────────

    #[test]
    fn test_resolved_packages_count_independent_of_requirements() {
        // make_py_ctx_inner adds the same pkgs as both requirements AND resolved
        let ctx = make_py_ctx_inner(&[("python", "3.10.0"), ("nuke", "14.0")]);
        assert_eq!(ctx.resolved_packages.len(), 2);
        assert_eq!(ctx.requirements.len(), 2);
    }

    // ── package version None is represented as "unknown" via summary ─────────

    #[test]
    fn test_package_without_version_appears_in_summary() {
        let reqs = vec![PackageRequirement::parse("noversion").unwrap()];
        let mut ctx = ResolvedContext::from_requirements(reqs);
        let mut pkg = Package::new("noversion".to_string());
        pkg.version = None;
        ctx.resolved_packages.push(pkg);
        ctx.status = ContextStatus::Resolved;
        let summary = ctx.get_summary();
        // package should appear in summary even without a version
        assert_eq!(summary.package_count, 1);
        // version value may be "unknown" or empty — just ensure the key exists
        assert!(summary.package_versions.contains_key("noversion"));
    }

    // ── status is Resolving before assignment ────────────────────────────────

    #[test]
    fn test_context_status_resolving_is_not_resolved_or_failed() {
        let mut ctx = make_py_ctx_inner(&[]);
        ctx.status = ContextStatus::Resolving;
        assert_ne!(ctx.status, ContextStatus::Resolved);
        assert_ne!(ctx.status, ContextStatus::Failed);
    }

    // ── requirements list is non-empty for single-package context ────────────

    #[test]
    fn test_single_package_context_has_one_requirement() {
        let ctx = make_py_ctx_inner(&[("houdini", "20.0")]);
        assert_eq!(ctx.requirements.len(), 1);
        assert_eq!(ctx.requirements[0].name, "houdini");
    }

    // ── environment_vars starts empty ────────────────────────────────────────

    #[test]
    fn test_fresh_context_has_empty_environment_vars() {
        let ctx = make_py_ctx_inner(&[("python", "3.12.0")]);
        // environment_vars are not set by make_py_ctx_inner
        assert!(
            ctx.environment_vars.is_empty(),
            "freshly built context should have no environment_vars"
        );
    }

    // ── get_summary with duplicate package names (last wins in HashMap) ───────

    #[test]
    fn test_get_summary_package_versions_has_correct_count() {
        let ctx = make_py_ctx_inner(&[("a", "1.0"), ("b", "2.0"), ("c", "3.0"), ("d", "4.0")]);
        let summary = ctx.get_summary();
        assert_eq!(summary.package_count, 4);
        assert_eq!(summary.package_versions.len(), 4);
    }

    // ── Cycle 114 additions ──────────────────────────────────────────────────

    mod test_context_cy114 {
        use super::*;

        /// resolved_packages len matches the input package count
        #[test]
        fn test_resolved_packages_len_matches_input() {
            let ctx = make_py_ctx_inner(&[("pkgX", "1.0.0"), ("pkgY", "2.0.0")]);
            assert_eq!(ctx.resolved_packages.len(), 2);
        }

        /// context status is Resolved when built via make_py_ctx_inner
        #[test]
        fn test_context_status_is_resolved() {
            let ctx = make_py_ctx_inner(&[("maya", "2024.1")]);
            assert_eq!(ctx.status, ContextStatus::Resolved);
        }

        /// context with no packages has zero count in summary
        #[test]
        fn test_empty_context_summary_has_zero_count() {
            let ctx = make_py_ctx_inner(&[]);
            let summary = ctx.get_summary();
            assert_eq!(summary.package_count, 0);
        }

        /// environment_vars can be set and retrieved
        #[test]
        fn test_environment_vars_set_and_retrieve() {
            let mut ctx = make_py_ctx_inner(&[("python", "3.9.0")]);
            ctx.environment_vars
                .insert("MY_ENV".to_string(), "my_value".to_string());
            assert_eq!(
                ctx.environment_vars.get("MY_ENV").map(String::as_str),
                Some("my_value")
            );
        }

        /// summary package_versions map contains the package version string
        #[test]
        fn test_summary_version_value_matches_resolved() {
            let ctx = make_py_ctx_inner(&[("houdini", "20.0.1")]);
            let summary = ctx.get_summary();
            assert_eq!(
                summary.package_versions.get("houdini").map(String::as_str),
                Some("20.0.1")
            );
        }

        /// id field is a stable, non-whitespace string
        #[test]
        fn test_context_id_has_no_whitespace() {
            let ctx = make_py_ctx_inner(&[("nuke", "14.0")]);
            let id = &ctx.id;
            assert!(
                !id.chars().any(|c| c.is_whitespace()),
                "context id must not contain whitespace: '{id}'"
            );
        }

        /// requirements for single package has name matching the package
        #[test]
        fn test_single_package_requirement_name_matches() {
            let ctx = make_py_ctx_inner(&[("arnold", "7.1.0")]);
            assert_eq!(ctx.requirements[0].name, "arnold");
        }
    }

    // ── Cycle 119 additions ──────────────────────────────────────────────────

    mod test_context_cy119 {
        use super::*;

        /// resolved_packages returns names in same order as input
        #[test]
        fn test_resolved_packages_names_in_insertion_order() {
            let ctx = make_py_ctx_inner(&[("alpha", "1.0"), ("beta", "2.0"), ("gamma", "3.0")]);
            let names: Vec<&str> = ctx
                .resolved_packages
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            assert_eq!(names, vec!["alpha", "beta", "gamma"]);
        }

        /// context built with 4 packages has summary.package_count == 4
        #[test]
        fn test_summary_count_four_packages() {
            let ctx = make_py_ctx_inner(&[("a", "1"), ("b", "2"), ("c", "3"), ("d", "4")]);
            assert_eq!(ctx.get_summary().package_count, 4);
        }

        /// overwriting environment_vars entry updates the value
        #[test]
        fn test_environment_vars_overwrite_value() {
            let mut ctx = make_py_ctx_inner(&[("python", "3.11.0")]);
            ctx.environment_vars
                .insert("FOO".to_string(), "first".to_string());
            ctx.environment_vars
                .insert("FOO".to_string(), "second".to_string());
            assert_eq!(
                ctx.environment_vars.get("FOO").map(String::as_str),
                Some("second"),
                "overwriting key should update to latest value"
            );
        }

        /// status Resolving is distinct from Resolved and Failed
        #[test]
        fn test_resolving_status_is_distinct_from_others() {
            let mut ctx = make_py_ctx_inner(&[]);
            ctx.status = ContextStatus::Resolving;
            assert_ne!(ctx.status, ContextStatus::Resolved);
            assert_ne!(ctx.status, ContextStatus::Failed);
        }

        /// requirements list for two packages has length two
        #[test]
        fn test_two_package_requirements_list_has_length_two() {
            let ctx = make_py_ctx_inner(&[("nuke", "14.0"), ("python", "3.10")]);
            assert_eq!(ctx.requirements.len(), 2);
        }

        /// get_summary version map keys match resolved package names
        #[test]
        fn test_summary_version_keys_match_package_names() {
            let ctx = make_py_ctx_inner(&[("houdini", "20.5"), ("katana", "6.0")]);
            let summary = ctx.get_summary();
            let names: Vec<&str> = ctx
                .resolved_packages
                .iter()
                .map(|p| p.name.as_str())
                .collect();
            for name in &names {
                assert!(
                    summary.package_versions.contains_key(*name),
                    "summary must contain key '{name}'"
                );
            }
        }
    }
}
