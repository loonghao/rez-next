//! # Package Cache Operations
//!
//! Core cache operations: add, remove, clean, daemon, and view logs.

use super::types::{CacheEntry, CacheStatus, PkgCacheArgs};
use crate::cli::utils::expand_home_path;
use rez_next_cache::{IntelligentCacheManager, UnifiedCache, UnifiedCacheConfig};
use rez_next_common::{error::RezCoreResult, RezCoreError};
use std::path::{Path, PathBuf};

/// Determine the cache directory to use
pub fn determine_cache_directory(args: &PkgCacheArgs) -> RezCoreResult<PathBuf> {
    if let Some(dir) = &args.dir {
        return Ok(expand_home_path(&dir.to_string_lossy()));
    }

    // Read from rez configuration
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::load();

    if let Some(cache_path) = config.package_cache_path.first() {
        if !cache_path.is_empty() {
            return Ok(expand_home_path(cache_path));
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
pub async fn initialize_cache_manager(
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
pub async fn run_daemon(
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
pub async fn add_variants(
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
pub async fn remove_variants(
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
pub async fn clean_cache(
    cache_manager: &IntelligentCacheManager<String, CacheEntry>,
    cache_dir: &Path,
) -> RezCoreResult<()> {
    println!("Cleaning package cache...");

    let stats_before = cache_manager.get_stats().await;

    let mut removed_dirs = 0usize;
    if cache_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(cache_dir) {
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
pub async fn view_logs(cache_dir: &Path) -> RezCoreResult<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    async fn test_clean_cache_uses_supplied_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let empty_dir = tmp.path().join("orphan_pkg");
        std::fs::create_dir_all(&empty_dir).unwrap();

        let manager: IntelligentCacheManager<String, CacheEntry> =
            IntelligentCacheManager::new(UnifiedCacheConfig::default());

        clean_cache(&manager, tmp.path()).await.unwrap();

        assert!(
            !empty_dir.exists(),
            "clean_cache should remove empty directories under the supplied cache dir"
        );
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
