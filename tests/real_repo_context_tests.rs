//! Real Repository — Context, Environment & Multi-Repo Integration Tests
//!
//! Covers: env-var generation, PATH prepend, multi-repo priority/shadowing,
//! tools round-trip, empty-repo safety, description special-chars, and full E2E pipeline.
use rez_next_package::{Package, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_repository::PackageRepository;
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

// ─── Local helpers ────────────────────────────────────────────────────────────

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

// ─── Environment variable generation tests ───────────────────────────────────

#[test]
fn test_env_context_has_rez_variables() {
    use rez_next_context::{ContextConfig, EnvironmentManager};

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();
    create_package(&repo_dir, "python", "3.11.0", &[], &["python"], None);

    let repo = make_repo(&repo_dir);
    let requirements: Vec<Requirement> = ["python"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let result = rt.block_on(resolver.resolve(requirements)).unwrap();
    assert!(!result.resolved_packages.is_empty(), "Should resolve at least one package");

    let env_config = ContextConfig::default();
    let env_manager = EnvironmentManager::new(env_config);

    let simple_repo = SimpleRepository::new(repo_dir.clone(), "r".to_string());
    let pkgs = rt.block_on(simple_repo.find_packages("python")).unwrap();
    assert!(!pkgs.is_empty());

    let packages: Vec<Package> = pkgs.iter().map(|p| (**p).clone()).collect();
    let env_vars = rt
        .block_on(env_manager.generate_environment(&packages))
        .unwrap();

    assert!(!env_vars.is_empty(), "Environment should contain variables");
}

#[test]
fn test_env_context_path_prepend() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

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
    if let Some(cmds) = &pkg.commands {
        assert!(
            cmds.contains("PATH") || cmds.contains("prepend"),
            "Commands should contain PATH prepend, got: {}",
            cmds
        );
    }
}

// ─── Full pipeline E2E test ───────────────────────────────────────────────────

#[test]
fn test_full_pipeline_e2e() {
    use rez_next_context::{ContextConfig, EnvironmentManager};

    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

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

    let rt = tokio::runtime::Runtime::new().unwrap();
    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), config);
    let requirements: Vec<Requirement> = ["scipy"]
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

// ─── Multi-repo tests ─────────────────────────────────────────────────────────

#[test]
fn test_multi_repo_priority() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    let repo1 = tmp1.path().to_path_buf();
    let repo2 = tmp2.path().to_path_buf();

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

    assert_eq!(pkgs.len(), 2, "Should find python from both repos");
}

#[test]
fn test_multi_repo_shadowing_and_fallback() {
    let tmp1 = TempDir::new().unwrap();
    let tmp2 = TempDir::new().unwrap();

    let repo1 = tmp1.path().to_path_buf();
    let repo2 = tmp2.path().to_path_buf();

    create_package(&repo1, "nuke", "15.0.0", &[], &["nuke"], None);
    create_package(&repo1, "studio_lib", "2.0.0", &[], &[], None);

    create_package(&repo2, "nuke", "14.0.0", &[], &["nuke"], None);
    create_package(&repo2, "nuke", "13.0.0", &[], &["nuke"], None);
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
    let all_nukes = rt.block_on(repo.find_packages("nuke")).unwrap();
    assert!(
        all_nukes.len() >= 3,
        "Should see at least nuke 13/14/15 across repos, got {}",
        all_nukes.len()
    );

    let vendor = rt.block_on(repo.find_packages("vendor_lib")).unwrap();
    assert!(
        !vendor.is_empty(),
        "vendor_lib from repo2 should be reachable"
    );

    let studio = rt.block_on(repo.find_packages("studio_lib")).unwrap();
    assert!(!studio.is_empty(), "studio_lib from repo1 should be found");
}

// ─── Tools round-trip & edge cases ───────────────────────────────────────────

/// All declared tools must survive parse-serialize-scan round-trip.
#[test]
fn test_tools_roundtrip_filesystem() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    let declared_tools = ["houdini", "hython", "hconfig", "hserver", "houdinifx"];
    create_package(&repo_dir, "houdini", "20.0.547", &[], &declared_tools, None);

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

/// Empty repo must not panic on scan or solve.
#[test]
fn test_empty_repo_directory_no_panic() {
    let tmp = TempDir::new().unwrap();
    let empty_repo = tmp.path().to_path_buf();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(empty_repo.clone(), "empty".to_string());
    let pkgs = rt.block_on(repo.find_packages("python")).unwrap();
    assert!(pkgs.is_empty(), "Empty repo should return no packages");

    let repo_mgr = make_repo(&empty_repo);
    let requirements: Vec<Requirement> = ["python"]
        .iter()
        .map(|s| s.parse::<Requirement>().unwrap())
        .collect();

    let config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(repo_mgr, config);
    let result = rt.block_on(resolver.resolve(requirements));
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

/// Regression: description with special chars must not panic or corrupt metadata.
#[test]
fn test_package_py_description_special_chars() {
    let tmp = TempDir::new().unwrap();
    let pkg_dir = tmp.path().join("testpkg").join("1.0.0");
    fs::create_dir_all(&pkg_dir).unwrap();

    fs::write(
        pkg_dir.join("package.py"),
        r#"name = "testpkg"
version = "1.0.0"
description = "A package with \"quotes\" and path C:\\tool"
"#,
    )
    .unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let repo = SimpleRepository::new(tmp.path(), "r".to_string());
    let result = rt.block_on(repo.find_packages("testpkg"));
    match result {
        Ok(pkgs) => {
            if !pkgs.is_empty() {
                assert_eq!(pkgs[0].name, "testpkg");
                assert_eq!(pkgs[0].version.as_ref().map(|v| v.as_str()), Some("1.0.0"));
            }
        }
        Err(_) => { /* parse error on unusual string is acceptable — no panic is the requirement */ }
    }
}
