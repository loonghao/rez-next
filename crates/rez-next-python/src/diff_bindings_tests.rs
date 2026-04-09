//! Unit tests for `diff_bindings`.
//!
//! Extracted from `diff_bindings.rs` to keep each file under 1000 lines.
//! Advanced repr/format/count tests → diff_bindings_advanced_tests.rs (Cycle 146)

use super::{compute_diff, format_diff, PyContextDiff, PyPackageDiff};
use rez_next_package::Package;
use rez_next_version::Version;

fn make_pkg(name: &str, ver: &str) -> Package {
    let mut p = Package::new(name.to_string());
    p.version = Some(Version::parse(ver).unwrap());
    p
}

#[test]
fn test_identical_contexts_no_diff() {
    let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&pkgs, &pkgs);
    let changed: Vec<_> = diffs
        .iter()
        .filter(|d| d.change_type != "unchanged")
        .collect();
    assert!(
        changed.is_empty(),
        "Identical contexts should have no changes"
    );
}

#[test]
fn test_added_package() {
    let old = vec![make_pkg("python", "3.9.0")];
    let new = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&old, &new);
    let added: Vec<_> = diffs.iter().filter(|d| d.change_type == "added").collect();
    assert_eq!(added.len(), 1, "Should detect 1 added package");
    assert_eq!(added[0].name, "maya");
}

#[test]
fn test_removed_package() {
    let old = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.1")];
    let new = vec![make_pkg("python", "3.9.0")];
    let diffs = compute_diff(&old, &new);
    let removed: Vec<_> = diffs
        .iter()
        .filter(|d| d.change_type == "removed")
        .collect();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].name, "maya");
    assert_eq!(removed[0].old_version.as_deref(), Some("2023.1"));
}

#[test]
fn test_upgraded_package() {
    let old = vec![make_pkg("python", "3.9.0")];
    let new = vec![make_pkg("python", "3.11.0")];
    let diffs = compute_diff(&old, &new);
    let upgraded: Vec<_> = diffs
        .iter()
        .filter(|d| d.change_type == "upgraded")
        .collect();
    assert_eq!(upgraded.len(), 1);
    assert_eq!(upgraded[0].old_version.as_deref(), Some("3.9.0"));
    assert_eq!(upgraded[0].new_version.as_deref(), Some("3.11.0"));
}

#[test]
fn test_downgraded_package() {
    let old = vec![make_pkg("maya", "2024.1")];
    let new = vec![make_pkg("maya", "2023.1")];
    let diffs = compute_diff(&old, &new);
    let down: Vec<_> = diffs
        .iter()
        .filter(|d| d.change_type == "downgraded")
        .collect();
    assert_eq!(down.len(), 1);
    assert_eq!(down[0].name, "maya");
}

#[test]
fn test_mixed_diff() {
    let old = vec![
        make_pkg("python", "3.9.0"),
        make_pkg("maya", "2023.1"),
        make_pkg("houdini", "19.5"),
    ];
    let new = vec![
        make_pkg("python", "3.11.0"), // upgraded
        make_pkg("houdini", "19.5"),  // unchanged
        make_pkg("nuke", "14.0"),     // added
                                      // maya removed
    ];
    let diffs = compute_diff(&old, &new);
    let added = diffs.iter().filter(|d| d.change_type == "added").count();
    let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    let upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
    let unchanged = diffs
        .iter()
        .filter(|d| d.change_type == "unchanged")
        .count();
    assert_eq!(added, 1, "nuke should be added");
    assert_eq!(removed, 1, "maya should be removed");
    assert_eq!(upgraded, 1, "python should be upgraded");
    assert_eq!(unchanged, 1, "houdini should be unchanged");
}

#[test]
fn test_empty_old_context() {
    let new = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&[], &new);
    let added = diffs.iter().filter(|d| d.change_type == "added").count();
    assert_eq!(added, 2, "All packages in new should be 'added'");
}

#[test]
fn test_empty_new_context() {
    let old = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&old, &[]);
    let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    assert_eq!(removed, 2, "All packages in old should be 'removed'");
}

#[test]
fn test_format_diff_no_changes() {
    let diffs = compute_diff(
        &[make_pkg("python", "3.9.0")],
        &[make_pkg("python", "3.9.0")],
    );
    let dummy = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: diffs.len(),
        diffs,
    };
    let output = format_diff(&dummy);
    assert_eq!(output, "  (no changes)");
}

#[test]
fn test_format_diff_with_changes() {
    let old = vec![make_pkg("python", "3.9.0")];
    let new = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&old, &new);
    let diff_obj = PyContextDiff {
        num_added: 1,
        num_removed: 0,
        num_upgraded: 1,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(output.contains("+ maya"), "Should show added package");
    assert!(output.contains("^ python"), "Should show upgraded package");
}

