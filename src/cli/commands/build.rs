//! Build command implementation
//!
//! Implements the `rez build` command for building packages from source.

use clap::Args;
use rez_core_build::{BuildManager, BuildOptions, BuildRequest, BuildStatus};
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_package::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Arguments for the build command
#[derive(Args, Clone, Debug)]
pub struct BuildArgs {
    /// Clear the current build before rebuilding
    #[arg(short = 'c', long = "clean")]
    pub clean: bool,

    /// Install the build to the local packages path
    #[arg(short = 'i', long = "install")]
    pub install: bool,

    /// Install to a custom package repository path
    #[arg(short = 'p', long = "prefix", value_name = "PATH")]
    pub prefix: Option<String>,

    /// Display resolve graph as an image if build environment fails to resolve
    #[arg(long = "fail-graph")]
    pub fail_graph: bool,

    /// Create build scripts rather than performing the full build
    #[arg(short = 's', long = "scripts")]
    pub scripts: bool,

    /// Just view the preprocessed package definition and exit
    #[arg(long = "view-pre")]
    pub view_pre: bool,

    /// The build process to use
    #[arg(long = "process", default_value = "local")]
    pub process: String,

    /// The build system to use (auto-detected if not specified)
    #[arg(short = 'b', long = "build-system")]
    pub build_system: Option<String>,

    /// Select variants to build (zero-indexed)
    #[arg(long = "variants", value_name = "INDEX")]
    pub variants: Option<Vec<usize>>,

    /// Arguments to pass to the build system
    #[arg(long = "ba", long = "build-args", value_name = "ARGS")]
    pub build_args: Option<String>,

    /// Arguments to pass to the child build system
    #[arg(long = "cba", long = "child-build-args", value_name = "ARGS")]
    pub child_build_args: Option<String>,

    /// Build in release mode
    #[arg(short = 'r', long = "release")]
    pub release: bool,

    /// Skip tests during build
    #[arg(long = "skip-tests")]
    pub skip_tests: bool,

    /// Force rebuild even if artifacts exist
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Package source (directory, URL, or Git repository)
    /// Examples:
    /// - Local directory: ./my-package or /path/to/package
    /// - Git repository: https://github.com/user/repo
    /// - Git with branch/tag: https://github.com/user/repo@main
    /// - HTTP archive: https://example.com/package.tar.gz
    /// - SSH Git: git@github.com:user/repo.git
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,

    /// Subdirectory within the source (for archives or repositories)
    #[arg(long = "subdir")]
    pub subdir: Option<String>,

    /// Git reference (branch, tag, or commit) for Git sources
    #[arg(long = "reference")]
    pub reference: Option<String>,
}

/// Execute the build command
pub fn execute(args: BuildArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔨 Starting package build...");
    }

    // Determine source directory first
    let (source_dir, package) = if let Some(ref source_url) = args.source {
        // Network or remote source
        fetch_and_load_remote_source(source_url, &args)?
    } else {
        // Local source (current directory)
        let working_dir = std::env::current_dir().map_err(|e| RezCoreError::Io(e))?;
        let package = load_current_package(&working_dir)?;
        (working_dir, package)
    };

    // Handle view-pre option
    if args.view_pre {
        return view_preprocessed_package_with_data(&package);
    }

    if args.verbose {
        println!(
            "📦 Building package: {} {}",
            package.name,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );
    }

    // Create build options
    let build_options = BuildOptions {
        force_rebuild: args.force || args.clean,
        skip_tests: args.skip_tests,
        release_mode: args.release,
        build_args: parse_build_args(&args.build_args),
        env_vars: HashMap::new(),
    };

    // Determine install path if installing
    let install_path = if args.install {
        Some(get_install_path(&args)?)
    } else {
        None
    };

    // Create build request
    let build_request = BuildRequest {
        package: package.clone(),
        context: None, // TODO: Resolve build context
        source_dir: source_dir.clone(),
        variant: None, // TODO: Handle variant selection
        options: build_options,
        install_path,
    };

    // Execute build
    execute_build(build_request, &args, &package, &source_dir)
}

/// Fetch and load package from remote source
fn fetch_and_load_remote_source(
    source_url: &str,
    args: &BuildArgs,
) -> RezCoreResult<(PathBuf, Package)> {
    use rez_core_build::{NetworkSource, SourceManager};
    use tempfile::TempDir;

    if args.verbose {
        println!("🌐 Fetching remote source: {}", source_url);
    }

    // Create source manager
    let source_manager = SourceManager::new();

    // Parse source URL
    let mut network_source = source_manager.parse_source(source_url)?;

    // Apply additional options from command line
    if let Some(ref subdir) = args.subdir {
        network_source.subdirectory = Some(subdir.clone());
    }
    if let Some(ref reference) = args.reference {
        network_source.reference = Some(reference.clone());
    }

    // Create temporary directory for fetching
    let temp_dir = TempDir::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create temp directory: {}", e)))?;

    // Fetch source
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create async runtime: {}", e)))?;

    let source_path = runtime.block_on(async {
        source_manager
            .fetch_source(&network_source, &temp_dir.path().to_path_buf())
            .await
    })?;

    if args.verbose {
        println!("📁 Source fetched to: {}", source_path.display());
    }

    // Load package from fetched source
    let package = load_current_package(&source_path)?;

    // Keep temp directory alive by converting to persistent path
    let persistent_path = copy_to_persistent_location(&source_path, &package)?;

    Ok((persistent_path, package))
}

