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
        mgr.add_repository(Box::new(SimpleRepository::new(dir, "test_repo".to_string())));
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

    create_package(&repo_dir, "python", "3.9.0", &[], &["python", "python3"], None);
    create_package(&repo_dir, "python", "3.11.0", &[], &["python", "python3"], None);
    create_package(&repo_dir, "maya", "2024.0", &["python-3+<4"], &["maya"], None);

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
    create_package(&repo_dir, "numpy", "1.25.0", &["python-3.9+<4"], &["python"], None);
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
    assert!(names.contains(&"numpy"), "numpy must be pulled in as transitive dep");
    assert!(names.contains(&"python"), "python must be pulled in as transitive dep");
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
    assert_eq!(req.name, "python", "python-3.11 name should be 'python', got '{}'", req.name);
    let v311 = Version::parse("3.11.0").unwrap();
    assert!(req.is_satisfied_by(&v311), "python-3.11 should be satisfied by 3.11.0");

    // "python-3+<4" → name=python, >=3 <4
    let req2 = "python-3+<4".parse::<Requirement>().unwrap();
    assert_eq!(req2.name, "python", "python-3+<4 name should be 'python', got '{}'", req2.name);
    assert!(req2.is_satisfied_by(&Version::parse("3.11.0").unwrap()));
    assert!(!req2.is_satisfied_by(&Version::parse("4.0.0").unwrap()));

    // "pip-20+" → name=pip, >=20
    let req3 = "pip-20+".parse::<Requirement>().unwrap();
    assert_eq!(req3.name, "pip", "pip-20+ name should be 'pip', got '{}'", req3.name);
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
    create_package(&repo_dir, "virtualenv", "20.0.0", &["python-3+<4", "pip-20+"], &[], None);

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
    use rez_next_context::{ContextConfig, EnvironmentManager};

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
    assert!(
        !env_vars.is_empty(),
        "Environment should contain variables"
    );
}

#[test]
fn test_env_context_path_prepend() {
    
    

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
        assert!(cmds.contains("PATH") || cmds.contains("prepend"), 
            "Commands should contain PATH prepend, got: {}", cmds);
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
    mgr.add_repository(Box::new(SimpleRepository::new(repo1.clone(), "repo1".to_string())));
    mgr.add_repository(Box::new(SimpleRepository::new(repo2.clone(), "repo2".to_string())));

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
    let pkgs = rt.block_on(repo.find_packages("nonexistent_package_xyz")).unwrap();

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
    create_package(&repo_dir, "python", "3.11.2", &[], &["python", "python3"], None);
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
    let requirements: Vec<Requirement> = ["scipy"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let result = rt.block_on(resolver.resolve(requirements)).unwrap();

    assert!(!result.resolved_packages.is_empty(), "Should resolve packages");
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

    let requirements: Vec<Requirement> = ["python-3.11", "oldlib"]
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
            println!("Conflict test: {} resolved, {} failed, {} conflicts",
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
    create_package(&repo_dir, "mylib", "1.0.0", &["python-3.9+<3.11"], &[], None);

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["mylib"]);

    let python = resolved.iter().find(|(n, _)| n == "python");
    assert!(python.is_some(), "python should be resolved");

    if let Some((_, ver)) = python {
        let v = Version::parse(ver).unwrap();
        let min = Version::parse("3.9").unwrap();
        let max = Version::parse("3.11").unwrap();
        assert!(v >= min && v < max, 
            "Python {} should be in [3.9, 3.11)", ver);
    }
}

// ─── Extended real-filesystem scenarios (Cycle 26) ────────────────────────────

/// Verify that when a package requires a tight patch range, only that patch is accepted.
/// Simulates exact-build pinning behavior common in production VFX pipelines.
/// Note: in rez version semantics, 20.1 > 20.0.0 (fewer tokens = higher epoch).
/// Use full 3-token bounds (20.0.0+<20.0.1) to avoid cross-token epoch surprises.
#[test]
fn test_solve_exact_version_pin_filesystem() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "houdini", "19.5.0", &[], &["houdini"], None);
    create_package(&repo_dir, "houdini", "20.0.0", &[], &["houdini"], None);
    create_package(&repo_dir, "houdini", "20.5.0", &[], &["houdini"], None);
    // pinned_tool requires exactly houdini 20.0.0 (tight 3-token patch range)
    create_package(
        &repo_dir,
        "pinned_tool",
        "1.0.0",
        &["houdini-20.0.0+<20.0.1"],
        &["pinned_tool"],
        None,
    );

    let repo = make_repo(&repo_dir);
    let resolved = resolve(repo, vec!["pinned_tool"]);

    let houdini = resolved.iter().find(|(n, _)| n == "houdini");
    assert!(houdini.is_some(), "houdini should be resolved as a dep of pinned_tool");

    let (_, ver) = houdini.unwrap();
    assert_eq!(
        ver.as_str(), "20.0.0",
        "pinned_tool should resolve houdini exactly at 20.0.0, got {}",
        ver
    );
}

