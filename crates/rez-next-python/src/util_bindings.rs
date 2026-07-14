//! Python bindings for utility functions

use pyo3::prelude::*;

/// Get the rez-next version
#[pyfunction(name = "get_rez_next_version")]
fn get_rez_next_version_py() -> String {
    rez_next_util::get_rez_next_version().to_string()
}

/// Get the current platform name
#[pyfunction(name = "get_platform_name")]
fn get_platform_name() -> String {
    rez_next_util::get_platform().to_string()
}

/// Get the platform architecture
#[pyfunction(name = "get_architecture")]
fn get_architecture_py() -> String {
    rez_next_util::get_architecture().to_string()
}

/// Get the platform ID (platform-architecture)
#[pyfunction(name = "get_platform_id")]
fn get_platform_id_py() -> String {
    rez_next_util::get_platform_id()
}

/// Check if running on Windows
#[pyfunction(name = "is_windows")]
fn is_windows_py() -> bool {
    rez_next_util::is_windows()
}

/// Check if running on Linux
#[pyfunction(name = "is_linux")]
fn is_linux_py() -> bool {
    rez_next_util::is_linux()
}

/// Check if running on macOS
#[pyfunction(name = "is_macos")]
fn is_macos_py() -> bool {
    rez_next_util::is_macos()
}

/// Check if running on Unix-like system
#[pyfunction(name = "is_unix")]
fn is_unix_py() -> bool {
    rez_next_util::is_unix()
}

/// Normalize a package name
#[pyfunction(name = "normalize_name")]
fn normalize_name_py(name: &str) -> String {
    rez_next_util::normalize_name(name)
}

/// Truncate a string
#[pyfunction(name = "truncate_string")]
fn truncate_string_py(s: &str, max_len: usize) -> String {
    rez_next_util::truncate(s, max_len)
}

/// Get the current executable name
#[pyfunction(name = "get_executable_name")]
fn get_executable_name_py() -> PyResult<String> {
    rez_next_util::get_executable_name().map_err(|e| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to get executable name: {:?}", e))
    })
}