#[test]
fn test_is_identical_true() {
    let diffs = compute_diff(
        &[make_pkg("python", "3.9.0")],
        &[make_pkg("python", "3.9.0")],
    );
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 1,
        diffs,
    };
    assert!(diff_obj.is_identical());
}

#[test]
fn test_is_identical_false_when_changed() {
    let old = vec![make_pkg("python", "3.9.0")];
    let new = vec![make_pkg("python", "3.11.0")];
    let diffs = compute_diff(&old, &new);
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 1,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    assert!(!diff_obj.is_identical());
}

#[test]
fn test_both_contexts_empty() {
    let diffs = compute_diff(&[], &[]);
    assert!(
        diffs.is_empty(),
        "Both empty contexts should produce no diffs"
    );
}

#[test]
fn test_changed_diffs_excludes_unchanged() {
    let old = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.1")];
    let new = vec![
        make_pkg("python", "3.11.0"), // upgraded
        make_pkg("maya", "2023.1"),   // unchanged
    ];
    let diffs = compute_diff(&old, &new);
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 1,
        num_downgraded: 0,
        num_unchanged: 1,
        diffs,
    };
    let changed = diff_obj.changed_diffs();
    assert_eq!(changed.len(), 1, "Only upgraded python should be returned");
    assert_eq!(changed[0].change_type, "upgraded");
}

#[test]
fn test_changed_diffs_empty_when_identical() {
    let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
    let diffs = compute_diff(&pkgs, &pkgs);
    let num_unchanged = diffs.len();
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged,
        diffs,
    };
    assert!(
        diff_obj.changed_diffs().is_empty(),
        "Identical contexts should have no changed_diffs"
    );
}

#[test]
fn test_sort_order_added_before_removed() {
    let old = vec![make_pkg("aaaa", "1.0.0")];
    let new = vec![make_pkg("zzzz", "1.0.0")];
    let diffs = compute_diff(&old, &new);
    assert_eq!(diffs[0].change_type, "added", "added should come first");
    assert_eq!(
        diffs[1].change_type, "removed",
        "removed should come second"
    );
}

#[test]
fn test_sort_order_within_same_type_alphabetical() {
    let old: Vec<Package> = vec![];
    let new = vec![
        make_pkg("zlib", "1.2.0"),
        make_pkg("alembic", "1.7.0"),
        make_pkg("mesa", "22.0.0"),
    ];
    let diffs = compute_diff(&old, &new);
    let names: Vec<&str> = diffs.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, vec!["alembic", "mesa", "zlib"]);
}

#[test]
fn test_package_diff_repr_added() {
    let d = PyPackageDiff {
        name: "maya".to_string(),
        old_version: None,
        new_version: Some("2024.1".to_string()),
        change_type: "added".to_string(),
    };
    let r = d.__repr__();
    assert_eq!(r, "PackageDiff(+maya 2024.1)");
}

#[test]
fn test_package_diff_repr_removed() {
    let d = PyPackageDiff {
        name: "houdini".to_string(),
        old_version: Some("19.5".to_string()),
        new_version: None,
        change_type: "removed".to_string(),
    };
    let r = d.__repr__();
    assert_eq!(r, "PackageDiff(-houdini 19.5)");
}

#[test]
fn test_package_diff_repr_upgraded() {
    let d = PyPackageDiff {
        name: "python".to_string(),
        old_version: Some("3.9.0".to_string()),
        new_version: Some("3.11.0".to_string()),
        change_type: "upgraded".to_string(),
    };
    let r = d.__repr__();
    assert_eq!(r, "PackageDiff(python: 3.9.0 -> 3.11.0)");
}

#[test]
fn test_package_diff_repr_downgraded() {
    let d = PyPackageDiff {
        name: "nuke".to_string(),
        old_version: Some("14.0".to_string()),
        new_version: Some("13.0".to_string()),
        change_type: "downgraded".to_string(),
    };
    let r = d.__repr__();
    assert_eq!(r, "PackageDiff(nuke: 14.0 -> 13.0 [downgrade])");
}

#[test]
fn test_context_diff_repr_format() {
    let diff_obj = PyContextDiff {
        num_added: 2,
        num_removed: 1,
        num_upgraded: 3,
        num_downgraded: 0,
        num_unchanged: 5,
        diffs: vec![],
    };
    let r = diff_obj.__repr__();
    assert_eq!(r, "ContextDiff(+2 -1 ^3 v0 =5)");
}

#[test]
fn test_format_diff_removed_uses_minus_prefix() {
    let diffs = vec![PyPackageDiff {
        name: "maya".to_string(),
        old_version: Some("2023.1".to_string()),
        new_version: None,
        change_type: "removed".to_string(),
    }];
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 1,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(
        output.contains("- maya 2023.1"),
        "Removed package should use '- ' prefix: got {output}"
    );
}

