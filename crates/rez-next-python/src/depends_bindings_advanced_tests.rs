//! Advanced unit tests for `depends_bindings` — field storage, count, repr tests.
//! Split from depends_bindings_tests.rs (Cycle 146) to keep file size ≤500 lines.
//! Cycle 161: removed 7 duplicate field-storage/str-repr tests; added 4 tempdir integration tests.

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

// __str__ == __repr__ contract — one canonical test covers direct and transitive:
#[test]
fn test_depends_entry_str_equals_repr_contract() {
    for dep_type in ["direct", "transitive"] {
        let e = PyDependsEntry {
            name: "mypkg".to_string(),
            version: "1.5.0".to_string(),
            requirement: "dep-2+".to_string(),
            dependency_type: dep_type.to_string(),
        };
        assert_eq!(e.__str__(), e.__repr__(), "__str__ must equal __repr__ for {dep_type}");
    }
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

// ── Cycle 161: tempdir integration tests ─────────────────────────────────────
//
// These tests write real package.py files into a tempdir repository and verify
// that compute_depends() correctly discovers direct and transitive dependants.

/// Write a minimal package.py to `repo_root/<name>/<version>/package.py`
fn write_package(repo_root: &std::path::Path, name: &str, version: &str, requires: &[&str]) {
    let pkg_dir = repo_root.join(name).join(version);
    std::fs::create_dir_all(&pkg_dir).unwrap();
    let reqs = if requires.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = requires.iter().map(|r| format!("    \"{r}\",")).collect();
        format!("requires = [\n{}\n]\n", list.join("\n"))
    };
    let content = format!("name = \"{name}\"\nversion = \"{version}\"\n{reqs}");
    std::fs::write(pkg_dir.join("package.py"), content).unwrap();
}

#[test]
fn test_compute_depends_real_repo_finds_direct_dependant() {
    // Arrange: repo with python-3.11 and maya-2024.1 that requires python-3.11
    let dir = tempfile::tempdir().unwrap();
    write_package(dir.path(), "python", "3.11", &[]);
    write_package(dir.path(), "maya", "2024.1", &["python-3.11"]);

    let result = compute_depends("python", None, &[dir.path().to_path_buf()], false)
        .expect("compute_depends must succeed");

    assert_eq!(result.queried_package, "python");
    assert_eq!(
        result.direct_dependants.len(),
        1,
        "maya must appear as direct dependant of python, got: {:?}",
        result.direct_dependants.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
    assert_eq!(result.direct_dependants[0].name, "maya");
    assert_eq!(result.direct_dependants[0].version, "2024.1");
    assert_eq!(result.direct_dependants[0].dependency_type, "direct");
}

#[test]
fn test_compute_depends_real_repo_transitive_discovers_indirect() {
    // python → maya (direct), nuke → maya (transitive to python)
    let dir = tempfile::tempdir().unwrap();
    write_package(dir.path(), "python", "3.11", &[]);
    write_package(dir.path(), "maya", "2024.1", &["python-3.11"]);
    write_package(dir.path(), "nuke", "15.0", &["maya-2024"]);

    let result = compute_depends("python", None, &[dir.path().to_path_buf()], true)
        .expect("compute_depends must succeed");

    assert_eq!(result.direct_dependants.len(), 1);
    assert_eq!(result.direct_dependants[0].name, "maya");

    assert_eq!(
        result.transitive_dependants.len(),
        1,
        "nuke should appear as transitive dependant via maya, got: {:?}",
        result.transitive_dependants.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
    assert_eq!(result.transitive_dependants[0].name, "nuke");
    assert_eq!(result.transitive_dependants[0].dependency_type, "transitive");
}

#[test]
fn test_compute_depends_real_repo_skips_target_package_itself() {
    // The target package (python) must not appear in its own dependant list.
    let dir = tempfile::tempdir().unwrap();
    write_package(dir.path(), "python", "3.11", &["python-3.10"]);

    let result = compute_depends("python", None, &[dir.path().to_path_buf()], false)
        .expect("compute_depends must succeed");

    let found_self = result.direct_dependants.iter().any(|e| e.name == "python");
    assert!(!found_self, "python must not appear as its own dependant");
}

#[test]
fn test_compute_depends_real_repo_version_range_excludes_non_matching() {
    // maya requires python-3.9, range filter ">=3.11" should exclude it.
    let dir = tempfile::tempdir().unwrap();
    write_package(dir.path(), "python", "3.11", &[]);
    write_package(dir.path(), "maya", "2024.1", &["python-3.9"]);

    // python-3.9 does NOT overlap with >=3.11, so maya should be excluded.
    let result =
        compute_depends("python", Some(">=3.11"), &[dir.path().to_path_buf()], false)
            .expect("compute_depends must succeed");

    assert!(
        result.direct_dependants.is_empty(),
        "maya requires python-3.9 which doesn't overlap >=3.11; should be excluded, got: {:?}",
        result.direct_dependants.iter().map(|e| &e.name).collect::<Vec<_>>()
    );
}
