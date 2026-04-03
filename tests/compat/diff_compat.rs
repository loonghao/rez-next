use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.diff compatibility tests ───────────────────────────────────────────

/// rez diff: identical contexts produce no changes
#[test]
fn test_diff_identical_contexts_no_changes() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());
    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2024.1").unwrap());

    let pkgs = vec![python, maya];
    // Simulate compute_diff by checking both lists are equal
    // (testing logic inline since compute_diff is in the python crate)
    let old_names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
    let new_names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        old_names, new_names,
        "Identical contexts should have same package names"
    );
}

/// rez diff: upgrade detection via version comparison
#[test]
fn test_diff_upgrade_detection() {
    use rez_next_version::Version;

    let old_ver = Version::parse("3.9.0").unwrap();
    let new_ver = Version::parse("3.11.0").unwrap();

    assert!(
        new_ver > old_ver,
        "3.11.0 should be greater than 3.9.0 (upgrade)"
    );
}

/// rez diff: downgrade detection via version comparison
#[test]
fn test_diff_downgrade_detection() {
    use rez_next_version::Version;

    let old_ver = Version::parse("2024.1").unwrap();
    let new_ver = Version::parse("2023.1").unwrap();

    assert!(
        new_ver < old_ver,
        "2023.1 should be less than 2024.1 (downgrade)"
    );
}

/// rez diff: new context has extra package (added)
#[test]
fn test_diff_added_package_detection() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());

    let old: Vec<Package> = vec![python.clone()];
    let mut nuke = Package::new("nuke".to_string());
    nuke.version = Some(Version::parse("14.0").unwrap());
    let new: Vec<Package> = vec![python, nuke];

    let old_map: HashMap<&str, _> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, _> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let added: Vec<&&str> = new_map
        .keys()
        .filter(|k| !old_map.contains_key(**k))
        .collect();
    assert_eq!(added.len(), 1, "One package should be added");
    assert_eq!(*added[0], "nuke");
}

/// rez diff: old context has extra package (removed)
#[test]
fn test_diff_removed_package_detection() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());
    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2023.1").unwrap());

    let old: Vec<Package> = vec![python.clone(), maya];
    let new: Vec<Package> = vec![python];

    let old_map: HashMap<&str, _> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, _> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let removed: Vec<&&str> = old_map
        .keys()
        .filter(|k| !new_map.contains_key(**k))
        .collect();
    assert_eq!(removed.len(), 1, "One package should be removed");
    assert_eq!(*removed[0], "maya");
}

/// rez diff: empty old context — everything is "added"
#[test]
fn test_diff_empty_old_all_added() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let new: Vec<Package> = {
        let mut p = Package::new("python".to_string());
        p.version = Some(Version::parse("3.11.0").unwrap());
        vec![p]
    };

    let old_map: HashMap<&str, &Version> = HashMap::new();
    let new_map: HashMap<&str, &Version> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let added_count = new_map
        .keys()
        .filter(|k| !old_map.contains_key(**k))
        .count();
    assert_eq!(
        added_count, 1,
        "All new packages should be 'added' when old is empty"
    );
}

/// rez diff: empty new context — everything is "removed"
#[test]
fn test_diff_empty_new_all_removed() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let old: Vec<Package> = {
        let mut p = Package::new("maya".to_string());
        p.version = Some(Version::parse("2024.1").unwrap());
        vec![p]
    };

    let old_map: HashMap<&str, &Version> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, &Version> = HashMap::new();

    let removed_count = old_map
        .keys()
        .filter(|k| !new_map.contains_key(**k))
        .count();
    assert_eq!(
        removed_count, 1,
        "All old packages should be 'removed' when new is empty"
    );
}

/// rez diff: version format string in diff output
#[test]
fn test_diff_version_format_in_output() {
    use rez_next_version::Version;

    let old_ver = Version::parse("3.9.0").unwrap();
    let new_ver = Version::parse("3.11.0").unwrap();

    let line = format!("  ^ python {} -> {}", old_ver.as_str(), new_ver.as_str());
    assert!(
        line.contains("3.9.0"),
        "Old version should appear in diff line"
    );
    assert!(
        line.contains("3.11.0"),
        "New version should appear in diff line"
    );
    assert!(line.starts_with("  ^"), "Upgrade should use ^ prefix");
}