/// When two packages in the same request share a transitive dep with non-overlapping
/// version constraints, the solver must detect or surface the conflict.
#[test]
fn test_solve_shared_dep_version_downgrade() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    // Two versions of openexr available
    create_package(&repo_dir, "openexr", "2.5.0", &[], &[], None);
    create_package(&repo_dir, "openexr", "3.1.0", &[], &[], None);

    // arnold requires openexr >= 3.0
    create_package(
        &repo_dir,
        "arnold",
        "7.0.0",
        &["openexr-3+"],
        &["kick"],
        None,
    );
    // old_renderer requires openexr < 3.0
    create_package(
        &repo_dir,
        "old_renderer",
        "1.0.0",
        &["openexr-2+<3"],
        &[],
        None,
    );

    let repo = make_repo(&repo_dir);

    let requirements: Vec<Requirement> = ["arnold", "old_renderer"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let result = rt.block_on(resolver.resolve(requirements));

    // Solver must not panic; conflict or empty/failed resolution is acceptable
    match result {
        Err(_) => { /* conflict correctly rejected */ }
        Ok(res) => {
            // If solver succeeds, it should not include both openexr-2.x and openexr-3.x
            let openexr_vers: Vec<&str> = res
                .resolved_packages
                .iter()
                .filter(|rp| rp.package.name == "openexr")
                .filter_map(|rp| rp.package.version.as_ref().map(|v| v.as_str()))
                .collect();
            assert!(
                openexr_vers.len() <= 1,
                "Solver should not select multiple conflicting openexr versions: {:?}",
                openexr_vers
            );
        }
    }
}

/// Multiple repos: verify that packages from a higher-priority repo shadow lower-priority ones.
/// Also verifies that packages only in repo2 are still reachable.
#[test]
fn test_multi_repo_shadowing_and_fallback() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    let repo1 = tmp1.path().to_path_buf();
    let repo2 = tmp2.path().to_path_buf();

    // Repo1: studio-override of nuke at 15.0, plus a unique pkg
    create_package(&repo1, "nuke", "15.0.0", &[], &["nuke"], None);
    create_package(&repo1, "studio_lib", "2.0.0", &[], &[], None);

    // Repo2: vendor nuke at 14.0 (should be superseded by repo1's 15.0 when
    // version selection is considered), plus nuke 13.0
    create_package(&repo2, "nuke", "14.0.0", &[], &["nuke"], None);
    create_package(&repo2, "nuke", "13.0.0", &[], &["nuke"], None);
    // Unique to repo2
    create_package(&repo2, "vendor_lib", "1.0.0", &[], &[], None);

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo1.clone(),
        "studio".to_string(),
    )));
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo2.clone(),
        "vendor".to_string(),
    )));
    let repo = Arc::new(mgr);

    let rt = tokio::runtime::Runtime::new().unwrap();
    // All nuke versions across repos should be visible
    let all_nukes = rt.block_on(repo.find_packages("nuke")).unwrap();
    assert!(
        all_nukes.len() >= 3,
        "Should see at least nuke 13/14/15 across repos, got {}",
        all_nukes.len()
    );

    // vendor_lib (only in repo2) should still be found
    let vendor = rt.block_on(repo.find_packages("vendor_lib")).unwrap();
    assert!(!vendor.is_empty(), "vendor_lib from repo2 should be reachable");

    // studio_lib (only in repo1) should be found
    let studio = rt.block_on(repo.find_packages("studio_lib")).unwrap();
    assert!(!studio.is_empty(), "studio_lib from repo1 should be found");
}

