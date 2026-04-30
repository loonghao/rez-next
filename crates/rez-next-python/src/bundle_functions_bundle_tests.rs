use crate::*;
use std::fs;

#[test]
fn test_bundle_context_creates_directory() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_create");
    let _ = fs::remove_dir_all(&tmp);
    let result = bundle_context(vec!["python-3.9".to_string()], tmp.to_str().unwrap(), false);
    assert!(result.is_ok(), "bundle_context must succeed: {:?}", result);
    assert!(tmp.exists(), "bundle directory must be created");
    assert!(tmp.join("bundle.yaml").exists(), "bundle.yaml must exist");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_manifest_contains_packages() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_manifest");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(
        vec!["python-3.9".to_string(), "maya-2024".to_string()],
        tmp.to_str().unwrap(),
        false,
    )
    .unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("python-3.9"),
        "manifest must contain python-3.9"
    );
    assert!(
        content.contains("maya-2024"),
        "manifest must contain maya-2024"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_skip_solve_recorded() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_skip");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), true).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("skip_solve: true"),
        "skip_solve must be true in manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_returns_dest_path() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_ret");
    let _ = fs::remove_dir_all(&tmp);
    let returned = bundle_context(vec![], tmp.to_str().unwrap(), false).unwrap();
    assert!(
        returned.contains("rez_test_bundle_ret"),
        "returned path should contain the dest dir name: {}",
        returned
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_manifest_header_comment() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_header");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["python-3.9".to_string()], tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("# rez bundle manifest"),
        "manifest should have header comment"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_skip_solve_false_recorded() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_skip_false");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("skip_solve: false"),
        "skip_solve false must appear in manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_single_package_manifest() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_single_pkg");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(
        vec!["houdini-20.0".to_string()],
        tmp.to_str().unwrap(),
        false,
    )
    .unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("houdini-20.0"),
        "single package must appear in manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_overwrite_replaces_manifest() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_overwrite");
    let _ = fs::remove_dir_all(&tmp);
    // First bundle
    bundle_context(
        vec!["old-pkg-1.0".to_string()],
        tmp.to_str().unwrap(),
        false,
    )
    .unwrap();
    // Second bundle with different contents
    bundle_context(
        vec!["new-pkg-2.0".to_string()],
        tmp.to_str().unwrap(),
        false,
    )
    .unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("new-pkg-2.0"),
        "overwritten manifest must have new package"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_dest_path_string_valid() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_path_valid");
    let _ = fs::remove_dir_all(&tmp);
    let result =
        bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    assert!(!result.is_empty(), "returned path must not be empty");
    assert!(
        !result.contains('\0'),
        "returned path must not contain null bytes"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_skip_solve_false_default() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_skip_default");
    let _ = fs::remove_dir_all(&tmp);
    // skip_solve=false is the default
    bundle_context(vec!["tool-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("skip_solve: false"),
        "default skip_solve must be false in manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_idempotent_manifest_write() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_idempotent");
    let _ = fs::remove_dir_all(&tmp);
    // Call twice with same args; second call should overwrite and give same content
    let pkgs = vec!["tool-1.0".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let first = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let second = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert_eq!(
        first, second,
        "idempotent bundle should produce same manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_manifest_packages_key_present() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_yaml_key");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["foo-1.0".to_string()];
    bundle_context(pkgs, tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("packages:"),
        "manifest must contain 'packages:' key"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_skip_solve_true_in_manifest() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_skip_true2");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), true).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("skip_solve: true"),
        "skip_solve must be true in manifest"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_two_packages_both_in_manifest() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_two_pkgs_check");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["alpha-1.0".to_string(), "beta-2.0".to_string()];
    bundle_context(pkgs, tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("alpha-1.0"),
        "manifest must list alpha-1.0"
    );
    assert!(content.contains("beta-2.0"), "manifest must list beta-2.0");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_nested_dest_created() {
    // Nested dest path should be created automatically
    let tmp = std::env::temp_dir()
        .join("rez_cy130_bundle_nested")
        .join("sub")
        .join("dir");
    let _ = fs::remove_dir_all(std::env::temp_dir().join("rez_cy130_bundle_nested"));
    let result = bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), false);
    assert!(
        result.is_ok(),
        "nested dest should be created: {:?}",
        result
    );
    assert!(
        tmp.join("bundle.yaml").exists(),
        "bundle.yaml must exist in nested dest"
    );
    let _ = fs::remove_dir_all(std::env::temp_dir().join("rez_cy130_bundle_nested"));
}

#[test]
fn test_bundle_context_returns_absolute_path() {
    let tmp = std::env::temp_dir().join("rez_cy130_abs_path");
    let _ = fs::remove_dir_all(&tmp);
    let result =
        bundle_context(vec!["tool-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    // The returned path must be an absolute-looking string (starts with / or drive letter)
    assert!(
        result.starts_with('/') || (result.len() > 2 && result.chars().nth(1) == Some(':')),
        "returned path should be absolute: {}",
        result
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_with_version_specifier_package() {
    // Packages with version range specifiers should survive the manifest roundtrip
    let tmp = std::env::temp_dir().join("rez_cy130_bundle_range_pkg");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["python-3+".to_string(), "numpy-1.20+,<2".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("python-3+"),
        "range pkg must appear in manifest"
    );
    assert!(
        content.contains("numpy-1.20+,<2"),
        "complex range pkg must appear"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_dest_path_returned_is_dest_dir() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_ret_path");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["realpkg-0.5".to_string()];
    let returned = bundle_context(pkgs, tmp.to_str().unwrap(), false).unwrap();
    // Normalize separators for comparison
    assert!(
        returned
            .replace('\\', "/")
            .contains("rez_test_bundle_ret_path"),
        "returned path should contain dest dir name, got: {returned}"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_yaml_contains_dash_item_format() {
    // Each package in the manifest appears as "  - <pkg>"
    let tmp = std::env::temp_dir().join("rez_test_bundle_dash_fmt");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["mypkg-1.0".to_string()];
    bundle_context(pkgs, tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("  - mypkg-1.0"),
        "manifest items must be in YAML list format: {content}"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_multiple_calls_safe() {
    // Calling bundle_context twice on the same dest overwrites cleanly
    let tmp = std::env::temp_dir().join("rez_test_bundle_twice");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["a-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    bundle_context(vec!["b-2.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
    assert!(
        content.contains("b-2.0"),
        "second bundle must overwrite first"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_three_packages_manifest_length() {
    let tmp = std::env::temp_dir().join("rez_test_bundle_three_len");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec![
        "p1-1.0".to_string(),
        "p2-2.0".to_string(),
        "p3-3.0".to_string(),
    ];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert_eq!(got.len(), 3, "should recover all 3 packages");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_bundle_context_result_has_no_null_bytes() {
    let tmp = std::env::temp_dir().join("rez_cy132_bundle_null");
    let _ = fs::remove_dir_all(&tmp);
    let returned =
        bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    assert!(
        !returned.contains('\0'),
        "bundle_context result must not contain null bytes"
    );
    let _ = fs::remove_dir_all(&tmp);
}
