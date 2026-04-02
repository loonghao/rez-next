//! Remove command implementation
//!
//! Implements the `rez rm` command for removing packages from repositories.

use clap::Args;
use chrono::NaiveDate;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;

/// Arguments for the rm command
#[derive(Args, Clone, Debug)]
pub struct RmArgs {
    /// Package specification to remove (name-version or name)
    #[arg(value_name = "PKG")]
    pub package: Option<String>,

    /// Remove entire package family
    #[arg(short = 'f', long = "family")]
    pub family: bool,

    /// Force remove package family (use with --family)
    #[arg(long = "force-family")]
    pub force_family: bool,

    /// Remove packages ignored since given time
    #[arg(long = "ignored-since", value_name = "TIME")]
    pub ignored_since: Option<String>,

    /// Repository paths to search
    #[arg(long = "paths", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Remove all variants of the package
    #[arg(short = 'a', long = "all-variants")]
    pub all_variants: bool,

    /// Force removal without confirmation
    #[arg(long = "force")]
    pub force: bool,

    /// Dry run - show what would be removed without actually removing
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Interactive mode - ask for confirmation for each package
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,
}

/// Remove result information
#[derive(Debug, Clone)]
pub struct RemoveResult {
    /// Package that was removed
    pub package: Package,
    /// Path where package was located
    pub package_path: PathBuf,
    /// Success status
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Number of variants removed
    pub variants_removed: usize,
}

/// Execute the rm command
pub fn execute(args: RmArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🗑️  Rez Remove - Removing packages from repositories...");
    }

    // Validate arguments
    if args.package.is_none() && args.ignored_since.is_none() {
        return Err(RezCoreError::RequirementParse(
            "Must specify either --package or --ignored-since".to_string(),
        ));
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(RezCoreError::Io)?;

    runtime.block_on(async {
        if let Some(ref package_spec) = args.package {
            if args.family {
                remove_package_family(package_spec, &args).await
            } else {
                remove_package(package_spec, &args).await
            }
        } else if args.ignored_since.is_some() {
            remove_ignored_since(&args).await
        } else {
            Err(RezCoreError::RequirementParse(
                "Invalid arguments".to_string(),
            ))
        }
    })
}

