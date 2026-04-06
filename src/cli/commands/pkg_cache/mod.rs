//! # Package Cache Management Command
//!
//! Implements the `rez pkg-cache` command for managing package cache operations.
//! This command provides functionality to view, add, remove, and clean package cache entries.
//!
//! ## Module Layout
//!
//! - `types`   — `PkgCacheArgs`, `PkgCacheMode`, `CacheEntry`, `CacheStatus`
//! - `ops`     — core operations: add/remove/clean/daemon/logs, plus directory resolution
//! - `display` — table rendering, disk scan, status formatting

mod display;
mod ops;
mod types;

pub use types::{CacheEntry, CacheStatus, PkgCacheArgs, PkgCacheMode};

use rez_next_common::error::RezCoreResult;

/// Execute the pkg-cache command
pub async fn execute(args: PkgCacheArgs) -> RezCoreResult<()> {
    // Determine cache directory
    let cache_dir = ops::determine_cache_directory(&args)?;

    // Initialize cache manager
    let cache_manager = ops::initialize_cache_manager(&cache_dir).await?;

    // Execute specific operation
    if args.daemon {
        ops::run_daemon(&cache_manager, &cache_dir).await
    } else if !args.add_variants.is_empty() {
        ops::add_variants(&cache_manager, &args.add_variants, &args).await
    } else if !args.remove_variants.is_empty() {
        ops::remove_variants(&cache_manager, &args.remove_variants).await
    } else if args.clean {
        ops::clean_cache(&cache_manager).await
    } else if args.logs {
        ops::view_logs(&cache_dir).await
    } else {
        // Default: show cache status
        display::show_cache_status(&cache_manager, &args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkg_cache_args_default() {
        let args = PkgCacheArgs::default();
        assert!(args.dir.is_none());
        assert!(args.add_variants.is_empty());
        assert!(args.remove_variants.is_empty());
        assert!(!args.clean);
        assert!(!args.logs);
        assert!(!args.daemon);
        assert!(!args.force);
        assert_eq!(
            args.columns,
            vec!["status", "package", "variant_uri", "cache_path"]
        );
    }
}
