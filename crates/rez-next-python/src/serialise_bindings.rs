//! Python bindings for package serialisation.
//!
//! Exposes `rez_next.serialise_` submodule with package serialisation functions.

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};
use serde_json::Value;

use rez_next_serialise::{
    as_block_string, dict_to_attributes_code, dump_package_data, dump_yaml,
    package_key_order, FileFormat, PackageSerialiseError,
};

/// Convert a Python object to serde_json::Value using Python's json module.
fn python_to_json(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Value> {
    let json_module = PyModule::import(py, "json")?;
    let dumps_fn = json_module.getattr("dumps")?;
    let json_str: String = dumps_fn.call1((data,))?.extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Dump package data to a file.
///
/// Args:
///     data: Package data (dict)
///     path: File path to write to
///     format: File format string ("yaml", "json", "python", "toml")
#[pyfunction]
#[pyo3(signature = (data, path, format))]
fn py_dump_package_data(
    py: Python<'_>,
    data: Bound<'_, PyAny>,
    path: String,
    format: String,
) -> PyResult<()> {
    let json_value = python_to_json(py, &data)?;

    let file_format = parse_format(&format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    let path_obj = std::path::Path::new(&path);

    dump_package_data(&json_value, path_obj, file_format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

    Ok(())
}

/// Dump data to a YAML string.
///
/// Args:
///     data: Data to serialise (dict)
///
/// Returns:
///     YAML string
#[pyfunction]
#[pyo3(signature = (data))]
fn py_dump_yaml(py: Python<'_>, data: Bound<'_, PyAny>) -> PyResult<String> {
    let json_value = python_to_json(py, &data)?;

    dump_yaml(&json_value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Format a string as a YAML block string.
///
/// Args:
///     s: String to format
///     indent: Indentation level (default: 0)
///
/// Returns:
///     Formatted block string
#[pyfunction]
#[pyo3(signature = (s, indent=0))]
fn py_as_block_string(s: String, indent: usize) -> String {
    as_block_string(&s, indent)
}

/// Convert a dict to Python attribute code (for package.py files).
///
/// Args:
///     data: Data to convert (dict)
///
/// Returns:
///     Python code string
#[pyfunction]
#[pyo3(signature = (data))]
fn py_dict_to_attributes_code(py: Python<'_>, data: Bound<'_, PyAny>) -> PyResult<String> {
    let json_value = python_to_json(py, &data)?;

    dict_to_attributes_code(&json_value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Get the standard package key order.
///
/// Returns:
///     List of key names in standard order
#[pyfunction]
fn py_package_key_order() -> Vec<&'static str> {
    package_key_order()
}

/// Register the `serialise_` submodule.
pub fn register_serialise_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "serialise_")?;

    m.add_function(wrap_pyfunction!(py_dump_package_data, &m)?)?;
    m.add_function(wrap_pyfunction!(py_dump_yaml, &m)?)?;
    m.add_function(wrap_pyfunction!(py_as_block_string, &m)?)?;
    m.add_function(wrap_pyfunction!(py_dict_to_attributes_code, &m)?)?;
    m.add_function(wrap_pyfunction!(py_package_key_order, &m)?)?;

    // Add FileFormat enum as class
    m.add_class::<PyFileFormat>()?;

    // Register submodule
    super::register_submodule(parent, "serialise_", &m)?;

    Ok(())
}

/// Python class for FileFormat enum.
#[pyclass(name = "FileFormat")]
struct PyFileFormat;

#[pymethods]
impl PyFileFormat {
    #[classattr]
    fn yaml() -> &'static str {
        "yaml"
    }

    #[classattr]
    fn json() -> &'static str {
        "json"
    }

    #[classattr]
    fn python() -> &'static str {
        "python"
    }

    #[classattr]
    fn toml() -> &'static str {
        "toml"
    }
}

/// Parse format string to FileFormat enum.
fn parse_format(s: &str) -> std::result::Result<FileFormat, PackageSerialiseError> {
    match s.to_lowercase().as_str() {
        "yaml" | "yml" => Ok(FileFormat::Yaml),
        "json" => Ok(FileFormat::Json),
        "python" | "py" => Ok(FileFormat::Python),
        "toml" => Ok(FileFormat::Toml),
        _ => Err(PackageSerialiseError::UnsupportedFormat(
            FileFormat::Yaml, // Placeholder
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_format() {
        assert!(parse_format("yaml").is_ok());
        assert!(parse_format("json").is_ok());
        assert!(parse_format("python").is_ok());
        assert!(parse_format("toml").is_ok());
    }
}
