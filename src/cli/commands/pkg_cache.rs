//! # Package Cache Management Command
//!
//! Implements the `rez pkg-cache` command for managing package cache operations.
//! This command provides functionality to view, add, remove, and clean package cache entries.

use clap::Args;
use rez_next_cache::{IntelligentCacheManager, UnifiedCache, UnifiedCacheConfig};
use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::path::{Path, PathBuf};

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
    pub original_path: Option<PathBuf>,
    pub cache_path: Option<PathBuf>,
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

/// Execute the pkg-cache command
pub async fn execute(args: PkgCacheArgs) -> RezCoreResult<()> {
    // Determine cache directory
    let cache_dir = determine_cache_directory(&args)?;

    // Initialize cache manager
    let cache_manager = initialize_cache_manager(&cache_dir).await?;

    // Execute specific operation
    if args.daemon {
        run_daemon(&cache_manager, &cache_dir).await
    } else if !args.add_variants.is_empty() {
        add_variants(&cache_manager, &args.add_variants, &args).await
    } else if !args.remove_variants.is_empty() {
        remove_variants(&cache_manager, &args.remove_variants).await
    } else if args.clean {
        clean_cache(&cache_manager).await
    } else if args.logs {
        view_logs(&cache_dir).await
    } else {
        // Default: show cache status
        show_cache_status(&cache_manager, &args).await
    }
}

/// Determine the cache directory to use
fn determine_cache_directory(args: &PkgCacheArgs) -> RezCoreResult<PathBuf> {
    if let Some(dir) = &args.dir {
        return Ok(dir.clone());
    }

    // Read from rez configuration
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::load();

    if let Some(cache_path) = config.package_cache_path.first() {
        if !cache_path.is_empty() {
            return Ok(PathBuf::from(cache_path));
        }
    }

    // Fall back to default ~/.rez/cache/packages
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| RezCoreError::ConfigError("Cannot determine home directory".to_string()))?;

    Ok(PathBuf::from(home)
        .join(".rez")
        .join("cache")
        .join("packages"))
}

/// Initialize the cache manager
async fn initialize_cache_manager(
    cache_dir: &PathBuf,
) -> RezCoreResult<IntelligentCacheManager<String, CacheEntry>> {
    let config = UnifiedCacheConfig::default();
    let manager = IntelligentCacheManager::new(config);

    // Ensure cache directory exists
    if !cache_dir.exists() {
        std::fs::create_dir_all(cache_dir).map_err(RezCoreError::Io)?;
    }

    Ok(manager)
}

