//! Python bindings for `rez.source` — context activation and source scripts.
//!
//! Equivalent to `rez source` command: writes a shell activation script for a
//! resolved context so users can activate it with `source <script>` or `.` in
//! POSIX shells, or `. <script>` in PowerShell.

use pyo3::prelude::*;
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
use std::path::PathBuf;

/// Supported activation modes matching rez's `rez source` command.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceMode {
    /// Return script content as string (no file I/O)
    Inline,
    /// Write activation script to a temp file and print path
    TempFile,
    /// Write activation script to the path specified
    File(PathBuf),
}

/// Python-exposed source manager — writes shell activation scripts.
///
/// Mirrors `rez source` CLI and `rez.source.SourceManager` Python API.
///
/// ## Usage (Python)
/// ```python
/// from rez_next.source import SourceManager
/// mgr = SourceManager(["python-3.9", "maya-2024"])
/// path = mgr.write_activation_script("/tmp/activate.sh", shell="bash")
/// # user does: source /tmp/activate.sh
/// ```
#[pyclass(name = "SourceManager")]
#[derive(Debug)]
pub struct PySourceManager {
    pub(crate) packages: Vec<String>,
    pub(crate) shell_type: String,
}

#[pymethods]
impl PySourceManager {
    /// Create a new SourceManager for the given package requirements.
    #[new]
    #[pyo3(signature = (packages, shell = None))]
    pub fn new(packages: Vec<String>, shell: Option<String>) -> Self {
        let shell_type = shell.unwrap_or_else(detect_current_shell);
        Self {
            packages,
            shell_type,
        }
    }

    /// Write an activation script to `dest_path`.
    ///
    /// Returns the absolute path of the written file.
    #[pyo3(signature = (dest_path, shell = None))]
    pub fn write_activation_script(
        &self,
        dest_path: &str,
        shell: Option<String>,
    ) -> PyResult<String> {
        let shell_name = shell.as_deref().unwrap_or(&self.shell_type);
        let script = build_activation_script(&self.packages, shell_name);

        let path = PathBuf::from(dest_path);
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            }
        }
        std::fs::write(&path, &script)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        Ok(path
            .canonicalize()
            .unwrap_or(path)
            .to_string_lossy()
            .to_string())
    }

    /// Write activation script to a temp file and return the path.
    #[pyo3(signature = (shell = None))]
    pub fn write_temp_activation_script(&self, shell: Option<String>) -> PyResult<String> {
        let shell_name = shell.as_deref().unwrap_or(&self.shell_type);
        let ext = match shell_name {
            "powershell" | "pwsh" => "ps1",
            "cmd" => "bat",
            _ => "sh",
        };
        let tmp_path =
            std::env::temp_dir().join(format!("rez_next_activate_{}.{}", std::process::id(), ext));
        self.write_activation_script(&tmp_path.to_string_lossy(), shell)
    }

    /// Return the activation script content as a string (no file I/O).
    #[pyo3(signature = (shell = None))]
    pub fn get_activation_script_content(&self, shell: Option<String>) -> String {
        let shell_name = shell.as_deref().unwrap_or(&self.shell_type);
        build_activation_script(&self.packages, shell_name)
    }

    /// The shell type this manager was created for.
    #[getter]
    pub fn shell_type(&self) -> &str {
        &self.shell_type
    }

    /// The package requirements this manager was created for.
    #[getter]
    pub fn packages(&self) -> Vec<String> {
        self.packages.clone()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "SourceManager(packages={:?}, shell='{}')",
            self.packages, self.shell_type
        )
    }
}

/// Detect the current shell based on environment variables.
pub(crate) fn detect_current_shell() -> String {
    // PowerShell
    if std::env::var("PSModulePath").is_ok() {
        return "powershell".to_string();
    }
    // POSIX: $SHELL
    if let Ok(shell) = std::env::var("SHELL") {
        let shell_lower = shell.to_lowercase();
        if shell_lower.contains("zsh") {
            return "zsh".to_string();
        }
        if shell_lower.contains("fish") {
            return "fish".to_string();
        }
        if shell_lower.contains("bash") {
            return "bash".to_string();
        }
    }
    // Windows CMD fallback
    if cfg!(target_os = "windows") {
        return "powershell".to_string();
    }
    "bash".to_string()
}

