//! Python bindings for deprecations module
//!
//! This module provides Python bindings for Rez deprecation warnings.
//!
//! Corresponds to rez's deprecations.py.

use pyo3::prelude::*;

/// Rez deprecation warning class
///
/// This is a simple warning class that can be used with Python's `warnings` module.
#[pyclass(name = "RezDeprecationWarning", skip_from_py_object)]
#[derive(Clone)]
pub struct PyRezDeprecationWarning;

#[pymethods]
impl PyRezDeprecationWarning {
    #[new]
    fn new() -> Self {
        Self
    }
}

/// Register the deprecations submodule
pub fn register_deprecations_submodule(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent_module.py(), "deprecations")?;

    // Add RezDeprecationWarning class
    m.add_class::<PyRezDeprecationWarning>()?;

    // Register in sys.modules for `from rez_next.deprecations import ...`
    let sys = parent_module.py().import("sys")?;
    let modules = sys.getattr("modules")?;
    let parent_name = parent_module.name()?;
    let full_name = format!("{}.{}", parent_name, "deprecations");
    modules.set_item(full_name.as_str(), &m)?;

    // Also register as submodule
    crate::register_submodule(parent_module, "deprecations", &m)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecations_module_creation() {
        // Just verify the struct can be created
        let _warning = PyRezDeprecationWarning::new();
    }
}
