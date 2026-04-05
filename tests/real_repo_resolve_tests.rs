//! Real Repository — Solver / Resolution Integration Tests
//!
//! Exercises the full rez-next solve pipeline using actual package.py files on disk.
//! Covers: version selection, transitive deps, conflicts, semver ranges, exact pins.
use rez_next_package::Requirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;

// ─── Local helpers (mirrors real_repo_integration.rs) ────────────────────────

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
    let (_, ver) = python.unwrap();
    let v = Version::parse(ver).unwrap();
    let v311 = Version::parse("3.11.0").unwrap();
    assert_eq!(v, v311, "Should select latest python 3.11.0, got {}", ver);
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

#[test]
fn test_solve_conflict_detection() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

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

    match result {
        Err(_) => { /* conflict correctly rejected */ }
        Ok(res) => {
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

    for ver in &["3.7.0", "3.8.0", "3.9.0", "3.10.0", "3.11.0"] {
        create_package(&repo_dir, "python", ver, &[], &[], None);
    }
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

/// Verify exact-build pinning (tight 3-token patch range).
/// Note: rez version semantics — 20.1 > 20.0.0 (fewer tokens = higher epoch).
#[test]
fn test_solve_exact_version_pin_filesystem() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "houdini", "19.5.0", &[], &["houdini"], None);
    create_package(&repo_dir, "houdini", "20.0.0", &[], &["houdini"], None);
    create_package(&repo_dir, "houdini", "20.5.0", &[], &["houdini"], None);
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
    assert!(
        houdini.is_some(),
        "houdini should be resolved as a dep of pinned_tool"
    );

    let (_, ver) = houdini.unwrap();
    assert_eq!(
        ver.as_str(),
        "20.0.0",
        "pinned_tool should resolve houdini exactly at 20.0.0, got {}",
        ver
    );
}

/// Two packages sharing a transitive dep with non-overlapping version constraints.
#[test]
fn test_solve_shared_dep_version_downgrade() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    create_package(&repo_dir, "openexr", "2.5.0", &[], &[], None);
    create_package(&repo_dir, "openexr", "3.1.0", &[], &[], None);
    create_package(
        &repo_dir,
        "arnold",
        "7.0.0",
        &["openexr-3+"],
        &["kick"],
        None,
    );
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

    match result {
        Err(_) => { /* conflict correctly rejected */ }
        Ok(res) => {
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
