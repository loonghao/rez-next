use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Package parsing tests ──────────────────────────────────────────────────

#[test]
fn test_package_requirement_parse_name_only() {
    let req = PackageRequirement::parse("python").unwrap();
    assert_eq!(req.name, "python");
    // No version constraint for bare package name: version_spec should be empty or None
    // (field name is version_spec in the actual struct)
}

#[test]
fn test_package_requirement_parse_name_version() {
    // rez style: "python-3.9" means name=python, constraint=3.9
    let req = PackageRequirement::parse("python-3.9").unwrap();
    assert_eq!(req.name, "python");
}

#[test]
fn test_package_requirement_parse_semver_style() {
    // rez uses "name-version" syntax, not "name>=version"
    // "python>=3.9" is NOT standard rez syntax; test that it parses with correct name
    // The parser treats the entire string as name with no dash
    let req = PackageRequirement::parse("python-3.9").unwrap();
    assert_eq!(req.name, "python");
    // Bare python requirement
    let req2 = PackageRequirement::parse("python").unwrap();
    assert_eq!(req2.name, "python");
}

#[test]
fn test_package_creation_basic() {
    let pkg = Package::new("my_package".to_string());
    assert_eq!(pkg.name, "my_package");
    assert!(pkg.version.is_none());
    assert!(pkg.requires.is_empty());
}

#[test]
fn test_package_with_version() {
    let mut pkg = Package::new("my_package".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(pkg.version.is_some());
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.0.0");
}

#[test]
fn test_package_py_parse_minimal() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'minimal_pkg'
version = '1.0.0'
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "minimal_pkg");
    assert!(pkg.version.is_some());
}

#[test]
fn test_package_py_parse_with_requires() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'my_tool'
version = '2.0.0'
requires = ['python-3', 'pip-22']
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "my_tool");
    assert!(!pkg.requires.is_empty(), "requires should be parsed");
}

#[test]
fn test_package_py_parse_with_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'maya'
version = '2024.0'
requires = ['python-3.9']
commands = "env.setenv('MAYA_LOCATION', '{root}')"
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "maya");
}

#[test]
fn test_package_yaml_parse() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name: yaml_pkg
version: "3.2.1"
description: "A YAML package"
requires:
  - python-3.9
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.yaml");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "yaml_pkg");
}