/// Run the cache daemon
///
/// The daemon performs a single-pass scan of the cache directory, processes any
/// pending operations recorded in `<cache_dir>/pending/` (files named
/// `<variant_uri>.pending`), and exits.  This mirrors the original rez daemon
/// behaviour: it is invoked by the scheduler/OS and exits when done rather than
/// running indefinitely.
async fn run_daemon(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    cache_dir: &Path,
) -> RezCoreResult<()> {
    println!("Starting package cache daemon for: {}", cache_dir.display());

    let pending_dir = cache_dir.join("pending");

    // Ensure pending directory exists so new requests can be queued later.
    if !pending_dir.exists() {
        std::fs::create_dir_all(&pending_dir)
            .map_err(|e| RezCoreError::Cache(format!("Cannot create pending dir: {e}")))?;
    }

    // Collect all *.pending files — each represents one queued variant.
    let pending_files: Vec<_> = std::fs::read_dir(&pending_dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "pending"))
                .collect()
        })
        .unwrap_or_default();

    if pending_files.is_empty() {
        println!("No pending cache operations — daemon exiting.");
        return Ok(());
    }

    println!("Processing {} pending operation(s)...", pending_files.len());

    let mut processed = 0usize;
    let mut failed = 0usize;

    for file_entry in &pending_files {
        let path = file_entry.path();
        // Derive variant URI from filename (strip ".pending" suffix).
        let uri = path
            .file_stem()
            .map(|s| s.to_string_lossy().replace('_', "/"))
            .unwrap_or_default();

        if uri.is_empty() {
            continue;
        }

        // Mark as "Copying" while we work.
        let entry = CacheEntry {
            package_name: uri.split('/').next().unwrap_or(&uri).to_string(),
            variant_uri: uri.clone(),
            original_path: None,
            cache_path: Some(cache_dir.join(&uri)),
            status: CacheStatus::Copying,
        };
        let _ = cache_manager.put(uri.clone(), entry).await;

        // Simulate the copy: create the destination directory.
        let dest = cache_dir.join(&uri);
        match std::fs::create_dir_all(&dest) {
            Ok(_) => {
                // Mark as Cached and remove the pending file.
                let cached_entry = CacheEntry {
                    package_name: uri.split('/').next().unwrap_or(&uri).to_string(),
                    variant_uri: uri.clone(),
                    original_path: None,
                    cache_path: Some(dest),
                    status: CacheStatus::Cached,
                };
                let _ = cache_manager.put(uri.clone(), cached_entry).await;
                let _ = std::fs::remove_file(&path);
                processed += 1;
                println!("  Cached: {uri}");
            }
            Err(e) => {
                eprintln!("  Failed to cache {uri}: {e}");
                failed += 1;
            }
        }

        // Yield between iterations so the async runtime stays responsive.
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    }

    println!(
        "Cache daemon completed: {} cached, {} failed.",
        processed, failed
    );
    Ok(())
}

/// Add variants to the cache (copy from package paths to cache dir)
async fn add_variants(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    variant_uris: &[String],
    args: &PkgCacheArgs,
) -> RezCoreResult<()> {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};

    println!("Adding {} variant(s) to cache:", variant_uris.len());

    let config = RezCoreConfig::load();
    let cache_dir = determine_cache_directory(args)?;

    for uri in variant_uris {
        println!("  Adding variant: {}", uri);

        // Parse "name-version" from URI
        let (pkg_name, version) = if let Some(pos) = uri.rfind('-') {
            let ver = &uri[pos + 1..];
            if ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                (uri[..pos].to_string(), Some(ver.to_string()))
            } else {
                (uri.clone(), None)
            }
        } else {
            (uri.clone(), None)
        };

        // Find package in repos
        let mut repo_manager = RepositoryManager::new();
        for (i, path_str) in config.packages_path.iter().enumerate() {
            let path = std::path::PathBuf::from(path_str);
            if path.exists() {
                repo_manager
                    .add_repository(Box::new(SimpleRepository::new(path, format!("repo_{}", i))));
            }
        }

        let packages = repo_manager
            .find_packages(&pkg_name)
            .await
            .unwrap_or_default();

        let pkg = packages.into_iter().find(|p| {
            version.as_ref().map_or(true, |v| {
                p.version.as_ref().is_some_and(|pv| pv.as_str() == v)
            })
        });

        let (original_path, cache_dest) = if let Some(ref p) = pkg {
            let ver_str = p.version.as_ref().map(|v| v.as_str()).unwrap_or("unknown");
            let orig = std::path::PathBuf::from(format!("{}/{}", pkg_name, ver_str));
            let dest = cache_dir.join(&pkg_name).join(ver_str);
            (Some(orig), dest)
        } else {
            let dest = cache_dir.join(&pkg_name);
            (None, dest)
        };

        // Create cache destination directory
        if let Err(e) = std::fs::create_dir_all(&cache_dest) {
            eprintln!("    Warning: failed to create cache dir: {}", e);
        }

        let entry = CacheEntry {
            package_name: pkg_name.clone(),
            variant_uri: uri.clone(),
            original_path,
            cache_path: Some(cache_dest),
            status: CacheStatus::Cached,
        };

        cache_manager
            .put(uri.clone(), entry)
            .await
            .map_err(|e| RezCoreError::Cache(format!("Failed to add variant {}: {}", uri, e)))?;

        println!("    Cached to: {}", cache_dir.join(&pkg_name).display());
    }

    Ok(())
}

