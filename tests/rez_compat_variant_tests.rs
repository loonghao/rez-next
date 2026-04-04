//! Rez Compat — ContextSummary, Package Variants Tests (Cycle 33)
//!
//! Covers:
//! - ContextSummary package_count and package_versions
//! - Package variant field manipulation and parsing

use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};

// ─── Cycle 33: ResolvedContextSummary + Package variant tests ─────────────────

/// rez context: get_summary() returns correct package_count.
#[test]
fn test_resolved_context_summary_num_packages() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;
    use rez_next_version::Version;

    let reqs = vec![
        PackageRequirement::new("pkgA".to_string()),
        PackageRequirement::new("pkgB".to_string()),
        PackageRequirement::new("pkgC".to_string()),
    ];
    let mut ctx = ResolvedContext::from_requirements(reqs);
    ctx.status = ContextStatus::Resolved;

    // Manually add resolved packages
    let mut p1 = Package::new("pkgA".to_string());
    p1.version = Some(Version::parse("1.0.0").unwrap());
    let mut p2 = Package::new("pkgB".to_string());
    p2.version = Some(Version::parse("2.0.0").unwrap());
    let mut p3 = Package::new("pkgC".to_string());
    p3.version = Some(Version::parse("3.0.0").unwrap());
    ctx.resolved_packages.push(p1);
    ctx.resolved_packages.push(p2);
    ctx.resolved_packages.push(p3);

    let summary = ctx.get_summary();
    assert_eq!(summary.package_count, 3, "summary.package_count should be 3");
    assert!(summary.package_versions.contains_key("pkgA"), "summary should contain pkgA");
    assert!(summary.package_versions.contains_key("pkgB"), "summary should contain pkgB");
    assert!(summary.package_versions.contains_key("pkgC"), "summary should contain pkgC");
}

/// rez context: get_summary() for empty context returns package_count=0.
#[test]
fn test_resolved_context_summary_empty() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    let summary = ctx.get_summary();
    assert_eq!(summary.package_count, 0, "empty context summary should have 0 packages");
    assert!(summary.package_versions.is_empty(), "empty context should have no package versions");
}

/// rez context: get_summary().package_versions maps name to version string.
#[test]
fn test_resolved_context_summary_version_mapping() {
    use rez_next_context::{ContextStatus, ResolvedContext};
    use rez_next_package::PackageRequirement;
    use rez_next_version::Version;

    let mut ctx = ResolvedContext::from_requirements(vec![
        PackageRequirement::new("numpy".to_string()),
    ]);
    ctx.status = ContextStatus::Resolved;

    let mut pkg = Package::new("numpy".to_string());
    pkg.version = Some(Version::parse("1.25.0").unwrap());
    ctx.resolved_packages.push(pkg);

    let summary = ctx.get_summary();
    assert_eq!(
        summary.package_versions.get("numpy").map(|s| s.as_str()),
        Some("1.25.0"),
        "summary.package_versions['numpy'] should be '1.25.0'"
    );
}

/// Package: variants field stores correct variant requirements lists.
#[test]
fn test_package_variants_field_populated() {
    let mut pkg = Package::new("mypackage".to_string());
    pkg.add_variant(vec!["python-3.9".to_string(), "numpy-1.20+".to_string()]);
    pkg.add_variant(vec!["python-3.10".to_string(), "numpy-1.24+".to_string()]);

    assert_eq!(pkg.num_variants(), 2, "package should have 2 variants");
    assert_eq!(pkg.variants[0], vec!["python-3.9", "numpy-1.20+"]);
    assert_eq!(pkg.variants[1], vec!["python-3.10", "numpy-1.24+"]);
}

/// Package: empty package has 0 variants.
#[test]
fn test_package_no_variants_by_default() {
    let pkg = Package::new("simple".to_string());
    assert_eq!(pkg.num_variants(), 0, "new package should have 0 variants by default");
    assert!(!pkg.is_variant(), "Package is_variant() should always be false");
}

/// Package: variants round-trip through package.py format.
#[test]
fn test_package_variants_loaded_from_package_py() {
    use std::fs;
    use tempfile::TempDir;
    use rez_next_package::PackageSerializer;

    let tmp = TempDir::new().unwrap();
    let pkg_dir = tmp.path().join("variantpkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).unwrap();
    let content = r#"name = 'variantpkg'
version = '1.0.0'
variants = [
    ['python-3.9', 'numpy-1.20+'],
    ['python-3.10', 'numpy-1.24+'],
]
"#;
    fs::write(pkg_dir.join("package.py"), content).unwrap();

    let pkg = PackageSerializer::load_from_file(&pkg_dir.join("package.py")).unwrap();
    assert_eq!(pkg.name, "variantpkg");
    assert_eq!(pkg.num_variants(), 2, "should parse 2 variants from package.py");
}

