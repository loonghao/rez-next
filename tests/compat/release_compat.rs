use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Release compatibility tests ────────────────────────────────────────────

/// Package version field is required for release
#[test]
fn test_release_package_version_required() {
    use rez_next_package::Package;

    let pkg = Package::new("mypkg".to_string());
    assert!(
        pkg.version.is_none(),
        "New package should have no version until set"
    );
}

/// Package with version can be serialized and used in release flow
#[test]
fn test_release_package_roundtrip_yaml() {
    use rez_next_package::serialization::PackageSerializer;

    let dir = tempfile::tempdir().unwrap();
    let yaml_path = dir.path().join("package.yaml");

    let content = "name: mypkg\nversion: '2.1.0'\ndescription: Test package for release\n";
    std::fs::write(&yaml_path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&yaml_path).unwrap();
    assert_eq!(pkg.name, "mypkg");
    let ver = pkg
        .version
        .as_ref()
        .expect("version must be set after parse");
    assert_eq!(ver.as_str(), "2.1.0");
}

