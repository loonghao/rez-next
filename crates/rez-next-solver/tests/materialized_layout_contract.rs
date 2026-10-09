use std::path::Path;
use std::sync::Arc;

use rez_next_package::Package;
use rez_next_solver::dependency_resolver::ResolvedPackageInfo;
use rez_next_version::Version;
use rstest::rstest;

fn resolved(hashed: bool, index: Option<usize>) -> ResolvedPackageInfo {
    let mut package = Package::new("python".into());
    package.version = Some(Version::parse("3.9").unwrap());
    package.filepath = Some(
        Path::new("repository/python/3.9/package.yaml")
            .to_string_lossy()
            .into_owned(),
    );
    package.variants = vec![vec!["python-3.9".into()]];
    package.hashed_variants = Some(hashed);
    ResolvedPackageInfo {
        package: Arc::new(package),
        variant_index: index,
        requested: true,
        required_by: vec![],
        satisfying_requirement: None,
    }
}

#[rstest]
#[case(false, "repository/python/3.9/python-3.9/package.yaml")]
#[case(
    true,
    "repository/python/3.9/32383fc90933199c950ca6823b6d1e0f634651b6/package.yaml"
)]
fn test_try_materialized_package_uses_core_layout_and_preserves_definition_format(
    #[case] hashed: bool,
    #[case] expected: &str,
) {
    let resolved = resolved(hashed, Some(0));
    let package = resolved.try_materialized_package().unwrap();
    assert_eq!(
        Path::new(package.filepath.as_ref().unwrap()),
        Path::new(expected)
    );
    assert_eq!(package.variants, resolved.package.variants);
    assert_eq!(
        resolved.package.filepath.as_deref(),
        Some(
            Path::new("repository/python/3.9/package.yaml")
                .to_str()
                .unwrap()
        )
    );
}

#[rstest]
#[case(None)]
#[case(Some(1))]
fn test_try_materialized_package_rejects_invalid_variant_selection(#[case] index: Option<usize>) {
    assert!(resolved(false, index).try_materialized_package().is_err());
}

#[rstest]
fn test_try_materialized_package_rejects_variant_without_definition_base() {
    let mut resolved = resolved(false, Some(0));
    Arc::make_mut(&mut resolved.package).filepath = None;
    assert!(resolved.try_materialized_package().is_err());
}

#[rstest]
fn test_try_materialized_package_preserves_rootless_nonvariant_virtual_binding() {
    let mut resolved = resolved(false, None);
    let package = Arc::make_mut(&mut resolved.package);
    package.filepath = None;
    package.variants.clear();
    assert!(
        resolved
            .try_materialized_package()
            .unwrap()
            .root()
            .is_none()
    );
}
