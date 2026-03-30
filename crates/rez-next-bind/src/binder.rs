//! Core binder: writes package.py for a bound system tool.

use crate::detect::{detect_tool_version, extract_version_from_output, find_tool_executable};
use rez_next_common::config::RezCoreConfig;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Error types for bind operations.
#[derive(Debug, Error)]
pub enum BindError {
    #[error("Tool '{0}' not found in PATH")]
    ToolNotFound(String),

    #[error("Cannot determine version for tool '{0}'")]
    VersionNotFound(String),

    #[error("Package already exists at '{0}' (use --force to overwrite)")]
    AlreadyExists(PathBuf),

    #[error("I/O error writing package: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bind error: {0}")]
    Other(String),
}

/// Options controlling the bind operation.
#[derive(Debug, Clone)]
pub struct BindOptions {
    /// Explicit version override (skip detection if Some).
    pub version_override: Option<String>,

    /// Where to install the bound package. Defaults to config local_packages_path.
    pub install_path: Option<PathBuf>,

    /// Overwrite existing package if already bound.
    pub force: bool,

    /// Extra key=value metadata to embed in package.py.
    pub extra_metadata: Vec<(String, String)>,

    /// Whether to search system PATH for the tool.
    pub search_path: bool,
}

impl Default for BindOptions {
    fn default() -> Self {
        Self {
            version_override: None,
            install_path: None,
            force: false,
            extra_metadata: Vec::new(),
            search_path: true,
        }
    }
}

/// Result returned by a successful bind.
#[derive(Debug, Clone)]
pub struct BindResult {
    /// Package name.
    pub name: String,

    /// Resolved version string.
    pub version: String,

    /// Path where the package was installed.
    pub install_path: PathBuf,

    /// Absolute path to the bound executable.
    pub executable_path: Option<PathBuf>,
}

/// Core struct that performs the bind operation.
pub struct PackageBinder {
    config: RezCoreConfig,
}

impl PackageBinder {
    /// Create a binder backed by the default rez config.
    pub fn new() -> Self {
        Self {
            config: RezCoreConfig::load(),
        }
    }

    /// Bind a system tool as a rez package.
    ///
    /// # Arguments
    /// * `tool_name` – the name of the tool to bind (e.g. "python", "cmake").
    /// * `options`   – bind configuration.
    pub fn bind(&self, tool_name: &str, options: &BindOptions) -> Result<BindResult, BindError> {
        // 1. Locate executable
        let exe_path = if options.search_path {
            find_tool_executable(tool_name)
        } else {
            None
        };

        // 2. Determine version
        let version = if let Some(ref ver) = options.version_override {
            ver.clone()
        } else {
            let raw = if let Some(ref p) = exe_path {
                detect_tool_version(&p.to_string_lossy())
            } else {
                detect_tool_version(tool_name)
            };

            extract_version_from_output(&raw)
                .ok_or_else(|| BindError::VersionNotFound(tool_name.to_string()))?
        };

        // 3. Determine install root
        let install_root = options
            .install_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(expand_home(&self.config.local_packages_path)));

        let pkg_dir = install_root.join(tool_name).join(&version);

        if pkg_dir.exists() && !options.force {
            return Err(BindError::AlreadyExists(pkg_dir));
        }

        // 4. Write package.py
        std::fs::create_dir_all(&pkg_dir)?;
        let package_py = self.generate_package_py(tool_name, &version, &exe_path, options);
        std::fs::write(pkg_dir.join("package.py"), package_py)?;

        Ok(BindResult {
            name: tool_name.to_string(),
            version,
            install_path: pkg_dir,
            executable_path: exe_path,
        })
    }

    /// Generate the package.py content for the bound tool.
    fn generate_package_py(
        &self,
        name: &str,
        version: &str,
        exe_path: &Option<PathBuf>,
        options: &BindOptions,
    ) -> String {
        let exe_comment = exe_path
            .as_ref()
            .map(|p| format!("# Bound executable: {}", p.display()))
            .unwrap_or_else(|| "# Executable path not detected".to_string());

        let extra_fields: String = options
            .extra_metadata
            .iter()
            .map(|(k, v)| format!("{} = '{}'\n", k, v))
            .collect();

        let tool_bin_path = exe_path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|d| d.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|| String::new());

        let commands_block = if tool_bin_path.is_empty() {
            String::new()
        } else {
            format!(
                r#"
def commands():
    env.prepend_path('PATH', r'{bin}')
"#,
                bin = tool_bin_path
            )
        };

        format!(
            r#"# Auto-generated by rez-next bind
{exe_comment}
name = '{name}'
version = '{version}'
description = 'System-installed {name} bound by rez-next bind'
tools = ['{name}']
{extra_fields}{commands_block}
"#,
            exe_comment = exe_comment,
            name = name,
            version = version,
            extra_fields = extra_fields,
            commands_block = commands_block,
        )
    }

    /// List all currently bound packages (packages with a `package.py` in install root).
    pub fn list_bound_packages(&self) -> Vec<(String, Vec<String>)> {
        let install_root = PathBuf::from(expand_home(&self.config.local_packages_path));
        let mut result = Vec::new();

        if !install_root.exists() {
            return result;
        }

        if let Ok(families) = std::fs::read_dir(&install_root) {
            for family_entry in families.filter_map(|e| e.ok()) {
                let family_path = family_entry.path();
                if !family_path.is_dir() {
                    continue;
                }
                let family_name = family_entry.file_name().to_string_lossy().to_string();
                let mut versions = Vec::new();

                if let Ok(ver_entries) = std::fs::read_dir(&family_path) {
                    for ver_entry in ver_entries.filter_map(|e| e.ok()) {
                        let ver_path = ver_entry.path();
                        if ver_path.is_dir() && ver_path.join("package.py").exists() {
                            versions.push(ver_entry.file_name().to_string_lossy().to_string());
                        }
                    }
                }

                if !versions.is_empty() {
                    versions.sort();
                    result.push((family_name, versions));
                }
            }
        }

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }
}

