//! Python bindings for DependencyConflicts
//!

use crate::solver_bindings::PyDependencyConflict;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_solver::DependencyConflicts;

#[pyclass(name = "DependencyConflicts", skip_from_py_object)]
#[derive(Clone)]
pub struct PyDependencyConflicts {
    inner: DependencyConflicts,
}

#[pymethods]
impl PyDependencyConflicts {
    /// Create a new empty DependencyConflicts.
    #[new]
    fn new() -> Self {
        PyDependencyConflicts {
            inner: DependencyConflicts::new(),
        }
    }

    /// Add a conflict to the collection.
    fn add_conflict(&mut self, conflict: PyDependencyConflict) {
        self.inner.add_conflict(conflict.inner.clone());
    }

    /// Remove all conflicts for a package.
    fn remove_conflicts(&mut self, package_name: &str) {
        self.inner.remove_conflicts(package_name);
    }

    /// Get all conflicts for a package.
    fn get_conflicts<'a>(&self, py: Python<'a>, package_name: &str) -> PyResult<Bound<'a, PyList>> {
        let conflicts = self.inner.get_conflicts(package_name);
        let py_list = PyList::empty(py);
        for conflict in conflicts {
            let py_conflict = PyDependencyConflict {
                inner: conflict.clone(),
            };
            py_list.append(py_conflict)?;
        }
        Ok(py_list)
    }

    /// Check if the collection is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of packages with conflicts.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!("<DependencyConflicts packages={}>", self.inner.len())
    }
}

/// Register DependencyConflicts type with the parent module
pub fn register_dependency_conflicts_type(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PyDependencyConflicts>()?;
    Ok(())
}
