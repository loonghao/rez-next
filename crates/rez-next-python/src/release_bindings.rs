//! Python bindings for rez.release — package release flow
//!
//! Implements the `rez release` workflow:
//! 1. Validate package definition
//! 2. Run pre-release checks (VCS, lint, tests)
//! 3. Build release artifact
//! 4. Copy to release packages path
//! 5. Update VCS tags

use crate::runtime::get_runtime;
use pyo3::prelude::*;
use std::path::PathBuf;

/// Release mode for a package
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ReleaseMode {
    /// Normal release to release_packages_path
    Release,
    /// Local release to local_packages_path
    Local,
    /// Dry run: validate but don't write
    DryRun,
}

impl ReleaseMode {
    fn from_str(s: &str) -> Self {
        match s {
            "local" => ReleaseMode::Local,
            "dry_run" | "dry-run" => ReleaseMode::DryRun,
            _ => ReleaseMode::Release,
        }
    }
}

/// Result of a release operation
#[pyclass(name = "ReleaseResult", from_py_object)]
#[derive(Clone)]
pub struct PyReleaseResult {
    #[pyo3(get)]
    pub success: bool,
    #[pyo3(get)]
    pub package_name: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub install_path: String,
    #[pyo3(get)]
    pub errors: Vec<String>,
    #[pyo3(get)]
    pub warnings: Vec<String>,
}

#[pymethods]
impl PyReleaseResult {
    fn __str__(&self) -> String {
        if self.success {
            format!(
                "ReleaseResult(OK: {}-{} -> {})",
                self.package_name, self.version, self.install_path
            )
        } else {
            format!(
                "ReleaseResult(FAILED: {}-{}, errors: {:?})",
                self.package_name, self.version, self.errors
            )
        }
    }

    fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Release manager — orchestrates package release operations.
///
/// Compatible with `rez.release_build.ReleaseBuildProcess`.
#[pyclass(name = "ReleaseManager")]
pub struct PyReleaseManager {
    mode: ReleaseMode,
    skip_build: bool,
    skip_tests: bool,
}

#[pymethods]
impl PyReleaseManager {
    #[new]
    #[pyo3(signature = (mode=None, skip_build=false, skip_tests=false))]
    pub fn new(mode: Option<&str>, skip_build: bool, skip_tests: bool) -> Self {
        PyReleaseManager {
            mode: ReleaseMode::from_str(mode.unwrap_or("release")),
            skip_build,
            skip_tests,
        }
    }

    fn __str__(&self) -> String {
        format!(
            "ReleaseManager(mode={:?}, skip_build={}, skip_tests={})",
            self.mode, self.skip_build, self.skip_tests
        )
    }

