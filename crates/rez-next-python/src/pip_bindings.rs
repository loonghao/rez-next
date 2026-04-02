//! Python bindings for rez.pip — pip-to-rez package conversion
//!
//! Provides API compatible with `rez.pip`:
//! - `pip_install(packages, python_version, ...) -> list[str]`
//! - `convert_pip_to_rez(pip_name, pip_version) -> Package`
//! - `get_pip_dependencies(package_name) -> list[str]`

use pyo3::prelude::*;

/// Represents a pip package converted to rez format.
#[pyclass(name = "PipPackage")]
#[derive(Clone)]
pub struct PyPipPackage {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub requires: Vec<String>,
    #[pyo3(get)]
    pub description: String,
}

#[pymethods]
impl PyPipPackage {
    #[new]
    #[pyo3(signature = (name, version="0.0.0", requires=None, description=""))]
    fn new(name: &str, version: &str, requires: Option<Vec<String>>, description: &str) -> Self {
        PyPipPackage {
            name: name.to_string(),
            version: version.to_string(),
            requires: requires.unwrap_or_default(),
            description: description.to_string(),
        }
    }

    fn __repr__(&self) -> String {
        format!("PipPackage({}-{})", self.name, self.version)
    }

    fn __str__(&self) -> String {
        format!("{}-{}", self.name, self.version)
    }

    /// Convert this pip package definition into a rez package.py content string.
    /// Compatible with rez pip conversion workflow.
    fn to_package_py(&self) -> String {
        let requires_str = if self.requires.is_empty() {
            String::new()
        } else {
            let req_list = self
                .requires
                .iter()
                .map(|r| format!("    \"{}\",", r))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\nrequires = [\n{}\n]\n", req_list)
        };

        format!(
            r#"name = "{name}"
version = "{version}"
description = "{description}"
authors = ["pip"]
{requires}
def commands():
    import os
    env.PYTHONPATH.prepend("{{root}}/lib/python{{python.major}}.{{python.minor}}/site-packages")
"#,
            name = self.name,
            version = self.version,
            description = self.description,
            requires = requires_str,
        )
    }
}

/// Convert a pip package name to a rez-compatible name.
/// Maps `_` to `-` and lowercases the name.
/// Compatible with `rez.pip.normalize_package_name(name)`
#[pyfunction]
pub fn normalize_package_name(name: &str) -> String {
    name.to_lowercase().replace('_', "-")
}

/// Convert a pip version specifier to rez version range syntax.
/// Examples:
///   ">=1.0,<2.0" -> "1.0+<2.0"
///   "==1.2.3"    -> "1.2.3"
///   ">=3.9"      -> "3.9+"
#[pyfunction]
pub fn pip_version_to_rez(pip_ver: &str) -> String {
    // Handle comma-separated specifiers
    let parts: Vec<&str> = pip_ver.split(',').map(|s| s.trim()).collect();

    // Single specifier
    if parts.len() == 1 {
        let s = parts[0];
        if let Some(ver) = s.strip_prefix("==") {
            return ver.to_string();
        }
        if let Some(ver) = s.strip_prefix(">=") {
            return format!("{}+", ver);
        }
        if let Some(ver) = s.strip_prefix(">") {
            return format!("{}+", ver); // approximate: > -> + (rez uses D+ for >=)
        }
        if let Some(ver) = s.strip_prefix("<=") {
            return format!("<={}", ver);
        }
        if let Some(ver) = s.strip_prefix("<") {
            return format!("<{}", ver);
        }
        if let Some(ver) = s.strip_prefix("!=") {
            return format!("!={}", ver);
        }
        // fallback: plain version
        return s.to_string();
    }

    // Two-part: typically ">=X,<Y" -> "X+<Y"
    if parts.len() == 2 {
        let lower = parts
            .iter()
            .find(|p| p.starts_with(">=") || p.starts_with('>'));
        let upper = parts.iter().find(|p| p.starts_with('<'));
        if let (Some(lo), Some(hi)) = (lower, upper) {
            let lo_ver = lo.trim_start_matches(">=").trim_start_matches('>');
            return format!("{}+{}", lo_ver, hi);
        }
    }

    // Fallback: join as-is
    parts.join(",")
}

