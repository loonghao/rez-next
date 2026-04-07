//! Custom Python exception classes for rez-next
//!
//! Defines a hierarchy of exception classes that mirror rez's exception hierarchy,
//! ensuring `except rez.exceptions.RezError` can be used as a catch-all.
//!
//! Hierarchy:
//! ```text
//! Exception
//! +-- RezError                    (base for all rez exceptions)
//!     +-- PackageNotFound
//!     +-- PackageFamilyNotFound
//!     +-- PackageVersionConflict
//!     +-- PackageRequestError
//!     +-- ResolveError
//!     |   +-- SolveFailure
//!     |   +-- PackageConflict
//!     +-- RezBuildError
//!     +-- RezReleaseError
//!     +-- ConfigurationError
//!     +-- PackageParseError
//!     +-- ContextBundleError
//!     +-- SuiteError
//!     +-- RexError
//!     |   +-- RexUndefinedVariableError
//!     +-- RezSystemError
//! ```

use pyo3::exceptions::PyException;
use pyo3::prelude::*;

// ─── Root exception ───────────────────────────────────────────────────────────

pyo3::create_exception!(
    rez_next,
    RezError,
    PyException,
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

pyo3::create_exception!(
    rez_next,
    ResolveError,
    RezError,
    "Raised when dependency resolution fails.\n\nEquivalent to rez.exceptions.ResolveError."
);

pyo3::create_exception!(rez_next, SolveFailure, ResolveError,
    "Raised when the solver cannot find any valid solution.\n\nEquivalent to rez.exceptions.SolveFailure."
);

pyo3::create_exception!(rez_next, PackageConflict, ResolveError,
    "Raised when two packages have mutually exclusive requirements.\n\nEquivalent to rez.exceptions.PackageConflict."
);

// ─── Build / release exceptions ───────────────────────────────────────────────

pyo3::create_exception!(
    rez_next,
    RezBuildError,
    RezError,
    "Raised when a package build fails.\n\nEquivalent to rez.exceptions.RezBuildError."
);

pyo3::create_exception!(
    rez_next,
    RezReleaseError,
    RezError,
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

pyo3::create_exception!(
    rez_next,
    SuiteError,
    RezError,
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

pyo3::create_exception!(
    rez_next,
    RezSystemError,
    RezError,
    "Raised for internal rez-next errors.\n\nEquivalent to rez.exceptions.RezSystemError."
);

// ─── Exception name constants (for documentation / mapping) ───────────────────


/// Map of exception class name -> parent class name (for hierarchy validation).
/// Only used in unit tests.
#[cfg(test)]
pub const EXCEPTION_HIERARCHY: &[(&str, &str)] = &[
    ("RezError", "Exception"),
    ("PackageNotFound", "RezError"),
    ("PackageFamilyNotFound", "RezError"),
    ("PackageVersionConflict", "RezError"),
    ("PackageRequestError", "RezError"),
    ("PackageParseError", "RezError"),
    ("ResolveError", "RezError"),
    ("SolveFailure", "ResolveError"),
    ("PackageConflict", "ResolveError"),
    ("RezBuildError", "RezError"),
    ("RezReleaseError", "RezError"),
    ("ConfigurationError", "RezError"),
    ("ContextBundleError", "RezError"),
    ("SuiteError", "RezError"),
    ("RexError", "RezError"),
    ("RexUndefinedVariableError", "RexError"),
    ("RezSystemError", "RezError"),
];

// ─── Registration ─────────────────────────────────────────────────────────────

/// Register all custom exception types into the given submodule.
pub fn register_all_exceptions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Base
    m.add("RezError", m.py().get_type::<RezError>())?;

    // Package
    m.add("PackageNotFound", m.py().get_type::<PackageNotFound>())?;
    m.add(
        "PackageFamilyNotFound",
        m.py().get_type::<PackageFamilyNotFound>(),
    )?;
    m.add(
        "PackageVersionConflict",
        m.py().get_type::<PackageVersionConflict>(),
    )?;
    m.add(
        "PackageRequestError",
        m.py().get_type::<PackageRequestError>(),
    )?;
    m.add("PackageParseError", m.py().get_type::<PackageParseError>())?;

    // Resolve
    m.add("ResolveError", m.py().get_type::<ResolveError>())?;
    m.add("SolveFailure", m.py().get_type::<SolveFailure>())?;
    m.add("PackageConflict", m.py().get_type::<PackageConflict>())?;

    // Build / release
    m.add("RezBuildError", m.py().get_type::<RezBuildError>())?;
    m.add("RezReleaseError", m.py().get_type::<RezReleaseError>())?;

    // Config
    m.add(
        "ConfigurationError",
        m.py().get_type::<ConfigurationError>(),
    )?;

    // Context / bundle
    m.add(
        "ContextBundleError",
        m.py().get_type::<ContextBundleError>(),
    )?;

    // Suite
    m.add("SuiteError", m.py().get_type::<SuiteError>())?;

    // Rex
    m.add("RexError", m.py().get_type::<RexError>())?;
    m.add(
        "RexUndefinedVariableError",
        m.py().get_type::<RexUndefinedVariableError>(),
    )?;

    // System
    m.add("RezSystemError", m.py().get_type::<RezSystemError>())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── EXCEPTION_HIERARCHY metadata tests ──────────────────────────────────



    #[test]
    fn test_exception_hierarchy_has_rez_error_root() {
        let root = EXCEPTION_HIERARCHY
            .iter()
            .find(|(name, _)| *name == "RezError");
        assert!(root.is_some(), "RezError must be in hierarchy");
        assert_eq!(root.unwrap().1, "Exception", "RezError must extend Exception");
    }

    #[test]
    fn test_exception_hierarchy_package_exceptions_extend_rez_error() {
        let pkg_exceptions = [
            "PackageNotFound",
            "PackageFamilyNotFound",
            "PackageVersionConflict",
            "PackageRequestError",
            "PackageParseError",
        ];
        for name in &pkg_exceptions {
            let entry = EXCEPTION_HIERARCHY
                .iter()
                .find(|(n, _)| *n == *name)
                .unwrap_or_else(|| panic!("{} must be in EXCEPTION_HIERARCHY", name));
            assert_eq!(
                entry.1, "RezError",
                "{} must extend RezError, got {}",
                name, entry.1
            );
        }
    }

    #[test]
    fn test_exception_hierarchy_resolve_subtypes_extend_resolve_error() {
        for name in &["SolveFailure", "PackageConflict"] {
            let entry = EXCEPTION_HIERARCHY
                .iter()
                .find(|(n, _)| *n == *name)
                .unwrap_or_else(|| panic!("{} must be in EXCEPTION_HIERARCHY", name));
            assert_eq!(
                entry.1, "ResolveError",
                "{} must extend ResolveError, got {}",
                name, entry.1
            );
        }
    }

    #[test]
    fn test_exception_hierarchy_rex_undefined_extends_rex_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "RexUndefinedVariableError")
            .expect("RexUndefinedVariableError must be in EXCEPTION_HIERARCHY");
        assert_eq!(
            entry.1, "RexError",
            "RexUndefinedVariableError must extend RexError"
        );
    }

    #[test]
    fn test_exception_hierarchy_total_count() {
        // 17 entries: 1 root + 16 subtypes
        assert_eq!(
            EXCEPTION_HIERARCHY.len(),
            17,
            "Expected 17 exception entries, got {}",
            EXCEPTION_HIERARCHY.len()
        );
    }

    #[test]
    fn test_exception_hierarchy_no_duplicate_names() {
        let mut names: Vec<&str> = EXCEPTION_HIERARCHY.iter().map(|(n, _)| *n).collect();
        let original_len = names.len();
        names.dedup();
        // After sort+dedup we check uniqueness
        let mut names2: Vec<&str> = EXCEPTION_HIERARCHY.iter().map(|(n, _)| *n).collect();
        names2.sort_unstable();
        names2.dedup();
        assert_eq!(
            names2.len(),
            original_len,
            "EXCEPTION_HIERARCHY contains duplicate names"
        );
    }

    #[test]
    fn test_exception_hierarchy_system_error_extends_rez_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "RezSystemError")
            .expect("RezSystemError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError");
    }

    #[test]
    fn test_exception_hierarchy_build_release_extend_rez_error() {
        for name in &["RezBuildError", "RezReleaseError"] {
            let entry = EXCEPTION_HIERARCHY
                .iter()
                .find(|(n, _)| *n == *name)
                .unwrap_or_else(|| panic!("{} must be in EXCEPTION_HIERARCHY", name));
            assert_eq!(
                entry.1, "RezError",
                "{} must extend RezError",
                name
            );
        }
    }

    #[test]
    fn test_exception_hierarchy_config_context_suite_extend_rez_error() {
        for name in &["ConfigurationError", "ContextBundleError", "SuiteError"] {
            let entry = EXCEPTION_HIERARCHY
                .iter()
                .find(|(n, _)| *n == *name)
                .unwrap_or_else(|| panic!("{} must be in EXCEPTION_HIERARCHY", name));
            assert_eq!(
                entry.1, "RezError",
                "{} must extend RezError",
                name
            );
        }
    }

    #[test]
    fn test_all_hierarchy_names_non_empty() {
        for (name, parent) in EXCEPTION_HIERARCHY {
            assert!(!name.is_empty(), "exception name must not be empty");
            assert!(!parent.is_empty(), "parent name must not be empty");
        }
    }

    // ─── Additional hierarchy completeness tests ──────────────────────────────

    #[test]
    fn test_resolve_error_extends_rez_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "ResolveError")
            .expect("ResolveError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError", "ResolveError must extend RezError");
    }

    #[test]
    fn test_every_name_is_pascal_case() {
        for (name, _) in EXCEPTION_HIERARCHY {
            let first = name.chars().next().expect("name must not be empty");
            assert!(
                first.is_ascii_uppercase(),
                "Exception name '{}' should start with uppercase (PascalCase)",
                name
            );
        }
    }

    #[test]
    fn test_parent_names_are_known_names_or_exception() {
        let all_names: Vec<&str> = EXCEPTION_HIERARCHY.iter().map(|(n, _)| *n).collect();
        for (name, parent) in EXCEPTION_HIERARCHY {
            let valid = *parent == "Exception" || all_names.contains(parent);
            assert!(
                valid,
                "Parent '{}' of '{}' must either be 'Exception' or appear as a name",
                parent,
                name
            );
        }
    }

    #[test]
    fn test_only_one_root_extending_exception() {
        let roots: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(_, parent)| *parent == "Exception")
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(
            roots.len(),
            1,
            "Only one exception should extend 'Exception', got: {:?}",
            roots
        );
        assert_eq!(roots[0], "RezError");
    }

    #[test]
    fn test_package_not_found_in_hierarchy() {
        assert!(
            EXCEPTION_HIERARCHY
                .iter()
                .any(|(n, _)| *n == "PackageNotFound"),
            "PackageNotFound must be in hierarchy"
        );
    }

    #[test]
    fn test_solve_failure_is_leaf_under_resolve_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "SolveFailure")
            .expect("SolveFailure must exist");
        assert_eq!(entry.1, "ResolveError");
        // SolveFailure itself is not a parent of anything
        let is_parent = EXCEPTION_HIERARCHY.iter().any(|(_, p)| *p == "SolveFailure");
        assert!(
            !is_parent,
            "SolveFailure should be a leaf (no children in hierarchy)"
        );
    }

    #[test]
    fn test_rex_error_is_direct_child_of_rez_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "RexError")
            .expect("RexError must exist");
        assert_eq!(entry.1, "RezError");
    }

    #[test]
    fn test_hierarchy_has_at_least_five_leaf_exceptions() {
        // A leaf is an exception that does not appear as a parent of any other
        let parent_names: std::collections::HashSet<&str> =
            EXCEPTION_HIERARCHY.iter().map(|(_, p)| *p).collect();
        let leaves: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(n, _)| !parent_names.contains(*n))
            .map(|(n, _)| *n)
            .collect();
        assert!(
            leaves.len() >= 5,
            "Expected at least 5 leaf exceptions, got {} : {:?}",
            leaves.len(),
            leaves
        );
    }

    #[test]
    fn test_package_conflict_parent_is_resolve_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "PackageConflict")
            .expect("PackageConflict must exist");
        assert_eq!(entry.1, "ResolveError");
    }

    // ─────── Cycle 98 additions ──────────────────────────────────────────────

    /// PackageParseError must be in the hierarchy and extend RezError
    #[test]
    fn test_package_parse_error_extends_rez_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "PackageParseError")
            .expect("PackageParseError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError");
    }

    /// ContextBundleError must be in the hierarchy and extend RezError
    #[test]
    fn test_context_bundle_error_in_hierarchy() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "ContextBundleError")
            .expect("ContextBundleError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError");
    }

    /// SuiteError is a leaf (nothing extends it)
    #[test]
    fn test_suite_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "SuiteError");
        assert!(
            !is_parent,
            "SuiteError should be a leaf with no children in hierarchy"
        );
    }

    /// ResolveError has exactly 2 direct children: SolveFailure and PackageConflict
    #[test]
    fn test_resolve_error_has_two_children() {
        let children: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(_, p)| *p == "ResolveError")
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(
            children.len(),
            2,
            "ResolveError should have exactly 2 children, got: {:?}",
            children
        );
        assert!(children.contains(&"SolveFailure"));
        assert!(children.contains(&"PackageConflict"));
    }

    /// RexError has exactly 1 direct child: RexUndefinedVariableError
    #[test]
    fn test_rex_error_has_one_child() {
        let children: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(_, p)| *p == "RexError")
            .map(|(n, _)| *n)
            .collect();
        assert_eq!(
            children.len(),
            1,
            "RexError should have exactly 1 child, got: {:?}",
            children
        );
        assert_eq!(children[0], "RexUndefinedVariableError");
    }

    /// PackageFamilyNotFound is a leaf (nothing extends it)
    #[test]
    fn test_package_family_not_found_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "PackageFamilyNotFound");
        assert!(
            !is_parent,
            "PackageFamilyNotFound should be a leaf with no children"
        );
    }

    /// ConfigurationError is a leaf
    #[test]
    fn test_configuration_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "ConfigurationError");
        assert!(
            !is_parent,
            "ConfigurationError should be a leaf with no children"
        );
    }

    // ─────── Cycle 104 additions ──────────────────────────────────────────────

    /// All exception names in the hierarchy are non-empty strings
    #[test]
    fn test_all_exception_names_non_empty() {
        for (name, _parent) in EXCEPTION_HIERARCHY {
            assert!(
                !name.is_empty(),
                "Found empty exception name in EXCEPTION_HIERARCHY"
            );
        }
    }

    /// All parent names in the hierarchy are non-empty strings
    #[test]
    fn test_all_parent_names_non_empty() {
        for (_name, parent) in EXCEPTION_HIERARCHY {
            assert!(
                !parent.is_empty(),
                "Found empty parent name in EXCEPTION_HIERARCHY"
            );
        }
    }



    /// RexUndefinedVariableError is a leaf (nothing extends it)
    #[test]
    fn test_rex_undefined_variable_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RexUndefinedVariableError");
        assert!(
            !is_parent,
            "RexUndefinedVariableError should be a leaf with no children"
        );
    }

    // ─────── Cycle 106 additions ──────────────────────────────────────────────

    /// RezBuildError is a leaf (nothing extends it)
    #[test]
    fn test_rez_build_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RezBuildError");
        assert!(!is_parent, "RezBuildError should be a leaf with no children");
    }

    /// RezReleaseError is a leaf (nothing extends it)
    #[test]
    fn test_rez_release_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RezReleaseError");
        assert!(
            !is_parent,
            "RezReleaseError should be a leaf with no children"
        );
    }


}


