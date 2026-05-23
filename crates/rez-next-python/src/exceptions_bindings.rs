//! Python bindings for Rez-compatible exception hierarchy.
//!
//! This module provides the same exception classes as `rez.exceptions`,
//! by importing them from `rez_next.exceptions` (the Python module).

use pyo3::prelude::*;

/// Register the `exceptions` submodule.
///
/// This function imports exception classes from `rez_next.exceptions`
/// and adds them to the given module.
pub fn register_all_exceptions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();

    // Import the Python exceptions module
    let py_exceptions = py.import("rez_next.exceptions")?;

    // Create our exceptions submodule
    let exceptions_mod = PyModule::new(py, "exceptions")?;

    // List of all exception classes to import
    let exc_names = [
        "RezError",
        "RezSystemError",
        "RezBindError",
        "RezPluginError",
        "ConfigurationError",
        "ResolveError",
        "PackageFamilyNotFoundError",
        "PackageNotFoundError",
        "ResourceError",
        "ResourceNotFoundError",
        "ResourceContentError",
        "PackageMetadataError",
        "PackageCommandError",
        "PackageRequestError",
        "PackageCopyError",
        "PackageMoveError",
        "ContextBundleError",
        "PackageCacheError",
        "PackageTestError",
        "InvalidPackageError",
        "ResolvedContextError",
        "RexError",
        "RexUndefinedVariableError",
        "RexStopError",
        "BuildError",
        "BuildSystemError",
        "BuildContextResolveError",
        "BuildProcessError",
        "ReleaseError",
        "ReleaseVCSError",
        "ReleaseHookError",
        "ReleaseHookCancellingError",
        "SuiteError",
        "PackageRepositoryError",
        "_NeverError",
        "RezGuiQTImportError",
    ];

    for name in exc_names {
        if let Ok(exc) = py_exceptions.getattr(name) {
            exceptions_mod.add(name, exc)?;
        }
    }

    // Register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("rez_next._native.exceptions", &exceptions_mod)?;

    // Also add to the parent module (m)
    m.add_submodule(&exceptions_mod)?;

    Ok(())
}