/// Register the util module
pub fn register_util_submodule(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let util_module = PyModule::new(py, "util")?;

    // Add platform functions
    util_module.add_function(wrap_pyfunction!(get_rez_next_version_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_platform_name, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_architecture_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_platform_id_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_windows_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_linux_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_macos_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_unix_py, &util_module)?)?;

    // Add string functions
    util_module.add_function(wrap_pyfunction!(normalize_name_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(truncate_string_py, &util_module)?)?;

    // Add executable functions
    util_module.add_function(wrap_pyfunction!(get_executable_name_py, &util_module)?)?;

    // Add which functions
    util_module.add_function(wrap_pyfunction!(which_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(which_all_py, &util_module)?)?;

    // Add filesystem functions
    util_module.add_function(wrap_pyfunction!(expand_user_path_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(ensure_dir_exists_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(ensure_parent_dir_exists_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(is_writable_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(safe_remove_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(copy_file_py, &util_module)?)?;

    // Add system functions
    util_module.add_function(wrap_pyfunction!(get_hostname_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_username_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_home_directory_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_fqdn_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(get_domain_py, &util_module)?)?;

    // Add base26 functions
    util_module.add_function(wrap_pyfunction!(get_next_base26_py, &util_module)?)?;
    util_module.add_function(wrap_pyfunction!(
        create_unique_base26_symlink_py,
        &util_module
    )?)?;

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

    #[test]
    fn test_expand_user_path() {
        let result = expand_user_path_py("~/test.txt");
        assert!(result.is_ok());
        let path = result.unwrap();
        // Should not start with ~
        assert!(!path.starts_with('~'));
    }

    #[test]
    fn test_ensure_dir_exists_creates_dir() {
        let dir = std::env::temp_dir().join("rez_next_test_ensure_dir");
        // Clean up first in case previous test run left it
        let _ = std::fs::remove_dir_all(&dir);
        let path_str = dir.to_string_lossy().to_string();
        let result = ensure_dir_exists_py(&path_str);
        assert!(result.is_ok());
        assert!(dir.exists() && dir.is_dir());
        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_writable() {
        // Temp directory should be writable
        let temp = std::env::temp_dir();
        assert!(is_writable_py(temp.to_str().unwrap()));
    }

    #[test]
    fn test_copy_file_creates_copy() {
        let dir = std::env::temp_dir().join("rez_next_test_copy");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src.txt");
        let dst = dir.join("dst.txt");
        std::fs::write(&src, b"hello").unwrap();
        let result = copy_file_py(src.to_str().unwrap(), dst.to_str().unwrap());
        assert!(result.is_ok());
        assert!(dst.exists());
        assert_eq!(std::fs::read_to_string(&dst).unwrap(), "hello");
        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Find an executable in PATH (like Unix `which` or Windows `where`)
#[pyfunction(name = "which")]
fn which_py(command: &str) -> Option<String> {
    rez_next_util::which(command).map(|p| p.to_string_lossy().to_string())
}

/// Find all executables with given name in PATH
#[pyfunction(name = "which_all")]
fn which_all_py(command: &str) -> Vec<String> {
    rez_next_util::which_all(command)
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect()
}

// ─── File System Utilities ─────────────────────────────────────────────

/// Expand a path starting with ~ to the user's home directory
#[pyfunction(name = "expand_user_path")]
fn expand_user_path_py(path: &str) -> PyResult<String> {
    rez_next_util::expand_user_path(path)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Ensure a directory exists, creating it and all parents if necessary
#[pyfunction(name = "ensure_dir_exists")]
fn ensure_dir_exists_py(path: &str) -> PyResult<()> {
    rez_next_util::ensure_dir_exists(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Ensure a file's parent directory exists
#[pyfunction(name = "ensure_parent_dir_exists")]
fn ensure_parent_dir_exists_py(path: &str) -> PyResult<()> {
    rez_next_util::ensure_parent_dir_exists(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Check if a path is writable
#[pyfunction(name = "is_writable")]
fn is_writable_py(path: &str) -> bool {
    rez_next_util::is_writable(path)
}

/// Safely remove a file or directory (recursively for directories)
#[pyfunction(name = "safe_remove")]
fn safe_remove_py(path: &str) -> PyResult<()> {
    rez_next_util::safe_remove(path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Copy a file, creating parent directories if necessary
#[pyfunction(name = "copy_file")]
fn copy_file_py(from: &str, to: &str) -> PyResult<u64> {
    rez_next_util::copy_file(from, to)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

// ─── System Utilities ─────────────────────────────────────────────

/// Get the current machine's hostname
#[pyfunction(name = "get_hostname")]
fn get_hostname_py() -> String {
    rez_next_util::get_hostname()
}

/// Get the current username
#[pyfunction(name = "get_username")]
fn get_username_py() -> String {
    rez_next_util::get_username()
}

/// Get the current user's home directory
#[pyfunction(name = "get_home_directory")]
fn get_home_directory_py() -> Option<String> {
    rez_next_util::get_home_directory().map(|p| p.to_string_lossy().to_string())
}

/// Get the current machine's fully qualified domain name (FQDN)
#[pyfunction(name = "get_fqdn")]
fn get_fqdn_py() -> String {
    rez_next_util::get_fqdn()
}

/// Get the current machine's domain
#[pyfunction(name = "get_domain")]
fn get_domain_py() -> String {
    rez_next_util::get_domain()
}

// ─── Base26 Utilities ─────────────────────────────────────────────

use rez_next_util::Base26Error;

/// Get the next Base26 string in sequence.
///
/// Sequence: a -> b -> ... -> z -> aa -> ab -> ...
///
/// Args:
///     prev: Optional previous Base26 string. If None, returns "a".
///
/// Returns:
///     Next Base26 string in sequence.
///
/// Raises:
///     ValueError: If prev is not a valid Base26 string (must be lowercase a-z only).
#[pyfunction(name = "get_next_base26", signature = (prev = None))]
fn get_next_base26_py(prev: Option<&str>) -> PyResult<String> {
    rez_next_util::get_next_base26(prev).map_err(|e| {
        let err_str = e.to_string();
        match e {
            Base26Error::InvalidBase26(_) => pyo3::exceptions::PyValueError::new_err(err_str),
            _ => pyo3::exceptions::PyRuntimeError::new_err(err_str),
        }
    })
}

/// Create a unique Base26-named symlink in the given directory (Unix-only).
#[cfg(unix)]
#[pyfunction(name = "create_unique_base26_symlink")]
fn create_unique_base26_symlink_py(path: &str, source: &str) -> PyResult<String> {
    rez_next_util::create_unique_base26_symlink(path, source)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| {
            let err_str = e.to_string();
            match e {
                Base26Error::RetryExhausted(_) => {
                    pyo3::exceptions::PyRuntimeError::new_err(err_str)
                }
                _ => pyo3::exceptions::PyIOError::new_err(err_str),
            }
        })
}

/// Stub for non-Unix platforms.
#[cfg(not(unix))]
#[pyfunction(name = "create_unique_base26_symlink")]
fn create_unique_base26_symlink_py(_path: &str, _source: &str) -> PyResult<String> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "create_unique_base26_symlink is only supported on Unix-like systems",
    ))
}
