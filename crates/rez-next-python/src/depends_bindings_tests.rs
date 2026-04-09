//! Unit tests for `depends_bindings` — split from the main module to keep file size ≤ 1000 lines.
//! Advanced field/count/repr tests → depends_bindings_advanced_tests.rs (Cycle 146)

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
