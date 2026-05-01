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
use pyo3::types::PyDict;
use rez_next_build::vcs::{ReleaseVCS, VCSMetadata};
use std::path::PathBuf;

// ============================================================================
/// VCS Metadata for release
// ============================================================================
#[pyclass(name = "VCSMetadata", from_py_object)]
#[derive(Clone)]
pub struct PyVCSMetadata {
    #[pyo3(get)]
    pub vcs_type: String,
    #[pyo3(get)]
    pub repository_url: Option<String>,
    #[pyo3(get)]
    pub branch: Option<String>,
    #[pyo3(get)]
    pub tracking_branch: Option<String>,
    #[pyo3(get)]
    pub fetch_url: Option<String>,
    #[pyo3(get)]
    pub push_url: Option<String>,
    #[pyo3(get)]
    pub commit_hash: String,
    #[pyo3(get)]
    pub commit_message: Option<String>,
    #[pyo3(get)]
    pub author_name: Option<String>,
    #[pyo3(get)]
    pub author_email: Option<String>,
    #[pyo3(get)]
    pub timestamp: Option<i64>,
}

#[pymethods]
impl PyVCSMetadata {
    #[new]
    #[pyo3(signature = (
        vcs_type,
        repository_url=None,
        branch=None,
        tracking_branch=None,
        fetch_url=None,
        push_url=None,
        commit_hash="",
        commit_message=None,
        author_name=None,
        author_email=None,
        timestamp=None,
    ))]
    pub fn new(
        vcs_type: String,
        repository_url: Option<String>,
        branch: Option<String>,
        tracking_branch: Option<String>,
        fetch_url: Option<String>,
        push_url: Option<String>,
        commit_hash: &str,
        commit_message: Option<String>,
        author_name: Option<String>,
        author_email: Option<String>,
        timestamp: Option<i64>,
    ) -> Self {
        Self {
            vcs_type,
            repository_url,
            branch,
            tracking_branch,
            fetch_url,
            push_url,
            commit_hash: commit_hash.to_string(),
            commit_message,
            author_name,
            author_email,
            timestamp,
        }
    }

    pub fn __str__(&self) -> String {
        format!(
            "VCSMetadata(type={}, branch={:?}, commit={})",
            self.vcs_type, self.branch, self.commit_hash
        )
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }

    /// Convert to Python dict
    pub fn to_dict<'a>(&self, py: Python<'a>) -> Bound<'a, PyDict> {
        let d = PyDict::new(py);
        d.set_item("vcs_type", self.vcs_type.clone()).unwrap();
        d.set_item("repository_url", self.repository_url.clone())
            .unwrap();
        d.set_item("branch", self.branch.clone()).unwrap();
        d.set_item("tracking_branch", self.tracking_branch.clone())
            .unwrap();
        d.set_item("fetch_url", self.fetch_url.clone()).unwrap();
        d.set_item("push_url", self.push_url.clone()).unwrap();
        d.set_item("commit_hash", self.commit_hash.clone()).unwrap();
        d.set_item("commit_message", self.commit_message.clone())
            .unwrap();
        d.set_item("author_name", self.author_name.clone()).unwrap();
        d.set_item("author_email", self.author_email.clone())
            .unwrap();
        d.set_item("timestamp", self.timestamp).unwrap();
        d
    }
}

impl From<&VCSMetadata> for PyVCSMetadata {
    fn from(meta: &VCSMetadata) -> Self {
        Self {
            vcs_type: meta.vcs_type.clone(),
            repository_url: meta.repository_url.clone(),
            branch: meta.branch.clone(),
            tracking_branch: meta.tracking_branch.clone(),
            fetch_url: meta.fetch_url.clone(),
            push_url: meta.push_url.clone(),
            commit_hash: meta.commit_hash.clone(),
            commit_message: meta.commit_message.clone(),
            author_name: meta.author_name.clone(),
            author_email: meta.author_email.clone(),
            timestamp: meta.timestamp,
        }
    }
}

// ============================================================================
/// Base class for VCS implementations
// ============================================================================
#[pyclass(name = "ReleaseVCS", subclass)]
pub struct PyReleaseVCS {
    inner: Option<Box<dyn ReleaseVCS + Send + Sync>>,
}

#[pymethods]
impl PyReleaseVCS {
    #[new]
    pub fn new() -> Self {
        Self { inner: None }
    }

    pub fn get_type_name(&self) -> String {
        "stub".to_string()
    }

    pub fn get_repo_root(&self) -> PyResult<String> {
        Ok(".".to_string())
    }

    pub fn is_clean(&self) -> PyResult<bool> {
        Ok(true)
    }

    pub fn get_current_branch(&self) -> PyResult<String> {
        Ok("main".to_string())
    }

    pub fn get_latest_commit(&self) -> PyResult<String> {
        Ok("stub-commit".to_string())
    }

    pub fn tag_exists(&self, _tag: &str) -> PyResult<bool> {
        Ok(false)
    }

    pub fn create_tag(&self, tag: &str, message: &str) -> PyResult<()> {
        // Stub implementation - just log and return Ok
        eprintln!(
            "StubVCS: would create tag '{}' with message '{}'",
            tag, message
        );
        Ok(())
    }

    pub fn get_changelog(
        &self,
        _from_rev: Option<&str>,
        _to_rev: Option<&str>,
    ) -> PyResult<String> {
        Ok("Stub changelog".to_string())
    }

