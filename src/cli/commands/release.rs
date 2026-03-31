//! # Release Command
//!
//! Build a package from source and deploy it to the release repository.
//! Implements VCS validation, build, tag creation, and deployment.

use clap::Args;
use rez_next_build::{BuildManager, BuildOptions, BuildRequest};
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_package::serialization::PackageSerializer;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Release command configuration
#[derive(Debug, Clone, Args)]
pub struct ReleaseArgs {
    /// Release message
    #[arg(short = 'm', long = "message")]
    pub message: Option<String>,

    /// Message file path
    #[arg(short = 'F', long = "message-file")]
    pub message_file: Option<PathBuf>,

    /// VCS type to use (git, svn, etc.)
    #[arg(long = "vcs")]
    pub vcs: Option<String>,

    /// Build system type
    #[arg(long = "buildsys")]
    pub buildsys: Option<String>,

    /// Build process type
    #[arg(long = "process", default_value = "local")]
    pub process: String,

    /// Skip repository error checks
    #[arg(long = "skip-repo-errors")]
    pub skip_repo_errors: bool,

    /// Ignore existing tag
    #[arg(long = "ignore-existing-tag")]
    pub ignore_existing_tag: bool,

    /// Don't ensure latest version
    #[arg(long = "no-latest")]
    pub no_latest: bool,

    /// Specific variants to release (comma-separated indices)
    #[arg(long = "variants")]
    pub variants: Option<String>,

    /// Working directory
    #[arg(short = 'w', long = "working-dir")]
    pub working_dir: Option<PathBuf>,