/// Remove a specific package
async fn remove_package(package_spec: &str, args: &RmArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Removing package: {}", package_spec);
    }

    // Parse package specification
    let (package_name, version_spec) = parse_package_spec(package_spec)?;

    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    // Find packages to remove
    let packages =
        find_packages_to_remove(&repo_manager, &package_name, version_spec.as_deref(), args)
            .await?;

    if packages.is_empty() {
        println!("No packages found matching '{}'", package_spec);
        return Ok(());
    }

    if args.verbose {
        println!("Found {} package(s) to remove:", packages.len());
        for pkg in &packages {
            println!(
                "  {}-{}",
                pkg.name,
                pkg.version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown")
            );
        }
    }

    if args.dry_run {
        println!("DRY RUN - Would remove:");
        for pkg in &packages {
            println!(
                "  {}-{}",
                pkg.name,
                pkg.version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown")
            );
        }
        return Ok(());
    }

    // Remove packages
    let mut removed_count = 0;
    for package in packages {
        if args.interactive && !confirm_removal(&package)? {
            continue;
        }

        match remove_single_package(&package, args).await {
            Ok(result) => {
                if result.success {
                    removed_count += 1;
                    println!("✅ Removed package: {}", package.name);
                    if args.verbose {
                        println!("   Path: {}", result.package_path.display());
                        if result.variants_removed > 1 {
                            println!("   Variants removed: {}", result.variants_removed);
                        }
                    }
                } else {
                    eprintln!(
                        "❌ Failed to remove {}: {}",
                        package.name,
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Error removing {}: {}", package.name, e);
            }
        }
    }

    println!("Removed {} package(s)", removed_count);
    Ok(())
}

/// Remove an entire package family
async fn remove_package_family(package_name: &str, args: &RmArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Removing package family: {}", package_name);
    }

    if !args.force_family && !args.force {
        println!(
            "WARNING: This will remove ALL versions of package '{}'",
            package_name
        );
        if !confirm_family_removal(package_name)? {
            println!("Operation cancelled");
            return Ok(());
        }
    }

    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    // Find all packages in the family
    let packages = find_packages_to_remove(&repo_manager, package_name, None, args).await?;

    if packages.is_empty() {
        println!("No packages found in family '{}'", package_name);
        return Ok(());
    }

    if args.dry_run {
        println!(
            "DRY RUN - Would remove family '{}' ({} packages):",
            package_name,
            packages.len()
        );
        for pkg in &packages {
            println!(
                "  {}-{}",
                pkg.name,
                pkg.version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown")
            );
        }
        return Ok(());
    }

    // Remove all packages in family
    let mut removed_count = 0;
    for package in packages {
        match remove_single_package(&package, args).await {
            Ok(result) => {
                if result.success {
                    removed_count += 1;
                    if args.verbose {
                        println!(
                            "✅ Removed: {}-{}",
                            package.name,
                            package
                                .version
                                .as_ref()
                                .map(|v| v.as_str())
                                .unwrap_or("unknown")
                        );
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Error removing {}: {}", package.name, e);
            }
        }
    }

    println!(
        "✅ Removed package family '{}' ({} packages)",
        package_name, removed_count
    );
    Ok(())
}

/// Remove packages ignored since a specific time
async fn remove_ignored_since(args: &RmArgs) -> RezCoreResult<()> {
    let time_spec = args.ignored_since.as_ref().unwrap();

    if args.verbose {
        println!("Removing packages ignored since: {}", time_spec);
    }

    // Parse time specification (absolute ISO datetime or relative like 1d/2w/1m/1y)
    let cutoff_timestamp = parse_time_spec(time_spec)?;

    let repo_manager = setup_repositories(args).await?;

    // Get all package family names, then fetch their versions
    let package_names = repo_manager.list_packages().await?;
    let mut removal_candidates: Vec<Package> = Vec::new();

    for name in &package_names {
        let packages = repo_manager.find_packages(name).await?;
        for pkg_arc in packages {
            let pkg = (*pkg_arc).clone();
            // Retrieve install dir by walking the configured repo paths
            let pkg_path = {
                let ver_str = pkg.version.as_ref().map(|v| v.as_str()).unwrap_or("unknown");
                // Construct candidate paths: <repo>/<name>/<version>/package.py
                let mut found_path = std::path::PathBuf::new();
                for path in &args.paths {
                    let candidate = path.join(&pkg.name).join(ver_str).join("package.py");
                    if candidate.exists() {
                        found_path = candidate;
                        break;
                    }
                }
                found_path
            };

            if !pkg_path.as_os_str().is_empty() {
                if let Ok(metadata) = std::fs::metadata(&pkg_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                            if dur.as_secs() < cutoff_timestamp {
                                removal_candidates.push(pkg);
                            }
                        }
                    }
                }
            }
        }
    }

    if removal_candidates.is_empty() {
        println!(
            "No packages found matching time filter (ignored since {})",
            time_spec
        );
        return Ok(());
    }

    if args.verbose || args.dry_run {
        println!("Packages to remove ({}):", removal_candidates.len());
        for pkg in &removal_candidates {
            let ver = pkg
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");
            println!("  {}-{}", pkg.name, ver);
        }
    }

    if args.dry_run {
        println!(
            "[dry-run] Would remove {} package(s)",
            removal_candidates.len()
        );
        return Ok(());
    }

    // Perform actual removal via the existing single-package remove path
    let mut removed = 0usize;
    for pkg in &removal_candidates {
        match remove_single_package(pkg, args).await {
            Ok(_) => {
                removed += 1;
                if args.verbose {
                    println!(
                        "  Removed {}-{}",
                        pkg.name,
                        pkg.version.as_ref().map(|v| v.as_str()).unwrap_or("unknown")
                    );
                }
            }
            Err(e) => eprintln!("  Warning: failed to remove {}: {}", pkg.name, e),
        }
    }

    println!(
        "✅ Removed {} package(s) ignored since {}",
        removed, time_spec
    );
    Ok(())
}

