//! Python bindings for the dependency Solver

use crate::context_bindings::PyResolvedContext;
use crate::{
    dependency_conflicts_bindings, package_variant_bindings, reduction_bindings,
    requirement_list_bindings, solver_state_bindings,
};
use crate::package_functions::expand_home;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rez_next_solver::{SolverConfig, SolverStatus};
use rez_next_solver::{
    ConflictResolution as RustConflictResolution,
    ConflictSeverity,
    DependencyConflict as RustDependencyConflict,
    FailureReason,
};
use std::path::PathBuf;

// ── SolverStatus Python bindings ─────────────────────────────────────────────

/// A member of the SolverStatus enum.
/// Compatible with `rez.solver.SolverStatus` members.
#[pyclass(name = "SolverStatusMember", from_py_object)]
#[derive(Clone)]
pub struct PySolverStatusMember {
    pub inner: SolverStatus,
}

#[pymethods]
impl PySolverStatusMember {
    /// Returns the name of this status (e.g., "pending", "solved").
    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    /// Returns the description of this status.
    #[getter]
    fn description(&self) -> String {
        self.inner.description().to_string()
    }

    fn __repr__(&self) -> String {
        format!(
            "<SolverStatus.{}: \"{}\">",
            self.inner.name(),
            self.inner.description()
        )
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_member) = other.extract::<PyRef<'_, PySolverStatusMember>>() {
            Ok(self.inner == other_member.inner)
        } else {
            Ok(false)
        }
    }
}

/// SolverStatus enum class, compatible with `rez.solver.SolverStatus`.
///
/// Usage:
///     from rez_next.solver_ import SolverStatus
///     status = SolverStatus.pending
///     print(status.name)  # "pending"
///     print(status.description)  # "The solve has not yet started."
pub fn register_solver_status(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    // Create the SolverStatus "class" as a module-like object with members
    let solver_status_module = PyModule::new(parent_module.py(), "SolverStatus")?;

    // Add member instances as attributes
    let pending = PySolverStatusMember {
        inner: SolverStatus::Pending,
    };
    let solved = PySolverStatusMember {
        inner: SolverStatus::Solved,
    };
    let exhausted = PySolverStatusMember {
        inner: SolverStatus::Exhausted,
    };
    let failed = PySolverStatusMember {
        inner: SolverStatus::Failed,
    };
    let cyclic = PySolverStatusMember {
        inner: SolverStatus::Cyclic,
    };
    let unsolved = PySolverStatusMember {
        inner: SolverStatus::Unsolved,
    };

    solver_status_module.add("pending", pending)?;
    solver_status_module.add("solved", solved)?;
    solver_status_module.add("exhausted", exhausted)?;
    solver_status_module.add("failed", failed)?;
    solver_status_module.add("cyclic", cyclic)?;
    solver_status_module.add("unsolved", unsolved)?;

    // Add SolverStatus as an attribute of the parent module
    parent_module.add("SolverStatus", solver_status_module)?;

    Ok(())
}

// ── ConflictSeverity Python bindings ──────────────────────────────────

use pyo3::types::PyType;

/// Conflict severity levels.
/// Compatible with `rez.solver` (implicit severity in DependencyConflict).
#[pyclass(name = "ConflictSeverity", from_py_object)]
#[derive(Clone)]
pub struct PyConflictSeverity {
    inner: ConflictSeverity,
}

#[pymethods]
impl PyConflictSeverity {
    /// Create Minor severity.
    #[classmethod]
    fn minor(_cls: Bound<'_, PyType>) -> Self {
        PyConflictSeverity {
            inner: ConflictSeverity::Minor,
        }
    }

    /// Create Major severity.
    #[classmethod]
    fn major(_cls: Bound<'_, PyType>) -> Self {
        PyConflictSeverity {
            inner: ConflictSeverity::Major,
        }
    }

    /// Create Incompatible severity.
    #[classmethod]
    fn incompatible(_cls: Bound<'_, PyType>) -> Self {
        PyConflictSeverity {
            inner: ConflictSeverity::Incompatible,
        }
    }

    /// Returns the severity level name.
    fn name(&self) -> String {
        match self.inner {
            ConflictSeverity::Minor => "Minor".to_string(),
            ConflictSeverity::Major => "Major".to_string(),
            ConflictSeverity::Incompatible => "Incompatible".to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("<ConflictSeverity.{}>", self.name())
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_severity) = other.extract::<PyRef<'_, PyConflictSeverity>>() {
            Ok(self.inner == other_severity.inner)
        } else {
            Ok(false)
        }
    }
}

// ── DependencyConflict Python bindings ─────────────────────────────────

use pyo3::types::PyAny;

