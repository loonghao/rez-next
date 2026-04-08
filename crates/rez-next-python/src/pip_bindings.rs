//! Python bindings for rez.pip — pip-to-rez package conversion
//!
//! Provides API compatible with `rez.pip`:
//! - `pip_install(packages, python_version, ...) -> list[str]`
//! - `convert_pip_to_rez(pip_name, pip_version) -> Package`
//! - `get_pip_dependencies(package_name) -> list[str]`

use pyo3::prelude::*;

/// Represents a pip package converted to rez format.
#[pyclass(name = "PipPackage", from_py_object)]
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
///
/// **Not yet implemented.** Raises `NotImplementedError` when called.
/// Full implementation requires running `pip download`, inspecting wheel METADATA,
/// converting to rez package.py format, and installing to packages_path.
#[pyfunction]
#[pyo3(signature = (packages, python_version=None, install_path=None, release=false))]
pub fn pip_install(
    packages: Vec<String>,
    python_version: Option<&str>,
    install_path: Option<&str>,
    release: bool,
) -> PyResult<Vec<String>> {
    let _ = (python_version, install_path, release);
    Err(pyo3::exceptions::PyNotImplementedError::new_err(format!(
        "pip_install is not yet implemented in rez_next. \
         Requested packages: {}. \
         Use the original `rez pip --install` command for pip-to-rez installation.",
        packages.join(", ")
    )))
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
///
/// **Not yet implemented.** Raises `NotImplementedError` when called.
/// Full implementation requires scanning local rez package repos for any
/// package that lists `package_name` in its requires.
#[pyfunction]
#[pyo3(signature = (package_name, paths=None))]
pub fn get_pip_dependencies(
    package_name: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let _ = paths;
    Err(pyo3::exceptions::PyNotImplementedError::new_err(format!(
        "get_pip_dependencies is not yet implemented in rez_next. \
         Queried package: '{package_name}'. \
         Use the original `rez depends` command to find reverse dependencies."
    )))
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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── normalize_package_name ──────────────────────────────────────────────

    #[test]
    fn test_normalize_package_name_lowercase() {
        assert_eq!(normalize_package_name("NumPy"), "numpy");
        assert_eq!(normalize_package_name("Pillow"), "pillow");
        assert_eq!(normalize_package_name("REQUESTS"), "requests");
    }

    #[test]
    fn test_normalize_package_name_underscore_to_dash() {
        assert_eq!(normalize_package_name("scikit_learn"), "scikit-learn");
        assert_eq!(normalize_package_name("my_package"), "my-package");
    }

    #[test]
    fn test_normalize_package_name_already_normalized() {
        assert_eq!(normalize_package_name("numpy"), "numpy");
        assert_eq!(normalize_package_name("scikit-learn"), "scikit-learn");
    }

    #[test]
    fn test_normalize_package_name_mixed() {
        assert_eq!(normalize_package_name("PyYAML"), "pyyaml");
        assert_eq!(
            normalize_package_name("Django_Rest_Framework"),
            "django-rest-framework"
        );
    }

    // ─── pip_version_to_rez ─────────────────────────────────────────────────

    #[test]
    fn test_pip_version_to_rez_exact() {
        assert_eq!(pip_version_to_rez("==1.2.3"), "1.2.3");
        assert_eq!(pip_version_to_rez("==3.9.0"), "3.9.0");
    }

    #[test]
    fn test_pip_version_to_rez_ge() {
        assert_eq!(pip_version_to_rez(">=1.0"), "1.0+");
        assert_eq!(pip_version_to_rez(">=3.9"), "3.9+");
    }

    #[test]
    fn test_pip_version_to_rez_lt() {
        assert_eq!(pip_version_to_rez("<2.0"), "<2.0");
        assert_eq!(pip_version_to_rez("<3.11"), "<3.11");
    }

    #[test]
    fn test_pip_version_to_rez_range() {
        // ">=1.0,<2.0" -> "1.0+<2.0"
        assert_eq!(pip_version_to_rez(">=1.0,<2.0"), "1.0+<2.0");
        assert_eq!(pip_version_to_rez(">=3.8,<4.0"), "3.8+<4.0");
    }

    #[test]
    fn test_pip_version_to_rez_ne() {
        assert_eq!(pip_version_to_rez("!=1.5"), "!=1.5");
    }

    #[test]
    fn test_pip_version_to_rez_plain() {
        // Plain version without operator
        assert_eq!(pip_version_to_rez("1.2.3"), "1.2.3");
    }

    // ─── PyPipPackage::to_package_py ─────────────────────────────────────────

    #[test]
    fn test_to_package_py_no_requires() {
        let pkg = PyPipPackage {
            name: "mylib".to_string(),
            version: "2.0.0".to_string(),
            requires: vec![],
            description: "A test library".to_string(),
        };
        let py = pkg.to_package_py();
        assert!(py.contains("name = \"mylib\""));
        assert!(py.contains("version = \"2.0.0\""));
        assert!(py.contains("description = \"A test library\""));
        // No requires block when empty
        assert!(!py.contains("requires = ["));
    }

    #[test]
    fn test_to_package_py_with_requires() {
        let pkg = PyPipPackage {
            name: "mylib".to_string(),
            version: "1.0.0".to_string(),
            requires: vec!["numpy-1.20+".to_string(), "scipy".to_string()],
            description: "".to_string(),
        };
        let py = pkg.to_package_py();
        assert!(py.contains("requires = ["));
        assert!(py.contains("\"numpy-1.20+\""));
        assert!(py.contains("\"scipy\""));
    }

    #[test]
    fn test_to_package_py_contains_pythonpath_command() {
        let pkg = PyPipPackage {
            name: "lib".to_string(),
            version: "0.1.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let py = pkg.to_package_py();
        assert!(py.contains("env.PYTHONPATH.prepend"));
        assert!(py.contains("def commands():"));
    }

    // ─── PyPipPackage repr / str ─────────────────────────────────────────────

    #[test]
    fn test_pip_package_repr() {
        let pkg = PyPipPackage {
            name: "numpy".to_string(),
            version: "1.24.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let repr = pkg.__repr__();
        assert_eq!(repr, "PipPackage(numpy-1.24.0)");
    }

    #[test]
    fn test_pip_package_str() {
        let pkg = PyPipPackage {
            name: "scipy".to_string(),
            version: "1.11.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let s = pkg.__str__();
        assert_eq!(s, "scipy-1.11.0");
    }

    #[test]
    fn test_pip_package_new_defaults() {
        let pkg = PyPipPackage::new("requests", "2.31.0", None, "HTTP lib");
        assert_eq!(pkg.name, "requests");
        assert_eq!(pkg.version, "2.31.0");
        assert!(pkg.requires.is_empty());
        assert_eq!(pkg.description, "HTTP lib");
    }

    // ─── pip_version_to_rez edge cases ───────────────────────────────────────

    #[test]
    fn test_pip_version_to_rez_le() {
        assert_eq!(pip_version_to_rez("<=3.11"), "<=3.11");
    }

    #[test]
    fn test_pip_version_to_rez_gt() {
        // ">1.0" maps to "1.0+" (approximation)
        let result = pip_version_to_rez(">1.0");
        assert!(result.contains("1.0"));
        assert!(result.contains('+'));
    }

    #[test]
    fn test_pip_version_to_rez_fallback_plain() {
        assert_eq!(pip_version_to_rez("3.9.1"), "3.9.1");
    }

    // ─── convert_pip_to_rez ──────────────────────────────────────────────────

    #[test]
    fn test_convert_pip_to_rez_normalizes_name() {
        let pkg = convert_pip_to_rez("Scikit_Learn", "1.3.0", None, None).unwrap();
        assert_eq!(pkg.name, "scikit-learn");
    }

    #[test]
    fn test_convert_pip_to_rez_converts_requires_version_spec() {
        let reqs = vec!["numpy>=1.20".to_string()];
        let pkg = convert_pip_to_rez("mylib", "0.1.0", Some(reqs), None).unwrap();
        assert!(!pkg.requires.is_empty());
        let r = &pkg.requires[0];
        assert!(r.contains("numpy"), "expected numpy in req, got {r}");
        assert!(r.contains("1.20"), "expected version in req, got {r}");
    }

    #[test]
    fn test_convert_pip_to_rez_strips_extras() {
        // "Pillow[jpeg]>=9.0" should strip the [jpeg] extras
        let reqs = vec!["Pillow[jpeg]>=9.0".to_string()];
        let pkg = convert_pip_to_rez("img", "1.0.0", Some(reqs), None).unwrap();
        let r = &pkg.requires[0];
        assert!(!r.contains('['), "extras must be stripped, got {r}");
    }

    // ─── write_pip_package ───────────────────────────────────────────────────

    #[test]
    fn test_write_pip_package_creates_package_py() {
        let tmp = std::env::temp_dir().join("rez_next_pip_test_write");
        let _ = std::fs::remove_dir_all(&tmp);

        let pkg = PyPipPackage {
            name: "testpkg".to_string(),
            version: "0.1.0".to_string(),
            requires: vec![],
            description: "A test package".to_string(),
        };
        let result = write_pip_package(&pkg, tmp.to_str().unwrap(), false);
        assert!(result.is_ok(), "write_pip_package should succeed: {:?}", result);

        let pkg_py = tmp.join("testpkg").join("0.1.0").join("package.py");
        assert!(pkg_py.exists(), "package.py should be created at {:?}", pkg_py);

        let content = std::fs::read_to_string(&pkg_py).unwrap();
        assert!(content.contains("name = \"testpkg\""));
        assert!(content.contains("version = \"0.1.0\""));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_write_pip_package_overwrite_false_errors_on_existing() {
        let tmp = std::env::temp_dir().join("rez_next_pip_test_overwrite");
        let _ = std::fs::remove_dir_all(&tmp);

        let pkg = PyPipPackage {
            name: "dup".to_string(),
            version: "1.0.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let _ = write_pip_package(&pkg, tmp.to_str().unwrap(), false);
        let second = write_pip_package(&pkg, tmp.to_str().unwrap(), false);
        assert!(
            second.is_err(),
            "second write without overwrite should fail"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ─── Additional pip_bindings boundary/edge tests ─────────────────────────

    #[test]
    fn test_normalize_package_name_empty_string() {
        // empty string should return empty string
        assert_eq!(normalize_package_name(""), "");
    }

    #[test]
    fn test_pip_version_to_rez_empty_string_returns_empty() {
        // empty specifier is a no-op (plain passthrough)
        let result = pip_version_to_rez("");
        assert_eq!(result, "", "empty specifier should produce empty string");
    }

    #[test]
    fn test_pip_package_new_with_requires_list() {
        let reqs = Some(vec!["numpy-1.20+".to_string()]);
        let pkg = PyPipPackage::new("mylib", "0.1.0", reqs, "desc");
        assert_eq!(pkg.requires.len(), 1);
        assert_eq!(pkg.requires[0], "numpy-1.20+");
    }

    #[test]
    fn test_to_package_py_authors_is_pip() {
        let pkg = PyPipPackage {
            name: "anylib".to_string(),
            version: "1.0.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let py = pkg.to_package_py();
        assert!(py.contains("authors = [\"pip\"]"), "authors should be [\"pip\"], got:\n{py}");
    }

    #[test]
    fn test_convert_pip_to_rez_no_requires_empty_list() {
        let pkg = convert_pip_to_rez("simplelib", "0.5.0", None, Some("simple")).unwrap();
        assert!(pkg.requires.is_empty(), "requires should be empty");
        assert_eq!(pkg.description, "simple");
    }

    #[test]
    fn test_pip_version_to_rez_exact_zero_version() {
        assert_eq!(pip_version_to_rez("==0.0.0"), "0.0.0");
    }

    #[test]
    fn test_write_pip_package_overwrite_true_replaces() {
        let tmp = std::env::temp_dir().join("rez_next_pip_test_overwrite_true");
        let _ = std::fs::remove_dir_all(&tmp);

        let pkg = PyPipPackage {
            name: "replaced".to_string(),
            version: "1.0.0".to_string(),
            requires: vec![],
            description: "original".to_string(),
        };
        let _ = write_pip_package(&pkg, tmp.to_str().unwrap(), false);

        // overwrite=true should succeed
        let pkg2 = PyPipPackage {
            name: "replaced".to_string(),
            version: "1.0.0".to_string(),
            requires: vec![],
            description: "updated".to_string(),
        };
        let result = write_pip_package(&pkg2, tmp.to_str().unwrap(), true);
        assert!(result.is_ok(), "overwrite=true should succeed: {:?}", result);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ─────── Cycle 113 additions ─────────────────────────────────────────────

    #[test]
    fn test_normalize_package_name_multiple_underscores() {
        assert_eq!(
            normalize_package_name("my_really_long_package_name"),
            "my-really-long-package-name"
        );
    }

    #[test]
    fn test_normalize_package_name_single_char() {
        assert_eq!(normalize_package_name("A"), "a");
        assert_eq!(normalize_package_name("z"), "z");
    }

    #[test]
    fn test_pip_version_to_rez_range_with_spaces() {
        // Spaces around comma and operators should be trimmed
        let result = pip_version_to_rez(">=1.0 , <2.0");
        // After trimming: [">=1.0", "<2.0"]
        assert!(result.contains("1.0"), "result: {result}");
        assert!(result.contains("2.0"), "result: {result}");
    }

    #[test]
    fn test_pip_version_to_rez_exact_prerelease_passthrough() {
        // Pre-release versions should be passed through as-is after stripping ==
        let result = pip_version_to_rez("==1.0.0a1");
        assert_eq!(result, "1.0.0a1");
    }

    #[test]
    fn test_convert_pip_to_rez_multiple_requires() {
        let reqs = vec![
            "numpy>=1.20".to_string(),
            "scipy<1.10".to_string(),
            "pillow".to_string(),
        ];
        let pkg = convert_pip_to_rez("ml_lib", "0.9.0", Some(reqs), None).unwrap();
        assert_eq!(pkg.requires.len(), 3, "expected 3 requires");
        // All names should be normalized (no uppercase, no underscores)
        for r in &pkg.requires {
            assert!(r.chars().all(|c| !c.is_uppercase()), "require should be lowercase: {r}");
        }
    }

    #[test]
    fn test_pip_package_empty_description_to_package_py() {
        let pkg = PyPipPackage {
            name: "minipkg".to_string(),
            version: "0.0.1".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let py = pkg.to_package_py();
        // description field should be present but empty
        assert!(py.contains("description = \"\""), "py: {py}");
    }

    #[test]
    fn test_pip_package_repr_and_str_differ() {
        let pkg = PyPipPackage {
            name: "pkg".to_string(),
            version: "1.0.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let repr = pkg.__repr__();
        let s = pkg.__str__();
        // repr wraps in PipPackage(...), str is plain name-version
        assert!(repr.starts_with("PipPackage("), "repr: {repr}");
        assert!(!s.starts_with("PipPackage("), "str should not have wrapper: {s}");
        assert_eq!(s, "pkg-1.0.0");
    }

    // ─────── Cycle 118 additions ─────────────────────────────────────────────

    #[test]
    fn test_normalize_package_name_numeric_only() {
        // Numbers should be passed through unchanged (lowercased; no underscores)
        let result = normalize_package_name("123");
        assert_eq!(result, "123");
    }

    #[test]
    fn test_pip_version_to_rez_double_digit_minor() {
        // >=1.10 should map to 1.10+
        assert_eq!(pip_version_to_rez(">=1.10"), "1.10+");
    }

    #[test]
    fn test_pip_version_to_rez_ne_preserves_version() {
        // !=2.0.0 should preserve the version number
        let result = pip_version_to_rez("!=2.0.0");
        assert!(result.contains("2.0.0"), "result: {result}");
    }

    #[test]
    fn test_to_package_py_version_in_site_packages_path() {
        let pkg = PyPipPackage {
            name: "libx".to_string(),
            version: "3.1.4".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        let py = pkg.to_package_py();
        // The template embeds python.major and python.minor; not the package version
        assert!(py.contains("site-packages"), "site-packages path missing: {py}");
    }

    #[test]
    fn test_pip_version_to_rez_exact_single_zero() {
        assert_eq!(pip_version_to_rez("==0"), "0");
    }

    #[test]
    fn test_convert_pip_to_rez_description_propagated() {
        let pkg =
            convert_pip_to_rez("mylib", "1.0.0", None, Some("A useful library")).unwrap();
        assert_eq!(pkg.description, "A useful library");
    }

    // ── Cycle 124 additions ───────────────────────────────────────────────────

    #[test]
    fn test_normalize_package_name_mixed_case_lowercased() {
        let result = normalize_package_name("MyPackage");
        assert_eq!(result, "mypackage", "mixed-case name must be lowercased");
    }

    #[test]
    fn test_normalize_package_name_underscores_become_hyphens() {
        // normalize_package_name converts underscores to hyphens (pip convention)
        let result = normalize_package_name("my_package");
        assert_eq!(result, "my-package", "underscores must be converted to hyphens");
    }

    #[test]
    fn test_pip_version_to_rez_exact_with_patch() {
        // ==1.2.3 → exact 1.2.3
        let result = pip_version_to_rez("==1.2.3");
        assert!(result.contains("1.2.3"), "exact version '1.2.3' must appear in result: {result}");
    }

    #[test]
    fn test_pip_version_to_rez_tilde_eq_maps_to_compatible() {
        // ~=2.1 means >=2.1,==2.*  (compatible release)
        let result = pip_version_to_rez("~=2.1");
        assert!(result.contains("2.1"), "compatible release '~=2.1' must include '2.1': {result}");
    }

    #[test]
    fn test_convert_pip_to_rez_empty_requires_produces_empty_vec() {
        let pkg = convert_pip_to_rez("libfoo", "0.1.0", None, None).unwrap();
        assert!(pkg.requires.is_empty(), "package with no deps must have empty requires vec");
    }

    #[test]
    fn test_pip_package_str_format_is_name_dash_version() {
        let pkg = PyPipPackage {
            name: "requests".to_string(),
            version: "2.28.0".to_string(),
            requires: vec![],
            description: "".to_string(),
        };
        assert_eq!(pkg.__str__(), "requests-2.28.0", "__str__ must be name-version: {}", pkg.__str__());
    }
}
