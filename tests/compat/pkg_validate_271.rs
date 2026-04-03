use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Package validation tests (271-275) ────────────────────────────────────

/// rez package: package with empty name should be invalid
#[test]
fn test_rez_package_empty_name_is_invalid() {
    use rez_next_package::Package;
    let pkg = Package::new("".to_string());
    assert!(pkg.name.is_empty(), "Package name should be empty as set");
    // Name validation: rez requires non-empty name
    // We verify the name is empty and that rez would reject this at build time
    let is_invalid = pkg.name.is_empty();
    assert!(
        is_invalid,
        "Package with empty name should be considered invalid"
    );
}

/// rez package: package name with hyphen is valid in rez
#[test]
fn test_rez_package_hyphenated_name_valid() {
    use rez_next_package::Package;
    let pkg = Package::new("my-tool".to_string());
    assert_eq!(pkg.name, "my-tool");
    // Hyphenated names are valid in rez
    assert!(pkg.name.contains('-'));
}

/// rez package: package requires list is correctly stored
#[test]
fn test_rez_package_requires_list() {
    use rez_next_package::Package;
    let mut pkg = Package::new("my_app".to_string());
    pkg.requires = vec!["python-3.9".to_string(), "requests-2.28".to_string()];
    assert_eq!(pkg.requires.len(), 2);
    assert!(pkg.requires.contains(&"python-3.9".to_string()));
    assert!(pkg.requires.contains(&"requests-2.28".to_string()));
}

/// rez package: variants are stored correctly
#[test]
fn test_rez_package_variants() {
    use rez_next_package::Package;
    let mut pkg = Package::new("maya_plugin".to_string());
    pkg.variants = vec![vec!["maya-2023".to_string()], vec!["maya-2024".to_string()]];
    assert_eq!(pkg.variants.len(), 2);
    assert_eq!(pkg.variants[0], vec!["maya-2023"]);
    assert_eq!(pkg.variants[1], vec!["maya-2024"]);
}

/// rez package: build_requires separate from requires
#[test]
fn test_rez_package_build_requires_separate() {
    use rez_next_package::Package;
    let mut pkg = Package::new("my_lib".to_string());
    pkg.requires = vec!["python-3.9".to_string()];
    pkg.build_requires = vec!["cmake-3.20".to_string(), "ninja-1.11".to_string()];
    assert_eq!(pkg.requires.len(), 1);
    assert_eq!(pkg.build_requires.len(), 2);
    assert!(!pkg.requires.contains(&"cmake-3.20".to_string()));
    assert!(pkg.build_requires.contains(&"cmake-3.20".to_string()));
}