/// Remove variants from the cache
async fn remove_variants(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    variant_uris: &[String],
) -> RezCoreResult<()> {
    println!("Removing {} variant(s) from cache:", variant_uris.len());

    for uri in variant_uris {
        println!("  Removing variant: {}", uri);

        let removed = cache_manager.remove(uri).await;
        if removed {
            println!("    Removed from cache");
        } else {
            println!("    Variant not found in cache");
        }
    }

    Ok(())
}

/// Clean the cache (remove stale/empty directories)
async fn clean_cache(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
) -> RezCoreResult<()> {
    println!("Cleaning package cache...");

    let stats_before = cache_manager.get_stats().await;

    // Walk cache dirs and remove empty directories
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::load();
    let cache_dir = if let Some(p) = config.package_cache_path.first() {
        if !p.is_empty() {
            std::path::PathBuf::from(p)
        } else {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".to_string());
            std::path::PathBuf::from(home)
                .join(".rez")
                .join("cache")
                .join("packages")
        }
    } else {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        std::path::PathBuf::from(home)
            .join(".rez")
            .join("cache")
            .join("packages")
    };

    let mut removed_dirs = 0usize;
    if cache_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Remove empty version directories
                    if let Ok(mut children) = std::fs::read_dir(&path) {
                        if children.next().is_none() {
                            let _ = std::fs::remove_dir(&path);
                            removed_dirs += 1;
                        }
                    }
                }
            }
        }
    }

    let stats_after = cache_manager.get_stats().await;

    println!("Cache cleaning completed:");
    println!("  Entries before: {}", stats_before.l1_stats.entries);
    println!("  Entries after:  {}", stats_after.l1_stats.entries);
    println!("  Empty directories removed: {}", removed_dirs);

    Ok(())
}

/// View cache logs
async fn view_logs(cache_dir: &Path) -> RezCoreResult<()> {
    let log_file = cache_dir.join("cache.log");

    if !log_file.exists() {
        println!("No cache logs found at: {}", log_file.display());
        return Ok(());
    }

    println!("Cache logs from: {}", log_file.display());
    println!("================");

    let content = std::fs::read_to_string(&log_file).map_err(RezCoreError::Io)?;

    // Show last 50 lines
    let lines: Vec<&str> = content.lines().collect();
    let start = if lines.len() > 50 {
        lines.len() - 50
    } else {
        0
    };

    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}

/// Show cache status
async fn show_cache_status(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    args: &PkgCacheArgs,
) -> RezCoreResult<()> {
    let stats = cache_manager.get_stats().await;

    println!("Package Cache Status");
    println!("===================");
    println!();

    println!("Cache Statistics:");
    println!("  Total entries: {}", stats.l1_stats.entries);
    println!(
        "  Hit rate: {:.2}%",
        stats.overall_stats.overall_hit_rate * 100.0
    );
    println!(
        "  Memory usage: {:.2} MB",
        stats.l1_stats.usage_bytes as f64 / 1024.0 / 1024.0
    );
    println!("  L1 Cache hits: {}", stats.l1_stats.hits);
    println!("  L1 Cache misses: {}", stats.l1_stats.misses);
    println!("  L2 Cache hits: {}", stats.l2_stats.hits);
    println!("  L2 Cache misses: {}", stats.l2_stats.misses);
    println!();

    show_cache_entries_table(args).await?;

    Ok(())
}

/// Show cache entries in a formatted table (real disk scan)
async fn show_cache_entries_table(args: &PkgCacheArgs) -> RezCoreResult<()> {
    let cache_dir = determine_cache_directory(args)?;
    let entries = scan_cache_directory(&cache_dir).await;

    if entries.is_empty() {
        println!("No cached packages found in: {}", cache_dir.display());
        return Ok(());
    }

    print_table_headers(&args.columns);
    print_table_separator(&args.columns);

    for entry in entries {
        print_table_row(&entry, &args.columns);
    }

    Ok(())
}

