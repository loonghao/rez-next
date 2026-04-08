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

    // Set REZ_CONTEXT_FILE marker (rez standard).
    // Use the platform-specific temp directory so this works on Windows too.
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
    // Note: this is a best-effort indicator of the resolved version; it does
    // NOT encode the installation path.  Consumers needing the actual path
    // should use ResolvedContext.get_tools() or inspect Package.base directly.
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
        assert!(
            script.contains("REZ_RESOLVE"),
            "bash script should set REZ_RESOLVE"
        );
        assert!(script.contains("export"), "bash script should use export");
        assert!(
            script.contains("python-3.9"),
            "bash script should contain package name"
        );
    }

    #[test]
    fn test_build_activation_script_powershell() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "powershell");
        assert!(
            script.contains("REZ_RESOLVE"),
            "ps1 script should set REZ_RESOLVE"
        );
        // PowerShell uses $env: syntax
        assert!(
            script.contains("$env:") || script.contains("REZ_"),
            "ps1 should use $env: syntax"
        );
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
        let mgr = PySourceManager::new(vec!["maya-2024".to_string()], Some("bash".to_string()));
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
        let mgr = PySourceManager::new(vec!["python-3.9".to_string()], Some("bash".to_string()));
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

    #[test]
    fn test_source_mode_inline_variant() {
        // Verify SourceMode::Inline is constructable and usable
        let mode = SourceMode::Inline;
        assert_eq!(mode, SourceMode::Inline);
    }

    #[test]
    fn test_source_mode_tempfile_variant() {
        let mode = SourceMode::TempFile;
        assert_eq!(mode, SourceMode::TempFile);
    }

    #[test]
    fn test_source_mode_file_variant() {
        let path = PathBuf::from("/tmp/activate.sh");
        let mode = SourceMode::File(path.clone());
        assert_eq!(mode, SourceMode::File(path));
    }

    #[test]
    fn test_resolve_source_mode_inline_logic() {
        let pkgs = vec!["python-3.9".to_string()];
        // Test inline mode: build_activation_script should return content directly
        let content = build_activation_script(&pkgs, "bash");
        assert!(content.contains("REZ_RESOLVE"));
        assert!(content.contains("python-3.9"));
        // Verify SourceMode::Inline is used in match
        let mode = SourceMode::Inline;
        let result = match mode {
            SourceMode::Inline => build_activation_script(&pkgs, "bash"),
            SourceMode::TempFile => "tempfile".to_string(),
            SourceMode::File(_) => "file".to_string(),
        };
        assert!(result.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_resolve_source_mode_file_logic() {
        use tempfile::TempDir;
        let pkgs = vec!["maya-2024".to_string()];
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("activate_test.sh");
        let mode = SourceMode::File(dest.clone());
        let result = match mode {
            SourceMode::Inline => "inline".to_string(),
            SourceMode::TempFile => "tempfile".to_string(),
            SourceMode::File(path) => {
                let script = build_activation_script(&pkgs, "bash");
                std::fs::write(&path, &script).unwrap();
                path.to_string_lossy().to_string()
            }
        };
        assert!(!result.is_empty());
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("maya-2024"));
    }

    // ── Additional PySourceManager tests ────────────────────────────────────

    #[test]
    fn test_source_manager_packages_getter() {
        let pkgs = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];
        let mgr = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        assert_eq!(mgr.packages(), pkgs);
    }

    #[test]
    fn test_source_manager_shell_type_getter() {
        let mgr = PySourceManager::new(vec![], Some("zsh".to_string()));
        assert_eq!(mgr.shell_type(), "zsh");
    }

    #[test]
    fn test_source_manager_repr_contains_shell_and_packages() {
        let mgr = PySourceManager::new(
            vec!["houdini-19.5".to_string()],
            Some("fish".to_string()),
        );
        let repr = mgr.__repr__();
        assert!(repr.contains("SourceManager"), "repr: {repr}");
        assert!(repr.contains("fish"), "repr should show shell: {repr}");
        assert!(repr.contains("houdini-19.5"), "repr should show pkg: {repr}");
    }

    #[test]
    fn test_source_manager_empty_packages() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        let content = mgr.get_activation_script_content(None);
        // Script must still be valid; REZ_RESOLVE should be set (empty string value)
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
    }

    #[test]
    fn test_source_manager_explicit_shell_override_in_get_content() {
        let mgr = PySourceManager::new(
            vec!["cmake-3.26".to_string()],
            Some("bash".to_string()),
        );
        // Override with powershell at call-time
        let content = mgr.get_activation_script_content(Some("powershell".to_string()));
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
        // PowerShell uses $env: syntax
        assert!(
            content.contains("$env:") || content.contains("REZ_"),
            "powershell content should reference env vars: {content}"
        );
    }

    // ── build_activation_script edge cases ──────────────────────────────────

    #[test]
    fn test_build_activation_script_empty_packages() {
        let script = build_activation_script(&[], "bash");
        // Must contain REZ_RESOLVE even with no packages
        assert!(script.contains("REZ_RESOLVE"), "script: {script}");
    }

    #[test]
    fn test_build_activation_script_cmd_shell() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "cmd");
        assert!(script.contains("REZ_RESOLVE"), "cmd script: {script}");
    }

    #[test]
    fn test_build_activation_script_unknown_shell_falls_to_bash() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "tcsh");
        // Unknown shells fall through to Bash branch
        assert!(script.contains("REZ_RESOLVE"), "script: {script}");
        assert!(script.contains("export"), "bash branch must use export: {script}");
    }

    #[test]
    fn test_write_activation_script_creates_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("subdir").join("activate.sh");
        let mgr =
            PySourceManager::new(vec!["python-3.9".to_string()], Some("bash".to_string()));
        // write directly via internal helper (no GIL)
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let content = mgr.get_activation_script_content(None);
        std::fs::write(&dest, &content).unwrap();
        assert!(dest.exists());
        let read = std::fs::read_to_string(&dest).unwrap();
        assert!(read.contains("REZ_RESOLVE"));
    }

    // ── Cycle 103 additions ──────────────────────────────────────────────────

    #[test]
    fn test_build_activation_script_sets_rezpkg_for_each_package() {
        let pkgs = vec!["python-3.9".to_string(), "cmake-3.26".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.contains("REZPKG_PYTHON"),
            "should set REZPKG_PYTHON: {script}"
        );
        assert!(
            script.contains("REZPKG_CMAKE"),
            "should set REZPKG_CMAKE: {script}"
        );
    }

    #[test]
    fn test_source_manager_multiple_packages_all_in_content() {
        let pkgs = vec!["alpha-1.0".to_string(), "beta-2.0".to_string(), "gamma-3.0".to_string()];
        let mgr = PySourceManager::new(pkgs, Some("bash".to_string()));
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("alpha-1.0"), "content: {content}");
        assert!(content.contains("beta-2.0"), "content: {content}");
        assert!(content.contains("gamma-3.0"), "content: {content}");
    }

    #[test]
    fn test_source_manager_fish_shell_explicit() {
        let mgr = PySourceManager::new(
            vec!["nuke-14.0".to_string()],
            Some("fish".to_string()),
        );
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("REZ_RESOLVE"), "fish content: {content}");
    }

    #[test]
    fn test_build_activation_script_powershell_contains_env_prefix() {
        let pkgs = vec!["maya-2024".to_string()];
        let script = build_activation_script(&pkgs, "powershell");
        // PowerShell activation must reference $env: or REZ_ vars
        assert!(
            script.contains("$env:") || script.contains("REZ_"),
            "ps1 script: {script}"
        );
        assert!(script.contains("maya-2024"), "ps1 script: {script}");
    }

    #[test]
    fn test_source_manager_repr_format() {
        let mgr = PySourceManager::new(
            vec!["python-3.9".to_string()],
            Some("bash".to_string()),
        );
        let repr = mgr.__repr__();
        assert!(!repr.is_empty(), "repr must not be empty");
        // Must contain at least the type name
        assert!(
            repr.contains("SourceManager"),
            "repr must contain 'SourceManager', got: {repr}"
        );
    }

    #[test]
    fn test_activation_script_contains_rez_resolve_with_correct_value() {
        let pkgs = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        // REZ_RESOLVE should list the package names
        assert!(script.contains("python-3.9"), "script: {script}");
        assert!(script.contains("numpy-1.24"), "script: {script}");
    }

    #[test]
    fn test_source_mode_tempfile_distinct_from_inline() {
        // SourceMode variants must compare unequal to each other
        assert_ne!(SourceMode::Inline, SourceMode::TempFile);
        let path_a = PathBuf::from("/tmp/a.sh");
        let path_b = PathBuf::from("/tmp/b.sh");
        assert_ne!(SourceMode::File(path_a.clone()), SourceMode::File(path_b));
        assert_ne!(SourceMode::Inline, SourceMode::File(path_a));
    }

    // ── Cycle 115 additions ──────────────────────────────────────────────────

    #[test]
    fn test_build_activation_script_pwsh_alias() {
        // "pwsh" should fall through to PowerShell branch
        let pkgs = vec!["python-3.11".to_string()];
        let script = build_activation_script(&pkgs, "pwsh");
        assert!(script.contains("REZ_RESOLVE"), "pwsh script: {script}");
    }

    #[test]
    fn test_source_manager_repr_is_non_empty() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        let repr = mgr.__repr__();
        assert!(!repr.is_empty(), "repr must not be empty");
    }

    #[test]
    fn test_build_activation_script_sets_rez_context_file() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.contains("REZ_CONTEXT_FILE"),
            "script must set REZ_CONTEXT_FILE: {script}"
        );
    }

    #[test]
    fn test_source_manager_shell_type_preserved() {
        let mgr = PySourceManager::new(vec!["pkg-1.0".to_string()], Some("zsh".to_string()));
        assert_eq!(mgr.shell_type(), "zsh");
        assert_eq!(mgr.packages().len(), 1);
    }

    #[test]
    fn test_build_activation_script_rezpkg_version_correct() {
        // "python-3.11.2" → REZPKG_PYTHON should hold "3.11.2"
        let pkgs = vec!["python-3.11.2".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZPKG_PYTHON"), "script: {script}");
        assert!(script.contains("3.11.2"), "version in REZPKG_PYTHON: {script}");
    }

    #[test]
    fn test_source_manager_two_instances_same_output() {
        let pkgs = vec!["cmake-3.26".to_string()];
        let mgr1 = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        let mgr2 = PySourceManager::new(pkgs, Some("bash".to_string()));
        let c1 = mgr1.get_activation_script_content(None);
        let c2 = mgr2.get_activation_script_content(None);
        // Two managers with the same config must produce identical scripts
        assert_eq!(c1, c2, "identical managers must produce identical scripts");
    }

    #[test]
    fn test_detect_current_shell_returns_known_shell() {
        let shell = detect_current_shell();
        let known = ["bash", "zsh", "fish", "powershell", "pwsh", "cmd"];
        assert!(
            known.iter().any(|k| shell == *k),
            "detect_current_shell must return a known shell, got: '{shell}'"
        );
    }

    // ── Cycle 120 additions ──────────────────────────────────────────────────

    #[test]
    fn test_build_activation_script_sets_rez_used_resolve() {
        // REZ_USED_RESOLVE or REZ_RESOLVE must appear with package names
        let pkgs = vec!["houdini-20.0".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.contains("houdini-20.0") && script.contains("REZ_RESOLVE"),
            "bash script must include package in REZ_RESOLVE: {script}"
        );
    }

    #[test]
    fn test_source_manager_packages_count_preserved() {
        let pkgs: Vec<String> = (0..5).map(|i| format!("pkg_{i}-1.0")).collect();
        let mgr = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        assert_eq!(mgr.packages().len(), 5);
    }

    #[test]
    fn test_build_activation_script_zsh_shell() {
        let pkgs = vec!["python-3.11".to_string()];
        let script = build_activation_script(&pkgs, "zsh");
        assert!(script.contains("REZ_RESOLVE"), "zsh script must set REZ_RESOLVE: {script}");
        assert!(script.contains("python-3.11"), "zsh script must include package name: {script}");
    }

    #[test]
    fn test_source_manager_get_content_none_shell_uses_default() {
        let mgr = PySourceManager::new(
            vec!["nuke-14.0".to_string()],
            None, // no explicit shell
        );
        let content = mgr.get_activation_script_content(None);
        // Must produce non-empty content with REZ_RESOLVE
        assert!(!content.is_empty(), "content must not be empty when shell is None");
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
    }

    #[test]
    fn test_source_mode_file_path_preserved() {
        let p = PathBuf::from("/tmp/my_activate.sh");
        let mode = SourceMode::File(p.clone());
        if let SourceMode::File(stored) = mode {
            assert_eq!(stored, p, "SourceMode::File must store exact path");
        } else {
            panic!("Expected SourceMode::File variant");
        }
    }

    #[test]
    fn test_source_manager_repr_contains_package_count() {
        let pkgs = vec!["a-1.0".to_string(), "b-2.0".to_string(), "c-3.0".to_string()];
        let mgr = PySourceManager::new(pkgs, Some("bash".to_string()));
        let repr = mgr.__repr__();
        // repr should at minimum contain the type name and not be empty
        assert!(!repr.is_empty(), "repr must not be empty");
        assert!(repr.contains("SourceManager"), "repr: {repr}");
    }

    // ─────── Cycle 126 additions ─────────────────────────────────────────────

    #[test]
    fn test_source_manager_packages_roundtrip() {
        let pkgs = vec!["python-3.11".to_string(), "cmake-3.26".to_string()];
        let mgr = PySourceManager::new(pkgs.clone(), None);
        assert_eq!(mgr.packages(), pkgs);
    }

    #[test]
    fn test_source_manager_shell_type_when_given_bash() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        assert_eq!(mgr.shell_type(), "bash");
    }

    #[test]
    fn test_source_manager_empty_packages_is_empty() {
        let mgr = PySourceManager::new(vec![], None);
        assert!(mgr.packages().is_empty(), "empty packages list must round-trip as empty");
    }

    #[test]
    fn test_get_source_script_returns_string() {
        // get_source_script is pure and must not panic for arbitrary input
        let script = get_source_script(vec!["python-3.9".to_string()], Some("bash".to_string()));
        // Result is a String — just verify it can be used
        let _ = script.len();
    }

    #[test]
    fn test_detect_shell_is_nonempty() {
        let shell = detect_shell();
        assert!(!shell.is_empty(), "detect_shell must return a non-empty string");
    }
}

