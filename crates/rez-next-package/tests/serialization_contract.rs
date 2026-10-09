use std::fs;

use rstest::rstest;

use rez_next_package::{PackageFormat, PackageInstallLayout, PackageSerializer};

const YAML: &str = r#"name: runtime
version: '01.2.RC1'
requires: ['base-1']
build_requires: ['cmake-3']
private_build_requires: ['compiler-1']
variants: [['platform-windows', 'arch-x86_64']]
hashed_variants: true
tools: ['runtime']
pre_commands: "env.setenv('PRE_PHASE', 'yes')"
commands: "env.prepend_path('PATH', '{root}/payload/bin')"
post_commands: "env.setenv('POST_PHASE', 'yes')"
build_command: 'custom-build --install'
relocatable: true
cachable: false
"#;

const JSON: &str = r#"{
  "name": "runtime",
  "version": "01.2.RC1",
  "requires": ["base-1"],
  "build_requires": ["cmake-3"],
  "private_build_requires": ["compiler-1"],
  "variants": [["platform-windows", "arch-x86_64"]],
  "hashed_variants": true,
  "tools": ["runtime"],
  "pre_commands": "env.setenv('PRE_PHASE', 'yes')",
  "commands": "env.prepend_path('PATH', '{root}/payload/bin')",
  "post_commands": "env.setenv('POST_PHASE', 'yes')",
  "build_command": "custom-build --install",
  "relocatable": true,
  "cachable": false
}"#;

#[rstest]
#[case("yaml", YAML)]
#[case("json", JSON)]
fn test_load_from_file_preserves_layout_and_activation_fields(
    #[case] extension: &str,
    #[case] content: &str,
) {
    let temporary = tempfile::tempdir().unwrap();
    let definition = temporary.path().join(format!("package.{extension}"));
    fs::write(&definition, content).unwrap();

    let package = PackageSerializer::load_from_file(&definition).unwrap();

    assert_eq!(package.name, "runtime");
    assert_eq!(package.version.as_ref().unwrap().as_str(), "01.2.RC1");
    assert_eq!(package.requires, ["base-1"]);
    assert_eq!(package.build_requires, ["cmake-3"]);
    assert_eq!(package.private_build_requires, ["compiler-1"]);
    assert_eq!(package.tools, ["runtime"]);
    assert_eq!(package.hashed_variants, Some(true));
    assert_eq!(package.relocatable, Some(true));
    assert_eq!(package.cachable, Some(false));
    assert_eq!(
        package.pre_commands.as_deref(),
        Some("env.setenv('PRE_PHASE', 'yes')")
    );
    assert_eq!(
        package.commands.as_deref(),
        Some("env.prepend_path('PATH', '{root}/payload/bin')")
    );
    assert_eq!(
        package.post_commands.as_deref(),
        Some("env.setenv('POST_PHASE', 'yes')")
    );
    assert_eq!(
        package.build_command.as_deref(),
        Some("custom-build --install")
    );
    let layout = PackageInstallLayout::for_variant(&package, Some(0)).unwrap();
    assert_eq!(
        layout
            .variant_relative_path()
            .to_string_lossy()
            .replace('\\', "/"),
        "runtime/01.2.RC1/c05c599993c2b1ed88babc35858f1d00995605f0"
    );
    assert_eq!(fs::read(definition).unwrap(), content.as_bytes());
}

#[rstest]
#[case(PackageFormat::Yaml, "name: runtime\nhashed_variants: 'true'\n")]
#[case(PackageFormat::Json, r#"{"name":"runtime","hashed_variants":"true"}"#)]
#[case(PackageFormat::Yaml, "name: runtime\nrequires: ['base-1', 42]\n")]
#[case(PackageFormat::Json, r#"{"name":"runtime","requires":["base-1",42]}"#)]
#[case(
    PackageFormat::Yaml,
    "name: runtime\nvariants: [['platform-windows', 42]]\n"
)]
#[case(
    PackageFormat::Json,
    r#"{"name":"runtime","variants":[["platform-windows",42]]}"#
)]
#[case(PackageFormat::Yaml, "name: runtime\ncommands: []\n")]
#[case(PackageFormat::Json, r#"{"name":"runtime","commands":[]}"#)]
fn test_load_from_string_rejects_malformed_semantic_fields(
    #[case] format: PackageFormat,
    #[case] content: &str,
) {
    assert!(PackageSerializer::load_from_string(content, format).is_err());
}

#[rstest]
#[case(
    PackageFormat::Yaml,
    "name: runtime\nhashed_variants: true\nhashed_variants: false\n"
)]
#[case(
    PackageFormat::Json,
    r#"{"name":"runtime","hashed_variants":true,"hashed_variants":false}"#
)]
fn test_load_from_string_rejects_duplicate_layout_fields(
    #[case] format: PackageFormat,
    #[case] content: &str,
) {
    assert!(PackageSerializer::load_from_string(content, format).is_err());
}

#[rstest]
#[case(PackageFormat::Yaml, "name: '../escape'\n")]
#[case(PackageFormat::Json, r#"{"name":"../escape"}"#)]
fn test_load_from_string_validates_the_complete_package(
    #[case] format: PackageFormat,
    #[case] content: &str,
) {
    assert!(PackageSerializer::load_from_string(content, format).is_err());
}