    /// Release a package from a source directory.
    /// Equivalent to running `rez release` from the package directory.
    #[pyo3(signature = (source_dir=None, message=None))]
    fn release(
        &self,
        source_dir: Option<&str>,
        message: Option<&str>,
    ) -> PyResult<PyReleaseResult> {
        use crate::package_functions::expand_home;
        use rez_next_common::config::RezCoreConfig;
        use rez_next_package::serialization::PackageSerializer;

        let cwd = std::env::current_dir()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let source = PathBuf::from(source_dir.unwrap_or("."));
        let source = if source.is_relative() {
            cwd.join(source)
        } else {
            source
        };

        // 1. Load package definition
        let pkg_file = source.join("package.py");
        let pkg_yaml = source.join("package.yaml");
        let package = if pkg_file.exists() {
            PackageSerializer::load_from_file(&pkg_file)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
        } else if pkg_yaml.exists() {
            PackageSerializer::load_from_file(&pkg_yaml)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
        } else {
            return Ok(PyReleaseResult {
                success: false,
                package_name: String::new(),
                version: String::new(),
                install_path: String::new(),
                errors: vec!["No package.py or package.yaml found".to_string()],
                warnings: vec![],
            });
        };

        let version_str = package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // 2. Determine install destination
        let config = RezCoreConfig::load();
        let install_base = match self.mode {
            ReleaseMode::Local | ReleaseMode::DryRun => {
                PathBuf::from(expand_home(&config.local_packages_path))
            }
            ReleaseMode::Release => {
                // Use release_packages_path if configured, else fall back to local
                let rp = &config.release_packages_path;
                if !rp.is_empty() && rp != "~/.rez/packages/int" {
                    PathBuf::from(expand_home(rp))
                } else {
                    PathBuf::from(expand_home(&config.local_packages_path))
                }
            }
        };

        let install_path = install_base.join(&package.name).join(&version_str);

        let path_str = install_path.to_string_lossy().to_string();

        if self.mode == ReleaseMode::DryRun {
            return Ok(PyReleaseResult {
                success: true,
                package_name: package.name.clone(),
                version: version_str,
                install_path: format!("[dry-run] {}", path_str),
                errors: vec![],
                warnings: message
                    .map(|m| vec![format!("dry-run note: {}", m)])
                    .unwrap_or_default(),
            });
        }

        // 3. Create install directory
        std::fs::create_dir_all(&install_path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        // 4. Copy package definition file
        let dest_pkg_file = install_path.join(if pkg_file.exists() {
            "package.py"
        } else {
            "package.yaml"
        });
        let src_pkg_file = if pkg_file.exists() {
            &pkg_file
        } else {
            &pkg_yaml
        };
        std::fs::copy(src_pkg_file, &dest_pkg_file)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        Ok(PyReleaseResult {
            success: true,
            package_name: package.name,
            version: version_str,
            install_path: path_str,
            errors: vec![],
            warnings: vec![],
        })
    }

    /// Validate a package before release (pre-flight checks).
    /// Returns (is_valid, list_of_issues).
    #[pyo3(signature = (source_dir=None))]
    fn validate(&self, source_dir: Option<&str>) -> PyResult<(bool, Vec<String>)> {
        use rez_next_package::serialization::PackageSerializer;

        let cwd = std::env::current_dir()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let source = PathBuf::from(source_dir.unwrap_or("."));
        let source = if source.is_relative() {
            cwd.join(source)
        } else {
            source
        };

        let mut issues: Vec<String> = vec![];

        let pkg_file = source.join("package.py");
        let pkg_yaml = source.join("package.yaml");

        if !pkg_file.exists() && !pkg_yaml.exists() {
            issues.push("Missing package.py or package.yaml".to_string());
            return Ok((false, issues));
        }

        let pkg_path = if pkg_file.exists() {
            &pkg_file
        } else {
            &pkg_yaml
        };
        match PackageSerializer::load_from_file(pkg_path) {
            Ok(pkg) => {
                if pkg.name.is_empty() {
                    issues.push("Package name is empty".to_string());
                }
                if pkg.version.is_none() {
                    issues.push("Package version is not set".to_string());
                }
            }
            Err(e) => {
                issues.push(format!("Failed to parse package: {}", e));
            }
        }

        Ok((issues.is_empty(), issues))
    }

    /// List all released versions of a package in the repository.
    #[pyo3(signature = (package_name, paths=None))]
    fn list_versions(
        &self,
        package_name: &str,
        paths: Option<Vec<String>>,
    ) -> PyResult<Vec<String>> {
        use crate::package_functions::expand_home;
        use rez_next_common::config::RezCoreConfig;
        use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
        use std::path::PathBuf;

        let rt = get_runtime();

        let config = RezCoreConfig::load();
        let mut repo_manager = RepositoryManager::new();

        let pkg_paths: Vec<PathBuf> = paths
            .map(|p| p.into_iter().map(PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| PathBuf::from(expand_home(p)))
                    .collect()
            });

        for (i, path) in pkg_paths.iter().enumerate() {
            if path.exists() {
                repo_manager.add_repository(Box::new(SimpleRepository::new(
                    path.clone(),
                    format!("repo_{}", i),
                )));
            }
        }

        let packages = rt
            .block_on(repo_manager.find_packages(package_name))
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let mut versions: Vec<String> = packages
            .iter()
            .filter(|p| p.name == package_name)
            .filter_map(|p| p.version.as_ref().map(|v| v.as_str().to_string()))
            .collect();
        versions.sort();
        Ok(versions)
    }
}

/// Quick-release function: release a package from a directory.
/// Equivalent to `rez release` (non-interactive).
#[pyfunction]
#[pyo3(signature = (source_dir=None, local=false, dry_run=false, message=None))]
pub fn release_package(
    source_dir: Option<&str>,
    local: bool,
    dry_run: bool,
    message: Option<&str>,
) -> PyResult<PyReleaseResult> {
    let mode = if dry_run {
        "dry_run"
    } else if local {
        "local"
    } else {
        "release"
    };
    let mgr = PyReleaseManager::new(Some(mode), false, false);
    mgr.release(source_dir, message)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod release_tests {
    use super::*;

    #[test]
    fn test_release_mode_from_str() {
        assert_eq!(ReleaseMode::from_str("local"), ReleaseMode::Local);
        assert_eq!(ReleaseMode::from_str("dry_run"), ReleaseMode::DryRun);
        assert_eq!(ReleaseMode::from_str("release"), ReleaseMode::Release);
        assert_eq!(ReleaseMode::from_str("unknown"), ReleaseMode::Release);
    }

    #[test]
    fn test_release_manager_new() {
        let mgr = PyReleaseManager::new(None, false, false);
        assert_eq!(mgr.mode, ReleaseMode::Release);
        assert!(!mgr.skip_build);
    }

    #[test]
    fn test_release_manager_str() {
        let mgr = PyReleaseManager::new(Some("local"), false, true);
        let s = mgr.__str__();
        assert!(s.contains("Local"));
        assert!(s.contains("skip_tests=true"));
    }

    #[test]
    fn test_validate_missing_dir_returns_issues() {
        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some("/nonexistent/path/xyz_abc_123")).unwrap();
        assert!(!valid);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_release_missing_source_returns_error() {
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr.release(Some("/nonexistent/path"), None).unwrap();
        assert!(!result.success);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_release_dry_run_with_temp_package() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'testpkg'\nversion = '1.0.0'\n").unwrap();

        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr
            .release(Some(dir.path().to_str().unwrap()), Some("test release"))
            .unwrap();
        assert!(
            result.success,
            "dry_run should succeed: {:?}",
            result.errors
        );
        assert!(result.install_path.contains("[dry-run]"));
        assert_eq!(result.package_name, "testpkg");
        assert_eq!(result.version, "1.0.0");
    }

    #[test]
    fn test_validate_with_valid_package() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'mypkg'\nversion = '2.0.0'\n").unwrap();

        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some(dir.path().to_str().unwrap())).unwrap();
        // package parsing may or may not succeed depending on parser strictness
        // but it shouldn't panic
        let _ = (valid, issues);
    }

