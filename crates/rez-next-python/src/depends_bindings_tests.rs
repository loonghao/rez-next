//! Unit tests for `depends_bindings` — split from the main module to keep file size ≤ 1000 lines.
use super::*;
use rez_next_package::Package;
use rez_next_version::Version;

fn make_pkg(name: &str, ver: &str, requires: &[&str]) -> Package {
    let mut p = Package::new(name.to_string());
    p.version = Some(Version::parse(ver).unwrap());
    p.requires = requires.iter().map(|s| s.to_string()).collect();
    p
}

#[test]
fn test_make_pkg_helper() {
    let pkg = make_pkg("maya", "2024.1", &["python-3.9", "arnold-7"]);
    assert_eq!(pkg.name, "maya");
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "2024.1");
    assert_eq!(pkg.requires.len(), 2);
    assert!(pkg.requires.contains(&"python-3.9".to_string()));
}

#[test]
fn test_compute_depends_empty_repo() {
    let result = compute_depends("python", None, &[], false);
    assert!(result.is_ok(), "Should succeed with empty repo");
    let r = result.unwrap();
    assert_eq!(r.queried_package, "python");
    assert!(r.direct_dependants.is_empty());
    assert!(r.transitive_dependants.is_empty());
}

#[test]
fn test_depends_entry_repr() {
    let entry = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3.9".to_string(),
        dependency_type: "direct".to_string(),
    };
    let repr = entry.__repr__();
    assert!(repr.contains("maya-2024.1"));
    assert!(repr.contains("python-3.9"));
    assert!(repr.contains("direct"));
}

#[test]
fn test_depends_result_format_empty() {
    let result = PyDependsResult {
        queried_package: "nonexistent".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    let output = result.format();
    assert!(output.contains("nonexistent"));
    assert!(output.contains("no dependants found"));
}

#[test]
fn test_depends_result_format_with_dependants() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![
            PyDependsEntry {
                name: "maya".to_string(),
                version: "2024.1".to_string(),
                requirement: "python-3.9".to_string(),
                dependency_type: "direct".to_string(),
            },
            PyDependsEntry {
                name: "houdini".to_string(),
                version: "20.0".to_string(),
                requirement: "python-3.10".to_string(),
                dependency_type: "direct".to_string(),
            },
        ],
        transitive_dependants: vec![],
    };
    let output = result.format();
    assert!(output.contains("python"));
    assert!(output.contains("maya-2024.1"));
    assert!(output.contains("houdini-20.0"));
    assert!(output.contains("Direct"));
}

#[test]
fn test_depends_result_all_dependants() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![PyDependsEntry {
            name: "maya".to_string(),
            version: "2024.1".to_string(),
            requirement: "python-3.9".to_string(),
            dependency_type: "direct".to_string(),
        }],
        transitive_dependants: vec![PyDependsEntry {
            name: "nuke".to_string(),
            version: "14.0".to_string(),
            requirement: "maya-2024".to_string(),
            dependency_type: "transitive".to_string(),
        }],
    };
    let all = result.all_dependants();
    assert_eq!(all.len(), 2);
}

#[test]
fn test_depends_entry_repr_format() {
    let entry = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3.9".to_string(),
        dependency_type: "direct".to_string(),
    };
    let repr = entry.__repr__();
    assert!(repr.contains("maya-2024.1"));
    assert!(repr.contains("python-3.9"));
    assert!(repr.contains("direct"));
}

#[test]
fn test_depends_result_format_with_transitive() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![PyDependsEntry {
            name: "maya".to_string(),
            version: "2024.1".to_string(),
            requirement: "python-3.9".to_string(),
            dependency_type: "direct".to_string(),
        }],
        transitive_dependants: vec![PyDependsEntry {
            name: "nuke".to_string(),
            version: "14.0".to_string(),
            requirement: "maya-2024".to_string(),
            dependency_type: "transitive".to_string(),
        }],
    };
    let output = result.format();
    assert!(output.contains("Direct"));
    assert!(output.contains("Transitive"));
    assert!(output.contains("nuke-14.0"));
}

#[test]
fn test_depends_entry_str_equals_repr() {
    let entry = PyDependsEntry {
        name: "arnold".to_string(),
        version: "7.0.0".to_string(),
        requirement: "python-3.11".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(entry.__str__(), entry.__repr__());
}

#[test]
fn test_total_count_deduplicates_same_pkg_different_slots() {
    let entry = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3".to_string(),
        dependency_type: "direct".to_string(),
    };
    let entry2 = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![entry],
        transitive_dependants: vec![entry2],
    };
    assert_eq!(result.total_count(), 1, "Same name+version deduped to 1");
}

#[test]
fn test_total_count_only_transitive() {
    let entry = PyDependsEntry {
        name: "nuke".to_string(),
        version: "14.0".to_string(),
        requirement: "maya-2024".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![entry],
    };
    assert_eq!(result.total_count(), 1);
}