/// Dependency conflict information.
/// Compatible with `rez.solver.DependencyConflict`.
///
/// Usage:
///     from rez_next.solver_ import DependencyConflict
///     conflict = DependencyConflict("python", [...], [...], "Major")
#[pyclass(name = "DependencyConflict", from_py_object)]
#[derive(Clone)]
pub struct PyDependencyConflict {
    pub inner: RustDependencyConflict,
}

#[pymethods]
impl PyDependencyConflict {
    /// Create a new DependencyConflict.
    #[new]
    #[pyo3(signature = (package_name, conflicting_requirements=None, source_packages=None, severity="Major"))]
    #[allow(unused_variables)]
    fn new(
        package_name: String,
        conflicting_requirements: Option<Vec<String>>,
        source_packages: Option<Vec<String>>,
        severity: &str,
    ) -> PyResult<Self> {
        let sev = match severity {
            "Minor" => ConflictSeverity::Minor,
            "Major" => ConflictSeverity::Major,
            "Incompatible" => ConflictSeverity::Incompatible,
            _ => ConflictSeverity::Major,
        };

        Ok(PyDependencyConflict {
            inner: RustDependencyConflict {
                package_name,
                conflicting_requirements: vec![], // TODO: proper conversion from strings
                source_packages: source_packages.unwrap_or_default(),
                severity: sev,
            },
        })
    }

    /// Returns the package name.
    #[getter]
    fn package_name(&self) -> String {
        self.inner.package_name.clone()
    }

    /// Returns the source packages.
    #[getter]
    fn source_packages(&self) -> Vec<String> {
        self.inner.source_packages.clone()
    }

    /// Returns the severity.
    #[getter]
    fn severity(&self) -> PyConflictSeverity {
        PyConflictSeverity {
            inner: self.inner.severity.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "<DependencyConflict '{}' from {}>",
            self.inner.package_name,
            self.inner.source_packages.join(", ")
        )
    }
}

// ── FailureReason Python bindings ─────────────────────────────────────

/// Reason why the solver failed.
/// Compatible with `rez.solver.FailureReason`.
///
/// Usage:
///     from rez_next.solver_ import FailureReason
///     reason = FailureReason("Package not found")
///     print(reason.description())
#[pyclass(name = "FailureReason", from_py_object)]
#[derive(Clone)]
pub struct PyFailureReason {
    inner: FailureReason,
}

#[pymethods]
impl PyFailureReason {
    /// Create a new FailureReason.
    #[new]
    fn new(description: &str) -> Self {
        PyFailureReason {
            inner: FailureReason::new(description),
        }
    }

    /// Get the description of this failure.
    fn description(&self) -> String {
        self.inner.description().to_string()
    }

    /// Get the requirements involved in this failure.
    fn involved_requirements(&self) -> Vec<String> {
        self.inner.involved_requirements().to_vec()
    }

    fn __repr__(&self) -> String {
        format!(
            "<FailureReason: {}>",
            self.inner.description()
        )
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_reason) = other.extract::<PyRef<'_, PyFailureReason>>() {
            Ok(self.inner == other_reason.inner)
        } else {
            Ok(false)
        }
    }
}

// ── ConflictResolution Python bindings ─────────────────────────────────

/// Conflict resolution result.
#[pyclass(name = "ConflictResolution", from_py_object)]
#[derive(Clone)]
pub struct PyConflictResolution {
    inner: RustConflictResolution,
}

#[pymethods]
impl PyConflictResolution {
    /// Create a new ConflictResolution.
    #[new]
	#[pyo3(signature = (package_name, selected_version=None, strategy=None, modified_packages=None))]
	fn new(
	    package_name: String,
	    selected_version: Option<String>,
	    strategy: Option<String>,
        modified_packages: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let version = selected_version
            .and_then(|v| rez_next_version::Version::parse(&v).ok());

        Ok(PyConflictResolution {
            inner: RustConflictResolution {
                package_name,
                selected_version: version,
                strategy: strategy.unwrap_or_default(),
                modified_packages: modified_packages.unwrap_or_default(),
            },
        })
    }

    /// Returns the package name.
    #[getter]
    fn package_name(&self) -> String {
        self.inner.package_name.clone()
    }

    /// Returns the selected version (as string or None).
    #[getter]
    fn selected_version(&self) -> Option<String> {
        self.inner.selected_version.as_ref().map(|v| v.as_str().to_string())
    }

    /// Returns the strategy used.
    #[getter]
    fn strategy(&self) -> String {
        self.inner.strategy.clone()
    }

