//! Real Repository Integration Tests
//!
//! These tests create actual package.py files on disk and exercise the full
//! rez-next pipeline: repository scan → solver resolve → context → env vars.
//! Derived from rez's own integration test suite concepts.
//!
//! See also:
//!   real_repo_resolve_tests.rs  — solver / resolution tests
//!   real_repo_context_tests.rs  — context, env-var, multi-repo, E2E tests
use rez_next_package::Requirement;
use rez_next_repository::simple_repository::SimpleRepository;
use rez_next_repository::PackageRepository;
use rez_next_version::Version;
use tempfile::TempDir;

#[path = "real_repo_test_helpers.rs"]
mod real_repo_test_helpers;

use real_repo_test_helpers::create_package;

// ─── Basic repository scan tests ─────────────────────────────────────────────


#[test]
fn test_repo_scan_finds_packages() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(
        &repo_dir,
        "python",
        "3.9.0",
        &[],
        &["python", "python3"],
        None,
    );
    create_package(
        &repo_dir,
        "python",
        "3.11.0",
        &[],
        &["python", "python3"],
        None,
    );
    create_package(
        &repo_dir,
        "maya",
        "2024.0",
        &["python-3+<4"],
        &["maya"],
        None,
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "test_repo".to_string());

    let pythons = rt.block_on(repo.find_packages("python")).unwrap();
    assert_eq!(pythons.len(), 2, "Should find 2 python versions");

    let mayas = rt.block_on(repo.find_packages("maya")).unwrap();
    assert_eq!(mayas.len(), 1, "Should find 1 maya version");
}

#[test]
fn test_repo_scan_reads_package_metadata() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(
        &repo_dir,
        "houdini",
        "20.0.547",
        &["python-3.10+<3.12"],
        &["houdini", "hython"],
        None,
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "test_repo".to_string());
    let pkgs = rt.block_on(repo.find_packages("houdini")).unwrap();

    assert!(!pkgs.is_empty(), "Should find houdini");
    let pkg = &pkgs[0];
    assert_eq!(pkg.name, "houdini");
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "20.0.547");
}

#[test]
fn test_repo_scan_multiple_packages() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    let packages = [
        ("python", "3.9.0"),
        ("python", "3.10.0"),
        ("python", "3.11.0"),
        ("numpy", "1.24.0"),
        ("numpy", "1.25.0"),
        ("scipy", "1.11.0"),
    ];

    for (name, ver) in &packages {
        create_package(&repo_dir, name, ver, &[], &[], None);
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "test_repo".to_string());

    let pythons = rt.block_on(repo.find_packages("python")).unwrap();
    assert_eq!(pythons.len(), 3);

    let numpys = rt.block_on(repo.find_packages("numpy")).unwrap();
    assert_eq!(numpys.len(), 2);

    let scipys = rt.block_on(repo.find_packages("scipy")).unwrap();
    assert_eq!(scipys.len(), 1);
}

// ─── Package.py parsing compatibility tests ──────────────────────────────────

#[test]
fn test_package_py_name_version_parsed() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "testpkg", "2.5.3", &[], &[], None);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(repo.find_packages("testpkg")).unwrap();

    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].name, "testpkg");
    assert_eq!(pkgs[0].version.as_ref().unwrap().as_str(), "2.5.3");
}

#[test]
fn test_package_py_requires_parsed() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(
        &repo_dir,
        "myapp",
        "1.0.0",
        &["python-3.9+<4", "numpy-1.20+", "requests"],
        &[],
        None,
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(repo.find_packages("myapp")).unwrap();

    assert_eq!(pkgs.len(), 1);
    let pkg = &pkgs[0];
    assert!(!pkg.requires.is_empty(), "Requires should be parsed");
    assert!(
        pkg.requires.iter().any(|r| r.starts_with("python")),
        "Should have python requirement"
    );
}

#[test]
fn test_package_py_tools_parsed() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(
        &repo_dir,
        "houdini",
        "20.0",
        &[],
        &["houdini", "hython", "hconfig"],
        None,
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(repo.find_packages("houdini")).unwrap();

    assert_eq!(pkgs.len(), 1);
    let pkg = &pkgs[0];
    assert!(!pkg.tools.is_empty(), "Tools should be parsed");
    assert!(pkg.tools.contains(&"houdini".to_string()));
    assert!(pkg.tools.contains(&"hython".to_string()));
}

// ─── Requirement parsing ──────────────────────────────────────────────────────

#[test]
fn test_requirement_parsing_rez_formats() {
    let req = "python-3.11".parse::<Requirement>().unwrap();
    assert_eq!(
        req.name, "python",
        "python-3.11 name should be 'python', got '{}'",
        req.name
    );
    let v311 = Version::parse("3.11.0").unwrap();
    assert!(
        req.is_satisfied_by(&v311),
        "python-3.11 should be satisfied by 3.11.0"
    );

    let req2 = "python-3+<4".parse::<Requirement>().unwrap();
    assert_eq!(req2.name, "python");
    assert!(req2.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req2.is_satisfied_by(&Version::parse("4.0.0").unwrap()));

    let req3 = "pip-20+".parse::<Requirement>().unwrap();
    assert_eq!(req3.name, "pip");
    assert!(req3.is_satisfied_by(&Version::parse("23.0.0").unwrap()));

    let req4 = "numpy-1.20+".parse::<Requirement>().unwrap();
    assert_eq!(req4.name, "numpy");
    assert!(req4.is_satisfied_by(&Version::parse("1.25.0").unwrap()));

    let req5 = "python-3.9+<3.11".parse::<Requirement>().unwrap();
    assert_eq!(req5.name, "python");
    assert!(req5.is_satisfied_by(&Version::parse("3.10.0").unwrap()));
    assert!(!req5.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req5.is_satisfied_by(&Version::parse("3.8.0").unwrap()));
}

// ─── Repository manager — missing packages ───────────────────────────────────

#[test]
fn test_repo_manager_missing_packages_returns_empty() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.11.0", &[], &[], None);

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir, "r".to_string());
    let pkgs = rt
        .block_on(repo.find_packages("nonexistent_package_xyz"))
        .unwrap();

    assert!(pkgs.is_empty(), "Should return empty for unknown package");
}