/// Copy source to a persistent location for building
fn copy_to_persistent_location(source_path: &PathBuf, package: &Package) -> RezCoreResult<PathBuf> {
    use std::fs;

    // Create build cache directory
    let cache_dir = std::env::temp_dir().join("rez-core-build-cache");
    fs::create_dir_all(&cache_dir).map_err(|e| {
        RezCoreError::BuildError(format!("Failed to create cache directory: {}", e))
    })?;

    // Create unique directory for this package
    let package_cache_dir = cache_dir.join(format!(
        "{}-{}",
        package.name,
        package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown")
    ));

    // Remove existing cache if present
    if package_cache_dir.exists() {
        fs::remove_dir_all(&package_cache_dir).map_err(|e| {
            RezCoreError::BuildError(format!("Failed to remove existing cache: {}", e))
        })?;
    }

    // Copy source to cache directory
    copy_dir_recursive(source_path, &package_cache_dir)?;

    Ok(package_cache_dir)
}

/// Recursively copy directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> RezCoreResult<()> {
    use std::fs;

    fs::create_dir_all(dest)
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create directory: {}", e)))?;

    for entry in fs::read_dir(src)
        .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read directory entry: {}", e))
        })?;

        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)
                .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
        }
    }

    Ok(())
}

/// Load package from current directory
fn load_current_package(working_dir: &PathBuf) -> RezCoreResult<Package> {
    use rez_core_package::serialization::PackageSerializer;

    // Look for package.py or package.yaml
    let package_py = working_dir.join("package.py");
    let package_yaml = working_dir.join("package.yaml");

    if package_py.exists() {
        // Use the existing PackageSerializer to load Python packages
        return PackageSerializer::load_from_file(&package_py)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse package.py: {}", e)));
    }

    if package_yaml.exists() {
        // Use the existing PackageSerializer to load YAML packages
        return PackageSerializer::load_from_file(&package_yaml).map_err(|e| {
            RezCoreError::PackageParse(format!("Failed to parse package.yaml: {}", e))
        });
    }

    Err(RezCoreError::PackageParse(
        "No package.py or package.yaml found in current directory".to_string(),
    ))
}

/// Parse build arguments string into vector
fn parse_build_args(args_str: &Option<String>) -> Vec<String> {
    match args_str {
        Some(args) => args.split_whitespace().map(|s| s.to_string()).collect(),
        None => Vec::new(),
    }
}

