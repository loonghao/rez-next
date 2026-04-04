//! Move command implementation
//!
//! Implements the `rez mv` command for moving packages between repositories.

use crate::cli::utils::expand_home_path;
use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;

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
    let runtime = tokio::runtime::Runtime::new().map_err(RezCoreError::Io)?;

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

    // Setup source repositories from config or provided paths
    let mut repo_manager = RepositoryManager::new();
    let source_paths: Vec<PathBuf> = if !args.source_paths.is_empty() {
        args.source_paths.clone()
    } else {
        use rez_next_common::config::RezCoreConfig;
        let config = RezCoreConfig::load();
        config
            .packages_path
            .iter()
            .map(|p| expand_home_path(p))
            .filter(|p| p.exists())
            .collect()
    };

    for (i, path) in source_paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    // Find source package and its location
    let (source_package, source_path) = find_source_package_with_path(
        &repo_manager,
        &package_name,
        version_spec.as_deref(),
        &source_paths,
    )
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

    // Determine destination directory (rez layout: dest/<name>/<version>/)
    let ver_str = source_package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");
    let dest_pkg_dir = args
        .destination_path
        .join(&source_package.name)
        .join(ver_str);

    // Check if destination exists
    if !args.force && dest_pkg_dir.exists() {
        return Err(RezCoreError::RequirementParse(
            "Package already exists at destination. Use --force to overwrite.".to_string(),
        ));
    }

    if args.dry_run {
        println!("DRY RUN - Would move:");
        println!("  Package: {}", source_package.name);
        if let Some(ref version) = source_package.version {
            println!("  Version: {}", version.as_str());
        }
        println!("  From: {}", source_path.display());
        println!("  To: {}", dest_pkg_dir.display());
        if args.all_variants {
            println!("  Variants: All variants would be moved");
        }
        if args.keep_source {
            println!("  Note: Source would be kept (copy mode)");
        }
        return Ok(());
    }

    // Perform the move
    let result = move_package_directory(&source_package, &source_path, &dest_pkg_dir, args).await?;

    if result.success {
        if args.keep_source {
            println!("Successfully copied package '{}'", source_package.name);
        } else {
            println!("Successfully moved package '{}'", source_package.name);
        }
        println!("   From: {}", result.source_path.display());
        println!("   To: {}", result.destination_path.display());
        if args.all_variants && result.variants_moved > 1 {
            println!("   Variants processed: {}", result.variants_moved);
        }
    } else {
        eprintln!(
            "Failed to move package: {}",
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
        if version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

/// Find source package and its path in repositories
async fn find_source_package_with_path(
    repo_manager: &RepositoryManager,
    package_name: &str,
    version_spec: Option<&str>,
    source_paths: &[PathBuf],
) -> RezCoreResult<(Package, PathBuf)> {
    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}' not found in any source repository",
            package_name
        )));
    }

    let pkg = if let Some(ver) = version_spec {
        packages
            .into_iter()
            .find(|p| p.version.as_ref().is_some_and(|v| v.as_str() == ver))
            .ok_or_else(|| {
                RezCoreError::RequirementParse(format!(
                    "Package '{}-{}' not found",
                    package_name, ver
                ))
            })?
    } else {
        let mut sorted = packages;
        sorted.sort_by(|a, b| {
            b.version
                .as_ref()
                .and_then(|bv| a.version.as_ref().map(|av| av.cmp(bv)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().next().unwrap()
    };

    let ver_str = pkg
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");
    for base_path in source_paths {
        let pkg_dir = base_path.join(&pkg.name).join(ver_str);
        if pkg_dir.exists() {
            return Ok(((*pkg).clone(), pkg_dir));
        }
        let pkg_dir2 = base_path.join(format!("{}-{}", pkg.name, ver_str));
        if pkg_dir2.exists() {
            return Ok(((*pkg).clone(), pkg_dir2));
        }
    }

    Err(RezCoreError::RequirementParse(format!(
        "Package '{}' found in index but filesystem directory not located",
        package_name
    )))
}

/// Move/copy package directory
async fn move_package_directory(
    source_package: &Package,
    source_path: &PathBuf,
    dest_root: &PathBuf,
    args: &MvArgs,
) -> RezCoreResult<MoveResult> {
    // Remove existing destination if force
    if args.force && dest_root.exists() {
        std::fs::remove_dir_all(dest_root).map_err(RezCoreError::Io)?;
    }

    // Copy source to destination
    copy_dir_recursive(source_path, dest_root)?;

    // Remove source unless keeping it
    if !args.keep_source {
        std::fs::remove_dir_all(source_path).map_err(|e| {
            RezCoreError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to remove source directory: {}", e),
            ))
        })?;
        if args.verbose {
            println!("Removed source: {}", source_path.display());
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
        destination_path: dest_root.clone(),
        success: true,
        error: None,
        variants_moved,
    })
}

fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> RezCoreResult<()> {
    std::fs::create_dir_all(dest).map_err(RezCoreError::Io)?;

    for entry in std::fs::read_dir(src).map_err(RezCoreError::Io)? {
        let entry = entry.map_err(RezCoreError::Io)?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(RezCoreError::Io)?;
        }
    }

    Ok(())
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