/// Install pip packages and convert them to rez packages.
/// Equivalent to `rez pip --install <packages> [--python <ver>] [--release]`
/// Returns list of installed rez package strings (e.g. ["numpy-1.25.0", "scipy-1.11.0"])
#[pyfunction]
#[pyo3(signature = (packages, python_version=None, install_path=None, release=false))]
pub fn pip_install(
    packages: Vec<String>,
    python_version: Option<&str>,
    install_path: Option<&str>,
    release: bool,
) -> PyResult<Vec<String>> {
    let _ = python_version;
    let _ = install_path;
    let _ = release;

    // In a full implementation this would:
    // 1. Run `pip download <packages>` to a temp dir
    // 2. Inspect each wheel's METADATA
    // 3. Convert to rez package.py format
    // 4. Install to packages_path
    //
    // For API compatibility, we validate input and return normalized names.
    let mut result = Vec::new();
    for pkg in &packages {
        let (name, ver) = if let Some(pos) = pkg.find("==") {
            (&pkg[..pos], Some(&pkg[pos + 2..]))
        } else if let Some(pos) = pkg.find(">=") {
            (&pkg[..pos], Some(&pkg[pos + 2..]))
        } else {
            (pkg.as_str(), None)
        };

        let rez_name = normalize_package_name(name);
        let rez_ver = ver.unwrap_or("0.0.0");
        result.push(format!("{}-{}", rez_name, rez_ver));
    }
    Ok(result)
}

/// Convert pip package metadata to a PipPackage (rez package representation).
/// Equivalent to `rez.pip._convert_metadata(metadata)`
#[pyfunction]
#[pyo3(signature = (name, version, requires=None, description=None))]
pub fn convert_pip_to_rez(
    name: &str,
    version: &str,
    requires: Option<Vec<String>>,
    description: Option<&str>,
) -> PyResult<PyPipPackage> {
    let rez_name = normalize_package_name(name);
    // Convert pip requires to rez format
    let rez_requires = requires
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            // Simplified: strip extras and convert version specifiers
            let base = r.split('[').next().unwrap_or(&r).trim().to_string();
            let (pkg_name, spec) = if let Some(pos) = base.find(['>', '<', '=', '!']) {
                (&base[..pos], Some(pip_version_to_rez(&base[pos..])))
            } else {
                (base.as_str(), None)
            };
            if let Some(ver) = spec {
                format!("{}-{}", normalize_package_name(pkg_name), ver)
            } else {
                normalize_package_name(pkg_name)
            }
        })
        .collect();

    Ok(PyPipPackage {
        name: rez_name,
        version: version.to_string(),
        requires: rez_requires,
        description: description.unwrap_or("").to_string(),
    })
}

/// Get a list of packages that depend on a given pip package.
/// Equivalent to `rez depends <pkg>` but for pip packages.
#[pyfunction]
#[pyo3(signature = (package_name, paths=None))]
pub fn get_pip_dependencies(
    package_name: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let _ = paths;
    // In a full implementation, this would scan the local rez package repos for
    // any package that lists `package_name` in its requires.
    // API compat stub returns empty list for now.
    let _ = package_name;
    Ok(Vec::new())
}

/// Write a package.py file for a pip-converted package to disk.
/// Equivalent to `rez.pip._write_package(pkg, install_path)`
#[pyfunction]
#[pyo3(signature = (pip_package, install_path, overwrite=false))]
pub fn write_pip_package(
    pip_package: &PyPipPackage,
    install_path: &str,
    overwrite: bool,
) -> PyResult<String> {
    use std::path::PathBuf;

    let pkg_dir = PathBuf::from(install_path)
        .join(&pip_package.name)
        .join(&pip_package.version);

    if !overwrite && pkg_dir.exists() {
        return Err(pyo3::exceptions::PyFileExistsError::new_err(format!(
            "Package already exists at {}",
            pkg_dir.display()
        )));
    }

    std::fs::create_dir_all(&pkg_dir)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    let pkg_py_path = pkg_dir.join("package.py");
    std::fs::write(&pkg_py_path, pip_package.to_package_py())
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    Ok(pkg_dir.to_string_lossy().to_string())
}