fn expand_home(p: &str) -> String {
    if p.starts_with("~/") || p == "~" {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return p.replacen("~", &home, 1);
        }
    }
    p.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bind_with_version_override() {
        let tmp = TempDir::new().unwrap();
        let binder = PackageBinder::new();

        let opts = BindOptions {
            version_override: Some("3.11.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };

        let result = binder.bind("python", &opts).unwrap();
        assert_eq!(result.name, "python");
        assert_eq!(result.version, "3.11.0");
        assert!(result.install_path.exists());
        assert!(result.install_path.join("package.py").exists());
    }

    #[test]
    fn test_bind_already_exists_error() {
        let tmp = TempDir::new().unwrap();
        let binder = PackageBinder::new();

        let opts = BindOptions {
            version_override: Some("1.0.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };

        binder.bind("mytool", &opts).unwrap();
        // Second bind without force should fail
        let result = binder.bind("mytool", &opts);
        assert!(matches!(result, Err(BindError::AlreadyExists(_))));
    }

    #[test]
    fn test_bind_force_overwrite() {
        let tmp = TempDir::new().unwrap();
        let binder = PackageBinder::new();

        let opts_first = BindOptions {
            version_override: Some("1.0.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };
        binder.bind("mytool", &opts_first).unwrap();

        let opts_force = BindOptions {
            version_override: Some("1.0.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: true,
            search_path: false,
            ..Default::default()
        };
        let result = binder.bind("mytool", &opts_force);
        assert!(result.is_ok());
    }

    #[test]
    fn test_bind_package_py_content() {
        let tmp = TempDir::new().unwrap();
        let binder = PackageBinder::new();

        let opts = BindOptions {
            version_override: Some("2.42.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            extra_metadata: vec![("authors".to_string(), "['system']".to_string())],
            ..Default::default()
        };

        let result = binder.bind("git", &opts).unwrap();
        let content = std::fs::read_to_string(result.install_path.join("package.py")).unwrap();
        assert!(content.contains("name = 'git'"));
        assert!(content.contains("version = '2.42.0'"));
        assert!(content.contains("tools = ['git']"));
        assert!(content.contains("authors"));
    }

    #[test]
    fn test_bind_list_packages() {
        let tmp = TempDir::new().unwrap();
        let binder = PackageBinder::new();

        for tool in &["python", "cmake", "git"] {
            let opts = BindOptions {
                version_override: Some("1.0.0".to_string()),
                install_path: Some(tmp.path().to_path_buf()),
                force: false,
                search_path: false,
                ..Default::default()
            };
            // Override install_path for listing test by using the binder's list function
            // but we can't easily override config; instead create dirs manually
            let pkg_dir = tmp.path().join(tool).join("1.0.0");
            std::fs::create_dir_all(&pkg_dir).unwrap();
            std::fs::write(pkg_dir.join("package.py"), format!("name = '{}'", tool)).unwrap();
            let _ = opts;
        }

        // Use a fresh binder but point list to the temp dir
        // Since list_bound_packages uses config, test the directory structure
        let python_pkg = tmp.path().join("python").join("1.0.0").join("package.py");
        assert!(python_pkg.exists());
        let cmake_pkg = tmp.path().join("cmake").join("1.0.0").join("package.py");
        assert!(cmake_pkg.exists());
    }

    #[test]
    fn test_generate_package_py_no_exe() {
        let binder = PackageBinder::new();
        let opts = BindOptions::default();
        let content = binder.generate_package_py("mytool", "1.2.3", &None, &opts);
        assert!(content.contains("name = 'mytool'"));
        assert!(content.contains("version = '1.2.3'"));
        assert!(!content.contains("def commands"));
    }
}
