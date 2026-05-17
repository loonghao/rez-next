//!
//! Python bindings for `package_py_utils` module.
//!
//! Exposes `expand_requirement` and `expand_requirements` to Python.

use pyo3::prelude::*;
use pyo3::types::PyAny;

/// Expand a requirement string with wildcards.
///
/// # Arguments
/// - `request`: Requirement string (e.g., "python-2.*", "boost-1.**")
/// - `query_func`: Optional Python callable that takes (name, version_spec) and returns version string
#[pyfunction]
#[pyo3(signature = (request, query_func=None))]
fn expand_requirement<'py>(
    _py: Python<'py>,
    request: &str,
    query_func: Option<Bound<'py, PyAny>>,
) -> PyResult<String> {
    if !request.contains('*') {
        return Ok(request.to_string());
    }

    let query_callback = |name: &str, _version_spec: Option<&str>| -> Option<String> {
        if let Some(ref func) = query_func {
            let args = (name.to_string(),);
            match func.call1(args) {
                Ok(result) => result.extract::<String>().ok(),
                Err(_) => None,
            }
        } else {
            None
        }
    };

    match rez_next_package::expand_requirement(request, &query_callback) {
        Ok(expanded) => Ok(expanded),
        Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e)),
    }
}

/// Expand multiple requirement strings.
#[pyfunction]
#[pyo3(signature = (requests, query_func=None))]
fn expand_requirements<'py>(
    _py: Python<'py>,
    requests: Vec<String>,
    query_func: Option<Bound<'py, PyAny>>,
) -> PyResult<Vec<String>> {
    let query_callback = |name: &str, _version_spec: Option<&str>| -> Option<String> {
        if let Some(ref func) = query_func {
            let args = (name.to_string(),);
            match func.call1(args) {
                Ok(result) => result.extract::<String>().ok(),
                Err(_) => None,
            }
        } else {
            None
        }
    };

    let result: Vec<String> = requests
        .iter()
        .map(|r| {
            rez_next_package::expand_requirement(r, &query_callback)
                .unwrap_or_else(|_| r.to_string())
        })
        .collect();

    Ok(result)
}

/// Register the `package_py_utils` submodule.
pub fn register_package_py_utils_submodule(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "package_py_utils")?;

    // Add functions to submodule
    m.add_function(wrap_pyfunction!(expand_requirement, &m)?)?;
    m.add_function(wrap_pyfunction!(expand_requirements, &m)?)?;

    // Register submodule in parent and sys.modules
    super::register_submodule(parent, "package_py_utils", &m)?;

    Ok(())
}
