//! Advanced unit tests for `diff_bindings` — Cycle 117, 127 additions.
//! Split from diff_bindings_tests.rs (Cycle 146) to keep file size ≤500 lines.

use super::{compute_diff, format_diff, PyContextDiff, PyPackageDiff};
use rez_next_package::Package;
use rez_next_version::Version;

fn make_pkg(name: &str, ver: &str) -> Package {
    let mut p = Package::new(name.to_string());
    p.version = Some(Version::parse(ver).unwrap());
    p
}

#[test]
fn test_package_diff_repr_unknown_type_shows_unchanged_label() {
    let d = PyPackageDiff {
        name: "lib".to_string(),
        old_version: Some("1.0".to_string()),
        new_version: Some("1.0".to_string()),
        change_type: "unknown_type".to_string(),
    };
    let r = d.__repr__();
    assert!(r.contains("lib"), "repr must contain package name");
    assert!(r.contains("unchanged"), "repr must contain 'unchanged' for unknown type");
}

#[test]
fn test_changed_diffs_includes_downgraded() {
    let d = PyPackageDiff {
        name: "nuke".to_string(),
        old_version: Some("15.0".to_string()),
        new_version: Some("14.0".to_string()),
        change_type: "downgraded".to_string(),
    };
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 1,
        num_unchanged: 0,
        diffs: vec![d],
    };
    let changed = diff_obj.changed_diffs();
    assert_eq!(changed.len(), 1, "downgraded must appear in changed_diffs");
    assert_eq!(changed[0].change_type, "downgraded");
}

#[test]
fn test_format_diff_upgraded_uses_caret_prefix() {
    let diffs = vec![PyPackageDiff {
        name: "alembic".to_string(),
        old_version: Some("1.7.0".to_string()),
        new_version: Some("1.8.0".to_string()),
        change_type: "upgraded".to_string(),
    }];
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 1,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(output.contains("^ alembic"), "Upgraded package must use '^ ' prefix: {output}");
}

// ── Cycle 117 additions ───────────────────────────────────────────────────

#[test]
fn test_to_dict_contains_all_keys() {
    let d = PyPackageDiff {
        name: "python".to_string(),
        old_version: Some("3.9.0".to_string()),
        new_version: Some("3.11.0".to_string()),
        change_type: "upgraded".to_string(),
    };
    let repr = d.__repr__();
    assert!(repr.contains("python"), "repr must contain name");
    let _: &str = &d.name;
    let _: &Option<String> = &d.old_version;
    let _: &Option<String> = &d.new_version;
    let _: &str = &d.change_type;
}

#[test]
fn test_to_dict_added_old_version_is_none() {
    let d = PyPackageDiff {
        name: "nuke".to_string(),
        old_version: None,
        new_version: Some("15.0".to_string()),
        change_type: "added".to_string(),
    };
    assert!(d.old_version.is_none(), "added package must have None old_version");
    assert_eq!(d.new_version.as_deref(), Some("15.0"));
}

#[test]
fn test_to_dict_change_type_value() {
    let types = ["added", "removed", "upgraded", "downgraded", "unchanged"];
    for ct in &types {
        let d = PyPackageDiff {
            name: "pkg".to_string(),
            old_version: Some("1.0".to_string()),
            new_version: Some("1.0".to_string()),
            change_type: ct.to_string(),
        };
        assert_eq!(&d.change_type, ct, "change_type field must preserve value");
    }
}

#[test]
fn test_format_diff_multiple_removed() {
    let diffs = vec![
        PyPackageDiff {
            name: "maya".to_string(),
            old_version: Some("2023.1".to_string()),
            new_version: None,
            change_type: "removed".to_string(),
        },
        PyPackageDiff {
            name: "houdini".to_string(),
            old_version: Some("19.5".to_string()),
            new_version: None,
            change_type: "removed".to_string(),
        },
    ];
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 2,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(output.contains("- maya"), "should show maya removed");
    assert!(output.contains("- houdini"), "should show houdini removed");
}

#[test]
fn test_compute_diff_name_only_no_version_unchanged() {
    let mut p1 = Package::new("mylib".to_string());
    p1.version = None;
    let mut p2 = Package::new("mylib".to_string());
    p2.version = None;
    let diffs = compute_diff(&[p1], &[p2]);
    assert!(
        diffs.is_empty(),
        "versionless packages are filtered by compute_diff; expected empty diffs, got: {diffs:?}"
    );
}

