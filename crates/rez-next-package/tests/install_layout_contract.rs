use std::path::Path;

use rez_next_package::{Package, PackageInstallLayout};
use rez_next_version::Version;
use rstest::rstest;

fn package() -> Package {
    let mut package = Package::new("My_Runtime".to_string());
    package.version = Some(Version::parse("01.2.RC1").unwrap());
    package
}

#[rstest]
#[case(true, "My_Runtime/01.2.RC1")]
#[case(false, "My_Runtime")]
fn test_for_variant_nonvariant_preserves_package_identity(
    #[case] versioned: bool,
    #[case] expected: &str,
) {
    let mut package = package();
    if !versioned {
        package.version = None;
    }
    let layout = PackageInstallLayout::for_variant(&package, None).unwrap();
    assert_eq!(layout.package_relative_path(), Path::new(expected));
    assert_eq!(layout.variant_relative_path(), Path::new(expected));
    assert!(layout.variant_subpath().as_os_str().is_empty());
}

#[rstest]
#[case(None)]
#[case(Some(false))]
fn test_for_variant_nonhashed_uses_declared_requirement_order(#[case] hashed: Option<bool>) {
    let mut package = package();
    package.hashed_variants = hashed;
    package.variants = vec![
        vec!["platform-windows".into(), "arch-x86_64".into()],
        vec!["arch-aarch64".into(), "platform-linux".into()],
    ];
    let layout = PackageInstallLayout::for_variant(&package, Some(1)).unwrap();
    assert_eq!(
        layout.variant_subpath(),
        Path::new("arch-aarch64/platform-linux")
    );
    assert_eq!(
        layout.variant_relative_path(),
        Path::new("My_Runtime/01.2.RC1/arch-aarch64/platform-linux")
    );
}

#[rstest]
#[case(vec!["platform-linux", "arch-x86_64"], "4cd91c759bd6e4d7ef09736380d9e5b18c81a0f2")]
#[case(vec!["arch-x86_64", "platform-linux"], "7d0fc396b1a7fb091d37d8536c327d1aca7c6088")]
#[case(vec!["python-3.9"], "32383fc90933199c950ca6823b6d1e0f634651b6")]
#[case(vec!["python-3+<4", "!pypy"], "bd54471ea1ee3f9dfb147d7d92eaec66491fe8f9")]
#[case(vec![], "97d170e1550eee4afc0af065b78cda302a97674c")]
fn test_for_variant_hashed_matches_rez_python_list_sha1(
    #[case] requirements: Vec<&str>,
    #[case] expected: &str,
) {
    let mut package = package();
    package.hashed_variants = Some(true);
    package.variants = vec![requirements.into_iter().map(str::to_string).collect()];
    let layout = PackageInstallLayout::for_variant(&package, Some(0)).unwrap();
    assert_eq!(layout.variant_subpath(), Path::new(expected));
}

#[rstest]
#[case("python==3.7.9")]
#[case("python-3.7+")]
#[case("!python")]
#[case("~python-3.7")]
fn test_for_variant_nonhashed_keeps_canonical_rez_text(#[case] requirement: &str) {
    let mut package = package();
    package.variants = vec![vec![requirement.to_string()]];
    assert_eq!(
        PackageInstallLayout::for_variant(&package, Some(0))
            .unwrap()
            .variant_subpath(),
        Path::new(requirement)
    );
}

#[rstest]
#[case("../escape")]
#[case("/absolute")]
#[case("C:\\escape")]
#[case("CON")]
#[case("LPT¹")]
fn test_for_variant_rejects_unsafe_package_name(#[case] name: &str) {
    let mut package = package();
    package.name = name.to_string();
    assert!(PackageInstallLayout::for_variant(&package, None).is_err());
}

#[rstest]
#[case("../escape")]
#[case("1/escape")]
#[case("1\\escape")]
#[case("1:stream")]
#[case("CON")]
#[case("1.")]
fn test_for_variant_rejects_unsafe_version_even_if_public_version_was_mutated(#[case] value: &str) {
    let mut package = package();
    package.version.as_mut().unwrap().string_repr = value.to_string();
    assert!(PackageInstallLayout::for_variant(&package, None).is_err());
}

#[rstest]
#[case("../escape", false)]
#[case("C:\\escape", true)]
#[case("python>=3", true)]
#[case("python<4", false)]
#[case("CON", false)]
#[case("python\n", true)]
#[case("namespace::python", true)]
fn test_for_variant_rejects_unsafe_or_unsupported_requirement(
    #[case] requirement: &str,
    #[case] hashed: bool,
) {
    let mut package = package();
    package.hashed_variants = Some(hashed);
    package.variants = vec![vec![requirement.to_string()]];
    assert!(PackageInstallLayout::for_variant(&package, Some(0)).is_err());
}

#[rstest]
fn test_for_variant_rejects_missing_out_of_range_and_spurious_selections() {
    let mut package = package();
    assert!(PackageInstallLayout::for_variant(&package, Some(0)).is_err());
    package.variants = vec![vec!["platform-linux".into()]];
    assert!(PackageInstallLayout::for_variant(&package, None).is_err());
    assert!(PackageInstallLayout::for_variant(&package, Some(1)).is_err());
    package.variants = vec![vec![]];
    assert!(PackageInstallLayout::for_variant(&package, Some(0)).is_err());
}
