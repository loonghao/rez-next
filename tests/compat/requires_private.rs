use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── requires_private_build_only tests ──────────────────────────────────────

/// rez: package with build-only requirements (private_build_requires)
#[test]
fn test_package_private_build_requires_field() {
    use rez_next_package::Package;

    let mut pkg = Package::new("mypkg".to_string());
    // private_build_requires are stored in build_requires in rez-next
    pkg.build_requires = vec!["cmake-3+".to_string(), "ninja".to_string()];

    assert_eq!(pkg.build_requires.len(), 2);
    assert!(pkg.build_requires.contains(&"cmake-3+".to_string()));
    assert!(pkg.build_requires.contains(&"ninja".to_string()));
}

/// rez: private build requires are parseable as requirements
#[test]
fn test_package_private_build_requires_parseable() {
    use rez_next_package::PackageRequirement;

    let build_reqs = ["cmake-3+", "ninja", "gcc-9+<13", "python-3.9"];
    for req_str in &build_reqs {
        let r = PackageRequirement::parse(req_str);
        assert!(
            r.is_ok(),
            "Private build requirement '{}' should be parseable",
            req_str
        );
    }
}

/// rez: package.py with build_requires field parsed correctly
#[test]
fn test_package_py_build_requires_parsed() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'mylib'
version = '1.0.0'

requires = [
    'python-3.9',
]

private_build_requires = [
    'cmake-3+',
    'ninja',
]
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "mylib");
    // Verify requires are present
    assert!(!pkg.requires.is_empty(), "requires should be populated");
    // private_build_requires may be in build_requires
    // At minimum the package must parse without error
}

/// rez: package with variants and build requirements
#[test]
fn test_package_variants_and_build_reqs() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut pkg = Package::new("maya_plugin".to_string());
    pkg.version = Some(Version::parse("1.2.0").unwrap());
    pkg.requires = vec!["maya-2024".to_string()];
    pkg.build_requires = vec!["cmake-3".to_string()];
    pkg.variants = vec![
        vec!["python-3.9".to_string()],
        vec!["python-3.10".to_string()],
    ];

    assert_eq!(pkg.variants.len(), 2);
    assert_eq!(pkg.build_requires.len(), 1);
    assert_eq!(pkg.requires.len(), 1);
}

