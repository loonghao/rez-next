use std::path::PathBuf;

use rez_next_build::{BuildConfig, BuildEnvironment, BuildManager, BuildRequest};
use rez_next_package::Package;
use rez_next_version::Version;
use rstest::rstest;

fn package() -> Package {
    let mut package = Package::new("runtime".into());
    package.version = Some(Version::parse("3.7.9").unwrap());
    package
}

#[rstest]
#[case(true, "runtime/3.7.9")]
#[case(false, "runtime")]
fn test_build_environment_custom_repository_uses_core_package_base(
    #[case] versioned: bool,
    #[case] expected: &str,
) {
    let temporary = tempfile::tempdir().unwrap();
    let build_root = temporary.path().join("build");
    let install_root = temporary.path().join("repository");
    let mut package = package();
    if !versioned {
        package.version = None;
    }
    let environment =
        BuildEnvironment::with_install_path(&package, &build_root, None, Some(&install_root))
            .unwrap();
    assert_eq!(environment.get_install_dir(), &install_root.join(expected));
}

#[rstest]
#[case(Some(1), None, "out of range")]
#[case(Some(0), Some(vec!["arch-aarch64".into()]), "do not match")]
#[tokio::test]
async fn test_build_manager_rejects_invalid_selected_layout_before_starting_process(
    #[case] index: Option<usize>,
    #[case] requirements: Option<Vec<String>>,
    #[case] expected: &str,
) {
    let mut package = package();
    package.variants = vec![vec!["platform-linux".into()]];
    let mut request = BuildRequest::new(package, None, PathBuf::from("unused-source"));
    request.variant_index = index;
    request.variant_requires = requirements;
    let mut manager = BuildManager::with_config(BuildConfig {
        max_concurrent_builds: 0,
        ..BuildConfig::default()
    });
    let error = manager.start_build(request).await.unwrap_err();
    assert!(error.to_string().contains(expected), "{error}");
}
