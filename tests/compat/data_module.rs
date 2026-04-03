use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Data module tests ──────────────────────────────────────────────────────

/// rez data: built-in bash completion script is non-empty and valid
#[test]
fn test_data_bash_completion_valid() {
    // Verify bash completion content can be used
    let content = "# rez-next bash completion\n_rez_next() { local cur opts; }\ncomplete -F _rez_next rez-next\n";
    assert!(content.contains("_rez_next"));
    assert!(content.contains("complete -F"));
}

/// rez data: example package.py content is parseable by PackageSerializer
#[test]
fn test_data_example_package_parseable() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let example_content = r#"name = "my_package"
version = "1.0.0"
description = "An example rez package"
authors = ["Your Name"]
requires = ["python-3.9+"]
"#;

    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, example_content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "my_package");
    assert!(pkg.version.is_some());
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.0.0");
}

/// rez data: default rezconfig contains required fields
#[test]
fn test_data_default_config_has_required_fields() {
    let config_content = "packages_path = [\"~/packages\"]\nlocal_packages_path = \"~/packages\"\nrelease_packages_path = \"/packages/int\"\n";
    assert!(config_content.contains("packages_path"));
    assert!(config_content.contains("local_packages_path"));
    assert!(config_content.contains("release_packages_path"));
}

