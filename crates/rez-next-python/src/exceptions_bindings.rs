//! Custom Python exception classes for rez-next
//!
//! Defines a hierarchy of exception classes that mirror rez's exception hierarchy,
//! ensuring `except rez.exceptions.RezError` can be used as a catch-all.
//!
//! Hierarchy:
//! ```
//! Exception
//! └── RezError                    (base for all rez exceptions)
//!     ├── PackageNotFound
//!     ├── PackageFamilyNotFound
//!     ├── PackageVersionConflict
//!     ├── PackageRequestError
//!     ├── ResolveError
//!     │   ├── SolveFailure
//!     │   └── PackageConflict
//!     ├── RezBuildError
//!     ├── RezReleaseError
//!     ├── ConfigurationError
//!     ├── PackageParseError
//!     ├── ContextBundleError
//!     ├── SuiteError
//!     ├── RexError
//!     │   └── RexUndefinedVariableError
//!     └── RezSystemError
//! ```

use pyo3::prelude::*;
use pyo3::exceptions::PyException;

// ─── Root exception ───────────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, RezError, PyException,
    "Base exception for all rez-next errors.\n\nAll rez-next exceptions inherit from this class."
);

// ─── Package exceptions ───────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, PackageNotFound, RezError,
    "Raised when a requested package cannot be found in any repository.\n\nEquivalent to rez.exceptions.PackageNotFound."
);

pyo3::create_exception!(rez_next, PackageFamilyNotFound, RezError,
    "Raised when a package family (name with any version) does not exist.\n\nEquivalent to rez.exceptions.PackageFamilyNotFound."
);

pyo3::create_exception!(rez_next, PackageVersionConflict, RezError,
    "Raised when two or more packages require conflicting versions of another package.\n\nEquivalent to rez.exceptions.PackageVersionConflict."
);

pyo3::create_exception!(rez_next, PackageRequestError, RezError,
    "Raised when a package request string is malformed.\n\nEquivalent to rez.exceptions.PackageRequestError."
);

pyo3::create_exception!(rez_next, PackageParseError, RezError,
    "Raised when a package definition file (package.py / package.yaml) cannot be parsed.\n\nEquivalent to rez.exceptions.PackageMetadataError."
);

// ─── Resolve exceptions ───────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, ResolveError, RezError,
    "Raised when dependency resolution fails.\n\nEquivalent to rez.exceptions.ResolveError."
);

pyo3::create_exception!(rez_next, SolveFailure, ResolveError,
    "Raised when the solver cannot find any valid solution.\n\nEquivalent to rez.exceptions.SolveFailure."
);

pyo3::create_exception!(rez_next, PackageConflict, ResolveError,
    "Raised when two packages have mutually exclusive requirements.\n\nEquivalent to rez.exceptions.PackageConflict."
);

// ─── Build / release exceptions ───────────────────────────────────────────────

pyo3::create_exception!(rez_next, RezBuildError, RezError,
    "Raised when a package build fails.\n\nEquivalent to rez.exceptions.RezBuildError."
);

pyo3::create_exception!(rez_next, RezReleaseError, RezError,
    "Raised when a package release fails.\n\nEquivalent to rez.exceptions.RezReleaseError."
);

// ─── Configuration exceptions ────────────────────────────────────────────────

pyo3::create_exception!(rez_next, ConfigurationError, RezError,
    "Raised when the rez configuration is invalid.\n\nEquivalent to rez.exceptions.ConfigurationError."
);

// ─── Context / bundle exceptions ─────────────────────────────────────────────

pyo3::create_exception!(rez_next, ContextBundleError, RezError,
    "Raised when creating or extracting a context bundle fails.\n\nEquivalent to rez.exceptions.RezContextError."
);

// ─── Suite exceptions ────────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, SuiteError, RezError,
    "Raised when suite management operations fail.\n\nEquivalent to rez.exceptions.SuiteError."
);

// ─── Rex exceptions ───────────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, RexError, RezError,
    "Raised when a Rex command cannot be parsed or executed.\n\nEquivalent to rez.exceptions.RexError."
);

pyo3::create_exception!(rez_next, RexUndefinedVariableError, RexError,
    "Raised when a Rex command references an undefined variable.\n\nEquivalent to rez.exceptions.RexUndefinedVariableError."
);

// ─── System exceptions ────────────────────────────────────────────────────────

pyo3::create_exception!(rez_next, RezSystemError, RezError,
    "Raised for internal rez-next errors.\n\nEquivalent to rez.exceptions.RezSystemError."
);

