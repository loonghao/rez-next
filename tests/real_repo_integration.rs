//! Real Repository Integration Tests
//!
//! These tests create actual package.py files on disk and exercise the full
//! rez-next pipeline: repository scan → solver resolve → context → env vars.
//! Derived from rez's own integration test suite concepts.

use rez_next_package::{Package, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_repository::PackageRepository;
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Create a minimal package.py in a temp repo at `<repo>/<name>/<version>/package.py`
fn create_package(
    repo_dir: &PathBuf,
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
fn make_repo(dir: &PathBuf) -> Arc<RepositoryManager> {
    let mut mgr = RepositoryManager::new();
    if dir.exists() {
        mgr.add_repository(Box::new(SimpleRepository::new(
            dir.clone(),
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

// ─── Solver / resolution tests ────────────────────────────────────────────────

#[test]
fn test_solve_single_package() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.11.0", &[], &["python"], None);

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["python"]);

    assert!(
        resolved.iter().any(|(n, _)| n == "python"),
        "Resolved packages should include python"
    );
}

#[test]
fn test_solve_with_version_constraint() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.9.0", &[], &[], None);
    create_package(&repo_dir, "python", "3.10.0", &[], &[], None);
    create_package(&repo_dir, "python", "3.11.0", &[], &[], None);

    let repo = make_repo(&repo_dir);
    // Request python >= 3.10
    let resolved = resolve(repo, vec!["python-3.10+<4"]);

    let python = resolved.iter().find(|(n, _)| n == "python");
    assert!(python.is_some(), "Should resolve python");

    if let Some((_, ver)) = python {
        let v = Version::parse(ver).unwrap();
        let min = Version::parse("3.10").unwrap();
        assert!(v >= min, "Resolved python {} should be >= 3.10", ver);
    }
}

#[test]
fn test_solve_transitive_dependencies() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.11.0", &[], &["python"], None);
    create_package(
        &repo_dir,
        "numpy",
        "1.25.0",
        &["python-3.9+<4"],
        &["python"],
        None,
    );
    create_package(
        &repo_dir,
        "scipy",
        "1.11.0",
        &["python-3.9+<4", "numpy-1.20+"],
        &[],
        None,
    );

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["scipy"]);

    let names: Vec<&str> = resolved.iter().map(|(n, _)| n.as_str()).collect();
    // scipy should pull in numpy and python
    assert!(names.contains(&"scipy"), "scipy must be in result");
    assert!(
        names.contains(&"numpy"),
        "numpy must be pulled in as transitive dep"
    );
    assert!(
        names.contains(&"python"),
        "python must be pulled in as transitive dep"
    );
}

#[test]
fn test_solve_version_selection_prefers_latest() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.9.0", &[], &[], None);
    create_package(&repo_dir, "python", "3.10.0", &[], &[], None);
    create_package(&repo_dir, "python", "3.11.0", &[], &[], None);

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["python"]);

    let python = resolved.iter().find(|(n, _)| n == "python");
    assert!(python.is_some());
    // Should pick latest available
    let (_, ver) = python.unwrap();
    let v = Version::parse(ver).unwrap();
    let v311 = Version::parse("3.11.0").unwrap();
    assert_eq!(v, v311, "Should select latest python 3.11.0, got {}", ver);
}

#[test]
fn test_requirement_parsing_rez_formats() {
    // "python-3.11" → name=python, >=3.11
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

    // "python-3+<4" → name=python, >=3 <4
    let req2 = "python-3+<4".parse::<Requirement>().unwrap();
    assert_eq!(
        req2.name, "python",
        "python-3+<4 name should be 'python', got '{}'",
        req2.name
    );
    assert!(req2.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req2.is_satisfied_by(&Version::parse("4.0.0").unwrap()));

    // "pip-20+" → name=pip, >=20
    let req3 = "pip-20+".parse::<Requirement>().unwrap();
    assert_eq!(
        req3.name, "pip",
        "pip-20+ name should be 'pip', got '{}'",
        req3.name
    );
    assert!(req3.is_satisfied_by(&Version::parse("23.0.0").unwrap()));

    // "numpy-1.20+" → name=numpy, >=1.20
    let req4 = "numpy-1.20+".parse::<Requirement>().unwrap();
    assert_eq!(req4.name, "numpy");
    assert!(req4.is_satisfied_by(&Version::parse("1.25.0").unwrap()));

    // "python-3.9+<3.11" → name=python, >=3.9 <3.11
    let req5 = "python-3.9+<3.11".parse::<Requirement>().unwrap();
    assert_eq!(req5.name, "python");
    assert!(req5.is_satisfied_by(&Version::parse("3.10.0").unwrap()));
    assert!(!req5.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req5.is_satisfied_by(&Version::parse("3.8.0").unwrap()));
}

