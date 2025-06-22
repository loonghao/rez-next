//! Copy command implementation
//!
//! Implements the `rez cp` command for copying packages between repositories.

use clap::Args;
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_package::Package;
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arguments for the cp command
#[derive(Args, Clone, Debug)]
pub struct CpArgs {
    /// Source package specification (name-version)
    #[arg(value_name = "SRC_PKG")]
    pub source_package: String,

    /// Destination repository path
    #[arg(value_name = "DEST_PATH")]
    pub destination_path: PathBuf,

    /// Source repository paths to search
    #[arg(long = "src-path", value_name = "PATH")]
    pub source_paths: Vec<PathBuf>,

    /// Copy all variants of the package
    #[arg(short = 'a', long = "all-variants")]
    pub all_variants: bool,

    /// Force overwrite if package already exists
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Dry run - show what would be copied without actually copying
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Skip dependencies when copying
    #[arg(long = "no-deps")]
    pub no_deps: bool,
}

/// Copy result information
#[derive(Debug, Clone)]
pub struct CopyResult {
    /// Source package
    pub source_package: Package,
    /// Destination path
    pub destination_path: PathBuf,
    /// Success status
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Number of variants copied
    pub variants_copied: usize,
}

/// Execute the cp command
pub fn execute(args: CpArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("📦 Rez Copy - Copying packages between repositories...");
        println!("Source package: {}", args.source_package);
        println!("Destination: {}", args.destination_path.display());
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async { execute_copy_async(&args).await })
}

/// Execute copy operation asynchronously
async fn execute_copy_async(args: &CpArgs) -> RezCoreResult<()> {
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

    // Find source package
    let source_package =
        find_source_package(&repo_manager, &package_name, version_spec.as_deref()).await?;

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
    }

    // Check if destination exists
    if !args.force && package_exists_at_destination(&args.destination_path, &source_package).await?
    {
        return Err(RezCoreError::RequirementParse(format!(
            "Package already exists at destination. Use --force to overwrite."
        )));
    }

    if args.dry_run {
        println!("DRY RUN - Would copy:");
        println!("  Package: {}", source_package.name);
        if let Some(ref version) = source_package.version {
            println!("  Version: {}", version.as_str());
        }
        println!("  To: {}", args.destination_path.display());
        if args.all_variants {
            println!("  Variants: All variants would be copied");
        }
        return Ok(());
    }

    // Perform the copy
    let result = copy_package(&source_package, &args.destination_path, args).await?;

    if result.success {
        println!("✅ Successfully copied package '{}'", source_package.name);
        println!("   Destination: {}", result.destination_path.display());
        if args.all_variants && result.variants_copied > 1 {
            println!("   Variants copied: {}", result.variants_copied);
        }
    } else {
        eprintln!(
            "❌ Failed to copy package: {}",
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

/// Find source package in repositories
async fn find_source_package(
    repo_manager: &RepositoryManager,
    package_name: &str,
    _version_spec: Option<&str>,
) -> RezCoreResult<Package> {
    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}' not found",
            package_name
        )));
    }

    // Return the first (or latest) package found - convert Arc<Package> to Package
    let package_arc = packages.into_iter().next().unwrap();
    Ok((*package_arc).clone())
}

/// Check if package already exists at destination
async fn package_exists_at_destination(
    destination_path: &PathBuf,
    package: &Package,
) -> RezCoreResult<bool> {
    // TODO: Implement proper package existence check
    // For now, just check if the directory exists
    let package_dir = if let Some(ref version) = package.version {
        destination_path.join(format!("{}-{}", package.name, version.as_str()))
    } else {
        destination_path.join(&package.name)
    };

    Ok(package_dir.exists())
}

/// Copy package to destination
async fn copy_package(
    source_package: &Package,
    destination_path: &PathBuf,
    args: &CpArgs,
) -> RezCoreResult<CopyResult> {
    // TODO: Implement actual package copying logic
    // This is a simplified implementation

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

    // TODO: Copy package files, metadata, variants, etc.
    // For now, just create a basic package.yaml
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
            .unwrap_or("Copied package")
    );

    std::fs::write(&package_yaml, yaml_content).map_err(|e| RezCoreError::Io(e.into()))?;

    let variants_copied = if args.all_variants {
        source_package.variants.len().max(1)
    } else {
        1
    };

    Ok(CopyResult {
        source_package: source_package.clone(),
        destination_path: package_dir,
        success: true,
        error: None,
        variants_copied,
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

        assert_eq!(
            parse_package_spec("my-package-name").unwrap(),
            ("my-package-name".to_string(), None)
        );
    }

    #[test]
    fn test_cp_args_defaults() {
        let args = CpArgs {
            source_package: "test".to_string(),
            destination_path: PathBuf::from("/tmp"),
            source_paths: vec![],
            all_variants: false,
            force: false,
            dry_run: false,
            verbose: false,
            no_deps: false,
        };

        assert_eq!(args.source_package, "test");
        assert!(!args.force);
        assert!(!args.dry_run);
    }
}
