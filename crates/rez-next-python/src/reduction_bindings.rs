//! Python bindings for Reduction and TotalReduction
//!
//! Defines `PyReduction` and `PyTotalReduction`, mapping
//! `rez_next_solver::{Reduction, TotalReduction}`.

use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_solver::{Reduction, TotalReduction};

#[pyclass(name = "Reduction", from_py_object)]
#[derive(Clone)]
pub struct PyReduction {
    inner: Reduction,
}

#[pymethods]
impl PyReduction {
    #[new]
    fn new(package_name: String, version: Option<String>, reason: String) -> Self {
        PyReduction {
            inner: Reduction::new(package_name, version, reason),
        }
    }

    #[getter]
    fn package_name(&self) -> String {
        self.inner.package_name.clone()
    }

    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    #[getter]
    fn reason(&self) -> String {
        self.inner.reason.clone()
    }

    #[getter]
    fn timestamp(&self) -> u64 {
        self.inner.timestamp
    }

    fn __repr__(&self) -> String {
        format!(
            "<Reduction package={} version={:?} reason={}>",
            self.inner.package_name, self.inner.version, self.inner.reason
        )
    }
}

#[pyclass(name = "TotalReduction", from_py_object)]
#[derive(Clone)]
pub struct PyTotalReduction {
    inner: TotalReduction,
}

#[pymethods]
impl PyTotalReduction {
    #[new]
    fn new() -> Self {
        PyTotalReduction {
            inner: TotalReduction::new(),
        }
    }

    fn add_reduction(&mut self, reduction: PyReduction) {
        self.inner.add_reduction(reduction.inner);
    }

    #[getter]
    fn reductions<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py_list = PyList::empty(py);
        for r in &self.inner.reductions {
            let py_r = PyReduction { inner: r.clone() };
            py_list.append(py_r)?;
        }
        Ok(py_list.into_any())
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[getter]
    fn total_count(&self) -> usize {
        self.inner.total_count
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!("<TotalReduction count={}>", self.inner.total_count)
    }
}

/// Register Reduction and TotalReduction types with the parent module
pub fn register_reduction_types(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PyReduction>()?;
    parent_module.add_class::<PyTotalReduction>()?;
    Ok(())
}
