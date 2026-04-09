//! Advanced unit tests for `depends_bindings` — field storage, count, repr tests.
//! Split from depends_bindings_tests.rs (Cycle 146) to keep file size ≤500 lines.

use super::*;

// ── Cycle 128 onwards ─────────────────────────────────────────────────────

#[test]
fn test_all_dependants_both_empty() {
    let result = PyDependsResult {
        queried_package: "ghost".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert!(result.all_dependants().is_empty());
}

#[test]
fn test_depends_result_queried_package_field() {
    let result = PyDependsResult {
        queried_package: "houdini".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert_eq!(result.queried_package, "houdini");
}

#[test]
fn test_compute_depends_none_version_empty_paths() {
    let result = compute_depends("maya", None, &[], false);
    assert!(
        result.is_ok(),
        "compute_depends with None version should return Ok, got: {:?}",
        result.err()
    );
}

#[test]
fn test_depends_entry_transitive_type() {
    let entry = PyDependsEntry {
        name: "houdini".to_string(),
        version: "20.0.547".to_string(),
        requirement: "python-3.10+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    assert_eq!(entry.dependency_type, "transitive");
    let repr = entry.__repr__();
    assert!(repr.contains("transitive"), "repr should show dependency_type");
}

#[test]
fn test_depends_entry_str_repr_identical() {
    let entry = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.0".to_string(),
        requirement: "python-3+".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(entry.__str__(), entry.__repr__());
}

#[test]
fn test_total_count_only_direct() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![
            PyDependsEntry {
                name: "maya".to_string(),
                version: "2024.0".to_string(),
                requirement: "python-3+".to_string(),
                dependency_type: "direct".to_string(),
            },
            PyDependsEntry {
                name: "houdini".to_string(),
                version: "20.0".to_string(),
                requirement: "python-3.10+".to_string(),
                dependency_type: "direct".to_string(),
            },
        ],
        transitive_dependants: vec![],
    };
    assert_eq!(result.total_count(), 2);
}

#[test]
fn test_all_dependants_total_equals_direct_plus_transitive() {
    let result = PyDependsResult {
        queried_package: "numpy".to_string(),
        direct_dependants: vec![PyDependsEntry {
            name: "scipy".to_string(),
            version: "1.11.0".to_string(),
            requirement: "numpy-1.20+".to_string(),
            dependency_type: "direct".to_string(),
        }],
        transitive_dependants: vec![
            PyDependsEntry {
                name: "pandas".to_string(),
                version: "2.0.0".to_string(),
                requirement: "numpy-1.20+".to_string(),
                dependency_type: "transitive".to_string(),
            },
            PyDependsEntry {
                name: "matplotlib".to_string(),
                version: "3.7.0".to_string(),
                requirement: "numpy-1.20+".to_string(),
                dependency_type: "transitive".to_string(),
            },
        ],
    };
    let all = result.all_dependants();
    assert_eq!(all.len(), 3, "direct(1) + transitive(2) = 3");
}

#[test]
fn test_format_contains_queried_package_name() {
    let result = PyDependsResult {
        queried_package: "rezpkg_xyz".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    let fmt = result.format();
    assert!(
        fmt.contains("rezpkg_xyz"),
        "format output should contain queried_package name"
    );
}

#[test]
fn test_format_empty_dependants_still_non_empty_output() {
    let result = PyDependsResult {
        queried_package: "somelib".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    let fmt = result.format();
    assert!(!fmt.is_empty(), "format output must always be non-empty");
}

#[test]
fn test_depends_entry_direct_type() {
    let e = PyDependsEntry {
        name: "myapp".to_string(),
        version: "1.0.0".to_string(),
        requirement: "python-3.9+".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(e.dependency_type, "direct");
}

#[test]
fn test_depends_entry_dependency_type_is_transitive() {
    let e = PyDependsEntry {
        name: "myapp".to_string(),
        version: "1.0.0".to_string(),
        requirement: "python-3.9+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    assert_eq!(e.dependency_type, "transitive");
}

#[test]
fn test_depends_result_queried_package_name_preserved() {
    let result = PyDependsResult {
        queried_package: "numpy".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert_eq!(result.queried_package, "numpy");
}

#[test]
fn test_depends_result_total_count_with_overlap() {
    let entry = PyDependsEntry {
        name: "myapp".to_string(),
        version: "2.0.0".to_string(),
        requirement: "numpy-1.24+".to_string(),
        dependency_type: "direct".to_string(),
    };
    let entry2 = PyDependsEntry {
        name: "myapp".to_string(),
        version: "2.0.0".to_string(),
        requirement: "numpy-1.24+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "numpy".to_string(),
        direct_dependants: vec![entry],
        transitive_dependants: vec![entry2],
    };
    assert_eq!(result.total_count(), 1, "duplicate entry should be deduplicated");
}

#[test]
fn test_depends_entry_repr_contains_name_and_version() {
    let e = PyDependsEntry {
        name: "scipy".to_string(),
        version: "1.10.0".to_string(),
        requirement: "numpy-1.24+".to_string(),
        dependency_type: "direct".to_string(),
    };
    let repr = e.__repr__();
    assert!(repr.contains("scipy"), "repr must contain name: {repr}");
    assert!(repr.contains("1.10.0"), "repr must contain version: {repr}");
    assert!(repr.contains("direct"), "repr must contain type: {repr}");
}

#[test]
fn test_depends_result_direct_dependants_count() {
    let entries: Vec<PyDependsEntry> = (0..5)
        .map(|i| PyDependsEntry {
            name: format!("pkg{i}"),
            version: "1.0.0".to_string(),
            requirement: "mylib-1+".to_string(),
            dependency_type: "direct".to_string(),
        })
        .collect();
    let result = PyDependsResult {
        queried_package: "mylib".to_string(),
        direct_dependants: entries,
        transitive_dependants: vec![],
    };
    assert_eq!(result.direct_dependants.len(), 5);
    assert_eq!(result.total_count(), 5);
}

#[test]
fn test_depends_entry_str_matches_repr_transitive() {
    let e = PyDependsEntry {
        name: "toolpkg".to_string(),
        version: "3.2.1".to_string(),
        requirement: "libpkg-2+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    assert_eq!(e.__str__(), e.__repr__(), "__str__ must equal __repr__");
}

#[test]
fn test_all_dependants_len_equals_direct_when_no_transitive() {
    let result = PyDependsResult {
        queried_package: "pkg".to_string(),
        direct_dependants: vec![
            PyDependsEntry {
                name: "a".to_string(),
                version: "1.0".to_string(),
                requirement: "pkg-1+".to_string(),
                dependency_type: "direct".to_string(),
            },
            PyDependsEntry {
                name: "b".to_string(),
                version: "2.0".to_string(),
                requirement: "pkg-1+".to_string(),
                dependency_type: "direct".to_string(),
            },
        ],
        transitive_dependants: vec![],
    };
    assert_eq!(result.all_dependants().len(), result.direct_dependants.len());
}

#[test]
fn test_format_two_direct_shows_both() {
    let result = PyDependsResult {
        queried_package: "lib".to_string(),
        direct_dependants: vec![
            PyDependsEntry {
                name: "app1".to_string(),
                version: "1.0.0".to_string(),
                requirement: "lib-1+".to_string(),
                dependency_type: "direct".to_string(),
            },
            PyDependsEntry {
                name: "app2".to_string(),
                version: "2.0.0".to_string(),
                requirement: "lib-1+".to_string(),
                dependency_type: "direct".to_string(),
            },
        ],
        transitive_dependants: vec![],
    };
    let output = result.format();
    assert!(output.contains("app1"), "format must include app1");
    assert!(output.contains("app2"), "format must include app2");
}

#[test]
fn test_depends_result_repr_shows_direct_count() {
    let result = PyDependsResult {
        queried_package: "mylib".to_string(),
        direct_dependants: vec![PyDependsEntry {
            name: "dep1".to_string(),
            version: "1.0".to_string(),
            requirement: "mylib-1+".to_string(),
            dependency_type: "direct".to_string(),
        }],
        transitive_dependants: vec![],
    };
    let repr = result.__repr__();
    assert!(repr.contains("direct=1"), "repr should show direct=1: {repr}");
}

#[test]
fn test_depends_entry_requirement_field_stored() {
    let e = PyDependsEntry {
        name: "someapp".to_string(),
        version: "3.0.0".to_string(),
        requirement: "lib-2+<3".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(e.requirement, "lib-2+<3");
}

#[test]
fn test_depends_result_empty_total_count_is_zero() {
    let result = PyDependsResult {
        queried_package: "orphan".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert_eq!(result.total_count(), 0);
}

#[test]
fn test_depends_entry_dependency_type_stored() {
    let e = PyDependsEntry {
        name: "consumer".to_string(),
        version: "0.1.0".to_string(),
        requirement: "provider-1".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(e.dependency_type, "direct");
}

#[test]
fn test_depends_result_queried_package_stored() {
    let result = PyDependsResult {
        queried_package: "numpy".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert_eq!(result.queried_package, "numpy");
}

#[test]
fn test_depends_result_total_count_only_transitive() {
    let e = PyDependsEntry {
        name: "indirectconsumer".to_string(),
        version: "1.0.0".to_string(),
        requirement: "lib-1+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "lib".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![e],
    };
    assert_eq!(result.total_count(), 1);
}

#[test]
fn test_depends_entry_repr_contains_requirement_string() {
    let e = PyDependsEntry {
        name: "pkgA".to_string(),
        version: "2.0.0".to_string(),
        requirement: "sharedlib-3+".to_string(),
        dependency_type: "direct".to_string(),
    };
    let repr = e.__repr__();
    assert!(repr.contains("sharedlib-3+"), "repr must contain requirement: {repr}");
}

#[test]
fn test_depends_entry_str_equals_repr_cy128() {
    let e = PyDependsEntry {
        name: "mypkg".to_string(),
        version: "1.5.0".to_string(),
        requirement: "dep-2+".to_string(),
        dependency_type: "transitive".to_string(),
    };
    assert_eq!(e.__str__(), e.__repr__(), "__str__ must equal __repr__");
}

#[test]
fn test_depends_with_version_range_filter() {
    let result = compute_depends("python", Some(">=3.9"), &[], false);
    assert!(result.is_ok());
}

#[test]
fn test_get_dependants_empty_paths() {
    let result = get_reverse_dependencies("python", None, Some(vec![]), false);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.direct_dependants.is_empty());
}
