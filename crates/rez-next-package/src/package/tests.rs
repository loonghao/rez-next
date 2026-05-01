//! Tests for Package and PackageRequirement.

use rez_next_version::Version;

use super::requirement::PackageRequirement;
use super::types::Package;

fn ver(s: &str) -> Version {
    Version::parse(s).unwrap()
}

#[test]
fn test_pkg_req_satisfied_no_constraint() {
    let r = PackageRequirement::parse("python").unwrap();
    assert!(r.satisfied_by(&ver("3.9.0")));
}

#[test]
fn test_pkg_req_satisfied_ge() {
    let r = PackageRequirement::with_version("python".into(), ">=3.8.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(r.satisfied_by(&ver("3.8.0")));
    assert!(!r.satisfied_by(&ver("3.7.0")));
}

#[test]
fn test_pkg_req_satisfied_lt() {
    let r = PackageRequirement::with_version("python".into(), "<3.10.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(!r.satisfied_by(&ver("3.10.0")));
}

#[test]
fn test_pkg_req_satisfied_ne() {
    let r = PackageRequirement::with_version("python".into(), "!=3.8.0".into());
    assert!(r.satisfied_by(&ver("3.9.0")));
    assert!(!r.satisfied_by(&ver("3.8.0")));
}

#[test]
fn test_pkg_req_satisfied_compatible() {
    let r = PackageRequirement::with_version("mylib".into(), "~=1.4.0".into());
    assert!(r.satisfied_by(&ver("1.4.0")));
    assert!(r.satisfied_by(&ver("1.4.5")));
    assert!(!r.satisfied_by(&ver("1.5.0")));
}

#[test]
fn test_package_new_and_validate() {
    let pkg = Package::new("mylib".to_string());
    assert_eq!(pkg.name, "mylib");
    assert!(pkg.version.is_none());
    assert!(pkg.validate().is_ok());
}

#[test]
fn test_package_empty_name_invalid() {
    assert!(Package::new("".to_string()).validate().is_err());
}

#[test]
fn test_conflict_requirement_parse() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert_eq!(req.name, "python");
    assert!(req.conflict, "!python must be a conflict requirement");
    assert!(!req.weak);
    assert!(req.version_spec.is_none());
}

#[test]
fn test_conflict_requirement_with_version() {
    let req = PackageRequirement::parse("!python-3.9").unwrap();
    assert_eq!(req.name, "python");
    assert!(req.conflict);
    assert_eq!(req.version_spec.as_deref(), Some("3.9"));
}

#[test]
fn test_conflict_requirement_to_string() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert_eq!(req.to_string(), "!python");
}

#[test]
fn test_conflict_requirement_with_version_to_string() {
    let req = PackageRequirement::parse("!python-3.9").unwrap();
    assert_eq!(req.to_string(), "!python-3.9");
}

#[test]
fn test_weak_requirement_to_string() {
    let req = PackageRequirement::parse("~numpy").unwrap();
    assert_eq!(req.name, "numpy");
    assert!(req.weak);
    assert!(!req.conflict);
    assert_eq!(req.to_string(), "~numpy");
}

#[test]
fn test_normal_requirement_not_conflict_not_weak() {
    let req = PackageRequirement::parse("maya-2024").unwrap();
    assert!(!req.conflict);
    assert!(!req.weak);
    assert_eq!(req.name, "maya");
    assert_eq!(req.version_spec.as_deref(), Some("2024"));
}

#[test]
fn test_conflict_takes_priority_over_weak() {
    let req = PackageRequirement::parse("!python").unwrap();
    assert!(req.conflict);
    assert!(!req.weak);
}

#[test]
fn test_package_with_version() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert!(pkg.validate().is_ok());
    assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("1.0.0"));
}

#[test]
fn test_package_with_requires() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.requires.push("python-3.9".to_string());
    pkg.requires.push("maya".to_string());
    assert_eq!(pkg.requires.len(), 2);
    assert!(pkg.validate().is_ok());
}

#[test]
fn test_package_with_tools() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.tools.push("mytool".to_string());
    pkg.tools.push("another_tool".to_string());
    assert_eq!(pkg.tools.len(), 2);
}

#[test]
fn test_package_with_commands() {
    let mut pkg = Package::new("test_pkg".to_string());
    // commands is Option<String> - stores the commands function body
    pkg.commands = Some("def commands():\n    return {'build': 'python build.py'}".to_string());
    assert!(pkg.commands.is_some());
    assert!(pkg.commands.as_ref().unwrap().contains("build"));
}

