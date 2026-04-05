//! Tests for package serialization/deserialization.

#[cfg(test)]
mod serialization_tests {

    use super::super::{PackageFormat, PackageSerializer, SerializationOptions};
    use crate::Package;
    use rez_next_version::Version;

    fn make_test_package() -> Package {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.version = Some(Version::parse("1.2.3").unwrap());
        pkg.description = Some("A test package for serialization".to_string());
        pkg.authors = vec!["Alice".to_string(), "Bob".to_string()];
        pkg.requires = vec!["python>=3.8".to_string(), "maya>=2022".to_string()];
        pkg.tools = vec!["my_tool".to_string()];
        pkg
    }

    fn minimal_opts() -> SerializationOptions {
        let mut opts = SerializationOptions::new();
        opts.include_metadata = false;
        opts
    }

    #[test]
    fn test_package_format_from_extension_yaml() {
        let p = std::path::Path::new("package.yaml");
        assert_eq!(PackageFormat::from_extension(p), Some(PackageFormat::Yaml));
    }

    #[test]
    fn test_package_format_from_extension_json() {
        let p = std::path::Path::new("package.json");
        assert_eq!(PackageFormat::from_extension(p), Some(PackageFormat::Json));
    }

    #[test]
    fn test_package_format_from_extension_py() {
        let p = std::path::Path::new("package.py");
        assert_eq!(
            PackageFormat::from_extension(p),
            Some(PackageFormat::Python)
        );
    }

    #[test]
    fn test_package_format_default_filename() {
        assert_eq!(PackageFormat::Yaml.default_filename(), "package.yaml");
        assert_eq!(PackageFormat::Json.default_filename(), "package.json");
        assert_eq!(PackageFormat::Python.default_filename(), "package.py");
    }

    #[test]
    fn test_serialization_options_default() {
        let opts = SerializationOptions::default();
        assert!(opts.pretty_print);
        assert!(opts.include_metadata);
    }

    #[test]
    fn test_serialization_options_minimal() {
        let opts = SerializationOptions::minimal();
        assert!(!opts.pretty_print);
        assert!(!opts.include_metadata);
    }

    #[test]
    fn test_serialize_to_yaml_string() {
        let pkg = make_test_package();
        let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
        assert!(!yaml.is_empty());
    }

    #[test]
    fn test_serialize_to_json_string() {
        let pkg = make_test_package();
        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        assert!(!json.is_empty());
    }

    #[test]
    fn test_write_yaml_and_read_back() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = make_test_package();
        PackageSerializer::save_to_file(&pkg, &yaml_path, PackageFormat::Yaml).unwrap();