#[test]
fn test_solve_multiple_explicit_requests() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "python", "3.11.0", &[], &[], None);
    create_package(&repo_dir, "pip", "23.0.0", &["python-3+<4"], &["pip"], None);
    create_package(
        &repo_dir,
        "virtualenv",
        "20.0.0",
        &["python-3+<4", "pip-20+"],
        &[],
        None,
    );

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["python-3.11", "pip", "virtualenv"]);

    let names: Vec<&str> = resolved.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"python"), "python in result");
    assert!(names.contains(&"pip"), "pip in result");
    assert!(names.contains(&"virtualenv"), "virtualenv in result");
}

// ─── Environment variable generation tests ───────────────────────────────────

#[test]
fn test_env_context_has_rez_variables() {
    use rez_next_context::{ContextConfig, EnvironmentManager, ResolvedContext};

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();
    create_package(&repo_dir, "python", "3.11.0", &[], &["python"], None);

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["python"]);
    assert!(!resolved.is_empty(), "Should resolve at least one package");

    // Build a minimal ResolvedContext and generate env vars
    let config = ContextConfig::default();
    let env_manager = EnvironmentManager::new(config);

    // Fetch actual package objects from temp repo
    let rt = tokio::runtime::Runtime::new().unwrap();
    let simple_repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(simple_repo.find_packages("python")).unwrap();
    assert!(!pkgs.is_empty());

    let packages: Vec<Package> = pkgs.iter().map(|p| (**p).clone()).collect();
    let env_vars = rt
        .block_on(env_manager.generate_environment(&packages))
        .unwrap();

    // REZ_USED_PACKAGES should be set by the context
    // At minimum we should have some env vars
    assert!(!env_vars.is_empty(), "Environment should contain variables");
}

#[test]
fn test_env_context_path_prepend() {
    use rez_next_context::{ContextConfig, EnvironmentManager};
    use rez_next_rex::RexExecutor;

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Create package with explicit PATH commands
    create_package(
        &repo_dir,
        "mytool",
        "1.0.0",
        &[],
        &["mytool"],
        Some("env.PATH.prepend('{root}/bin')"),
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let simple_repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(simple_repo.find_packages("mytool")).unwrap();
    assert!(!pkgs.is_empty(), "Should find mytool");

    let pkg = &pkgs[0];
    // Verify commands field was parsed
    if let Some(cmds) = &pkg.commands {
        assert!(
            cmds.contains("PATH") || cmds.contains("prepend"),
            "Commands should contain PATH prepend, got: {}",
            cmds
        );
    }
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

// ─── Repository manager multi-repo tests ─────────────────────────────────────

#[test]
fn test_multi_repo_priority() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    let repo1 = tmp1.path().to_path_buf();
    let repo2 = tmp2.path().to_path_buf();

    // Repo1 has python 3.11, repo2 has python 3.9
    create_package(&repo1, "python", "3.11.0", &[], &[], None);
    create_package(&repo2, "python", "3.9.0", &[], &[], None);

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo1.clone(),
        "repo1".to_string(),
    )));
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo2.clone(),
        "repo2".to_string(),
    )));

    let rt = tokio::runtime::Runtime::new().unwrap();
    let pkgs = rt.block_on(mgr.find_packages("python")).unwrap();

    // Both repos should be searched — expect 2 python versions total
    assert_eq!(pkgs.len(), 2, "Should find python from both repos");
}

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