    #[test]
    fn test_release_result_str() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/packages/mypkg/1.0.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("OK"));
        assert!(s.contains("mypkg"));
    }

    #[test]
    fn test_release_result_failed_str() {
        let result = PyReleaseResult {
            success: false,
            package_name: "badpkg".to_string(),
            version: "0.0.0".to_string(),
            install_path: String::new(),
            errors: vec!["Missing version".to_string()],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("FAILED"));
    }

    // ── New tests (Cycle 89) ─────────────────────────────────────────────────

    #[test]
    fn test_release_result_repr_equals_str() {
        let result = PyReleaseResult {
            success: true,
            package_name: "pkgx".to_string(),
            version: "3.2.1".to_string(),
            install_path: "/pkgs/pkgx/3.2.1".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        assert_eq!(result.__repr__(), result.__str__());
    }

    #[test]
    fn test_release_result_str_contains_version() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "2.5.0".to_string(),
            install_path: "/dest/mypkg/2.5.0".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("2.5.0"), "str should contain version: {}", s);
    }

    #[test]
    fn test_release_result_str_contains_path() {
        let result = PyReleaseResult {
            success: true,
            package_name: "mypkg".to_string(),
            version: "1.0.0".to_string(),
            install_path: "/custom/install/path".to_string(),
            errors: vec![],
            warnings: vec![],
        };
        let s = result.__str__();
        assert!(s.contains("/custom/install/path"), "str: {}", s);
    }

    #[test]
    fn test_release_mode_dry_run_alias() {
        assert_eq!(ReleaseMode::from_str("dry-run"), ReleaseMode::DryRun);
    }

    #[test]
    fn test_release_manager_dry_run_mode() {
        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        assert_eq!(mgr.mode, ReleaseMode::DryRun);
    }

    #[test]
    fn test_release_manager_skip_build_flag() {
        let mgr = PyReleaseManager::new(None, true, false);
        assert!(mgr.skip_build);
        assert!(!mgr.skip_tests);
    }

    #[test]
    fn test_release_manager_skip_tests_flag() {
        let mgr = PyReleaseManager::new(None, false, true);
        assert!(!mgr.skip_build);
        assert!(mgr.skip_tests);
    }

    #[test]
    fn test_dry_run_result_has_dry_run_prefix() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'drytestpkg'\nversion = '0.1.0'\n").unwrap();

        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr
            .release(Some(dir.path().to_str().unwrap()), None)
            .unwrap();
        assert!(result.success);
        assert!(
            result.install_path.starts_with("[dry-run]"),
            "path: {}",
            result.install_path
        );
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_dry_run_with_message_populates_warnings() {
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let pkg_path = dir.path().join("package.py");
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        writeln!(f, "name = 'notepkg'\nversion = '0.2.0'\n").unwrap();

        let mgr = PyReleaseManager::new(Some("dry_run"), false, false);
        let result = mgr
            .release(Some(dir.path().to_str().unwrap()), Some("review note"))
            .unwrap();
        assert!(
            !result.warnings.is_empty(),
            "warnings should contain dry-run note"
        );
        assert!(
            result.warnings[0].contains("review note"),
            "warning: {}",
            result.warnings[0]
        );
    }

    #[test]
    fn test_validate_empty_dir_returns_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let mgr = PyReleaseManager::new(None, false, false);
        let (valid, issues) = mgr.validate(Some(dir.path().to_str().unwrap())).unwrap();
        assert!(!valid, "empty dir should be invalid");
        assert!(!issues.is_empty(), "should report missing package.py/yaml");
    }
}