#[test]
fn test_format_diff_downgraded_uses_v_prefix() {
    let diffs = vec![PyPackageDiff {
        name: "nuke".to_string(),
        old_version: Some("14.0".to_string()),
        new_version: Some("13.0".to_string()),
        change_type: "downgraded".to_string(),
    }];
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 1,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(
        output.contains("v nuke"),
        "Downgraded package should use 'v ' prefix: got {output}"
    );
}

// ── Cycle 101 additions ───────────────────────────────────────────────────

#[test]
fn test_compute_diff_single_unchanged() {
    let pkg = make_pkg("python", "3.10.0");
    let diffs = compute_diff(std::slice::from_ref(&pkg), std::slice::from_ref(&pkg));
    let unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
    assert_eq!(unchanged, 1, "Single identical package should be unchanged");
}

#[test]
fn test_package_diff_repr_unchanged_contains_name() {
    let d = PyPackageDiff {
        name: "boost".to_string(),
        old_version: Some("1.83.0".to_string()),
        new_version: Some("1.83.0".to_string()),
        change_type: "unchanged".to_string(),
    };
    let r = d.__repr__();
    assert!(r.contains("boost"), "repr must contain package name: {r}");
    assert!(r.contains("unchanged"), "repr must say unchanged: {r}");
}

#[test]
fn test_context_diff_is_identical_with_only_unchanged() {
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 3,
        diffs: vec![],
    };
    assert!(diff_obj.is_identical(), "all-unchanged diff must be identical");
}

#[test]
fn test_context_diff_is_not_identical_with_downgrade() {
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 1,
        num_unchanged: 0,
        diffs: vec![],
    };
    assert!(!diff_obj.is_identical(), "downgrade means not identical");
}

#[test]
fn test_format_diff_upgraded_shows_versions() {
    let diffs = vec![PyPackageDiff {
        name: "python".to_string(),
        old_version: Some("3.9.0".to_string()),
        new_version: Some("3.11.0".to_string()),
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
    assert!(output.contains("3.9.0"), "should show old version: {output}");
    assert!(output.contains("3.11.0"), "should show new version: {output}");
}

#[test]
fn test_compute_diff_multiple_upgrades() {
    let old = vec![
        make_pkg("python", "3.9.0"),
        make_pkg("numpy", "1.24.0"),
    ];
    let new = vec![
        make_pkg("python", "3.11.0"),
        make_pkg("numpy", "1.26.0"),
    ];
    let diffs = compute_diff(&old, &new);
    let upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
    assert_eq!(upgraded, 2, "both packages should be upgraded");
}

#[test]
fn test_context_diff_repr_zero_all() {
    let diff_obj = PyContextDiff {
        num_added: 0,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs: vec![],
    };
    let r = diff_obj.__repr__();
    assert_eq!(r, "ContextDiff(+0 -0 ^0 v0 =0)");
}

// ── Cycle 112 additions ───────────────────────────────────────────────────

#[test]
fn test_package_diff_str_equals_repr() {
    let d = PyPackageDiff {
        name: "python".to_string(),
        old_version: Some("3.9.0".to_string()),
        new_version: Some("3.11.0".to_string()),
        change_type: "upgraded".to_string(),
    };
    assert_eq!(d.__str__(), d.__repr__(), "__str__ and __repr__ must be identical");
}

#[test]
fn test_format_diff_added_uses_plus_prefix() {
    let diffs = vec![PyPackageDiff {
        name: "houdini".to_string(),
        old_version: None,
        new_version: Some("20.0".to_string()),
        change_type: "added".to_string(),
    }];
    let diff_obj = PyContextDiff {
        num_added: 1,
        num_removed: 0,
        num_upgraded: 0,
        num_downgraded: 0,
        num_unchanged: 0,
        diffs,
    };
    let output = format_diff(&diff_obj);
    assert!(output.contains("+ houdini"), "Added package must use '+ ' prefix: {output}");
}

#[test]
fn test_compute_diff_counts_are_correct() {
    let old = vec![
        make_pkg("a", "1.0"),
        make_pkg("b", "1.0"),
        make_pkg("c", "2.0"),
    ];
    let new = vec![
        make_pkg("a", "1.1"),  // upgraded
        make_pkg("b", "1.0"),  // unchanged
        make_pkg("d", "3.0"),  // added; c removed
    ];
    let diffs = compute_diff(&old, &new);
    assert_eq!(diffs.iter().filter(|d| d.change_type == "added").count(), 1);
    assert_eq!(diffs.iter().filter(|d| d.change_type == "removed").count(), 1);
    assert_eq!(diffs.iter().filter(|d| d.change_type == "upgraded").count(), 1);
    assert_eq!(diffs.iter().filter(|d| d.change_type == "unchanged").count(), 1);
}
