//! Python bindings for RequirementList
//!
//! Defines `PyRequirementList`, mapping `rez_next_solver::RequirementList`.

use crate::package_bindings::PyPackageRequirement;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_solver::RequirementList;

#[pyclass(name = "RequirementList", from_py_object)]
#[derive(Clone)]
pub struct PyRequirementList {
    inner: RequirementList,
}

#[pymethods]
impl PyRequirementList {
    /// Create a new empty RequirementList.
    #[new]
    fn new() -> Self {
        PyRequirementList {
            inner: RequirementList::new(),
        }
    }

    /// Add a requirement to the list.
    fn add_requirement(&mut self, requirement: PyPackageRequirement) {
        self.inner.add_requirement(requirement.0);
    }

    /// Remove all requirements for a package.
    fn remove_requirements(&mut self, package_name: &str) {
        self.inner.remove_requirements(package_name);
    }

    /// Get all requirements for a package.
    fn get_requirements<'py>(&self, py: Python<'py>, package_name: &str) -> PyResult<Bound<'py, PyAny>> {
        let requirements = self.inner.get_requirements(package_name);
        let py_list = PyList::empty(py);
        for req in requirements {
            let py_req = PyPackageRequirement(req.clone());
            py_list.append(py_req)?;
        }
        Ok(py_list.into_any())
    }

    /// Check if the list is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of packages with requirements.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "<RequirementList packages={}>",
            self.inner.len()
        )
    }
}

/// Register RequirementList type with the parent module
pub fn register_requirement_list_type(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PyRequirementList>()?;
    Ok(())
}