#[test]
fn test_package_validate_invalid_name() {
    let pkg = Package::new("".to_string());
    assert!(pkg.validate().is_err());
}

#[test]
fn test_package_requirement_parse_variants() {
    // Test various requirement formats
    let r1 = PackageRequirement::parse("python").unwrap();
    assert_eq!(r1.name, "python");
    assert!(r1.version_spec.is_none());

    let r2 = PackageRequirement::parse("python-3.9").unwrap();
    assert_eq!(r2.name, "python");
    assert_eq!(r2.version_spec, Some("3.9".to_string()));

    let r3 = PackageRequirement::parse("~python").unwrap();
    assert_eq!(r3.name, "python");
    assert!(r3.weak);
    assert!(!r3.conflict);

    let r4 = PackageRequirement::parse("!python-3.9").unwrap();
    assert_eq!(r4.name, "python");
    assert!(r4.conflict);
    assert!(!r4.weak);
}

#[test]
fn test_package_clone_and_eq() {
    let pkg1 = Package::new("clone_me".to_string());
    let pkg2 = pkg1.clone();
    assert_eq!(pkg1.name, pkg2.name);
}

#[test]
fn test_package_requirement_display_format() {
    let r1 = PackageRequirement::new("python".to_string());
    assert_eq!(r1.to_string(), "python");

    let r2 = PackageRequirement::with_version("python".to_string(), "3.9".to_string());
    assert_eq!(r2.to_string(), "python-3.9");

    let r3 = PackageRequirement::parse("~python-3.9").unwrap();
    assert_eq!(r3.to_string(), "~python-3.9");
}

#[test]
fn test_package_qualified_name_with_version() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert_eq!(pkg.qualified_name(), "test_pkg-1.0.0");
}

#[test]
fn test_package_qualified_name_without_version() {
    let pkg = Package::new("test_pkg".to_string());
    assert_eq!(pkg.qualified_name(), "test_pkg");
}

#[test]
fn test_package_as_exact_requirement_with_version() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.version = Some(Version::parse("1.0.0").unwrap());
    assert_eq!(pkg.as_exact_requirement(), "test_pkg==1.0.0");
}

#[test]
fn test_package_as_exact_requirement_without_version() {
    let pkg = Package::new("test_pkg".to_string());
    assert_eq!(pkg.as_exact_requirement(), "test_pkg");
}

#[test]
fn test_package_num_variants_empty() {
    let pkg = Package::new("test_pkg".to_string());
    assert_eq!(pkg.num_variants(), 0);
}

#[test]
fn test_package_num_variants_with_variants() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.variants.push(vec!["variant1".to_string()]);
    pkg.variants.push(vec!["variant2".to_string()]);
    assert_eq!(pkg.num_variants(), 2);
}

#[test]
fn test_package_is_valid() {
    let pkg = Package::new("test_pkg".to_string());
    assert!(pkg.is_valid());
}

#[test]
fn test_package_is_package_always_true() {
    let pkg = Package::new("test_pkg".to_string());
    assert!(pkg.is_package());
}

#[test]
fn test_package_is_variant_always_false() {
    let pkg = Package::new("test_pkg".to_string());
    assert!(!pkg.is_variant());
}

#[test]
fn test_package_add_requirement() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.add_requirement("python-3.9".to_string());
    assert_eq!(pkg.requires.len(), 1);
    assert_eq!(pkg.requires[0], "python-3.9");
}

#[test]
fn test_package_add_build_requirement() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.add_build_requirement("numpy".to_string());
    assert_eq!(pkg.build_requires.len(), 1);
    assert_eq!(pkg.build_requires[0], "numpy");
}

#[test]
fn test_package_add_variant() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.add_variant(vec!["maya".to_string(), "houdini".to_string()]);
    assert_eq!(pkg.variants.len(), 1);
    assert_eq!(pkg.variants[0].len(), 2);
}

#[test]
fn test_package_validate_invalid_characters() {
    let pkg = Package::new("my@pkg".to_string());
    assert!(pkg.validate().is_err());
}

#[test]
fn test_package_validate_empty_requirement() {
    let mut pkg = Package::new("test_pkg".to_string());
    pkg.requires.push("".to_string());
    assert!(pkg.validate().is_err());
}
