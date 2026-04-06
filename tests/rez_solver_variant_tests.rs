//! Solver Variant Tests (Cycle 34)
//!
//! Covers:
//! - ResolvedPackageInfo.variant_index is None for packages without variants
//! - ResolvedPackageInfo.variant_index is Some(0) when first variant is compatible
//! - ResolvedPackageInfo.variant_index is Some for any package with variants
//! - variant_index correctness in multi-package resolution

use rez_next_package::Requirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::sync::Arc;
use tempfile::TempDir;

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Build a repository with plain packages (name, version, requires[]).
fn build_plain_repo(packages: &[(&str, &str, &[&str])]) -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();
    for (name, version, requires) in packages {
        let pkg_dir = tmp.path().join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let req_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        std::fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n{}", name, version, req_block),
        )
        .unwrap();
    }
    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp.path(),
        "plain_repo".to_string(),
    )));
    (tmp, Arc::new(mgr))
}

/// Build a repository that includes packages with variants.
/// `variant_packages`: (name, version, base_requires[], variants[][])
#[allow(clippy::type_complexity)]
fn build_variant_repo(
    base_packages: &[(&str, &str, &[&str])],
    variant_packages: &[(&str, &str, &[&str], &[&[&str]])],
) -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();

    for (name, version, requires) in base_packages {
        let pkg_dir = tmp.path().join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let req_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        std::fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n{}", name, version, req_block),
        )
        .unwrap();
    }

    for (name, version, requires, variants) in variant_packages {
        let pkg_dir = tmp.path().join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let req_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        let variant_rows: Vec<String> = variants
            .iter()
            .map(|row| {
                let cols: Vec<String> = row.iter().map(|s| format!("'{}'", s)).collect();
                format!("    [{}],", cols.join(", "))
            })
            .collect();
        let variants_block = format!("variants = [\n{}\n]\n", variant_rows.join("\n"));
        std::fs::write(
            pkg_dir.join("package.py"),
            format!(
                "name = '{}'\nversion = '{}'\n{}{}",
                name, version, req_block, variants_block
            ),
        )
        .unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        tmp.path(),
        "variant_repo".to_string(),
    )));
    (tmp, Arc::new(mgr))
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// ─── Cycle 34: variant_index tests ───────────────────────────────────────────

/// Solver variant: package without variants has variant_index = None.
#[test]
fn test_solver_no_variant_gives_none_index() {
    let (_tmp, repo) = build_plain_repo(&[("simple_pkg", "1.0.0", &[])]);
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["simple_pkg".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();
    assert_eq!(result.resolved_packages.len(), 1);
    assert_eq!(
        result.resolved_packages[0].variant_index,
        None,
        "package without variants should have variant_index = None"
    );
}

/// Solver variant: package with 2 variants, first compatible → index=0.
#[test]
fn test_solver_variant_first_compatible_selected() {
    let (_tmp, repo) = build_variant_repo(
        &[("python", "3.9.0", &[])],
        &[("mylib", "1.0.0", &[], &[&["python-3.9"], &["python-3.10"]])],
    );
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["mylib".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();

    let mylib_info = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "mylib")
        .expect("mylib should be resolved");
    assert_eq!(
        mylib_info.variant_index,
        Some(0),
        "variant[0] (python-3.9) should be selected when python-3.9.0 is available"
    );
}

/// Solver variant: package with variants → variant_index is Some (regardless of which).
#[test]
fn test_solver_variant_index_is_some_when_variants_exist() {
    let (_tmp, repo) = build_variant_repo(
        &[],
        &[("pkg_v", "2.0.0", &[], &[&["dep-1.0"], &["dep-2.0"]])],
    );
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["pkg_v".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();

    let info = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "pkg_v")
        .expect("pkg_v should be resolved");
    assert!(
        info.variant_index.is_some(),
        "package with variants should have Some(variant_index)"
    );
}

/// Solver variant: multi-package resolution selects correct variant based on resolved deps.
#[test]
fn test_solver_variant_index_in_multi_package_result() {
    let (_tmp, repo) = build_variant_repo(
        &[("base_dep", "1.5.0", &[])],
        &[("top_pkg", "1.0.0", &[], &[&["base_dep-1.5"], &["base_dep-2.0"]])],
    );
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["top_pkg".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();

    let top_info = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "top_pkg")
        .expect("top_pkg should be resolved");
    assert_eq!(
        top_info.variant_index,
        Some(0),
        "top_pkg should select variant[0] (base_dep-1.5) matching base_dep-1.5.0"
    );
}

/// Solver variant: package has only one variant, always resolves to index=0.
#[test]
fn test_solver_single_variant_always_index_zero() {
    let (_tmp, repo) = build_variant_repo(
        &[("tool", "1.0.0", &[])],
        &[("app", "1.0.0", &[], &[&["tool-1.0"]])],
    );
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["app".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();

    let app_info = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "app")
        .expect("app should be resolved");
    assert_eq!(
        app_info.variant_index,
        Some(0),
        "package with single variant should always select index 0"
    );
}

/// Solver variant: both variant packages in a mixed repo resolve with correct indices.
#[test]
fn test_solver_multiple_variant_packages_independent_indices() {
    let (_tmp, repo) = build_variant_repo(
        &[("python", "3.9.0", &[]), ("os_lib", "2.0.0", &[])],
        &[
            ("pkgA", "1.0.0", &[], &[&["python-3.9"], &["python-3.10"]]),
            ("pkgB", "1.0.0", &[], &[&["os_lib-2.0"], &["os_lib-3.0"]]),
        ],
    );
    let mut resolver = DependencyResolver::new(Arc::clone(&repo), SolverConfig::default());
    let reqs: Vec<Requirement> = vec!["pkgA".parse().unwrap(), "pkgB".parse().unwrap()];
    let result = rt().block_on(resolver.resolve(reqs)).unwrap();

    let pkga = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "pkgA");
    let pkgb = result
        .resolved_packages
        .iter()
        .find(|p| p.package.name == "pkgB");

    assert!(pkga.is_some(), "pkgA should be resolved");
    assert!(pkgb.is_some(), "pkgB should be resolved");
    assert!(
        pkga.unwrap().variant_index.is_some(),
        "pkgA should have some variant_index"
    );
    assert!(
        pkgb.unwrap().variant_index.is_some(),
        "pkgB should have some variant_index"
    );
}
