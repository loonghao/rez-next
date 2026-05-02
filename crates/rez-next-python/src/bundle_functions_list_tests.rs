use crate::*;
use std::fs;

#[test]
fn test_list_bundles_empty_directory() {
    let tmp = std::env::temp_dir().join("rez_test_list_bundles_empty");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let result = list_bundles(Some(tmp.to_str().unwrap())).unwrap();
    assert!(result.is_empty(), "no bundles in empty dir");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_list_bundles_finds_bundle_dirs() {
    let base = std::env::temp_dir().join("rez_test_list_bundles_found");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // Create two bundle directories
    let b1 = base.join("bundle_alpha");
    let b2 = base.join("bundle_beta");
    fs::create_dir_all(&b1).unwrap();
    fs::create_dir_all(&b2).unwrap();
    fs::write(b1.join("bundle.yaml"), b"packages:\n  - python-3.9\n").unwrap();
    fs::write(b2.join("bundle.yaml"), b"packages:\n  - maya-2024\n").unwrap();

    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert!(
        result.contains(&"bundle_alpha".to_string()),
        "should find bundle_alpha: {:?}",
        result
    );
    assert!(
        result.contains(&"bundle_beta".to_string()),
        "should find bundle_beta: {:?}",
        result
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_sorted() {
    let base = std::env::temp_dir().join("rez_test_list_bundles_sorted");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    for name in ["zzz_bundle", "aaa_bundle", "mmm_bundle"] {
        let bdir = base.join(name);
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("bundle.yaml"), b"packages:\n").unwrap();
    }

    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    let mut sorted = result.clone();
    sorted.sort();
    assert_eq!(result, sorted, "list_bundles should return sorted results");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_nonexistent_path_returns_empty() {
    let result = list_bundles(Some("/nonexistent_bundle_search_path_xyz")).unwrap();
    assert!(
        result.is_empty(),
        "nonexistent path should return empty list"
    );
}

#[test]
fn test_list_bundles_only_dirs_with_yaml() {
    let base = std::env::temp_dir().join("rez_test_list_bundles_filter");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // Dir with bundle.yaml → should be listed
    let valid = base.join("valid_bundle");
    fs::create_dir_all(&valid).unwrap();
    fs::write(valid.join("bundle.yaml"), b"packages:\n").unwrap();

    // Dir without bundle.yaml → should NOT be listed
    let invalid = base.join("not_a_bundle");
    fs::create_dir_all(&invalid).unwrap();

    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert!(
        result.contains(&"valid_bundle".to_string()),
        "valid_bundle must be listed"
    );
    assert!(
        !result.contains(&"not_a_bundle".to_string()),
        "not_a_bundle must NOT be listed"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_bundle_unbundle_empty_package_list() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_empty_pkgs");
    let _ = fs::remove_dir_all(&tmp);
    // Bundle with empty package list
    bundle_context(vec![], tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert!(
        got.is_empty(),
        "unbundle of empty bundle should return empty list"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_list_bundles_ignores_files_not_dirs() {
    let base = std::env::temp_dir().join("rez_test_list_bundles_files");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();

    // A regular file should not be listed as a bundle
    fs::write(base.join("bundle.yaml"), b"packages:\n").unwrap();

    // A directory with bundle.yaml should be listed
    let bdir = base.join("real_bundle");
    fs::create_dir_all(&bdir).unwrap();
    fs::write(bdir.join("bundle.yaml"), b"packages:\n  - python-3.9\n").unwrap();

    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert!(
        result.contains(&"real_bundle".to_string()),
        "real_bundle must appear"
    );
    // The file named bundle.yaml at the base level should not be listed
    assert!(
        !result.iter().any(|s| s.is_empty()),
        "no empty-name bundles"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_multiple_bundle_dirs_sorted() {
    let base = std::env::temp_dir().join("rez_test_list_bundles_multi_sorted");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    for name in ["delta_b", "alpha_b", "charlie_b", "beta_b"] {
        let bdir = base.join(name);
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("bundle.yaml"), b"packages:\n").unwrap();
    }
    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert_eq!(result.len(), 4, "should find all 4 bundles");
    let mut sorted = result.clone();
    sorted.sort();
    assert_eq!(result, sorted, "bundles should be returned in sorted order");
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_counts_correctly() {
    let base = std::env::temp_dir().join("rez_cy130_list_count");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    for name in ["b1", "b2", "b3"] {
        let bdir = base.join(name);
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("bundle.yaml"), b"packages:\n").unwrap();
    }
    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert_eq!(
        result.len(),
        3,
        "should find exactly 3 bundles, got: {:?}",
        result
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_default_path_returns_result() {
    // Calling with None should not panic; returns an empty or non-empty list
    let result = list_bundles(None);
    assert!(
        result.is_ok(),
        "list_bundles(None) must not error: {:?}",
        result
    );
}

#[test]
fn test_list_bundles_with_no_valid_bundles_returns_empty() {
    // A directory with subdirs but no bundle.yaml in them → empty list
    let base = std::env::temp_dir().join("rez_test_list_no_yaml");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    fs::create_dir_all(base.join("not_a_bundle")).unwrap();
    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert!(
        result.is_empty(),
        "dirs without bundle.yaml must not be listed"
    );
    let _ = fs::remove_dir_all(&base);
}

#[test]
fn test_list_bundles_returns_sorted_alphabetically() {
    let base = std::env::temp_dir().join("rez_cy132_list_sorted_alpha");
    let _ = fs::remove_dir_all(&base);
    fs::create_dir_all(&base).unwrap();
    for name in ["z_bundle", "a_bundle", "m_bundle"] {
        let bdir = base.join(name);
        fs::create_dir_all(&bdir).unwrap();
        fs::write(bdir.join("bundle.yaml"), b"packages:\n").unwrap();
    }
    let result = list_bundles(Some(base.to_str().unwrap())).unwrap();
    assert_eq!(result.len(), 3, "should find all 3 bundles");
    assert_eq!(
        result[0], "a_bundle",
        "first must be a_bundle (alphabetical)"
    );
    assert_eq!(
        result[2], "z_bundle",
        "last must be z_bundle (alphabetical)"
    );
    let _ = fs::remove_dir_all(&base);
}