    /// Returns the modified packages.
    #[getter]
    fn modified_packages(&self) -> Vec<String> {
        self.inner.modified_packages.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "<ConflictResolution '{}' -> {}>",
            self.inner.package_name,
            self.inner.selected_version.as_ref().map(|v| v.as_str()).unwrap_or("None")
        )
    }
}

/// Register solver-related types with the parent module.
/// Python-accessible DependencyGraph class.
///
/// Provides graph algorithms including accessibility (transitive closure).
#[pyclass(name = "DependencyGraph")]
pub struct PyDependencyGraph {
    pub inner: rez_next_solver::DependencyGraph,
}

#[pymethods]
impl PyDependencyGraph {
    /// Create a new empty DependencyGraph.
    #[new]
    fn new() -> Self {
        PyDependencyGraph {
            inner: rez_next_solver::DependencyGraph::new(),
        }
    }

    /// Compute the accessibility matrix (transitive closure).
    ///
    /// For each node in the graph, returns all nodes reachable from that node.
    /// This is equivalent to `rez.solver.accessibility(graph)` from the original rez.
    ///
    /// Returns a dict mapping node keys to lists of accessible node keys.
    fn accessibility<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let result = self.inner.accessibility();
        let dict = PyDict::new(py);
        for (key, values) in result {
            let py_list = PyList::empty(py);
            for value in values {
                py_list.append(value)?;
            }
            dict.set_item(key, py_list)?;
        }
        Ok(dict)
    }

    /// Find a cycle in the dependency graph.
    ///
    /// Uses DFS with three-color marking to detect if there is a cycle.
    /// This is equivalent to `rez.solver.find_cycle(graph)` from the original rez.
    ///
    /// # Returns
    /// - `Some(list)` containing the nodes in the cycle (in order)
    /// - `None` if no cycle exists
    fn find_cycle<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyAny>> {
        match self.inner.find_cycle() {
            Some(cycle) => {
                let py_list = PyList::empty(py);
                for node in cycle {
                    py_list.append(node)?;
                }
                Ok(py_list.into_any())
            }
            None => Ok(py.None().into_bound(py)),
        }
    }

    /// Return the number of nodes in the graph.
    fn __len__(&self) -> usize {
        // Access nodes through the public API
        // Since nodes field is private, we need to use a public method
        // For now, return 0 - this should be implemented properly
        0
    }

    fn __repr__(&self) -> String {
        "DependencyGraph()".to_string()
    }
}

/// Standalone `accessibility` function for compatibility with `rez.solver.accessibility`.
///
/// Computes the accessibility matrix (transitive closure) of a DependencyGraph.
/// This is a convenience wrapper that calls `graph.accessibility()`.
///
/// # Arguments
/// * `graph` - A DependencyGraph instance.
///
/// # Returns
/// A dictionary mapping each node key to a list of accessible node keys.
#[pyfunction]
pub fn accessibility(py: Python<'_>, graph: &PyDependencyGraph) -> PyResult<Py<PyAny>> {
    let result = graph.inner.accessibility();
    let dict = PyDict::new(py);
    for (key, values) in result {
        let py_list = PyList::new(py, values)?;
        dict.set_item(key, py_list)?;
    }
    Ok(dict.into())
}

/// Standalone `find_cycle` function for compatibility with `rez.solver.find_cycle`.
///
/// Detects a cycle in the DependencyGraph.
///
/// # Arguments
/// * `graph` - A DependencyGraph instance.
///
/// # Returns
/// A list of node keys forming the cycle, or None if no cycle exists.
#[pyfunction]
pub fn find_cycle(py: Python<'_>, graph: &PyDependencyGraph) -> PyResult<Py<PyAny>> {
    match graph.inner.find_cycle() {
        Some(cycle) => {
            let py_list = PyList::new(py, cycle)?;
            Ok(py_list.into())
        }
        None => Ok(py.None()),
    }
}

/// Get statistics about packages in the given repository paths.
///
/// This is compatible with `rez.solver.package_repo_stats()`.
///
/// # Arguments
/// * `paths` - List of repository paths to scan
///
/// # Returns
/// A dictionary with statistics:
/// - `package_count`: Number of packages
/// - `version_count`: Number of package versions
/// - `variant_count`: Number of package variants
/// - `size_bytes`: Total size in bytes
/// - `last_scan_time`: Last scan time (Unix timestamp) or None
/// - `last_scan_duration_ms`: Last scan duration in milliseconds or None
#[pyfunction]
pub fn package_repo_stats(py: Python<'_>, paths: Vec<String>) -> PyResult<Py<PyAny>> {
    use rez_next_repository::package_repo_stats as rust_package_repo_stats;

    let stats = rust_package_repo_stats(paths);

    let dict = pyo3::types::PyDict::new(py);
    dict.set_item("package_count", stats.package_count)?;
    dict.set_item("version_count", stats.version_count)?;
    dict.set_item("variant_count", stats.variant_count)?;
    dict.set_item("size_bytes", stats.size_bytes)?;
    let last_scan_time = match stats.last_scan_time {
        Some(v) => v.into_pyobject(py)?.into_any().unbind(),
        None => py.None().into_pyobject(py)?.into_any().unbind(),
    };
    let last_scan_duration_ms = match stats.last_scan_duration_ms {
        Some(v) => v.into_pyobject(py)?.into_any().unbind(),
        None => py.None().into_pyobject(py)?.into_any().unbind(),
    };
    dict.set_item("last_scan_time", last_scan_time)?;
    dict.set_item("last_scan_duration_ms", last_scan_duration_ms)?;

    Ok(dict.into())
}