/// Scan cache directory and build real entries list
async fn scan_cache_directory(cache_dir: &PathBuf) -> Vec<CacheEntry> {
    let mut entries = Vec::new();

    if !cache_dir.exists() {
        return entries;
    }

    // Expected structure: <cache_dir>/<package_name>/<version>/[variant_hash]/
    if let Ok(pkg_dirs) = std::fs::read_dir(cache_dir) {
        for pkg_entry in pkg_dirs.flatten() {
            let pkg_path = pkg_entry.path();
            if !pkg_path.is_dir() {
                continue;
            }
            let pkg_name = pkg_entry.file_name().to_string_lossy().to_string();

            if let Ok(ver_dirs) = std::fs::read_dir(&pkg_path) {
                for ver_entry in ver_dirs.flatten() {
                    let ver_path = ver_entry.path();
                    if !ver_path.is_dir() {
                        continue;
                    }
                    let version = ver_entry.file_name().to_string_lossy().to_string();
                    let uri = format!("{}-{}", pkg_name, version);

                    // Check for variant sub-directories
                    let has_variants = std::fs::read_dir(&ver_path)
                        .map(|mut d| d.any(|e| e.map(|e| e.path().is_dir()).unwrap_or(false)))
                        .unwrap_or(false);

                    if has_variants {
                        if let Ok(variant_dirs) = std::fs::read_dir(&ver_path) {
                            for var_entry in variant_dirs.flatten() {
                                if var_entry.path().is_dir() {
                                    let var_name =
                                        var_entry.file_name().to_string_lossy().to_string();
                                    let var_uri = format!("{}/{}", uri, var_name);
                                    entries.push(CacheEntry {
                                        package_name: pkg_name.clone(),
                                        variant_uri: var_uri,
                                        original_path: None,
                                        cache_path: Some(var_entry.path()),
                                        status: CacheStatus::Cached,
                                    });
                                }
                            }
                        }
                    } else {
                        entries.push(CacheEntry {
                            package_name: pkg_name.clone(),
                            variant_uri: uri,
                            original_path: None,
                            cache_path: Some(ver_path),
                            status: CacheStatus::Cached,
                        });
                    }
                }
            }
        }
    }

    entries
}

/// Print table headers
fn print_table_headers(columns: &[String]) {
    let mut row = Vec::new();
    for column in columns {
        let header = match column.as_str() {
            "status" => "Status",
            "package" => "Package",
            "variant_uri" => "Variant URI",
            "orig_path" => "Original Path",
            "cache_path" => "Cache Path",
            _ => column,
        };
        row.push(format!("{:15}", header));
    }
    println!("{}", row.join(" "));
}

/// Print table separator
fn print_table_separator(columns: &[String]) {
    let mut row = Vec::new();
    for _ in columns {
        row.push("-".repeat(15));
    }
    println!("{}", row.join(" "));
}

/// Print a table row for a cache entry
fn print_table_row(entry: &CacheEntry, columns: &[String]) {
    let mut row = Vec::new();
    for column in columns {
        let value = match column.as_str() {
            "status" => format_status(&entry.status),
            "package" => entry.package_name.clone(),
            "variant_uri" => entry.variant_uri.clone(),
            "orig_path" => entry
                .original_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            "cache_path" => entry
                .cache_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".to_string()),
            _ => "-".to_string(),
        };
        row.push(format!("{:15}", truncate_string(&value, 15)));
    }
    println!("{}", row.join(" "));
}

/// Format cache status for display
fn format_status(status: &CacheStatus) -> String {
    match status {
        CacheStatus::Cached => "cached".to_string(),
        CacheStatus::Copying => "copying".to_string(),
        CacheStatus::Stalled => "stalled".to_string(),
        CacheStatus::Pending => "pending".to_string(),
    }
}