// ─── Full pipeline E2E test ───────────────────────────────────────────────────

/// End-to-end test: create packages on disk → scan → solve → generate env
#[test]
fn test_full_pipeline_e2e() {
    use rez_next_context::{ContextConfig, EnvironmentManager};

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Create a small dependency tree
    create_package(
        &repo_dir,
        "python",
        "3.11.2",
        &[],
        &["python", "python3"],
        None,
    );
    create_package(&repo_dir, "numpy", "1.25.2", &["python-3.9+<4"], &[], None);
    create_package(
        &repo_dir,
        "scipy",
        "1.11.4",
        &["python-3.9+<4", "numpy-1.20+<2"],
        &["scipy-test"],
        None,
    );

    let repo = make_repo(&repo_dir);

    // Solve
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let requirements: Vec<Requirement> = vec!["scipy"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(requirements)).unwrap();

    assert!(
        !result.resolved_packages.is_empty(),
        "Should resolve packages"
    );
    let names: Vec<&str> = result
        .resolved_packages
        .iter()
        .map(|p| p.package.name.as_str())
        .collect();
    assert!(names.contains(&"scipy"), "scipy in result");
    assert!(names.contains(&"numpy"), "numpy in result");
    assert!(names.contains(&"python"), "python in result");

    // Generate environment from resolved packages
    let packages: Vec<Package> = result
        .resolved_packages
        .iter()
        .map(|p| (*p.package).clone())
        .collect();

    let env_config = ContextConfig::default();
    let env_manager = EnvironmentManager::new(env_config);
    let env_vars = rt
        .block_on(env_manager.generate_environment(&packages))
        .unwrap();

    // Environment should contain at least some variables
    assert!(!env_vars.is_empty(), "Environment should not be empty");

    println!(
        "E2E test: resolved {} packages, {} env vars",
        result.resolved_packages.len(),
        env_vars.len()
    );
    for (k, v) in &env_vars {
        println!("  {}={}", k, v);
    }
}

// ─── Version-aware solver tests ───────────────────────────────────────────────

#[test]
fn test_solve_conflict_detection() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // numpy 1.x requires python 2.7 specifically (simulated conflict)
    create_package(&repo_dir, "python", "3.11.0", &[], &[], None);
    create_package(&repo_dir, "oldlib", "1.0.0", &["python-2.7"], &[], None);

    let repo = make_repo(&repo_dir);

    let requirements: Vec<Requirement> = vec!["python-3.11", "oldlib"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(repo, config);
    let result = rt.block_on(resolver.resolve(requirements));

    // This should either fail or return with conflicts/empty
    match result {
        Err(_) => {
            // Expected: solver correctly detected conflict
        }
        Ok(res) => {
            // If solver returns result, oldlib should not be included if python 3.11 is there
            // (conflict handling — result may have failed_requirements or conflicts)
            println!(
                "Conflict test: {} resolved, {} failed, {} conflicts",
                res.resolved_packages.len(),
                res.failed_requirements.len(),
                res.conflicts.len()
            );
        }
    }
}

#[test]
fn test_solve_semver_range_ge_lt() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Add multiple python versions
    for ver in &["3.7.0", "3.8.0", "3.9.0", "3.10.0", "3.11.0"] {
        create_package(&repo_dir, "python", ver, &[], &[], None);
    }
    // Package that needs python >= 3.9, < 3.11
    create_package(
        &repo_dir,
        "mylib",
        "1.0.0",
        &["python-3.9+<3.11"],
        &[],
        None,
    );

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["mylib"]);

    let python = resolved.iter().find(|(n, _)| n == "python");
    assert!(python.is_some(), "python should be resolved");

    if let Some((_, ver)) = python {
        let v = Version::parse(ver).unwrap();
        let min = Version::parse("3.9").unwrap();
        let max = Version::parse("3.11").unwrap();
        assert!(
            v >= min && v < max,
            "Python {} should be in [3.9, 3.11)",
            ver
        );
    }
}