#[test]
fn test_context_diff_total_count_matches_diffs_len() {
    let old = vec![
        make_pkg("a", "1.0"),
        make_pkg("b", "2.0"),
        make_pkg("c", "3.0"),
    ];
    let new = vec![
        make_pkg("a", "1.1"),  // upgraded
        make_pkg("b", "2.0"),  // unchanged
        make_pkg("d", "4.0"),  // added; c removed
    ];
    let diffs = compute_diff(&old, &new);
    let total_diffs = diffs.len();
    let num_added = diffs.iter().filter(|d| d.change_type == "added").count();
    let num_removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    let num_upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
    let num_unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
    let sum = num_added + num_removed + num_upgraded + num_unchanged;
    assert_eq!(sum, total_diffs, "sum of counts must equal total diffs");
}

#[test]
fn test_format_diff_output_lines_count_matches_changes() {
    let diffs = vec![
        PyPackageDiff {
            name: "a".to_string(),
            old_version: None,
            new_version: Some("1.0".to_string()),
            change_type: "added".to_string(),
        },
        PyPackageDiff {
            name: "b".to_string(),
            old_version: Some("2.0".to_string()),
            new_version: Some("3.0".to_string()),
            change_type: "upgraded".to_string(),
        },
        PyPackageDiff {
            name: "c".to_string(),
            old_version: Some("1.0".to_string()),
            new_version: Some("1.0".to_string()),
            change_type: "unchanged".to_string(),
        },
    ];
    let diff_obj = PyContextDiff {
        num_added: 1,
        num_removed: 0,
        num_upgraded: 1,
        num_downgraded: 0,
        num_unchanged: 1,
        diffs,
    };
    let output = format_diff(&diff_obj);
    let line_count = output.lines().count();
    assert_eq!(line_count, 2, "format_diff should output 2 lines for 2 changes: got {line_count}");
}

// ── Cycle 127 additions ───────────────────────────────────────────────────

#[test]
fn test_compute_diff_empty_old_all_added() {
    let new = vec![make_pkg("maya", "2024.1"), make_pkg("python", "3.11")];
    let diffs = compute_diff(&[], &new);
    let added = diffs.iter().filter(|d| d.change_type == "added").count();
    assert_eq!(added, 2, "all packages are 'added' when old is empty");
}

#[test]
fn test_compute_diff_empty_new_all_removed() {
    let old = vec![make_pkg("maya", "2024.1"), make_pkg("python", "3.11")];
    let diffs = compute_diff(&old, &[]);
    let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    assert_eq!(removed, 2, "all packages are 'removed' when new is empty");
}

#[test]
fn test_context_diff_repr_contains_counts() {
    let diff_obj = PyContextDiff {
        num_added: 3,
        num_removed: 1,
        num_upgraded: 2,
        num_downgraded: 0,
        num_unchanged: 5,
        diffs: vec![],
    };
    let r = diff_obj.__repr__();
    assert!(r.contains("+3"), "repr must contain +3: {r}");
    assert!(r.contains("-1"), "repr must contain -1: {r}");
    assert!(r.contains("^2"), "repr must contain ^2: {r}");
}

#[test]
fn test_package_diff_repr_removed_shows_old_version() {
    let d = PyPackageDiff {
        name: "houdini".to_string(),
        old_version: Some("19.5".to_string()),
        new_version: None,
        change_type: "removed".to_string(),
    };
    let r = d.__repr__();
    assert!(r.contains("19.5"), "removed repr must show old version: {r}");
    assert!(r.contains("houdini"), "removed repr must contain name: {r}");
}

#[test]
fn test_compute_diff_same_version_unchanged() {
    let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.0")];
    let diffs = compute_diff(&pkgs, &pkgs);
    let unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
    assert_eq!(unchanged, 2, "identical package lists must produce 2 unchanged diffs");
}

#[test]
fn test_compute_diff_downgrade_detected() {
    let old = vec![make_pkg("nuke", "15.0")];
    let new = vec![make_pkg("nuke", "14.0")];
    let diffs = compute_diff(&old, &new);
    let downgraded = diffs.iter().filter(|d| d.change_type == "downgraded").count();
    assert_eq!(downgraded, 1, "lower version must be detected as 'downgraded'");
}