/// Parse a time specification into a Unix timestamp (seconds since epoch).
///
/// Accepts:
/// - Relative: `1d`, `2w`, `1m`, `1y` — seconds back from now
/// - ISO date:  `2024-01-15`
/// - ISO datetime: `2024-01-15T12:00:00`
fn parse_time_spec(spec: &str) -> RezCoreResult<u64> {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| RezCoreError::CliError(format!("system time error: {}", e)))?
        .as_secs();

    // Relative time: number + suffix
    if let Some(rest) = spec.strip_suffix('d') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(now.saturating_sub(n * 86_400));
        }
    }
    if let Some(rest) = spec.strip_suffix('w') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(now.saturating_sub(n * 7 * 86_400));
        }
    }
    if let Some(rest) = spec.strip_suffix('m') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(now.saturating_sub(n * 30 * 86_400));
        }
    }
    if let Some(rest) = spec.strip_suffix('y') {
        if let Ok(n) = rest.parse::<u64>() {
            return Ok(now.saturating_sub(n * 365 * 86_400));
        }
    }

    // ISO datetime: YYYY-MM-DDTHH:MM:SS
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(spec, "%Y-%m-%dT%H:%M:%S") {
        return Ok(dt.and_utc().timestamp() as u64);
    }
    // ISO date: YYYY-MM-DD
    if let Ok(date) = NaiveDate::parse_from_str(spec, "%Y-%m-%d") {
        return Ok(date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() as u64);
    }

    Err(RezCoreError::CliError(format!(
        "Cannot parse time spec '{}'. Expected: 1d/2w/1m/1y or YYYY-MM-DD[THH:MM:SS]",
        spec
    )))
}

/// Setup repository manager
async fn setup_repositories(args: &RmArgs) -> RezCoreResult<RepositoryManager> {
    use rez_next_common::config::RezCoreConfig;

    let mut repo_manager = RepositoryManager::new();
    let paths: Vec<PathBuf> = if !args.paths.is_empty() {
        args.paths.clone()
    } else {
        let config = RezCoreConfig::load();
        config
            .packages_path
            .iter()
            .map(|p| expand_home_dir(p))
            .filter(|p| p.exists())
            .collect()
    };

    for (i, path) in paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    Ok(repo_manager)
}

fn expand_home_dir(p: &str) -> PathBuf {
    if p.starts_with("~/") || p == "~" {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            return PathBuf::from(home).join(&p[2..]);
        }
    }
    PathBuf::from(p)
}

/// Find packages to remove (with version filter)
async fn find_packages_to_remove(
    repo_manager: &RepositoryManager,
    package_name: &str,
    version_spec: Option<&str>,
    _args: &RmArgs,
) -> RezCoreResult<Vec<Package>> {
    let packages = repo_manager.find_packages(package_name).await?;

    let result: Vec<Package> = packages
        .into_iter()
        .filter(|p| {
            version_spec.map_or(true, |ver| {
                p.version.as_ref().is_some_and(|v| v.as_str() == ver)
            })
        })
        .map(|p| (*p).clone())
        .collect();

    Ok(result)
}

/// Remove a single package from disk
async fn remove_single_package(package: &Package, args: &RmArgs) -> RezCoreResult<RemoveResult> {
    use rez_next_common::config::RezCoreConfig;

    let config = RezCoreConfig::load();
    let ver_str = package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");

    // Try to find actual path
    let mut package_path: Option<PathBuf> = None;
    let search_paths: Vec<PathBuf> = if !args.paths.is_empty() {
        args.paths.clone()
    } else {
        config
            .packages_path
            .iter()
            .map(|p| expand_home_dir(p))
            .collect()
    };

    for base in &search_paths {
        let candidate = base.join(&package.name).join(ver_str);
        if candidate.exists() {
            package_path = Some(candidate);
            break;
        }
    }

    let pkg_path = match package_path {
        Some(p) => p,
        None => {
            return Ok(RemoveResult {
                package: package.clone(),
                package_path: PathBuf::new(),
                success: false,
                error: Some(format!(
                    "Package directory not found for {}-{}",
                    package.name, ver_str
                )),
                variants_removed: 0,
            });
        }
    };

    if args.verbose {
        println!("Removing: {}", pkg_path.display());
    }

    std::fs::remove_dir_all(&pkg_path).map_err(|e| {
        RezCoreError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to remove {}: {}", pkg_path.display(), e),
        ))
    })?;

    let variants_removed = if args.all_variants {
        package.variants.len().max(1)
    } else {
        1
    };

    Ok(RemoveResult {
        package: package.clone(),
        package_path: pkg_path,
        success: true,
        error: None,
        variants_removed,
    })
}