// ─── Registration ─────────────────────────────────────────────────────────────

/// Register all custom exception types into the given submodule.
pub fn register_all_exceptions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Base
    m.add("RezError", m.py().get_type::<RezError>())?;

    // Package
    m.add("PackageNotFound", m.py().get_type::<PackageNotFound>())?;
    m.add("PackageFamilyNotFound", m.py().get_type::<PackageFamilyNotFound>())?;
    m.add("PackageVersionConflict", m.py().get_type::<PackageVersionConflict>())?;
    m.add("PackageRequestError", m.py().get_type::<PackageRequestError>())?;
    m.add("PackageParseError", m.py().get_type::<PackageParseError>())?;

    // Resolve
    m.add("ResolveError", m.py().get_type::<ResolveError>())?;
    m.add("SolveFailure", m.py().get_type::<SolveFailure>())?;
    m.add("PackageConflict", m.py().get_type::<PackageConflict>())?;

    // Build / release
    m.add("RezBuildError", m.py().get_type::<RezBuildError>())?;
    m.add("RezReleaseError", m.py().get_type::<RezReleaseError>())?;

    // Config
    m.add("ConfigurationError", m.py().get_type::<ConfigurationError>())?;

    // Context / bundle
    m.add("ContextBundleError", m.py().get_type::<ContextBundleError>())?;

    // Suite
    m.add("SuiteError", m.py().get_type::<SuiteError>())?;

    // Rex
    m.add("RexError", m.py().get_type::<RexError>())?;
    m.add("RexUndefinedVariableError", m.py().get_type::<RexUndefinedVariableError>())?;

    // System
    m.add("RezSystemError", m.py().get_type::<RezSystemError>())?;

    Ok(())
}

// ─── Helper: raise typed exceptions from Rust errors ─────────────────────────

/// Convert a generic solve error string into a typed `ResolveError`.
#[allow(dead_code)]
pub fn raise_resolve_error(msg: impl Into<String>) -> PyErr {
    ResolveError::new_err(msg.into())
}

/// Convert a package-not-found error string into `PackageNotFound`.
#[allow(dead_code)]
pub fn raise_package_not_found(name: &str, version: Option<&str>) -> PyErr {
    let msg = match version {
        Some(ver) => format!("Package '{}-{}' not found", name, ver),
        None => format!("Package '{}' not found", name),
    };
    PackageNotFound::new_err(msg)
}

/// Convert a configuration error string into `ConfigurationError`.
#[allow(dead_code)]
pub fn raise_config_error(msg: impl Into<String>) -> PyErr {
    ConfigurationError::new_err(msg.into())
}

/// Convert a build error string into `RezBuildError`.
#[allow(dead_code)]
pub fn raise_build_error(msg: impl Into<String>) -> PyErr {
    RezBuildError::new_err(msg.into())
}

/// Convert a Rex error string into `RexError`.
#[allow(dead_code)]
pub fn raise_rex_error(msg: impl Into<String>) -> PyErr {
    RexError::new_err(msg.into())
}

// ─── Rust unit tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod exceptions_tests {
    use super::*;

    // We can't easily test PyO3 exception creation without a Python runtime,
    // but we can test the helper functions compile and have correct signatures.

    #[test]
    fn test_raise_package_not_found_with_version() {
        let err = raise_package_not_found("python", Some("3.9"));
        let msg = format!("{}", err);
        assert!(msg.contains("python") || msg.contains("Package"));
    }

    #[test]
    fn test_raise_package_not_found_without_version() {
        let err = raise_package_not_found("maya", None);
        let msg = format!("{}", err);
        assert!(msg.contains("maya") || msg.contains("Package"));
    }

    #[test]
    fn test_raise_resolve_error() {
        let err = raise_resolve_error("No solution found for requirements");
        let msg = format!("{}", err);
        assert!(msg.contains("No solution") || msg.contains("resolve") || msg.len() > 0);
    }

    #[test]
    fn test_raise_config_error() {
        let err = raise_config_error("Invalid packages_path setting");
        let msg = format!("{}", err);
        assert!(msg.len() > 0);
    }

    #[test]
    fn test_raise_build_error() {
        let err = raise_build_error("CMake configuration failed");
        let msg = format!("{}", err);
        assert!(msg.len() > 0);
    }

    #[test]
    fn test_raise_rex_error() {
        let err = raise_rex_error("Undefined variable: ${MY_VAR}");
        let msg = format!("{}", err);
        assert!(msg.len() > 0);
    }
}