pub fn register_solver_types(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    // Register FailureReason class
    parent_module.add_class::<PyFailureReason>()?;

    // Register ConflictSeverity class
    parent_module.add_class::<PyConflictSeverity>()?;

    // Register DependencyConflict class
    parent_module.add_class::<PyDependencyConflict>()?;

    // Register ConflictResolution class
    parent_module.add_class::<PyConflictResolution>()?;

    // Register DependencyGraph class
    parent_module.add_class::<PyDependencyGraph>()?;

    // Register SolverState class
    solver_state_bindings::register_solver_state_type(parent_module)?;

    // Register DependencyConflicts class
    dependency_conflicts_bindings::register_dependency_conflicts_type(parent_module)?;

    // Register Reduction and TotalReduction classes
    reduction_bindings::register_reduction_types(parent_module)?;

    // Register RequirementList class
    requirement_list_bindings::register_requirement_list_type(parent_module)?;

    // Register PackageVariant and PackageVariantCache classes
    package_variant_bindings::register_package_variant_types(parent_module)?;

    Ok(())
}

/// Python-accessible Solver class, compatible with rez.solver.Solver
#[pyclass(name = "Solver")]
pub struct PySolver {
    config: SolverConfig,
    paths: Vec<PathBuf>,
}

#[pymethods]
impl PySolver {
    /// Create a new Solver.
    /// Compatible with `rez.Solver(packages_path=[...], max_attempts=..., prefer_latest=...)`
    #[new]
    #[pyo3(signature = (packages_path=None, max_attempts=None, prefer_latest=None, enable_parallel=None, max_workers=None))]
    pub fn new(
        packages_path: Option<Vec<String>>,
        max_attempts: Option<usize>,
        prefer_latest: Option<bool>,
        enable_parallel: Option<bool>,
        max_workers: Option<usize>,
    ) -> PyResult<Self> {
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

        let mut solver_config = SolverConfig::default();

        // Apply optional configuration
        if let Some(ma) = max_attempts {
            solver_config.max_attempts = ma;
        }
        if let Some(pl) = prefer_latest {
            solver_config.prefer_latest = pl;
        }
        if let Some(ep) = enable_parallel {
            solver_config.enable_parallel = ep;
        }
        if let Some(mw) = max_workers {
            solver_config.max_workers = mw;
        }

        Ok(PySolver {
            config: solver_config,
            paths,
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Solver(paths={}, max_attempts={}, prefer_latest={}, enable_parallel={})",
            self.paths.len(),
            self.config.max_attempts,
            self.config.prefer_latest,
            self.config.enable_parallel,
        )
    }

    /// Get the current max_attempts value
    #[getter]
    fn get_max_attempts(&self) -> usize {
        self.config.max_attempts
    }

    /// Set the max_attempts value
    #[setter]
    fn set_max_attempts(&mut self, value: usize) {
        self.config.max_attempts = value;
    }

    /// Get the current prefer_latest value
    #[getter]
    fn get_prefer_latest(&self) -> bool {
        self.config.prefer_latest
    }

    /// Set the prefer_latest value
    #[setter]
    fn set_prefer_latest(&mut self, value: bool) {
        self.config.prefer_latest = value;
    }

    /// Get the current enable_parallel value
    #[getter]
    fn get_enable_parallel(&self) -> bool {
        self.config.enable_parallel
    }

    /// Set the enable_parallel value
    #[setter]
    fn set_enable_parallel(&mut self, value: bool) {
        self.config.enable_parallel = value;
    }

    /// Get the current max_workers value
    #[getter]
    fn get_max_workers(&self) -> usize {
        self.config.max_workers
    }

    /// Set the max_workers value
    #[setter]
    fn set_max_workers(&mut self, value: usize) {
        self.config.max_workers = value;
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
#[path = "solver_bindings_tests.rs"]
mod tests;