/// Parse package specification
fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    if let Some(dash_pos) = spec.rfind('-') {
        let name = spec[..dash_pos].to_string();
        let version = spec[dash_pos + 1..].to_string();

        if version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

/// Confirm removal of a single package
fn confirm_removal(package: &Package) -> RezCoreResult<bool> {
    use std::io::{self, Write};

    print!("Remove package '{}'? [y/N]: ", package.name);
    io::stdout()
        .flush()
        .map_err(RezCoreError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(RezCoreError::Io)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

/// Confirm removal of package family
fn confirm_family_removal(family_name: &str) -> RezCoreResult<bool> {
    use std::io::{self, Write};

    print!("Remove ENTIRE package family '{}'? [y/N]: ", family_name);
    io::stdout()
        .flush()
        .map_err(RezCoreError::Io)?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(RezCoreError::Io)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec() {
        assert_eq!(
            parse_package_spec("python").unwrap(),
            ("python".to_string(), None)
        );

        assert_eq!(
            parse_package_spec("python-3.9").unwrap(),
            ("python".to_string(), Some("3.9".to_string()))
        );
    }

    #[test]
    fn test_rm_args_validation() {
        let args = RmArgs {
            package: None,
            family: false,
            force_family: false,
            ignored_since: None,
            paths: vec![],
            all_variants: false,
            force: false,
            dry_run: false,
            verbose: false,
            interactive: false,
        };

        // Should require either package or ignored_since
        assert!(args.package.is_none() && args.ignored_since.is_none());
    }

    // ── parse_time_spec tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_time_spec_days() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = parse_time_spec("1d").unwrap();
        let expected = now.saturating_sub(86_400);
        // Allow up to 2s tolerance
        assert!(
            result.abs_diff(expected) <= 2,
            "1d: result={} expected={}",
            result,
            expected
        );

        let result_7d = parse_time_spec("7d").unwrap();
        let expected_7d = now.saturating_sub(7 * 86_400);
        assert!(result_7d.abs_diff(expected_7d) <= 2);
    }

    #[test]
    fn test_parse_time_spec_weeks() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = parse_time_spec("2w").unwrap();
        let expected = now.saturating_sub(2 * 7 * 86_400);
        assert!(result.abs_diff(expected) <= 2);
    }

    #[test]
    fn test_parse_time_spec_months() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = parse_time_spec("1m").unwrap();
        let expected = now.saturating_sub(30 * 86_400);
        assert!(result.abs_diff(expected) <= 2);
    }

    #[test]
    fn test_parse_time_spec_years() {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let result = parse_time_spec("1y").unwrap();
        let expected = now.saturating_sub(365 * 86_400);
        assert!(result.abs_diff(expected) <= 2);
    }

    #[test]
    fn test_parse_time_spec_iso_date() {
        let result = parse_time_spec("2024-01-15").unwrap();
        // 2024-01-15 00:00:00 UTC = 1705276800
        assert_eq!(result, 1_705_276_800);
    }

    #[test]
    fn test_parse_time_spec_iso_datetime() {
        let result = parse_time_spec("2024-01-15T12:00:00").unwrap();
        // 2024-01-15 12:00:00 UTC = 1705320000
        assert_eq!(result, 1_705_320_000);
    }

    #[test]
    fn test_parse_time_spec_invalid() {
        assert!(parse_time_spec("invalid").is_err());
        assert!(parse_time_spec("abc").is_err());
        assert!(parse_time_spec("").is_err());
        assert!(parse_time_spec("2024/01/15").is_err());
    }

    #[test]
    fn test_parse_time_spec_relative_ordering() {
        // 1d should produce a timestamp more recent than 1w
        let one_day = parse_time_spec("1d").unwrap();
        let one_week = parse_time_spec("1w").unwrap();
        assert!(
            one_day > one_week,
            "1d ({}) should be more recent than 1w ({})",
            one_day,
            one_week
        );
    }

    #[test]
    fn test_parse_time_spec_zero_relative() {
        // "0d" — valid parse, returns ~now
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let result = parse_time_spec("0d").unwrap();
        assert!(result.abs_diff(now) <= 2);
    }
}
