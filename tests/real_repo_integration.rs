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
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_repository::PackageRepository;
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal package.py in a temp repo at `<repo>/<name>/<version>/package.py`
fn create_package(
    repo_dir: &std::path::Path,
    name: &str,
    version: &str,
    requires: &[&str],
    tools: &[&str],
    commands: Option<&str>,
) {
    let pkg_dir = repo_dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();

    let requires_str = requires
        .iter()
        .map(|r| format!("    \"{}\",", r))
        .collect::<Vec<_>>()
        .join("\n");

    let tools_str = tools
        .iter()
        .map(|t| format!("    \"{}\",", t))
        .collect::<Vec<_>>()
        .join("\n");

    let cmd_block = if let Some(cmd) = commands {
        format!(
            r#"
def commands():
    {}
"#,
            cmd
        )
    } else {
        format!(
            r#"
def commands():
    env.{upper}_ROOT.set("{{{{root}}}}")
    env.PATH.prepend("{{{{root}}}}/bin")
"#,
            upper = name.to_uppercase()
        )
    };

    let requires_block = if requires.is_empty() {
        String::new()
    } else {
        format!("requires = [\n{}\n]\n", requires_str)
    };

    let tools_block = if tools.is_empty() {
        String::new()
    } else {
        format!("tools = [\n{}\n]\n", tools_str)
    };

    let content = format!(
        r#"name = "{name}"
version = "{version}"
description = "Test package {name}-{version}"
{requires_block}{tools_block}{cmd_block}"#,
        name = name,
        version = version,
        requires_block = requires_block,
        tools_block = tools_block,
        cmd_block = cmd_block,
    );

    fs::write(pkg_dir.join("package.py"), content).unwrap();
}

/// Build a RepositoryManager from a single temp dir
fn make_repo(dir: &std::path::Path) -> Arc<RepositoryManager> {
    let mut mgr = RepositoryManager::new();
    if dir.exists() {
        mgr.add_repository(Box::new(SimpleRepository::new(
            dir,
            "test_repo".to_string(),
        )));
    }
    Arc::new(mgr)
}

/// Run the solver synchronously and return resolved package names+versions
fn resolve(repo: Arc<RepositoryManager>, reqs: Vec<&str>) -> Vec<(String, String)> {
    let requirements: Vec<Requirement> = reqs
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let result = rt.block_on(resolver.resolve(requirements)).unwrap();

    result
        .resolved_packages
        .iter()
        .map(|info| {
            let name = info.package.name.clone();
            let ver = info
                .package
                .version
                .as_ref()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default();
            (name, ver)
        })
        .collect()
}

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

// ─── Smoke-test: make_repo + resolve helpers work end-to-end ─────────────────

#[test]
fn test_smoke_make_repo_resolve() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();
    create_package(&repo_dir, "python", "3.11.0", &[], &["python"], None);

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["python"]);
    assert!(resolved.iter().any(|(n, _)| n == "python"));
}
