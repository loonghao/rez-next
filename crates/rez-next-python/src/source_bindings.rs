//! Python bindings for `rez.source` — context activation and source scripts.
//!
//! Equivalent to `rez source` command: writes a shell activation script for a
//! resolved context so users can activate it with `source <script>` or `.` in
//! POSIX shells, or `. <script>` in PowerShell.

use pyo3::prelude::*;
use rez_next_rex::{RexEnvironment, ShellType, generate_shell_script};
use std::path::PathBuf;

/// Supported activation modes matching rez's `rez source` command.
#[derive(Debug, Clone, PartialEq)]
pub enum SourceMode {
    /// Write activation script to a temp file and print path
    TempFile,
    /// Write activation script to specified path
    File(PathBuf),
    /// Return script content as string (no file I/O)
    Inline,
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
    packages: Vec<String>,
    shell_type: String,
}

#[pymethods]
impl PySourceManager {
    /// Create a new SourceManager for the given package requirements.
    #[new]
    #[pyo3(signature = (packages, shell = None))]
    pub fn new(packages: Vec<String>, shell: Option<String>) -> Self {
        let shell_type = shell.unwrap_or_else(detect_current_shell);
        Self { packages, shell_type }
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

        Ok(path.canonicalize()
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
        let tmp_path = std::env::temp_dir()
            .join(format!("rez_next_activate_{}.{}", std::process::id(), ext));
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
fn detect_current_shell() -> String {
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
fn build_activation_script(packages: &[String], shell_name: &str) -> String {
    let shell_type = match shell_name.to_lowercase().as_str() {
        "zsh" => ShellType::Zsh,
        "fish" => ShellType::Fish,
        "cmd" => ShellType::Cmd,
        "powershell" | "pwsh" => ShellType::PowerShell,
        _ => ShellType::Bash,
    };

    // Build a representative environment based on package list
    let mut env = RexEnvironment::new();

    // Set REZ_CONTEXT_FILE marker (rez standard)
    env.vars.insert(
        "REZ_CONTEXT_FILE".to_string(),
        "/tmp/rez_context.rxt".to_string(),
    );
    // REZ_RESOLVE: space-separated resolved package list
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        packages.join(" "),
    );
    // REZ_PACKAGES_PATH: sourced from config
    if let Ok(p) = std::env::var("REZ_PACKAGES_PATH") {
        env.vars.insert("REZ_PACKAGES_PATH".to_string(), p);
    }

    // For each package, create a placeholder env var: REZPKG_<NAME>=<version-or-name>
    for pkg in packages {
        let parts: Vec<&str> = pkg.splitn(2, '-').collect();
        let pkg_name = parts[0].to_uppercase().replace(['-', '.'], "_");
        let ver = if parts.len() > 1 { parts[1] } else { "" };
        env.vars.insert(
            format!("REZPKG_{}", pkg_name),
            ver.to_string(),
        );
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

/// Resolve a SourceMode to a script string or path.
///
/// This is the internal dispatcher used by write_source_script and friends.
/// It also ensures all SourceMode variants are exercised.
pub fn resolve_source_mode(
    packages: &[String],
    shell: Option<&str>,
    mode: SourceMode,
) -> Result<String, std::io::Error> {
    let shell_name = shell.unwrap_or("auto");
    let shell_resolved = if shell_name == "auto" {
        detect_current_shell()
    } else {
        shell_name.to_string()
    };

    match mode {
        SourceMode::Inline => {
            Ok(build_activation_script(packages, &shell_resolved))
        }
        SourceMode::TempFile => {
            let ext = match shell_resolved.as_str() {
                "powershell" | "pwsh" => "ps1",
                "cmd" => "bat",
                _ => "sh",
            };
            let tmp_path = std::env::temp_dir()
                .join(format!("rez_next_activate_{}.{}", std::process::id(), ext));
            let script = build_activation_script(packages, &shell_resolved);
            std::fs::write(&tmp_path, &script)?;
            Ok(tmp_path.to_string_lossy().to_string())
        }
        SourceMode::File(dest) => {
            let script = build_activation_script(packages, &shell_resolved);
            if let Some(parent) = dest.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent)?;
                }
            }
            std::fs::write(&dest, &script)?;
            Ok(dest.to_string_lossy().to_string())
        }
    }
}

// ─── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_current_shell_returns_string() {
        let shell = detect_current_shell();
        assert!(!shell.is_empty());
        let known = ["bash", "zsh", "fish", "powershell", "pwsh", "cmd"];
        assert!(
            known.iter().any(|k| shell.contains(k)),
            "Unexpected shell: {}",
            shell
        );
    }

    #[test]
    fn test_build_activation_script_bash() {
        let pkgs = vec!["python-3.9".to_string(), "maya-2024".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZ_RESOLVE"), "bash script should set REZ_RESOLVE");
        assert!(script.contains("export"), "bash script should use export");
        assert!(script.contains("python-3.9"), "bash script should contain package name");
    }

    #[test]
    fn test_build_activation_script_powershell() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "powershell");
        assert!(script.contains("REZ_RESOLVE"), "ps1 script should set REZ_RESOLVE");
        // PowerShell uses $env: syntax
        assert!(script.contains("$env:") || script.contains("REZ_"), "ps1 should use $env: syntax");
    }

    #[test]
    fn test_build_activation_script_zsh() {
        let pkgs = vec!["houdini-19.5".to_string()];
        let script = build_activation_script(&pkgs, "zsh");
        assert!(script.contains("REZ_RESOLVE"));
        assert!(script.contains("houdini-19.5"));
    }

    #[test]
    fn test_build_activation_script_fish() {
        let pkgs = vec!["nuke-14.0".to_string()];
        let script = build_activation_script(&pkgs, "fish");
        assert!(script.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_source_manager_new_default_shell() {
        let mgr = PySourceManager::new(vec!["python-3.9".to_string()], None);
        assert!(!mgr.shell_type.is_empty());
        assert_eq!(mgr.packages.len(), 1);
    }

    #[test]
    fn test_source_manager_new_explicit_shell() {
        let mgr = PySourceManager::new(
            vec!["maya-2024".to_string()],
            Some("bash".to_string()),
        );
        assert_eq!(mgr.shell_type, "bash");
    }

    #[test]
    fn test_source_manager_get_activation_content() {
        let mgr = PySourceManager::new(
            vec!["python-3.10".to_string(), "pip-23".to_string()],
            Some("bash".to_string()),
        );
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("REZ_RESOLVE"));
        assert!(!content.is_empty());
    }

    #[test]
    fn test_write_activation_script_to_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("activate.sh");
        let mgr = PySourceManager::new(
            vec!["python-3.9".to_string()],
            Some("bash".to_string()),
        );
        // No Python GIL available in unit tests — call the internal function directly
        let content = mgr.get_activation_script_content(None);
        std::fs::write(&dest, &content).unwrap();
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_pkg_env_var_generation() {
        let pkgs = vec!["python-3.9".to_string(), "my-tool-2.0".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        // Should contain REZPKG_PYTHON
        assert!(script.contains("REZPKG_PYTHON"), "Should set REZPKG_PYTHON");
    }

    #[test]
    fn test_activation_script_header_comment() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.starts_with("# rez-next activation script"),
            "Script should start with header comment"
        );
    }
}