        assert!(yaml_path.exists(), "package.yaml should be written");
        let content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(!content.is_empty(), "yaml content should not be empty");
    }

    #[test]
    fn test_write_python_package_py() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let py_path = tmp.path().join("package.py");

        let pkg = make_test_package();
        PackageSerializer::save_to_file(&pkg, &py_path, PackageFormat::Python).unwrap();

        assert!(py_path.exists(), "package.py should be written");
        let content = std::fs::read_to_string(&py_path).unwrap();
        assert!(content.contains("name"), "package.py should contain name");
        assert!(
            content.contains("test_pkg"),
            "package.py should contain package name"
        );
    }

    #[test]
    fn test_load_from_yaml_string() {
        let yaml = r#"
name: my_package
version: "2.0.0"
description: My test package
authors:
  - Alice
requires:
  - python>=3.8
"#;
        let pkg = PackageSerializer::load_from_yaml(yaml).unwrap();
        assert_eq!(pkg.name, "my_package");
        assert!(pkg.version.is_some());
        assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("2.0.0"));
        assert_eq!(pkg.description, Some("My test package".to_string()));
    }

    #[test]
    fn test_load_from_python_string() {
        let python = r#"
name = "pytools"
version = "1.0.0"
description = "Python tools package"
requires = ["python>=3.7"]
"#;
        let pkg = PackageSerializer::load_from_python(python).unwrap();
        assert_eq!(pkg.name, "pytools");
        assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("1.0.0"));
        assert_eq!(pkg.description, Some("Python tools package".to_string()));
        assert_eq!(pkg.requires, vec!["python>=3.7"]);
    }

    #[test]
    fn test_yaml_roundtrip() {
        let pkg = make_test_package();
        let yaml_str = PackageSerializer::save_to_yaml(&pkg).unwrap();
        let pkg2 = PackageSerializer::load_from_yaml(&yaml_str).unwrap();
        assert_eq!(pkg.name, pkg2.name);
        assert_eq!(
            pkg.version.as_ref().map(|v| v.as_str()),
            pkg2.version.as_ref().map(|v| v.as_str())
        );
    }

    #[test]
    fn test_package_metadata_creation() {
        use super::super::PackageMetadata;
        let meta = PackageMetadata::new("yaml".to_string());
        assert_eq!(meta.format, "yaml");
        assert!(!meta.serialized_at.is_empty());
    }

    #[test]
    fn test_package_format_mime_type() {
        assert_eq!(PackageFormat::Yaml.mime_type(), "application/x-yaml");
        assert_eq!(PackageFormat::Json.mime_type(), "application/json");
        assert_eq!(PackageFormat::Python.mime_type(), "text/x-python");
    }

    // ── YAML save_to_file roundtrip tests ─────────────────────────────────────

    #[test]
    fn test_yaml_file_roundtrip_all_fields() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let mut pkg = Package::new("full_pkg".to_string());
        pkg.version = Some(Version::parse("2.5.0").unwrap());
        pkg.description = Some("Full field test package".to_string());
        pkg.authors = vec!["Dev1".to_string(), "Dev2".to_string()];
        pkg.requires = vec!["python-3.9".to_string(), "maya-2023".to_string()];
        pkg.tools = vec!["tool_a".to_string(), "tool_b".to_string()];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();
        assert_eq!(loaded.name, "full_pkg");
        assert_eq!(loaded.version.as_ref().map(|v| v.as_str()), Some("2.5.0"));
        assert_eq!(
            loaded.description,
            Some("Full field test package".to_string())
        );
        assert!(loaded.authors.contains(&"Dev1".to_string()));
        assert!(loaded.authors.contains(&"Dev2".to_string()));
    }

    #[test]
    fn test_yaml_file_roundtrip_requires() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let mut pkg = Package::new("dep_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.requires = vec![
            "python-3.9".to_string(),
            "numpy-1.20".to_string(),
            "scipy-1.7".to_string(),
        ];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();

        assert_eq!(loaded.requires.len(), 3);
        assert!(loaded.requires.contains(&"python-3.9".to_string()));
        assert!(loaded.requires.contains(&"numpy-1.20".to_string()));
        assert!(loaded.requires.contains(&"scipy-1.7".to_string()));
    }

    #[test]
    fn test_json_file_roundtrip_name_version() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let json_path = tmp.path().join("package.json");

        let mut pkg = Package::new("json_pkg".to_string());
        pkg.version = Some(Version::parse("3.1.2").unwrap());
        pkg.description = Some("JSON test".to_string());

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &json_path,
            PackageFormat::Json,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&json_path).unwrap();

        assert_eq!(loaded.name, "json_pkg");
        assert_eq!(loaded.version.as_ref().map(|v| v.as_str()), Some("3.1.2"));
    }

    #[test]
    fn test_save_yaml_string_matches_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = make_test_package();

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        let yaml_string = PackageSerializer::save_to_yaml(&pkg).unwrap();

        let from_file = PackageSerializer::load_from_file(&yaml_path).unwrap();
        let from_string = PackageSerializer::load_from_yaml(&yaml_string).unwrap();

        assert_eq!(from_file.name, from_string.name);
        assert_eq!(
            from_file.version.as_ref().map(|v| v.as_str()),
            from_string.version.as_ref().map(|v| v.as_str()),
        );
    }

    #[test]
    fn test_minimal_package_yaml_roundtrip() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("package.yaml");

        let pkg = Package::new("minimal_pkg".to_string());
        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();

        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();
        assert_eq!(loaded.name, "minimal_pkg");
        assert!(loaded.version.is_none());
        assert!(loaded.description.is_none());
        assert!(loaded.requires.is_empty());
    }

    #[test]
    fn test_format_detection_all_extensions() {
        use std::path::Path;
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.yaml")),
            Some(PackageFormat::Yaml)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.yml")),
            Some(PackageFormat::Yaml)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.json")),
            Some(PackageFormat::Json)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.py")),
            Some(PackageFormat::Python)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.bin")),
            Some(PackageFormat::Binary)
        );
        assert_eq!(
            PackageFormat::from_extension(Path::new("pkg.toml")),
            Some(PackageFormat::Toml)
        );
        assert_eq!(PackageFormat::from_extension(Path::new("pkg.xyz")), None);
    }

    // ── build_requires / private_build_requires / variants tests ─────────────

    #[test]
    fn test_build_requires_json_roundtrip() {
        let mut pkg = Package::new("build_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.build_requires = vec!["cmake-3.20".to_string(), "ninja-1.10".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        assert!(
            json.contains("cmake-3.20"),
            "JSON should have cmake in build_requires"
        );
        let loaded = PackageSerializer::load_from_json(&json).unwrap();
        assert_eq!(loaded.build_requires.len(), 2);
        assert!(loaded.build_requires.contains(&"cmake-3.20".to_string()));
        assert!(loaded.build_requires.contains(&"ninja-1.10".to_string()));
    }

    #[test]
    fn test_build_requires_yaml_roundtrip() {
        let mut pkg = Package::new("yaml_build_pkg".to_string());
        pkg.version = Some(Version::parse("2.0.0").unwrap());
        pkg.build_requires = vec!["gcc-11".to_string(), "python-3.9".to_string()];

        let yaml = PackageSerializer::save_to_yaml(&pkg).unwrap();
        assert!(yaml.contains("gcc-11"), "YAML should have build_requires");
        let loaded = PackageSerializer::load_from_yaml(&yaml).unwrap();
        assert_eq!(loaded.build_requires.len(), 2);
    }

    #[test]
    fn test_private_build_requires_json_roundtrip() {
        let mut pkg = Package::new("private_build_pkg".to_string());
        pkg.private_build_requires = vec!["internal_lib-1.0".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        let loaded = PackageSerializer::load_from_json(&json).unwrap();
        assert_eq!(loaded.private_build_requires.len(), 1);
        assert!(loaded
            .private_build_requires
            .contains(&"internal_lib-1.0".to_string()));
    }

    #[test]
    fn test_build_requires_empty_by_default() {
        let pkg = Package::new("default_pkg".to_string());
        assert!(pkg.build_requires.is_empty());
        assert!(pkg.private_build_requires.is_empty());
    }

    #[test]
    fn test_add_build_requirement() {
        let mut pkg = Package::new("add_req_pkg".to_string());
        pkg.add_build_requirement("cmake-3.25".to_string());
        pkg.add_build_requirement("make-4.3".to_string());
        assert_eq!(pkg.build_requires.len(), 2);
        assert!(pkg.build_requires.contains(&"cmake-3.25".to_string()));
    }

    #[test]
    fn test_save_to_python_includes_build_requires() {
        let mut pkg = Package::new("py_build_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.build_requires = vec!["cmake-3".to_string()];

        let py = PackageSerializer::save_to_python(&pkg).unwrap();
        assert!(
            py.contains("build_requires"),
            "Python output should have build_requires"
        );
        assert!(py.contains("cmake-3"), "Python output should list cmake-3");
    }

    #[test]
    fn test_yaml_file_roundtrip_build_requires() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let yaml_path = tmp.path().join("build_pkg.yaml");

        let mut pkg = Package::new("file_build_pkg".to_string());
        pkg.version = Some(Version::parse("1.5.0").unwrap());
        pkg.build_requires = vec!["cmake-3.20".to_string(), "boost-1.80".to_string()];

        PackageSerializer::save_to_file_with_options(
            &pkg,
            &yaml_path,
            PackageFormat::Yaml,
            Some(minimal_opts()),
        )
        .unwrap();
        let loaded = PackageSerializer::load_from_file(&yaml_path).unwrap();

        assert_eq!(
            loaded.build_requires.len(),
            2,
            "build_requires should be preserved in YAML file"
        );
        assert!(loaded.build_requires.contains(&"cmake-3.20".to_string()));
        assert!(loaded.build_requires.contains(&"boost-1.80".to_string()));
    }

    #[test]
    fn test_both_requires_and_build_requires() {
        let mut pkg = Package::new("combo_pkg".to_string());
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.build_requires = vec!["cmake-3.20".to_string()];

        let json = PackageSerializer::save_to_json(&pkg).unwrap();
        let loaded = PackageSerializer::load_from_json(&json).unwrap();

        assert_eq!(loaded.requires, vec!["python-3.9".to_string()]);
        assert_eq!(loaded.build_requires, vec!["cmake-3.20".to_string()]);
    }
}