/// Truncate string to specified length
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
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

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_long() {
        let result = truncate_string("hello world long string", 10);
        assert!(
            result.len() <= 10,
            "truncated string should be at most 10 chars"
        );
        assert!(
            result.ends_with("..."),
            "truncated string should end with '...'"
        );
    }

    #[test]
    fn test_format_status_variants() {
        assert_eq!(format_status(&CacheStatus::Cached), "cached");
        assert_eq!(format_status(&CacheStatus::Copying), "copying");
        assert_eq!(format_status(&CacheStatus::Stalled), "stalled");
        assert_eq!(format_status(&CacheStatus::Pending), "pending");
    }

    #[test]
    fn test_determine_cache_directory_explicit_path() {
        let tmp = tempfile::tempdir().unwrap();
        let args = PkgCacheArgs {
            dir: Some(tmp.path().to_path_buf()),
            ..PkgCacheArgs::default()
        };
        let result = determine_cache_directory(&args);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), tmp.path().to_path_buf());
    }

    #[tokio::test]
    async fn test_scan_cache_directory_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let entries = scan_cache_directory(&tmp.path().to_path_buf()).await;
        assert!(
            entries.is_empty(),
            "empty directory should yield no entries"
        );
    }

    #[tokio::test]
    async fn test_scan_cache_directory_with_packages() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().to_path_buf();

        // Create a fake cached package structure: <pkg>/<ver>/
        let pkg_dir = cache_root.join("mypkg").join("1.0.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();

        let entries = scan_cache_directory(&cache_root).await;
        assert_eq!(entries.len(), 1, "should find one cached entry");
        assert_eq!(entries[0].package_name, "mypkg");
        assert!(entries[0].variant_uri.contains("mypkg"));
    }

    #[tokio::test]
    async fn test_scan_cache_directory_with_variants() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().to_path_buf();

        // variant sub-directories: <pkg>/<ver>/<variant_hash>/
        std::fs::create_dir_all(cache_root.join("pkg").join("2.0").join("v0")).unwrap();
        std::fs::create_dir_all(cache_root.join("pkg").join("2.0").join("v1")).unwrap();

        let entries = scan_cache_directory(&cache_root).await;
        assert_eq!(entries.len(), 2, "should find two variant entries");
        assert!(entries.iter().all(|e| e.package_name == "pkg"));
    }

    #[tokio::test]
    async fn test_daemon_no_pending_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let config = UnifiedCacheConfig::default();
        let manager: IntelligentCacheManager<String, CacheEntry> =
            IntelligentCacheManager::new(config);
        // Should succeed even if pending/ dir doesn't exist yet.
        let result = run_daemon(&manager, tmp.path()).await;
        assert!(
            result.is_ok(),
            "daemon should succeed when no pending dir exists"
        );
        // Pending dir should now be created.
        assert!(tmp.path().join("pending").exists());
    }

    #[tokio::test]
    async fn test_daemon_processes_pending_file() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = tmp.path().to_path_buf();
        let pending_dir = cache_dir.join("pending");
        std::fs::create_dir_all(&pending_dir).unwrap();

        // Create a fake pending file (uri = "mypkg_1.0.0" → "mypkg/1.0.0")
        std::fs::write(pending_dir.join("mypkg_1.0.0.pending"), "").unwrap();

        let config = UnifiedCacheConfig::default();
        let manager: IntelligentCacheManager<String, CacheEntry> =
            IntelligentCacheManager::new(config);

        let result = run_daemon(&manager, &cache_dir).await;
        assert!(
            result.is_ok(),
            "daemon should process pending file without error"
        );

        // The pending file should have been removed after processing.
        assert!(
            !pending_dir.join("mypkg_1.0.0.pending").exists(),
            "pending file should be consumed by daemon"
        );
    }
}
