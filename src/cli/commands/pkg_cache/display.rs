//! # Package Cache Display
//!
//! Table rendering and status formatting for the `rez pkg-cache` command.

use super::types::{CacheEntry, CacheStatus, PkgCacheArgs};
use rez_next_cache::{IntelligentCacheManager, UnifiedCache};
use rez_next_common::error::RezCoreResult;
use std::path::PathBuf;

/// Show cache status overview and table
pub async fn show_cache_status(
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
pub async fn show_cache_entries_table(args: &PkgCacheArgs) -> RezCoreResult<()> {
    use super::ops::determine_cache_directory;
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
pub async fn scan_cache_directory(cache_dir: &PathBuf) -> Vec<CacheEntry> {
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
pub fn format_status(status: &CacheStatus) -> String {
    match status {
        CacheStatus::Cached => "cached".to_string(),
        CacheStatus::Copying => "copying".to_string(),
        CacheStatus::Stalled => "stalled".to_string(),
        CacheStatus::Pending => "pending".to_string(),
    }
}

/// Truncate string to specified length
pub fn truncate_string(s: &str, max_len: usize) -> String {
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
}
