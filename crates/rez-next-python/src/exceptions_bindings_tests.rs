//! Tests for exceptions_bindings — exception hierarchy validation.

#[cfg(test)]
mod tests {
    use crate::exceptions_bindings::EXCEPTION_HIERARCHY;

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
            assert_eq!(entry.1, "RezError", "{} must extend RezError", name);
        }
    }

    #[test]
    fn test_exception_hierarchy_config_context_suite_extend_rez_error() {
        for name in &["ConfigurationError", "ContextBundleError", "SuiteError"] {
            let entry = EXCEPTION_HIERARCHY
                .iter()
                .find(|(n, _)| *n == *name)
                .unwrap_or_else(|| panic!("{} must be in EXCEPTION_HIERARCHY", name));
            assert_eq!(entry.1, "RezError", "{} must extend RezError", name);
        }
    }

    #[test]
    fn test_all_hierarchy_names_non_empty() {
        for (name, parent) in EXCEPTION_HIERARCHY {
            assert!(!name.is_empty(), "exception name must not be empty");
            assert!(!parent.is_empty(), "parent name must not be empty");
        }
    }

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
                parent, name
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
        let is_parent = EXCEPTION_HIERARCHY.iter().any(|(_, p)| *p == "SolveFailure");
        assert!(!is_parent, "SolveFailure should be a leaf (no children in hierarchy)");
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

    #[test]
    fn test_package_parse_error_extends_rez_error() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "PackageParseError")
            .expect("PackageParseError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError");
    }

    #[test]
    fn test_context_bundle_error_in_hierarchy() {
        let entry = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "ContextBundleError")
            .expect("ContextBundleError must be in EXCEPTION_HIERARCHY");
        assert_eq!(entry.1, "RezError");
    }

    #[test]
    fn test_suite_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY.iter().any(|(_, p)| *p == "SuiteError");
        assert!(!is_parent, "SuiteError should be a leaf with no children in hierarchy");
    }

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

    #[test]
    fn test_package_family_not_found_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "PackageFamilyNotFound");
        assert!(!is_parent, "PackageFamilyNotFound should be a leaf with no children");
    }

    #[test]
    fn test_configuration_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "ConfigurationError");
        assert!(!is_parent, "ConfigurationError should be a leaf with no children");
    }

    #[test]
    fn test_all_exception_names_non_empty() {
        for (name, _parent) in EXCEPTION_HIERARCHY {
            assert!(!name.is_empty(), "Found empty exception name in EXCEPTION_HIERARCHY");
        }
    }

    #[test]
    fn test_all_parent_names_non_empty() {
        for (_name, parent) in EXCEPTION_HIERARCHY {
            assert!(!parent.is_empty(), "Found empty parent name in EXCEPTION_HIERARCHY");
        }
    }

    #[test]
    fn test_rex_undefined_variable_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RexUndefinedVariableError");
        assert!(!is_parent, "RexUndefinedVariableError should be a leaf with no children");
    }

    #[test]
    fn test_rez_build_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RezBuildError");
        assert!(!is_parent, "RezBuildError should be a leaf with no children");
    }

    #[test]
    fn test_rez_release_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "RezReleaseError");
        assert!(!is_parent, "RezReleaseError should be a leaf with no children");
    }

    #[test]
    fn test_package_version_conflict_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "PackageVersionConflict");
        assert!(!is_parent, "PackageVersionConflict should be a leaf with no children");
    }

    #[test]
    fn test_package_request_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "PackageRequestError");
        assert!(!is_parent, "PackageRequestError should be a leaf with no children");
    }

    #[test]
    fn test_package_parse_error_is_leaf() {
        let is_parent = EXCEPTION_HIERARCHY
            .iter()
            .any(|(_, p)| *p == "PackageParseError");
        assert!(!is_parent, "PackageParseError should be a leaf with no children");
    }

    #[test]
    fn test_rez_error_has_many_descendants() {
        let non_root: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(n, _)| *n != "RezError")
            .map(|(n, _)| *n)
            .collect();
        assert!(
            non_root.len() >= 10,
            "Expected at least 10 non-root exceptions, got {}",
            non_root.len()
        );
    }

    #[test]
    fn test_no_self_loop_in_hierarchy() {
        for (name, parent) in EXCEPTION_HIERARCHY {
            assert_ne!(name, parent, "Exception '{}' must not be its own parent", name);
        }
    }

    #[test]
    fn test_all_rez_exceptions_end_with_error_or_failure() {
        for (name, _) in EXCEPTION_HIERARCHY {
            let has_error_suffix = name.ends_with("Error")
                || name.ends_with("Failure")
                || name.ends_with("Conflict")
                || name.ends_with("NotFound");
            assert!(
                has_error_suffix,
                "Exception '{}' should end with Error/Failure/Conflict/NotFound",
                name
            );
        }
    }

    #[test]
    fn test_package_exceptions_contain_package_in_name() {
        let pkg_exceptions = [
            "PackageNotFound",
            "PackageFamilyNotFound",
            "PackageVersionConflict",
            "PackageRequestError",
            "PackageParseError",
            "PackageConflict",
        ];
        for name in &pkg_exceptions {
            assert!(name.contains("Package"), "Expected 'Package' in name, got '{}'", name);
        }
    }

    #[test]
    fn test_all_non_root_entries_are_reachable_from_rez_error() {
        let names: std::collections::HashSet<&str> =
            EXCEPTION_HIERARCHY.iter().map(|(n, _)| *n).collect();
        for (name, parent) in EXCEPTION_HIERARCHY {
            if *name == "RezError" {
                continue;
            }
            assert!(
                names.contains(parent) || *parent == "Exception",
                "Parent '{}' of '{}' must be in hierarchy or be 'Exception'",
                parent, name
            );
        }
    }

    #[test]
    fn test_rez_error_has_correct_direct_children_count() {
        let direct_children: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(_, p)| *p == "RezError")
            .map(|(n, _)| *n)
            .collect();
        assert!(
            direct_children.len() >= 10,
            "RezError should have at least 10 direct children, got {} : {:?}",
            direct_children.len(),
            direct_children
        );
    }

    #[test]
    fn test_second_level_resolve_subtypes_are_known() {
        let depth2: Vec<&str> = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(_, p)| *p == "ResolveError")
            .map(|(n, _)| *n)
            .collect();
        let expected = ["SolveFailure", "PackageConflict"];
        for name in &expected {
            assert!(
                depth2.contains(name),
                "Expected '{}' to be a child of ResolveError, found: {:?}",
                name, depth2
            );
        }
    }

    #[test]
    fn test_rez_system_error_not_equal_to_rez_build_error() {
        let system = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "RezSystemError")
            .expect("RezSystemError must exist");
        let build = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "RezBuildError")
            .expect("RezBuildError must exist");
        assert_ne!(system.0, build.0, "RezSystemError and RezBuildError should be distinct");
    }

    #[test]
    fn test_at_least_twelve_leaf_exceptions() {
        let parent_set: std::collections::HashSet<&str> =
            EXCEPTION_HIERARCHY.iter().map(|(_, p)| *p).collect();
        let leaf_count = EXCEPTION_HIERARCHY
            .iter()
            .filter(|(n, _)| !parent_set.contains(*n))
            .count();
        assert!(
            leaf_count >= 12,
            "Expected at least 12 leaf exceptions, got {}",
            leaf_count
        );
    }

    #[test]
    fn test_package_not_found_and_family_are_siblings() {
        let parent_of_pnf = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "PackageNotFound")
            .map(|(_, p)| *p)
            .expect("PackageNotFound must exist");
        let parent_of_pfnf = EXCEPTION_HIERARCHY
            .iter()
            .find(|(n, _)| *n == "PackageFamilyNotFound")
            .map(|(_, p)| *p)
            .expect("PackageFamilyNotFound must exist");
        assert_eq!(
            parent_of_pnf, parent_of_pfnf,
            "PackageNotFound and PackageFamilyNotFound should be siblings"
        );
    }

    #[test]
    fn test_all_exception_names_are_non_empty() {
        for (name, _parent) in EXCEPTION_HIERARCHY.iter() {
            assert!(!name.is_empty(), "exception name must not be empty");
        }
    }

    #[test]
    fn test_all_parent_names_are_non_empty() {
        for (_name, parent) in EXCEPTION_HIERARCHY.iter() {
            assert!(!parent.is_empty(), "parent name must not be empty");
        }
    }

    #[test]
    fn test_rez_error_appears_as_parent() {
        let has_rez_error_parent = EXCEPTION_HIERARCHY.iter().any(|(_, p)| *p == "RezError");
        assert!(has_rez_error_parent, "RezError must appear as a parent in the hierarchy");
    }

    #[test]
    fn test_no_self_loop_in_hierarchy_cy124() {
        for (name, parent) in EXCEPTION_HIERARCHY.iter() {
            assert_ne!(name, parent, "exception '{name}' must not be its own parent");
        }
    }

    #[test]
    fn test_all_exception_names_pascal_case() {
        for (name, _) in EXCEPTION_HIERARCHY.iter() {
            let first_char = name.chars().next().expect("name must be non-empty");
            assert!(
                first_char.is_ascii_uppercase(),
                "exception name '{name}' must start with uppercase"
            );
        }
    }

    #[test]
    fn test_rez_build_error_present_in_hierarchy() {
        let found = EXCEPTION_HIERARCHY.iter().any(|(n, _)| *n == "RezBuildError");
        assert!(found, "RezBuildError must be present in the hierarchy");
    }
}