    /// Dry run — show what would happen without doing it
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Parse variant indices from string
fn parse_variants(variants_str: &str) -> RezCoreResult<Vec<usize>> {
    variants_str
        .split(',')
        .map(|s| s.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| RezCoreError::CliError(format!("Invalid variant indices: {}", e)))
}

/// Execute the release command
pub fn execute(args: ReleaseArgs) -> RezCoreResult<()> {
    let working_dir = args
        .working_dir
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if args.verbose {
        println!("Working directory: {}", working_dir.display());
    }

    // 1. Load package from current directory
    let package = load_developer_package(&working_dir)?;
    let version_str = package
        .version
        .as_ref()
        .map(|v| v.as_str().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    println!("Releasing package: {}-{}", package.name, version_str);

    // 2. Determine VCS type
    let vcs_type = args
        .vcs
        .as_deref()
        .unwrap_or_else(|| detect_vcs(&working_dir));

    if args.verbose {
        println!("VCS: {}", vcs_type);
    }

    // 3. Validate repository state
    if !args.skip_repo_errors {
        validate_vcs_state(&working_dir, vcs_type, &args)?;
    }

    // 4. Check for existing release tag
    if !args.ignore_existing_tag {
        let tag = format!("{}-{}", package.name, version_str);
        if check_tag_exists(&working_dir, &tag) {
            return Err(RezCoreError::BuildError(format!(
                "Release tag '{}' already exists. Use --ignore-existing-tag to override.",
                tag
            )));
        }
    }

    if args.dry_run {
        println!("[dry-run] Would release {}-{}", package.name, version_str);
        println!("[dry-run] VCS: {}", vcs_type);
        println!("[dry-run] Release repository: {}", get_release_path());
        return Ok(());
    }

    // 5. Build package variants
    println!("Building package...");
    let install_path = PathBuf::from(get_release_path());
    let build_options = BuildOptions {
        force_rebuild: false,
        skip_tests: false,
        release_mode: true,
        build_args: Vec::new(),
        env_vars: std::collections::HashMap::new(),
    };

    let build_request = BuildRequest {
        package: package.clone(),
        context: None,
        source_dir: working_dir.clone(),
        variant: None,
        options: build_options,
        install_path: Some(install_path),
    };

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create runtime: {}", e)))?;

    let mut build_manager = BuildManager::new();
    let build_id = rt.block_on(build_manager.start_build(build_request))?;
    let result = rt.block_on(build_manager.wait_for_build(&build_id))?;

    if !result.success {
        return Err(RezCoreError::BuildError(format!(
            "Build failed: {}",
            result.errors
        )));
    }

    println!("Build successful.");

    // 6. Create release tag in VCS
    let tag = format!("{}-{}", package.name, version_str);
    let message = args
        .message
        .clone()
        .or_else(|| read_message_file(args.message_file.as_ref()))
        .unwrap_or_else(|| format!("Release {}", tag));

    if vcs_type == "git" {
        create_git_tag(&working_dir, &tag, &message, args.verbose)?;
    }

    println!("Released {}-{} successfully!", package.name, version_str);
    Ok(())
}

/// Detect VCS type from working directory
fn detect_vcs(working_dir: &Path) -> &'static str {
    if working_dir.join(".git").exists() {
        "git"
    } else if working_dir.join(".svn").exists() {
        "svn"
    } else if working_dir.join(".hg").exists() {
        "hg"
    } else {
        "unknown"
    }
}

/// Load developer package from working directory
fn load_developer_package(working_dir: &Path) -> RezCoreResult<rez_next_package::Package> {
    let package_py = working_dir.join("package.py");
    let package_yaml = working_dir.join("package.yaml");

    if package_py.exists() {
        PackageSerializer::load_from_file(&package_py)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse package.py: {}", e)))
    } else if package_yaml.exists() {
        PackageSerializer::load_from_file(&package_yaml)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse package.yaml: {}", e)))
    } else {
        Err(RezCoreError::PackageParse(
            "No package.py or package.yaml found in current directory".to_string(),
        ))
    }
}

/// Validate VCS state (check for uncommitted changes, unpushed commits, etc.)
fn validate_vcs_state(working_dir: &Path, vcs_type: &str, args: &ReleaseArgs) -> RezCoreResult<()> {
    if vcs_type == "git" {
        // Check for uncommitted changes
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(working_dir)
            .output()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to run git status: {}", e)))?;

        if !output.stdout.is_empty() {
            let dirty_files = String::from_utf8_lossy(&output.stdout);
            return Err(RezCoreError::BuildError(format!(
                "Repository has uncommitted changes:\n{}\nCommit or stash changes before releasing.",
                dirty_files
            )));
        }

        // Check for unpushed commits
        let output = Command::new("git")
            .args(["log", "@{u}..", "--oneline"])
            .current_dir(working_dir)
            .output();

        if let Ok(output) = output {
            if !output.stdout.is_empty() {
                let unpushed = String::from_utf8_lossy(&output.stdout);
                if args.verbose {
                    println!("Warning: Repository has unpushed commits:\n{}", unpushed);
                }
            }
        }
    }
    Ok(())
}

/// Check if a git tag already exists
fn check_tag_exists(working_dir: &Path, tag: &str) -> bool {
    Command::new("git")
        .args(["tag", "-l", tag])
        .current_dir(working_dir)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Create a git tag at the current HEAD
fn create_git_tag(
    working_dir: &Path,
    tag: &str,
    message: &str,
    verbose: bool,
) -> RezCoreResult<()> {
    if verbose {
        println!("Creating git tag: {}", tag);
    }

    let output = Command::new("git")
        .args(["tag", "-a", tag, "-m", message])
        .current_dir(working_dir)
        .output()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create git tag: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RezCoreError::BuildError(format!(
            "Failed to create tag '{}': {}",
            tag, stderr
        )));
    }

    // Push the tag to origin if possible
    let push_output = Command::new("git")
        .args(["push", "origin", tag])
        .current_dir(working_dir)
        .output();

    if let Ok(push) = push_output {
        if !push.status.success() && verbose {
            let stderr = String::from_utf8_lossy(&push.stderr);
            println!("Warning: Could not push tag to origin: {}", stderr);
        }
    }

    Ok(())
}

/// Read release message from file
fn read_message_file(path: Option<&PathBuf>) -> Option<String> {
    path.and_then(|p| std::fs::read_to_string(p).ok())
}

/// Get release packages path from config
fn get_release_path() -> String {
    RezCoreConfig::load().release_packages_path
}
