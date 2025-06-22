//! Status command implementation
//!
//! Implements the `rez status` command for displaying package and repository status.

use clap::Args;
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arguments for the status command
#[derive(Args, Clone, Debug)]
pub struct StatusArgs {
    /// Show status for specific package
    #[arg(value_name = "PKG")]
    pub package: Option<String>,

    /// Repository paths to check
    #[arg(long = "paths", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Show detailed information
    #[arg(short = 'd', long = "detailed")]
    pub detailed: bool,

    /// Show repository statistics
    #[arg(short = 'r', long = "repos")]
    pub repos: bool,

    /// Show package counts by family
    #[arg(short = 'f', long = "families")]
    pub families: bool,

    /// Show recent packages (last N days)
    #[arg(long = "recent", value_name = "DAYS")]
    pub recent: Option<u32>,

    /// Show packages with issues
    #[arg(long = "issues")]
    pub issues: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Repository status information
#[derive(Debug, Clone)]
pub struct RepositoryStatus {
    pub path: PathBuf,
    pub package_count: usize,
    pub family_count: usize,
    pub total_size: u64,
    pub accessible: bool,
    pub error: Option<String>,
}

/// Package family status
#[derive(Debug, Clone)]
pub struct FamilyStatus {
    pub name: String,
    pub version_count: usize,
    pub latest_version: Option<String>,
    pub total_variants: usize,
}

/// Execute the status command
pub fn execute(args: StatusArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("📊 Rez Status - Analyzing package and repository status...");
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async { execute_status_async(&args).await })
}

/// Execute status operation asynchronously
async fn execute_status_async(args: &StatusArgs) -> RezCoreResult<()> {
    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    if let Some(ref package_name) = args.package {
        // Show status for specific package
        show_package_status(&repo_manager, package_name, args).await?;
    } else if args.repos {
        // Show repository status
        show_repository_status(&repo_manager, args).await?;
    } else if args.families {
        // Show package family status
        show_family_status(&repo_manager, args).await?;
    } else {
        // Show general status overview
        show_general_status(&repo_manager, args).await?;
    }

    Ok(())
}

/// Setup repository manager
async fn setup_repositories(args: &StatusArgs) -> RezCoreResult<RepositoryManager> {
    let mut repo_manager = RepositoryManager::new();
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from("./local_packages")]
    } else {
        args.paths.clone()
    };

    for (i, path) in paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    Ok(repo_manager)
}

/// Show status for a specific package
async fn show_package_status(
    repo_manager: &RepositoryManager,
    package_name: &str,
    args: &StatusArgs,
) -> RezCoreResult<()> {
    if args.verbose {
        println!("Checking status for package: {}", package_name);
    }

    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        println!("Package '{}' not found", package_name);
        return Ok(());
    }

    println!("Package: {}", package_name);
    println!("Versions found: {}", packages.len());
    println!();

    for (i, package) in packages.iter().enumerate() {
        println!(
            "Version {}: {}",
            i + 1,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );

        if args.detailed {
            if let Some(ref desc) = package.description {
                println!("  Description: {}", desc);
            }
            if !package.requires.is_empty() {
                println!("  Requires: {}", package.requires.join(", "));
            }
            if !package.variants.is_empty() {
                println!("  Variants: {}", package.variants.len());
            }
            println!();
        }
    }

    Ok(())
}

/// Show repository status
async fn show_repository_status(
    _repo_manager: &RepositoryManager,
    args: &StatusArgs,
) -> RezCoreResult<()> {
    println!("Repository Status");
    println!("================");
    println!();

    let paths = if args.paths.is_empty() {
        vec![PathBuf::from("./local_packages")]
    } else {
        args.paths.clone()
    };

    for path in &paths {
        let status = analyze_repository_status(path).await?;

        println!("Repository: {}", status.path.display());
        if status.accessible {
            println!("  Status: ✅ Accessible");
            println!("  Packages: {}", status.package_count);
            println!("  Families: {}", status.family_count);
            if args.detailed {
                println!("  Total size: {} bytes", status.total_size);
            }
        } else {
            println!("  Status: ❌ Inaccessible");
            if let Some(ref error) = status.error {
                println!("  Error: {}", error);
            }
        }
        println!();
    }

    Ok(())
}