/// View preprocessed package definition
fn view_preprocessed_package(args: &BuildArgs) -> RezCoreResult<()> {
    let working_dir = std::env::current_dir().map_err(|e| RezCoreError::Io(e))?;

    let package = load_current_package(&working_dir)?;

    // Print package information in Python format
    println!("# Preprocessed package definition");
    println!("name = '{}'", package.name);

    if let Some(ref version) = package.version {
        println!("version = '{}'", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description = '{}'", description);
    }

    if !package.requires.is_empty() {
        println!("requires = [");
        for req in &package.requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    if !package.build_requires.is_empty() {
        println!("build_requires = [");
        for req in &package.build_requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    Ok(())
}

/// View preprocessed package definition with package data
fn view_preprocessed_package_with_data(package: &Package) -> RezCoreResult<()> {
    // Print package information in Python format
    println!("# Preprocessed package definition");
    println!("name = '{}'", package.name);

    if let Some(ref version) = package.version {
        println!("version = '{}'", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description = '{}'", description);
    }

    if !package.requires.is_empty() {
        println!("requires = [");
        for req in &package.requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    if !package.build_requires.is_empty() {
        println!("build_requires = [");
        for req in &package.build_requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    Ok(())
}

/// Execute the build process
fn execute_build(
    request: BuildRequest,
    args: &BuildArgs,
    package: &Package,
    source_dir: &PathBuf,
) -> RezCoreResult<()> {
    // Create build manager
    let mut build_manager = BuildManager::new();

    if args.verbose {
        println!("🔧 Configuring build environment...");
    }

    // Start build process
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create async runtime: {}", e)))?;

    let build_id: String = runtime.block_on(async { build_manager.start_build(request).await })?;

    if args.verbose {
        println!("🚀 Build started with ID: {}", build_id);
    }

    // Wait for build completion
    let build_result = runtime.block_on(async { build_manager.wait_for_build(&build_id).await })?;

    if !build_result.success {
        return Err(RezCoreError::BuildError(format!(
            "Build failed: {}",
            build_result.errors
        )));
    }

    // Installation is handled by the build system's install step
    // No need for separate installation logic here

    if args.verbose {
        println!("✅ Build completed successfully!");
    }

    Ok(())
}

/// Expand and normalize path with support for various path formats
fn expand_path(path: &str) -> RezCoreResult<String> {
    let expanded = if path.starts_with("~/") {
        // Handle home directory expansion
        expand_home_path(path)?
    } else if path.starts_with("\\\\") {
        // Handle UNC paths (\\server\share\path)
        validate_unc_path(path)?
    } else if path.len() >= 2 && path.chars().nth(1) == Some(':') {
        // Handle Windows drive paths (C:\path, D:\path, etc.)
        validate_drive_path(path)?
    } else if path.starts_with('/') {
        // Handle Unix absolute paths
        path.to_string()
    } else {
        // Handle relative paths - convert to absolute
        let current_dir = std::env::current_dir().map_err(|e| {
            RezCoreError::ConfigError(format!("Cannot get current directory: {}", e))
        })?;
        current_dir.join(path).to_string_lossy().to_string()
    };

    // Normalize the path
    let normalized = normalize_path(&expanded)?;
    Ok(normalized)
}

/// Expand home directory paths
fn expand_home_path(path: &str) -> RezCoreResult<String> {
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let home_path = Path::new(&home);
        let expanded = home_path.join(&path[2..]);
        Ok(expanded.to_string_lossy().to_string())
    } else {
        Err(RezCoreError::ConfigError(
            "Cannot determine home directory".to_string(),
        ))
    }
}

/// Validate and normalize UNC paths
fn validate_unc_path(path: &str) -> RezCoreResult<String> {
    if !path.starts_with("\\\\") {
        return Err(RezCoreError::ConfigError(
            "Invalid UNC path format".to_string(),
        ));
    }

    // Basic UNC path validation: \\server\share\path
    let parts: Vec<&str> = path[2..].split('\\').collect();
    if parts.len() < 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(RezCoreError::ConfigError(
            "UNC path must be in format \\\\server\\share\\path".to_string(),
        ));
    }

    Ok(path.to_string())
}

/// Validate and normalize Windows drive paths
fn validate_drive_path(path: &str) -> RezCoreResult<String> {
    if path.len() < 2 {
        return Err(RezCoreError::ConfigError(
            "Invalid drive path format".to_string(),
        ));
    }

    let drive_char = path.chars().nth(0).unwrap();
    if !drive_char.is_ascii_alphabetic() || path.chars().nth(1) != Some(':') {
        return Err(RezCoreError::ConfigError(
            "Drive path must start with a letter followed by colon (e.g., C:)".to_string(),
        ));
    }

    Ok(path.to_string())
}

/// Normalize path separators and resolve . and .. components
fn normalize_path(path: &str) -> RezCoreResult<String> {
    let path_buf = PathBuf::from(path);

    // Use canonicalize if the path exists, otherwise just normalize
    if path_buf.exists() {
        match path_buf.canonicalize() {
            Ok(canonical) => Ok(canonical.to_string_lossy().to_string()),
            Err(_) => {
                // If canonicalize fails, fall back to basic normalization
                Ok(path_buf.to_string_lossy().to_string())
            }
        }
    } else {
        // For non-existent paths, do basic normalization
        let mut components = Vec::new();
        for component in path_buf.components() {
            match component {
                std::path::Component::CurDir => {
                    // Skip "." components
                    continue;
                }
                std::path::Component::ParentDir => {
                    // Handle ".." components
                    if !components.is_empty()
                        && components.last() != Some(&std::path::Component::ParentDir)
                    {
                        components.pop();
                    } else {
                        components.push(component);
                    }
                }
                _ => {
                    components.push(component);
                }
            }
        }

        let normalized: PathBuf = components.iter().collect();
        Ok(normalized.to_string_lossy().to_string())
    }
}

/// Get installation path
fn get_install_path(args: &BuildArgs) -> RezCoreResult<PathBuf> {
    use rez_core_common::config::RezCoreConfig;

    let install_path_str = if let Some(ref prefix) = args.prefix {
        prefix.clone()
    } else {
        // Use default local packages path
        let config = RezCoreConfig::default();
        expand_path(&config.local_packages_path)?
    };

    Ok(PathBuf::from(install_path_str))
}

/// Generate package.py content
fn generate_package_content(package: &Package) -> RezCoreResult<String> {
    use rez_core_package::serialization::{PackageFormat, PackageSerializer};

    PackageSerializer::save_to_string(package, PackageFormat::Python).map_err(|e| {
        RezCoreError::PackageParse(format!("Failed to generate package content: {}", e))
    })
}
