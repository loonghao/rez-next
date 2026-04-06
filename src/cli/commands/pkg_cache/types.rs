//! # Package Cache Types
//!
//! Data structures and argument definitions for the `rez pkg-cache` command.

use clap::Args;
use std::path::PathBuf;

/// Package cache management arguments
#[derive(Args, Clone, Debug)]
pub struct PkgCacheArgs {
    /// Package cache directory path
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Add variants to the cache
    #[arg(short = 'a', long = "add-variants", value_name = "URI")]
    pub add_variants: Vec<String>,

    /// Remove variants from cache
    #[arg(short = 'r', long = "remove-variants", value_name = "URI")]
    pub remove_variants: Vec<String>,

    /// Remove unused variants and other cache files pending deletion
    #[arg(long)]
    pub clean: bool,

    /// View logs
    #[arg(long)]
    pub logs: bool,

    /// Run as a daemon that adds pending variants to the cache, then exits
    #[arg(long, hide = true)]
    pub daemon: bool,

    /// Override package cache mode
    #[arg(long, value_enum)]
    pub pkg_cache_mode: Option<PkgCacheMode>,

    /// Columns to print
    #[arg(short = 'c', long = "columns", value_delimiter = ',')]
    pub columns: Vec<String>,

    /// Force a package add, even if package is not cachable
    #[arg(short = 'f', long)]
    pub force: bool,
}

/// Package cache mode
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum PkgCacheMode {
    /// Synchronous mode - block until packages are cached
    Sync,
    /// Asynchronous mode - don't block while packages are cached
    Async,
}

/// Cache entry status
#[derive(Debug, Clone)]
pub enum CacheStatus {
    /// Variant is cached and available
    Cached,
    /// Variant is currently being copied
    Copying,
    /// Copy operation has stalled
    Stalled,
    /// Variant is pending cache operation
    Pending,
}

/// Cache entry information
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub package_name: String,
    pub variant_uri: String,
    pub original_path: Option<std::path::PathBuf>,
    pub cache_path: Option<std::path::PathBuf>,
    pub status: CacheStatus,
}

impl Default for PkgCacheArgs {
    fn default() -> Self {
        Self {
            dir: None,
            add_variants: Vec::new(),
            remove_variants: Vec::new(),
            clean: false,
            logs: false,
            daemon: false,
            pkg_cache_mode: None,
            columns: vec![
                "status".to_string(),
                "package".to_string(),
                "variant_uri".to_string(),
                "cache_path".to_string(),
            ],
            force: false,
        }
    }
}
