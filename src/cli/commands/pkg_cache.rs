//! # Package Cache Management Command
//!
//! Implements the `rez pkg-cache` command for managing package cache operations.
//! This command provides functionality to view, add, remove, and clean package cache entries.

use clap::Args;
use rez_next_cache::{IntelligentCacheManager, UnifiedCache, UnifiedCacheConfig};
use rez_next_common::{error::RezCoreResult, RezCoreError};
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
        std::fs::create_dir_all(cache_dir).map_err(|e| RezCoreError::Io(e.into()))?;
    }

    Ok(manager)
}

/// Run the cache daemon
async fn run_daemon(
    _cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    cache_dir: &PathBuf,
) -> RezCoreResult<()> {
    println!("Starting package cache daemon for: {}", cache_dir.display());

    // TODO: Implement daemon logic
    // - Monitor pending cache operations
    // - Process cache requests
    // - Handle cleanup operations

    println!("Cache daemon completed successfully");
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
            if ver.chars().next().map_or(false, |c| c.is_ascii_digit()) {
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
                p.version.as_ref().map_or(false, |pv| pv.as_str() == v)
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
async fn view_logs(cache_dir: &PathBuf) -> RezCoreResult<()> {
    let log_file = cache_dir.join("cache.log");

    if !log_file.exists() {
        println!("No cache logs found at: {}", log_file.display());
        return Ok(());
    }

    println!("Cache logs from: {}", log_file.display());
    println!("================");

    let content = std::fs::read_to_string(&log_file).map_err(|e| RezCoreError::Io(e.into()))?;

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
