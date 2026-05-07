//! Python bindings for utility functions

use pyo3::prelude::*;

/// Get the rez-next version
#[pyfunction]
fn get_rez_next_version_py() -> String {
    rez_next_util::get_rez_next_version().to_string()
}

/// Get the current platform name
#[pyfunction]
fn get_platform_name() -> String {
    rez_next_util::get_platform().to_string()
}

/// Get the platform architecture
#[pyfunction]
fn get_architecture_py() -> String {
    rez_next_util::get_architecture().to_string()
}

/// Get the platform ID (platform-architecture)
#[pyfunction]
fn get_platform_id_py() -> String {
    rez_next_util::get_platform_id()
}

/// Check if running on Windows
#[pyfunction]
fn is_windows_py() -> bool {
    rez_next_util::is_windows()
}

/// Check if running on Linux
#[pyfunction]
fn is_linux_py() -> bool {
    rez_next_util::is_linux()
}

/// Check if running on macOS
#[pyfunction]
fn is_macos_py() -> bool {
    rez_next_util::is_macos()
}

/// Check if running on Unix-like system
#[pyfunction]
fn is_unix_py() -> bool {
    rez_next_util::is_unix()
}

/// Normalize a package name
#[pyfunction]
fn normalize_name_py(name: &str) -> String {
    rez_next_util::normalize_name(name)
}

/// Truncate a string
#[pyfunction]
fn truncate_string_py(s: &str, max_len: usize) -> String {
    rez_next_util::truncate(s, max_len)
}

/// Get the current executable name
#[pyfunction]
fn get_executable_name_py() -> PyResult<String> {
    rez_next_util::get_executable_name()
        .map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!(
                "Failed to get executable name: {:?}",
                e
            ))
        })
}

/// Register the util module
pub fn register_util_submodule(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let util_module = PyModule::new(py, "util")?;

    // Add functions
    util_module.add_function(wrap_pyfunction!(get_rez_next_version_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_platform_name, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_architecture_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_platform_id_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_windows_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_linux_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_macos_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_unix_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(normalize_name_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(truncate_string_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_executable_name_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(which_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(which_all_py, &util_module)?)?;

    // Register in parent module
    parent.add_submodule(&util_module)?;

    // Register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("rez_next._native.util", &util_module)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_rez_next_version() {
        let result = get_rez_next_version_py();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_platform_detection() {
        let windows = is_windows_py();
        let linux = is_linux_py();
        let macos = is_macos_py();

        // At least one should be true
        assert!(windows || linux || macos);
    }

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name_py("Hello World"), "hello_world");
        assert_eq!(normalize_name_py("maya-2024"), "maya_2024");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string_py("hello", 10), "hello");
        assert_eq!(truncate_string_py("hello world", 8), "hello...");
    }
}

/// Find an executable in PATH (like Unix `which` or Windows `where`)
#[pyfunction]
fn which_py(command: &str) -> Option<String> {
    rez_next_util::which(command).map(|p| p.to_string_lossy().to_string())
}

/// Find all executables with given name in PATH
#[pyfunction]
fn which_all_py(command: &str) -> Vec<String> {
    rez_next_util::which_all(command)
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}
