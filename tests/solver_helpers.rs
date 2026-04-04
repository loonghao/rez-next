//! Shared test helpers for solver integration tests.
//!
//! Used by:
//! - rez_solver_advanced_tests.rs
//! - rez_solver_edge_case_tests.rs
//! - rez_solver_graph_tests.rs
//! - rez_solver_platform_tests.rs
//!
//! Each consumer must include:
//!   `#[path = "solver_helpers.rs"] mod solver_helpers;`
//! then call `solver_helpers::build_test_repo(...)`.

use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::sync::Arc;
use tempfile::TempDir;

/// Build a temporary package repository with multiple packages.
///
/// `packages` is a slice of `(name, version, requires)` tuples.
/// Returns the `TempDir` (must be kept alive for the lifetime of the test) and
/// an `Arc<RepositoryManager>` pointing at the temporary directory.
pub fn build_test_repo(packages: &[(&str, &str, &[&str])]) -> (TempDir, Arc<RepositoryManager>) {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().to_path_buf();

    for (name, version, requires) in packages {
        let pkg_dir = repo_dir.join(name).join(version);
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let requires_block = if requires.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = requires.iter().map(|r| format!("    '{}',", r)).collect();
            format!("requires = [\n{}\n]\n", items.join("\n"))
        };
        std::fs::write(
            pkg_dir.join("package.py"),
            format!(
                "name = '{}'\nversion = '{}'\n{}",
                name, version, requires_block
            ),
        )
        .unwrap();
    }

    let mut mgr = RepositoryManager::new();
    mgr.add_repository(Box::new(SimpleRepository::new(
        repo_dir.clone(),
        "test_repo".to_string(),
    )));
    (tmp, Arc::new(mgr))
}
