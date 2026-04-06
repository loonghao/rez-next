//! Shared repository-manager helpers for real repository integration tests.
//!
//! Used by:
//! - `real_repo_context_tests.rs`
//! - `real_repo_resolve_tests.rs`

use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::Path;
use std::sync::Arc;

/// Build a repository manager for a temporary real repository directory.
pub fn make_repo(dir: &Path) -> Arc<RepositoryManager> {
    let mut manager = RepositoryManager::new();
    if dir.exists() {
        manager.add_repository(Box::new(SimpleRepository::new(
            dir,
            "test_repo".to_string(),
        )));
    }
    Arc::new(manager)
}
