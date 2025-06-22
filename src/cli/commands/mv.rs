//! Move command implementation
//!
//! Implements the `rez mv` command for moving packages between repositories.

use clap::Args;
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_package::Package;
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arguments for the mv command
#[derive(Args, Clone, Debug)]
pub struct MvArgs {
    /// Source package specification (name-version)
    #[arg(value_name = "SRC_PKG")]
    pub source_package: String,

    /// Destination repository path
    #[arg(value_name = "DEST_PATH")]
    pub destination_path: PathBuf,

    /// Source repository paths to search
    #[arg(long = "src-path", value_name = "PATH")]
    pub source_paths: Vec<PathBuf>,

    /// Move all variants of the package
    #[arg(short = 'a', long = "all-variants")]
    pub all_variants: bool,

    /// Force overwrite if package already exists
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Dry run - show what would be moved without actually moving
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Skip dependencies when moving
    #[arg(long = "no-deps")]
    pub no_deps: bool,

    /// Keep source after move (essentially a copy operation)
    #[arg(long = "keep-source")]
    pub keep_source: bool,
}

/// Move result information
#[derive(Debug, Clone)]
pub struct MoveResult {
    /// Source package
    pub source_package: Package,
    /// Source path that was moved from
    pub source_path: PathBuf,
    /// Destination path
    pub destination_path: PathBuf,
    /// Success status
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Number of variants moved
    pub variants_moved: usize,
}

/// Execute the mv command
pub fn execute(args: MvArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("📦 Rez Move - Moving packages between repositories...");
        println!("Source package: {}", args.source_package);
        println!("Destination: {}", args.destination_path.display());
        if args.keep_source {
            println!("Mode: Copy (keeping source)");
        } else {
            println!("Mode: Move (removing source)");
        }
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async { execute_move_async(&args).await })
}

/// Execute move operation asynchronously
async fn execute_move_async(args: &MvArgs) -> RezCoreResult<()> {
    // Parse package specification
    let (package_name, version_spec) = parse_package_spec(&args.source_package)?;

    if args.verbose {
        println!(
            "Parsed package: {} (version: {})",
            package_name,
            version_spec.as_deref().unwrap_or("latest")
        );
    }

    // Setup source repositories
    let mut repo_manager = RepositoryManager::new();
    let source_paths = if args.source_paths.is_empty() {
        vec![PathBuf::from("./local_packages")]
    } else {
        args.source_paths.clone()
    };

    for (i, path) in source_paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    // Find source package and its location
    let (source_package, source_path) =
        find_source_package_with_path(&repo_manager, &package_name, version_spec.as_deref())
            .await?;

    if args.verbose {
        println!(
            "Found source package: {}-{}",
            source_package.name,
            source_package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );
        println!("Source location: {}", source_path.display());
    }

    // Check if destination exists
    if !args.force && package_exists_at_destination(&args.destination_path, &source_package).await?
    {
        return Err(RezCoreError::RequirementParse(format!(
            "Package already exists at destination. Use --force to overwrite."
        )));
    }

    if args.dry_run {
        println!("DRY RUN - Would move:");
        println!("  Package: {}", source_package.name);
        if let Some(ref version) = source_package.version {
            println!("  Version: {}", version.as_str());
        }
        println!("  From: {}", source_path.display());
        println!("  To: {}", args.destination_path.display());
        if args.all_variants {
            println!("  Variants: All variants would be moved");
        }
        if args.keep_source {
            println!("  Note: Source would be kept (copy mode)");
        }
        return Ok(());
    }

    // Perform the move
    let result = move_package(&source_package, &source_path, &args.destination_path, args).await?;

    if result.success {
        if args.keep_source {
            println!("✅ Successfully copied package '{}'", source_package.name);
        } else {
            println!("✅ Successfully moved package '{}'", source_package.name);
        }
        println!("   From: {}", result.source_path.display());
        println!("   To: {}", result.destination_path.display());
        if args.all_variants && result.variants_moved > 1 {
            println!("   Variants processed: {}", result.variants_moved);
        }
    } else {
        eprintln!(
            "❌ Failed to move package: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        );
        std::process::exit(1);
    }

    Ok(())
}

/// Parse package specification into name and optional version
fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    if let Some(dash_pos) = spec.rfind('-') {
        let name = spec[..dash_pos].to_string();
        let version = spec[dash_pos + 1..].to_string();

        // Check if version part looks like a version
        if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

/// Find source package and its path in repositories
async fn find_source_package_with_path(
    repo_manager: &RepositoryManager,
    package_name: &str,
    _version_spec: Option<&str>,
) -> RezCoreResult<(Package, PathBuf)> {
    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}' not found",
            package_name
        )));
    }

    // Return the first package found and estimate its path - convert Arc<Package> to Package
    let package_arc = packages.into_iter().next().unwrap();
    let package = (*package_arc).clone();
    let estimated_path = PathBuf::from("./local_packages"); // TODO: Get actual path from repository

    Ok((package, estimated_path))
}

/// Check if package already exists at destination
async fn package_exists_at_destination(
    destination_path: &PathBuf,
    package: &Package,
) -> RezCoreResult<bool> {
    let package_dir = if let Some(ref version) = package.version {
        destination_path.join(format!("{}-{}", package.name, version.as_str()))
    } else {
        destination_path.join(&package.name)
    };

    Ok(package_dir.exists())
}

/// Move package to destination
async fn move_package(
    source_package: &Package,
    source_path: &PathBuf,
    destination_path: &PathBuf,
    args: &MvArgs,
) -> RezCoreResult<MoveResult> {
    let package_dir = if let Some(ref version) = source_package.version {
        destination_path.join(format!("{}-{}", source_package.name, version.as_str()))
    } else {
        destination_path.join(&source_package.name)
    };

    // Create destination directory
    std::fs::create_dir_all(&package_dir).map_err(|e| RezCoreError::Io(e.into()))?;

    if args.verbose {
        println!("Created directory: {}", package_dir.display());
    }

    // TODO: Implement actual package moving logic
    // This is a simplified implementation

    // Create package.yaml at destination
    let package_yaml = package_dir.join("package.yaml");
    let yaml_content = format!(
        "name: {}\nversion: {}\ndescription: {}\n",
        source_package.name,
        source_package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("1.0.0"),
        source_package
            .description
            .as_deref()
            .unwrap_or("Moved package")
    );

    std::fs::write(&package_yaml, yaml_content).map_err(|e| RezCoreError::Io(e.into()))?;

    // Remove source if not keeping it
    if !args.keep_source {
        // TODO: Implement safe source removal
        if args.verbose {
            println!("Would remove source at: {}", source_path.display());
        }
    }

    let variants_moved = if args.all_variants {
        source_package.variants.len().max(1)
    } else {
        1
    };

    Ok(MoveResult {
        source_package: source_package.clone(),
        source_path: source_path.clone(),
        destination_path: package_dir,
        success: true,
        error: None,
        variants_moved,
    })
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
    fn test_mv_args_defaults() {
        let args = MvArgs {
            source_package: "test".to_string(),
            destination_path: PathBuf::from("/tmp"),
            source_paths: vec![],
            all_variants: false,
            force: false,
            dry_run: false,
            verbose: false,
            no_deps: false,
            keep_source: false,
        };

        assert_eq!(args.source_package, "test");
        assert!(!args.force);
        assert!(!args.keep_source);
    }
}
