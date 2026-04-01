//! Copy command implementation
//!
//! Implements the `rez cp` command for copying packages between repositories.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;

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
            .map(|p| expand_home_dir(p))
            .filter(|p| p.exists())
            .collect()
    };

    for (i, path) in source_paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    // Find source package
    let (source_package, source_root) = find_source_package_with_path(
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
        println!("Source path: {}", source_root.display());
    }

    // Determine destination package directory (rez layout: dest/<name>/<version>/)
    let dest_pkg_dir = args.destination_path.join(&source_package.name).join(
        source_package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown"),
    );

    // Check if destination exists
    if !args.force && dest_pkg_dir.exists() {
        return Err(RezCoreError::RequirementParse(
            "Package already exists at destination. Use --force to overwrite.".to_string(),
        ));
    }

    if args.dry_run {
        println!("DRY RUN - Would copy:");
        println!("  Package: {}", source_package.name);
        if let Some(ref version) = source_package.version {
            println!("  Version: {}", version.as_str());
        }
        println!("  From: {}", source_root.display());
        println!("  To:   {}", dest_pkg_dir.display());
        return Ok(());
    }

    // Perform the copy
    let result = copy_package_directory(&source_root, &dest_pkg_dir, &source_package, args).await?;

    if result.success {
        println!("Successfully copied package '{}'", source_package.name);
        println!("   Destination: {}", result.destination_path.display());
        if args.all_variants && result.variants_copied > 1 {
            println!("   Variants copied: {}", result.variants_copied);
        }
    } else {
        eprintln!(
            "Failed to copy package: {}",
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        );
        std::process::exit(1);
    }

    Ok(())
}

/// Expand ~ in path
fn expand_home_dir(p: &str) -> PathBuf {
    if p.starts_with("~/") || p == "~" {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            return PathBuf::from(home).join(&p[2..]);
        }
    }
    PathBuf::from(p)
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

/// Find source package and its root directory
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
            .find(|p| p.version.as_ref().map_or(false, |v| v.as_str() == ver))
            .ok_or_else(|| {
                RezCoreError::RequirementParse(format!(
                    "Package '{}-{}' not found",
                    package_name, ver
                ))
            })?
    } else {
        // Latest version
        let mut sorted = packages;
        sorted.sort_by(|a, b| {
            b.version
                .as_ref()
                .and_then(|bv| a.version.as_ref().map(|av| av.cmp(bv)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().next().unwrap()
    };

    // Find the actual filesystem path
    let ver_str = pkg
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");
    for base_path in source_paths {
        // rez layout: <base>/<name>/<version>/
        let pkg_dir = base_path.join(&pkg.name).join(ver_str);
        if pkg_dir.exists() {
            return Ok(((*pkg).clone(), pkg_dir));
        }
        // Alternative: <base>/<name>-<version>/
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

/// Check if package already exists at destination (rez layout)
async fn package_exists_at_destination(
    destination_path: &PathBuf,
    package: &Package,
) -> RezCoreResult<bool> {
    let ver_str = package
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");
    let pkg_dir = destination_path.join(&package.name).join(ver_str);
    Ok(pkg_dir.exists())
}

/// Copy entire package directory recursively
async fn copy_package_directory(
    source_root: &PathBuf,
    dest_root: &PathBuf,
    source_package: &Package,
    args: &CpArgs,
) -> RezCoreResult<CopyResult> {
    // Remove existing destination if force
    if args.force && dest_root.exists() {
        std::fs::remove_dir_all(dest_root).map_err(|e| RezCoreError::Io(e.into()))?;
    }

    // Recursively copy directory
    copy_dir_recursive(source_root, dest_root)?;

    let variants_copied = if args.all_variants {
        source_package.variants.len().max(1)
    } else {
        1
    };

    Ok(CopyResult {
        source_package: source_package.clone(),
        destination_path: dest_root.clone(),
        success: true,
        error: None,
        variants_copied,
    })
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> RezCoreResult<()> {
    std::fs::create_dir_all(dest).map_err(|e| RezCoreError::Io(e.into()))?;

    for entry in std::fs::read_dir(src).map_err(|e| RezCoreError::Io(e.into()))? {
        let entry = entry.map_err(|e| RezCoreError::Io(e.into()))?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path).map_err(|e| RezCoreError::Io(e.into()))?;
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
