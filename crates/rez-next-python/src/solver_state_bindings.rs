//! Python bindings for SolverState
//!

use crate::solver_bindings::PySolverStatusMember;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rez_next_solver::SolverState;

#[pyclass(name = "SolverState", skip_from_py_object)]
#[derive(Clone)]
pub struct PySolverState {
    pub(crate) inner: SolverState,
}

#[pymethods]
impl PySolverState {
    #[getter]
    fn status(&self) -> PySolverStatusMember {
        PySolverStatusMember {
            inner: self.inner.status,
        }
    }

    #[getter]
    fn resolved_count(&self) -> usize {
        self.inner.resolved_count()
    }

    #[getter]
    fn failed_count(&self) -> usize {
        self.inner.failed_count()
    }

    #[getter]
    fn packages_considered(&self) -> usize {
        self.inner.packages_considered
    }

    #[getter]
    fn variants_evaluated(&self) -> usize {
        self.inner.variants_evaluated
    }

    #[getter]
    fn backtrack_steps(&self) -> usize {
        self.inner.backtrack_steps
    }

    #[getter]
    fn resolution_time_ms(&self) -> u64 {
        self.inner.resolution_time_ms
    }

    #[getter]
    fn metadata<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        let meta = self.inner.metadata.clone();
        let py_dict = PyDict::new(py);
        for (k, v) in meta {
            py_dict.set_item(k, v)?;
        }
        Ok(py_dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<SolverState status={:?} resolved={} failed={}>",
            self.inner.status,
            self.inner.resolved_count(),
            self.inner.failed_count()
        )
    }
}

/// Register SolverState type with the parent module
pub fn register_solver_state_type(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PySolverState>()?;
    Ok(())
}
