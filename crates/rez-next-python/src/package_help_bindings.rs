//! Python bindings for PackageHelp.

use pyo3::prelude::*;

use rez_next_package::{HelpSection, PackageHelp};
use rez_next_package::Package;
use rez_next_version::VersionRange;
use crate::package_bindings::PyPackage;

/// Python wrapper for HelpSection
#[pyclass(name = "HelpSection", skip_from_py_object)]
#[derive(Clone)]
pub struct PyHelpSection {
    inner: HelpSection,
}

#[pymethods]
impl PyHelpSection {
    /// Create a new HelpSection
    #[new]
    fn new(name: String, uri: String) -> Self {
        Self {
            inner: HelpSection { name, uri },
        }
    }

    /// Get the section name
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// Set the section name
    #[setter]
    fn set_name(&mut self, name: String) {
        self.inner.name = name;
    }

    /// Get the section URI
    #[getter]
    fn uri(&self) -> String {
        self.inner.uri.clone()
    }

    /// Set the section URI
    #[setter]
    fn set_uri(&mut self, uri: String) {
        self.inner.uri = uri;
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "HelpSection(name={:?}, uri={:?})",
            self.inner.name, self.inner.uri
        )
    }
}

/// Python wrapper for PackageHelp
#[pyclass(name = "PackageHelp")]
pub struct PyPackageHelp {
    inner: PackageHelp,
}

#[pymethods]
impl PyPackageHelp {
    /// Create a new PackageHelp object.
    ///
    /// Args:
    ///     package_name: Package to search
    ///     version_range: Optional version range string (e.g., ">=1.0")
    ///     packages: List of Package objects to search
    #[new]
    fn new(package_name: String, version_range: Option<String>, packages: Vec<PyPackage>) -> PyResult<Self> {
        // Parse version range if provided
        let version_range_parsed = if let Some(range_str) = version_range {
            match VersionRange::parse(&range_str) {
                Ok(range) => Some(range),
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Invalid version range: {}", e),
                    ))
                }
            }
        } else {
            None
        };

        // Convert PyPackage objects to Package
        let packages_rust: Vec<Package> = packages.into_iter().map(|p| p.0.clone()).collect();

        // Create PackageHelp
        let inner = PackageHelp::new(&package_name, version_range_parsed.as_ref(), &packages_rust);

        Ok(Self { inner })
    }

    /// Check if help was found.
    #[getter]
    fn success(&self) -> bool {
        self.inner.success()
    }

    /// Get help sections.
    #[getter]
    fn sections(&self) -> Vec<PyHelpSection> {
        self.inner
            .sections()
            .iter()
            .map(|s| PyHelpSection {
                inner: HelpSection {
                    name: s.name.clone(),
                    uri: s.uri.clone(),
                },
            })
            .collect()
    }

    /// Print help sections.
    fn print_info(&self) {
        for (i, section) in self.inner.sections().iter().enumerate() {
            println!("  {}:\t{} ({})", i + 1, section.name, section.uri);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_section_creation() {
        if Python::try_attach(|_py| {
            let section = PyHelpSection::new("Documentation".to_string(), "https://example.com".to_string());
            assert_eq!(section.name(), "Documentation");
            assert_eq!(section.uri(), "https://example.com");
        })
        .is_some()
        {
            // Test passed
        }
    }

    #[test]
    fn test_package_help_creation() {
        if Python::try_attach(|_py| {
            let result = PyPackageHelp::new("mypackage".to_string(), None, Vec::new());
            assert!(result.is_ok());
        })
        .is_some()
        {
            // Test passed
        }
    }
}
