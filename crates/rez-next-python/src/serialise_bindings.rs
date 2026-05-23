//! Python bindings for package serialisation.
//!
//! Exposes `rez_next.serialise_` submodule with package serialisation functions.
//!
//! This module aligns with Rez's `package_serialise.py` interface:
//! - `dump_package_data(data, buf, format_, skip_attributes)`
//! - Supports file-like objects (SupportsWrite protocol)
//! - Validates data against package_serialise_schema

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyModule};
use serde_json::Value;

use rez_next_serialise::{
    FileFormat, PackageSerialiseError, as_block_string, dict_to_attributes_code, dump_yaml,
    package_key_order, read_package_data,
};

/// Convert a Python object to serde_json::Value using Python's json module.
fn python_to_json(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Value> {
    let json_module = PyModule::import(py, "json")?;
    let dumps_fn = json_module.getattr("dumps")?;
    let json_str: String = dumps_fn.call1((data,))?.extract()?;
    serde_json::from_str(&json_str)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Dump package data to a file-like object (SupportsWrite protocol).
///
/// This function aligns with Rez's `package_serialise.dump_package_data` interface.
///
/// Args:
///     data: Package data (dict) - must conform to package_serialise_schema
///     buf: File-like object with write() method (SupportsWrite protocol)
///     format_: File format ("py", "yaml") - "txt" is not supported
///     skip_attributes: Optional list of attribute names to skip
#[pyfunction]
#[pyo3(signature = (data, buf, format_, skip_attributes=None))]
fn py_dump_package_data(
    py: Python<'_>,
    data: Bound<'_, PyAny>,
    buf: Bound<'_, PyAny>,
    format_: String,
    skip_attributes: Option<Vec<String>>,
) -> PyResult<()> {
    // Check if format_ is "txt" (not supported, align with Rez)
    if format_.to_lowercase() == "txt" {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "'txt' format is not supported for package definition export".to_string(),
        ));
    }

    // Convert data to JSON Value
    let mut json_value = python_to_json(py, &data)?;

    // Apply skip_attributes filter
    if let Some(ref skip) = skip_attributes {
        if let Value::Object(ref mut map) = json_value {
            for key in skip {
                map.remove(key.as_str());
            }
        }
    }

    // Validate against schema (align with Rez's package_serialise_schema)
    validate_package_data(&json_value)
        .map_err(PyErr::new::<pyo3::exceptions::PyValueError, _>)?;

    // Serialise data to string
    let serialised = serialise_to_string(&json_value, &format_)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    // Write to buffer (SupportsWrite protocol)
    let write_method = buf.getattr("write")?;
    let py_bytes = PyBytes::new(py, serialised.as_bytes());
    write_method.call1((py_bytes,))?;

    Ok(())
}

/// Validate package data against schema.
///
/// This aligns with Rez's package_serialise_schema validation.
fn validate_package_data(data: &Value) -> std::result::Result<(), String> {
    // Basic validation: 'name' is required (align with Rez's package_serialise_schema)
    if let Value::Object(map) = data {
        if !map.contains_key("name") {
            return Err("Missing required field: 'name'".to_string());
        }

        // Validate name is a string
        if let Some(name) = map.get("name") {
            if !name.is_string() {
                return Err("Field 'name' must be a string".to_string());
            }
        }

        // TODO: Add more validation to fully align with Rez's package_serialise_schema
        // - version should be a string or Version object
        // - requires should be a list of strings
        // - tests should conform to tests_schema
        // - etc.
    }

    Ok(())
}

/// Serialise data to string based on format.
fn serialise_to_string(
    data: &Value,
    format_: &str,
) -> std::result::Result<String, PackageSerialiseError> {
    match format_.to_lowercase().as_str() {
        "py" | "python" => dict_to_attributes_code(data),
        "yaml" | "yml" => dump_yaml(data),
        "json" => serde_json::to_string_pretty(data)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string())),
        _ => Err(PackageSerialiseError::UnsupportedFormat(
            // Convert format_ string to FileFormat enum
            match format_.to_lowercase().as_str() {
                "py" | "python" => FileFormat::Python,
                "yaml" | "yml" => FileFormat::Yaml,
                "json" => FileFormat::Json,
                _ => FileFormat::Yaml, // Default fallback
            },
        )),
    }
}

/// Read package data from a file.
///
/// Args:
///     path: File path to read from
///     format: File format string ("yaml", "json", "python", "toml")
///
/// Returns:
///     Package data (dict)
#[pyfunction]
#[pyo3(signature = (path, format))]
fn py_read_package_data(
    py: Python<'_>,
    path: String,
    format: String,
) -> PyResult<Bound<'_, PyAny>> {
    let file_format = parse_format(&format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    let path_obj = std::path::Path::new(&path);

    let json_value: serde_json::Value = read_package_data(path_obj, file_format)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

    // Convert JSON value to Python object
    let json_str = serde_json::to_string(&json_value)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;

    let json_module = PyModule::import(py, "json")?;
    let loads_fn = json_module.getattr("loads")?;
    let py_obj = loads_fn.call1((json_str,))?;

    Ok(py_obj)
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
///
/// This aligns with Rez's `package_key_order` constant.
#[pyfunction]
fn py_package_key_order() -> Vec<&'static str> {
    package_key_order()
}

/// Register the `serialise_` submodule.
pub fn register_serialise_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "serialise_")?;

    m.add_function(wrap_pyfunction!(py_dump_package_data, &m)?)?;
    m.add_function(wrap_pyfunction!(py_read_package_data, &m)?)?;
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
        "txt" => Err(PackageSerialiseError::UnsupportedFormat(FileFormat::Yaml)), // Align with Rez: txt not supported
        _ => Err(PackageSerialiseError::UnsupportedFormat(FileFormat::Yaml)),
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
        assert!(parse_format("txt").is_err()); // Align with Rez: txt not supported
    }
}
