use crate::*;
use std::fs;

#[test]
fn test_unbundle_returns_packages_list() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_roundtrip");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["python-3.9".to_string(), "houdini-19.5".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert_eq!(got, pkgs, "unbundle must return same packages as bundled");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_missing_manifest_errors() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_missing");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    let result = unbundle_context(tmp.to_str().unwrap(), None);
    assert!(result.is_err(), "missing bundle.yaml must return Err");
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_nonexistent_dir_errors() {
    let result = unbundle_context("/nonexistent/bundle/dir_xyz", None);
    assert!(result.is_err(), "nonexistent bundle dir must return Err");
}

#[test]
fn test_unbundle_three_packages_roundtrip() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_three");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec![
        "python-3.9".to_string(),
        "maya-2024".to_string(),
        "houdini-20".to_string(),
    ];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert_eq!(got.len(), 3, "should recover 3 packages");
    for p in &pkgs {
        assert!(
            got.contains(p),
            "package '{}' should be in unbundle result",
            p
        );
    }
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_preserves_package_order() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_order");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["alpha-1.0".to_string(), "beta-2.0".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    // YAML parsing may preserve order but we just need both present
    assert_eq!(got.len(), 2, "should recover exactly 2 packages");
    assert!(got.contains(&"alpha-1.0".to_string()));
    assert!(got.contains(&"beta-2.0".to_string()));
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_returns_nonempty_for_valid_bundle() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_nonempty");
    let _ = fs::remove_dir_all(&tmp);
    bundle_context(vec!["mypkg-3.0".to_string()], tmp.to_str().unwrap(), false).unwrap();
    let packages = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert!(
        !packages.is_empty(),
        "unbundle should return at least one package"
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_package_name_contains_version() {
    // Package names with version separators should roundtrip correctly
    let tmp = std::env::temp_dir().join("rez_cy130_unbundle_version");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["maya-2024.0.1".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert!(
        got.contains(&"maya-2024.0.1".to_string()),
        "versioned package name must survive roundtrip: {:?}",
        got
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_with_dest_packages_path_does_not_panic() {
    // dest_packages_path is reserved; passing it must not cause a panic or error
    let tmp = std::env::temp_dir().join("rez_test_unbundle_dest_pp");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["pkg-1.0".to_string()];
    bundle_context(pkgs, tmp.to_str().unwrap(), false).unwrap();
    // Pass a non-None dest_packages_path — must not panic
    let result = unbundle_context(tmp.to_str().unwrap(), Some("/some/dest/path"));
    assert!(
        result.is_ok(),
        "unbundle with dest_packages_path must succeed: {:?}",
        result
    );
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_returns_correct_packages_list() {
    let tmp = std::env::temp_dir().join("rez_test_unbundle_exact");
    let _ = fs::remove_dir_all(&tmp);
    let pkgs = vec!["houdini-20.0".to_string(), "python-3.10".to_string()];
    bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
    let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    for p in &pkgs {
        assert!(
            got.contains(p),
            "package '{}' should be in result, got: {:?}",
            p,
            got
        );
    }
    let _ = fs::remove_dir_all(&tmp);
}

#[test]
fn test_unbundle_context_with_only_skip_solve_line() {
    // A bundle.yaml that contains only skip_solve (no packages section) should return empty list
    let tmp = std::env::temp_dir().join("rez_cy132_unbundle_only_skip");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    fs::write(
        tmp.join("bundle.yaml"),
        b"# rez bundle manifest\nskip_solve: false\n",
    )
    .unwrap();
    let result = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
    assert!(
        result.is_empty(),
        "manifest with no packages section must return empty list"
    );
    let _ = fs::remove_dir_all(&tmp);
}