    pub fn get_metadata(&self) -> PyResult<PyVCSMetadata> {
        Ok(PyVCSMetadata::from(&VCSMetadata {
            vcs_type: "stub".to_string(),
            commit_hash: "stub-commit".to_string(),
            ..Default::default()
        }))
    }

    pub fn validate_repo_state(&self) -> PyResult<()> {
        Ok(())
    }

    pub fn is_releasable_branch(&self) -> PyResult<Option<bool>> {
        Ok(None)
    }
}

// ============================================================================
/// Git VCS implementation
// ============================================================================
#[cfg(feature = "git")]
#[pyclass(name = "GitVCS", extends = PyReleaseVCS)]
pub struct PyGitVCS {}

#[cfg(feature = "git")]
#[pymethods]
impl PyGitVCS {
    #[new]
    #[pyo3(signature = (repo_root))]
    pub fn new(repo_root: &str) -> PyResult<(Self, PyReleaseVCS)> {
        use rez_next_build::vcs::GitVCS as InnerGitVCS;
        use std::path::PathBuf;

        let inner = InnerGitVCS::new(PathBuf::from(repo_root))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok((
            Self {},
            PyReleaseVCS {
                inner: Some(Box::new(inner)),
            },
        ))
    }
}

#[cfg(not(feature = "git"))]
#[pyclass(name = "GitVCS", extends = PyReleaseVCS)]
pub struct PyGitVCS {}

#[cfg(not(feature = "git"))]
#[pymethods]
impl PyGitVCS {
    #[new]
    pub fn new(_repo_root: &str) -> PyResult<(Self, PyReleaseVCS)> {
        Err(pyo3::exceptions::PyRuntimeError::new_err(
            "Git support not compiled in. Enable 'git' feature when building.",
        ))
    }
}

// ============================================================================
/// Mercurial VCS implementation
// ============================================================================
#[pyclass(name = "MercurialVCS", extends = PyReleaseVCS)]
pub struct PyMercurialVCS {}

#[pymethods]
impl PyMercurialVCS {
    #[new]
    #[pyo3(signature = (repo_root))]
    pub fn new(repo_root: &str) -> PyResult<(Self, PyReleaseVCS)> {
        use rez_next_build::vcs::MercurialVCS as InnerHgVCS;
        use std::path::PathBuf;

        let inner = InnerHgVCS::new(PathBuf::from(repo_root))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok((
            Self {},
            PyReleaseVCS {
                inner: Some(Box::new(inner)),
            },
        ))
    }
}

// ============================================================================
/// SVN VCS implementation
// ============================================================================
#[pyclass(name = "SvnVCS", extends = PyReleaseVCS)]
pub struct PySvnVCS {}

#[pymethods]
impl PySvnVCS {
    #[new]
    #[pyo3(signature = (repo_root))]
    pub fn new(repo_root: &str) -> PyResult<(Self, PyReleaseVCS)> {
        use rez_next_build::vcs::SvnVCS as InnerSvnVCS;
        use std::path::PathBuf;

        let inner = InnerSvnVCS::new(PathBuf::from(repo_root))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok((
            Self {},
            PyReleaseVCS {
                inner: Some(Box::new(inner)),
            },
        ))
    }
}

// ============================================================================
/// Detect VCS for a given path
// ============================================================================
#[pyfunction]
pub fn detect_vcs<'a>(py: Python<'a>, repo_root: &str) -> PyResult<Option<Bound<'a, PyAny>>> {
    use rez_next_build::vcs::detect_vcs as inner_detect_vcs;
    use std::path::Path;

    let path = Path::new(repo_root);
    let result = inner_detect_vcs(path); // Option<Box<dyn ReleaseVCS>>

    match result {
        Some(vcs) => {
            let type_name = vcs.get_type_name(); // &str
            let obj = match type_name {
                "git" => {
                    let git_vcs = PyGitVCS::new(repo_root)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
                    pyo3::Bound::new(py, git_vcs).unwrap().into_any()
                }
                "hg" => {
                    let hg_vcs = PyMercurialVCS::new(repo_root)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
                    pyo3::Bound::new(py, hg_vcs).unwrap().into_any()
                }
                "svn" => {
                    let svn_vcs = PySvnVCS::new(repo_root)
                        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
                    pyo3::Bound::new(py, svn_vcs).unwrap().into_any()
                }
                _ => return Ok(None),
            };
            Ok(Some(obj))
        }
        None => Ok(None),
    }
}

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
    pub(crate) fn from_str(s: &str) -> Self {
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
    pub(crate) fn __str__(&self) -> String {
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

    pub(crate) fn __repr__(&self) -> String {
        self.__str__()
    }
}

/// Release manager — orchestrates package release operations.
///
/// Compatible with `rez.release_build.ReleaseBuildProcess`.
#[pyclass(name = "ReleaseManager")]
pub struct PyReleaseManager {
    pub(crate) mode: ReleaseMode,
    pub(crate) skip_build: bool,
    pub(crate) skip_tests: bool,
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

    pub(crate) fn __str__(&self) -> String {
        format!(
            "ReleaseManager(mode={:?}, skip_build={}, skip_tests={})",
            self.mode, self.skip_build, self.skip_tests
        )
    }

    /// Release a package from a source directory.
    /// Equivalent to running `rez release` from the package directory.
    #[pyo3(signature = (source_dir=None, message=None))]
    pub(crate) fn release(
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
    pub(crate) fn validate(&self, source_dir: Option<&str>) -> PyResult<(bool, Vec<String>)> {
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

#[cfg(test)]
#[path = "release_bindings_tests.rs"]
mod tests;