/// Show package family status
async fn show_family_status(
    repo_manager: &RepositoryManager,
    args: &StatusArgs,
) -> RezCoreResult<()> {
    println!("Package Family Status");
    println!("====================");
    println!();

    // Get all packages
    let packages = repo_manager.find_packages("").await?;

    // Group by family
    let mut families: HashMap<String, Vec<_>> = HashMap::new();
    for package in packages {
        families
            .entry(package.name.clone())
            .or_default()
            .push(package);
    }

    if families.is_empty() {
        println!("No packages found");
        return Ok(());
    }

    // Sort families by name
    let mut family_names: Vec<_> = families.keys().cloned().collect();
    family_names.sort();

    println!(
        "{:<20} {:<10} {:<15} {:<10}",
        "FAMILY", "VERSIONS", "LATEST", "VARIANTS"
    );
    println!(
        "{:<20} {:<10} {:<15} {:<10}",
        "------", "--------", "------", "--------"
    );

    for family_name in family_names {
        let packages = families.get(&family_name).unwrap();
        let version_count = packages.len();

        // Find latest version (simplified)
        let latest_version = packages
            .iter()
            .filter_map(|p| p.version.as_ref())
            .map(|v| v.as_str())
            .max()
            .unwrap_or("unknown");

        let total_variants: usize = packages.iter().map(|p| p.variants.len().max(1)).sum();

        println!(
            "{:<20} {:<10} {:<15} {:<10}",
            family_name, version_count, latest_version, total_variants
        );
    }

    println!();
    println!("Total families: {}", families.len());

    Ok(())
}

/// Show general status overview
async fn show_general_status(
    repo_manager: &RepositoryManager,
    args: &StatusArgs,
) -> RezCoreResult<()> {
    println!("Rez Status Overview");
    println!("==================");
    println!();

    // Get all packages
    let packages = repo_manager.find_packages("").await?;

    // Calculate statistics
    let total_packages = packages.len();
    let mut families: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_variants = 0;

    for package in &packages {
        families.insert(package.name.clone());
        total_variants += package.variants.len().max(1);
    }

    println!("📦 Package Statistics:");
    println!("  Total packages: {}", total_packages);
    println!("  Package families: {}", families.len());
    println!("  Total variants: {}", total_variants);
    println!();

    // Repository information
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from("./local_packages")]
    } else {
        args.paths.clone()
    };

    println!("📁 Repository Information:");
    for path in &paths {
        println!("  {}", path.display());
    }
    println!();

    if args.recent.is_some() {
        println!("📅 Recent packages: Feature not yet implemented");
        println!();
    }

    if args.issues {
        println!("⚠️  Package issues: Feature not yet implemented");
        println!();
    }

    if args.verbose {
        println!("✅ Status check completed");
    }

    Ok(())
}

/// Analyze repository status
async fn analyze_repository_status(path: &PathBuf) -> RezCoreResult<RepositoryStatus> {
    let accessible = path.exists() && path.is_dir();

    if !accessible {
        return Ok(RepositoryStatus {
            path: path.clone(),
            package_count: 0,
            family_count: 0,
            total_size: 0,
            accessible: false,
            error: Some("Directory does not exist or is not accessible".to_string()),
        });
    }

    // Count packages (simplified - count subdirectories)
    let mut package_count = 0;
    let mut total_size = 0;

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                package_count += 1;
            }
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }
    }

    Ok(RepositoryStatus {
        path: path.clone(),
        package_count,
        family_count: package_count, // Simplified
        total_size,
        accessible: true,
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_args_defaults() {
        let args = StatusArgs {
            package: None,
            paths: vec![],
            detailed: false,
            repos: false,
            families: false,
            recent: None,
            issues: false,
            verbose: false,
        };

        assert!(args.package.is_none());
        assert!(!args.detailed);
        assert!(!args.repos);
    }

    #[tokio::test]
    async fn test_analyze_repository_status() {
        let temp_dir = std::env::temp_dir();
        let status = analyze_repository_status(&temp_dir).await.unwrap();

        assert!(status.accessible);
        assert!(status.error.is_none());
    }
}