#[test]
fn test_all_dependants_direct_first() {
    let direct = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3.9".to_string(),
        dependency_type: "direct".to_string(),
    };
    let trans = PyDependsEntry {
        name: "nuke".to_string(),
        version: "14.0".to_string(),
        requirement: "maya-2024".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![direct],
        transitive_dependants: vec![trans],
    };
    let all = result.all_dependants();
    assert_eq!(all[0].dependency_type, "direct");
    assert_eq!(all[1].dependency_type, "transitive");
}

#[test]
fn test_format_sorted_alphabetically() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![
            PyDependsEntry {
                name: "zbrush".to_string(),
                version: "2024.0".to_string(),
                requirement: "python-3".to_string(),
                dependency_type: "direct".to_string(),
            },
            PyDependsEntry {
                name: "arnold".to_string(),
                version: "7.0".to_string(),
                requirement: "python-3.11".to_string(),
                dependency_type: "direct".to_string(),
            },
        ],
        transitive_dependants: vec![],
    };
    let output = result.format();
    let arnold_pos = output.find("arnold").unwrap();
    let zbrush_pos = output.find("zbrush").unwrap();
    assert!(arnold_pos < zbrush_pos, "arnold should appear before zbrush");
}

#[test]
fn test_depends_entry_repr_format_houdini() {
    let entry = PyDependsEntry {
        name: "houdini".to_string(),
        version: "20.0.506".to_string(),
        requirement: "python-3.10+<3.12".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let repr = entry.__repr__();
    assert!(repr.starts_with("DependsEntry("));
    assert!(repr.contains("houdini-20.0.506"));
    assert!(repr.contains("python-3.10+<3.12"));
    assert!(repr.contains("transitive"));
}

#[test]
fn test_compute_depends_transitive_false_empty_repo() {
    let result = compute_depends("arnold", None, &[], false);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.transitive_dependants.is_empty(), "No transitive when flag=false");
}

#[test]
fn test_compute_depends_transitive_true_empty_repo() {
    let result = compute_depends("arnold", None, &[], true);
    assert!(result.is_ok());
    let r = result.unwrap();
    assert!(r.transitive_dependants.is_empty(), "Empty repo → 0 transitive");
}

#[test]
fn test_all_dependants_three_entries() {
    let make = |name: &str, ver: &str| PyDependsEntry {
        name: name.to_string(),
        version: ver.to_string(),
        requirement: "python-3".to_string(),
        dependency_type: "direct".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![make("a", "1.0"), make("b", "2.0"), make("c", "3.0")],
        transitive_dependants: vec![],
    };
    assert_eq!(result.all_dependants().len(), 3);
}

#[test]
fn test_format_contains_direct_label() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![PyDependsEntry {
            name: "maya".to_string(),
            version: "2024.1".to_string(),
            requirement: "python-3.9".to_string(),
            dependency_type: "direct".to_string(),
        }],
        transitive_dependants: vec![],
    };
    let output = result.format();
    assert!(
        output.contains("Direct"),
        "format() should contain 'Direct' label, got: {output}"
    );
}

#[test]
fn test_format_transitive_only_no_direct_label() {
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![PyDependsEntry {
            name: "nuke".to_string(),
            version: "14.0".to_string(),
            requirement: "maya-2024".to_string(),
            dependency_type: "transitive".to_string(),
        }],
    };
    let output = result.format();
    assert!(
        output.contains("Transitive") || !output.contains("Direct:"),
        "format with no direct entries should not show Direct section: {output}"
    );
}

#[test]
fn test_depends_entry_version_field() {
    let entry = PyDependsEntry {
        name: "houdini".to_string(),
        version: "20.0.506".to_string(),
        requirement: "python-3.10".to_string(),
        dependency_type: "direct".to_string(),
    };
    assert_eq!(entry.version, "20.0.506");
    assert_eq!(entry.name, "houdini");
}

#[test]
fn test_total_count_zero() {
    let result = PyDependsResult {
        queried_package: "ghost".to_string(),
        direct_dependants: vec![],
        transitive_dependants: vec![],
    };
    assert_eq!(result.total_count(), 0);
}

#[test]
fn test_print_depends_empty_paths_contains_package_name() {
    let output = print_depends("arnold", None, Some(vec![]), false);
    assert!(output.is_ok());
    let s = output.unwrap();
    assert!(
        s.contains("arnold"),
        "print_depends output should contain package name: {s}"
    );
}

#[test]
fn test_get_dependants_empty_paths_returns_empty_vec() {
    let result = get_dependants("python", None, Some(vec![]));
    assert!(result.is_ok());
    assert!(result.unwrap().is_empty());
}

#[test]
fn test_total_count_deduplication() {
    let entry = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3.9".to_string(),
        dependency_type: "direct".to_string(),
    };
    let entry2 = PyDependsEntry {
        name: "maya".to_string(),
        version: "2024.1".to_string(),
        requirement: "python-3.9".to_string(),
        dependency_type: "transitive".to_string(),
    };
    let result = PyDependsResult {
        queried_package: "python".to_string(),
        direct_dependants: vec![entry],
        transitive_dependants: vec![entry2],
    };
    assert_eq!(result.total_count(), 1);
}

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