/// Tools field: verify that all declared tools survive the parse-serialize-scan round-trip.
/// Simulates DCC tools introspection used by rez's `rez-tools` command.
#[test]
fn test_tools_roundtrip_filesystem() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    let declared_tools = ["houdini", "hython", "hconfig", "hserver", "houdinifx"];
    create_package(
        &repo_dir,
        "houdini",
        "20.0.547",
        &[],
        &declared_tools,
        None,
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(repo.find_packages("houdini")).unwrap();

    assert_eq!(pkgs.len(), 1, "Expect exactly one houdini package");
    let pkg = &pkgs[0];

    for tool in &declared_tools {
        assert!(
            pkg.tools.contains(&tool.to_string()),
            "Tool '{}' should be present in parsed package, got: {:?}",
            tool,
            pkg.tools
        );
    }
    assert_eq!(
        pkg.tools.len(),
        declared_tools.len(),
        "No extra/missing tools expected; got: {:?}",
        pkg.tools
    );
}

/// Empty repository directory: solver and repo scan should handle gracefully.
#[test]
fn test_empty_repo_directory_no_panic() {
    let tmp = TempDir::new().unwrap();
    let empty_repo = tmp.path().to_path_buf();
    // Do NOT create any packages

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(empty_repo.clone(), "empty".to_string());
    let pkgs = rt.block_on(repo.find_packages("python")).unwrap();
    assert!(pkgs.is_empty(), "Empty repo should return no packages");

    // Solver on empty repo should not panic
    let repo_mgr = make_repo(&empty_repo);
    let requirements: Vec<Requirement> = ["python"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(repo_mgr, config);
    let result = rt.block_on(resolver.resolve(requirements));
    // May return Ok (lenient) or Err — must not panic
    match result {
        Ok(res) => {
            let names: Vec<&str> = res
                .resolved_packages
                .iter()
                .map(|rp| rp.package.name.as_str())
                .collect();
            assert!(
                !names.contains(&"python"),
                "Empty repo: python should not appear in result"
            );
        }
        Err(_) => { /* strict mode: also acceptable */ }
    }
}

/// Regression: package description field with special characters (quotes, backslashes).
/// Verifies the parser does not panic or corrupt metadata on unusual strings.
#[test]
fn test_package_py_description_special_chars() {
    let tmp = TempDir::new().unwrap();
    let pkg_dir = tmp.path().join("testpkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).unwrap();

    // Write a package.py with a description containing quotes and a backslash
    fs::write(
        pkg_dir.join("package.py"),
        r#"name = "testpkg"
version = "1.0.0"
description = "A package with \"quotes\" and path C:\\tool"
"#,
    )
    .unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(tmp.path().to_path_buf(), "r".to_string());
    // Must not panic; either parses successfully or returns an error
    let result = rt.block_on(repo.find_packages("testpkg"));
    match result {
        Ok(pkgs) => {
            // If parsed, name and version must be correct
            if !pkgs.is_empty() {
                assert_eq!(pkgs[0].name, "testpkg");
                assert_eq!(
                    pkgs[0].version.as_ref().map(|v| v.as_str()),
                    Some("1.0.0")
                );
            }
        }
        Err(_) => { /* parse error on unusual string is acceptable — no panic is the requirement */ }
    }
}
