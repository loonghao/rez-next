//! Package query and management functions exposed to Python.
//!
//! This module is split into sub-modules to keep each file under 1000 lines:
//! - `query`: Package query functions (get_latest_package, get_package, iter_packages, etc.)
//! - `management`: Package management functions (copy_package, move_package, remove_package)
//! - `utility`: Utility functions (get_completions, package_schema, dump_package_data, etc.)
//! - `uri`: URI functions (get_package_from_uri, get_variant_from_uri, etc.) - in parent `package_uri_functions.rs`

use std::path::PathBuf;

// ── Sub-modules ───────────────────────────────────────────────────────────────────
pub(crate) mod management;
pub(crate) mod query;
pub(crate) mod utility;

// ── Re-exports for lib.rs ───────────────────────────────────────────────────────
pub use management::{copy_package, create_package, move_package, remove_package};
pub use query::{
    get_latest_package, get_latest_package_from_string, get_package, get_package_family_names,
    get_package_from_string, iter_package_families, iter_packages, resolve_packages, walk_packages,
};
pub use utility::{
    dump_package_data, get_completions, get_developer_package, get_last_release_time,
    package_family_schema, package_release_keys, package_schema, schema_keys, test_function,
    variant_schema,
};

// ── Shared helper functions ─────────────────────────────────────────────────────

/// Expand `~` in path strings.
pub(crate) fn expand_home(p: &str) -> String {
    if (p.starts_with("~/") || p == "~")
        && let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
    {
        return p.replacen("~", &home, 1);
    }
    p.to_string()
}

/// Build a `RepositoryManager` from the provided or configured package paths.
pub(crate) fn make_repo_manager(
    paths: Option<Vec<String>>,
) -> rez_next_repository::simple_repository::RepositoryManager {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};

    let config = RezCoreConfig::load();
    let mut repo_manager = RepositoryManager::new();

    let pkg_paths: Vec<PathBuf> = paths
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    for (i, path) in pkg_paths.iter().enumerate() {
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path.clone(),
                format!("repo_{}", i),
            )));
        }
    }

    repo_manager
}

/// Recursively copy a directory tree (used by tests and legacy paths).
#[cfg(test)]
pub(crate) fn copy_dir_recursive(
    src: &std::path::Path,
    dest: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

// ── Test modules ─────────────────────────────────────────────────────────────────
#[cfg(test)]
#[path = "../package_functions_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../package_functions_extra_tests.rs"]
mod extra_tests;

#[cfg(test)]
#[path = "../package_functions_version_tests.rs"]
mod version_tests;

#[cfg(test)]
#[path = "../package_functions_move_tests.rs"]
mod move_tests;
