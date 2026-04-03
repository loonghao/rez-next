use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Package is_valid() / validate() tests (Phase 93) ─────────────────────

/// rez package: valid package passes is_valid()
#[test]
fn test_package_is_valid_basic() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(
        pkg.is_valid(),
        "Package with valid name and version should be valid"
    );
}

/// rez package: empty name fails is_valid()
#[test]
fn test_package_is_valid_empty_name() {
    use rez_next_package::Package;

    let pkg = Package::new("".to_string());
    assert!(
        !pkg.is_valid(),
        "Package with empty name should not be valid"
    );
}

/// rez package: invalid name chars fails validate()
#[test]
fn test_package_validate_invalid_name_chars() {
    use rez_next_package::Package;

    let pkg = Package::new("bad@pkg!name".to_string());
    assert!(
        pkg.validate().is_err(),
        "Package with special chars in name should fail validate()"
    );
    let err_msg = pkg.validate().unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid package name"),
        "Error should mention invalid name: {}",
        err_msg
    );
}

/// rez package: empty requirement in requires fails validate()
#[test]
fn test_package_validate_empty_requirement() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("mypkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    pkg.requires.push("".to_string()); // Empty requirement
    assert!(
        pkg.validate().is_err(),
        "Package with empty requirement should fail validate()"
    );
    assert!(
        !pkg.is_valid(),
        "is_valid() should return false for package with empty requirement"
    );
}

/// rez package: valid name formats (hyphen, underscore) pass is_valid()
#[test]
fn test_package_is_valid_name_variants() {
    use rez_next_package::Package;

    for name in &["my-pkg", "my_pkg", "MyPkg2", "pkg123"] {
        let pkg = Package::new(name.to_string());
        assert!(pkg.is_valid(), "Package name '{}' should be valid", name);
    }
}

/// rez package: empty build_requires entry fails validate()
#[test]
fn test_package_validate_empty_build_requirement() {
    use rez_next_package::Package;

    let mut pkg = Package::new("buildpkg".to_string());
    pkg.build_requires.push("cmake".to_string());
    pkg.build_requires.push("".to_string()); // invalid entry
    let result = pkg.validate();
    assert!(
        result.is_err(),
        "Empty build requirement should fail validation"
    );
}