/// Build an activation script string for the given packages and shell.
pub(crate) fn build_activation_script(packages: &[String], shell_name: &str) -> String {
    let shell_type = match shell_name.to_lowercase().as_str() {
        "zsh" => ShellType::Zsh,
        "fish" => ShellType::Fish,
        "cmd" => ShellType::Cmd,
        "powershell" | "pwsh" => ShellType::PowerShell,
        _ => ShellType::Bash,
    };

    // Build a representative environment based on package list
    let mut env = RexEnvironment::new();

    // Set REZ_CONTEXT_FILE marker (rez standard).
    let context_file = std::env::temp_dir()
        .join("rez_context.rxt")
        .to_string_lossy()
        .into_owned();
    env.vars
        .insert("REZ_CONTEXT_FILE".to_string(), context_file);
    // REZ_RESOLVE: space-separated resolved package list
    env.vars
        .insert("REZ_RESOLVE".to_string(), packages.join(" "));
    // REZ_PACKAGES_PATH: sourced from config
    if let Ok(p) = std::env::var("REZ_PACKAGES_PATH") {
        env.vars.insert("REZ_PACKAGES_PATH".to_string(), p);
    }

    // For each package, set REZPKG_<NAME>=<version>.
    for pkg in packages {
        let parts: Vec<&str> = pkg.splitn(2, '-').collect();
        let pkg_name = parts[0].to_uppercase().replace(['-', '.'], "_");
        let ver = if parts.len() > 1 { parts[1] } else { "" };
        env.vars
            .insert(format!("REZPKG_{}", pkg_name), ver.to_string());
    }

    let mut script = generate_shell_script(&env, &shell_type);

    // Add a header comment for clarity
    let header = format!(
        "# rez-next activation script ({})\n# packages: {}\n",
        shell_name,
        packages.join(", ")
    );
    script = format!("{}{}", header, script);
    script
}

// ─── Module-level free functions ────────────────────────────────────────────

/// Write a rez activation script for the given packages.
///
/// Equivalent to `rez source <packages...> --output <dest>`.
#[pyfunction]
#[pyo3(signature = (packages, dest_path, shell = None))]
pub fn write_source_script(
    packages: Vec<String>,
    dest_path: &str,
    shell: Option<String>,
) -> PyResult<String> {
    let mgr = PySourceManager::new(packages, shell);
    mgr.write_activation_script(dest_path, None)
}

/// Return activation script content as a string.
#[pyfunction]
#[pyo3(signature = (packages, shell = None))]
pub fn get_source_script(packages: Vec<String>, shell: Option<String>) -> String {
    let mgr = PySourceManager::new(packages, shell);
    mgr.get_activation_script_content(None)
}

/// Detect the current shell type.
#[pyfunction]
pub fn detect_shell() -> String {
    detect_current_shell()
}

/// Resolve an activation mode (by string) to a script string or file path.
///
/// `mode_str` can be:
/// - `"inline"` → returns script content as a string
/// - `"tempfile"` → writes to a temp file, returns the path
/// - `"file:<dest_path>"` → writes to `dest_path`, returns the path
///
/// ## Python Usage
/// ```python
/// from rez_next.source import resolve_source_mode
/// content = resolve_source_mode(["python-3.9"], "bash", "inline")
/// path = resolve_source_mode(["python-3.9"], "bash", "tempfile")
/// path = resolve_source_mode(["python-3.9"], "bash", "file:/tmp/activate.sh")
/// ```
#[pyfunction]
#[pyo3(signature = (packages, shell, mode_str))]
pub fn resolve_source_mode(
    packages: Vec<String>,
    shell: String,
    mode_str: String,
) -> PyResult<String> {
    let shell_resolved = if shell == "auto" || shell.is_empty() {
        detect_current_shell()
    } else {
        shell
    };

    let mode = if mode_str == "inline" {
        SourceMode::Inline
    } else if mode_str == "tempfile" {
        SourceMode::TempFile
    } else if let Some(path) = mode_str.strip_prefix("file:") {
        SourceMode::File(PathBuf::from(path))
    } else {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown mode '{}'. Use 'inline', 'tempfile', or 'file:<path>'",
            mode_str
        )));
    };

    match mode {
        SourceMode::Inline => Ok(build_activation_script(&packages, &shell_resolved)),
        SourceMode::TempFile => {
            let ext = match shell_resolved.as_str() {
                "powershell" | "pwsh" => "ps1",
                "cmd" => "bat",
                _ => "sh",
            };
            let tmp_path = std::env::temp_dir().join(format!(
                "rez_next_activate_{}.{}",
                std::process::id(),
                ext
            ));
            let script = build_activation_script(&packages, &shell_resolved);
            std::fs::write(&tmp_path, &script)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            Ok(tmp_path.to_string_lossy().to_string())
        }
        SourceMode::File(dest) => {
            let script = build_activation_script(&packages, &shell_resolved);
            if let Some(parent) = dest.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                }
            }
            std::fs::write(&dest, &script)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            Ok(dest.to_string_lossy().to_string())
        }
    }
}

#[cfg(test)]
#[path = "source_bindings_tests.rs"]
mod source_bindings_tests;
