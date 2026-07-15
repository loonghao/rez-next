//! # Release Command
//!
//! Build a package from source and deploy it to the release repository.
//! Implements VCS validation, build, tag creation, and deployment.

use clap::Args;
use rez_next_build::{BuildManager, BuildOptions, BuildRequest};
use rez_next_common::{RezCoreError, config::RezCoreConfig, error::RezCoreResult};
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
    ensure_supported_vcs(vcs_type)?;

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

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create runtime: {}", e)))?;

    let mut build_manager = BuildManager::new();
    for variant_index in release_variant_indices(&package, args.variants.as_deref())? {
        let variant_requires = variant_index.map(|index| package.variants[index].clone());
        let context = super::build::resolve_build_context(
            &package,
            variant_requires.as_deref(),
            args.verbose,
        )?;
        let build_request = BuildRequest {
            package: package.clone(),
            context,
            source_dir: working_dir.clone(),
            variant_index,
            variant_requires,
            options: build_options.clone(),
            install_path: Some(install_path.clone()),
        };
        let build_ids = rt.block_on(build_manager.start_build(build_request))?;
        let build_id = build_ids
            .first()
            .ok_or_else(|| RezCoreError::BuildError("Release did not start a build".to_string()))?;
        let result = rt.block_on(build_manager.wait_for_build(build_id))?;

        if !result.success {
            return Err(RezCoreError::BuildError(format!(
                "Build failed for variant {}: {}",
                variant_index.map_or_else(|| "none".to_string(), |index| index.to_string()),
                result.errors
            )));
        }
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

fn release_variant_indices(
    package: &rez_next_package::Package,
    selected: Option<&str>,
) -> RezCoreResult<Vec<Option<usize>>> {
    if package.variants.is_empty() {
        if selected.is_some_and(|value| !value.trim().is_empty()) {
            return Err(RezCoreError::BuildError(format!(
                "Package '{}' has no variants",
                package.name
            )));
        }
        return Ok(vec![None]);
    }

    let Some(selected) = selected else {
        return Ok((0..package.variants.len()).map(Some).collect());
    };
    let mut indices = Vec::new();
    for value in selected
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let index = value
            .parse::<usize>()
            .map_err(|_| RezCoreError::BuildError(format!("Invalid variant index '{value}'")))?;
        if index >= package.variants.len() {
            return Err(RezCoreError::BuildError(format!(
                "Variant index {index} is out of range for package '{}' ({} variants)",
                package.name,
                package.variants.len()
            )));
        }
        if !indices.contains(&Some(index)) {
            indices.push(Some(index));
        }
    }
    if indices.is_empty() {
        return Err(RezCoreError::BuildError(
            "No release variants were selected".to_string(),
        ));
    }
    Ok(indices)
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

fn ensure_supported_vcs(vcs_type: &str) -> RezCoreResult<()> {
    if vcs_type == "git" {
        return Ok(());
    }
    Err(RezCoreError::BuildError(format!(
        "Release VCS '{vcs_type}' is not supported; only git releases currently provide transactional tagging"
    )))
}

/// Load developer package from working directory
fn load_developer_package(working_dir: &Path) -> RezCoreResult<rez_next_package::Package> {
    let package_py = working_dir.join("package.py");

    if package_py.exists() {
        PackageSerializer::load_from_file(&package_py)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to parse package.py: {}", e)))
    } else {
        Err(RezCoreError::PackageParse(
            "No package.py found in current directory".to_string(),
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

        if let Ok(output) = output
            && !output.stdout.is_empty()
        {
            let unpushed = String::from_utf8_lossy(&output.stdout);
            if args.verbose {
                println!("Warning: Repository has unpushed commits:\n{}", unpushed);
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

    let push = Command::new("git")
        .args(["push", "origin", tag])
        .current_dir(working_dir)
        .output()
        .map_err(|error| {
            let _ = Command::new("git")
                .args(["tag", "-d", tag])
                .current_dir(working_dir)
                .output();
            RezCoreError::BuildError(format!("Failed to push tag '{tag}': {error}"))
        })?;

    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        let _ = Command::new("git")
            .args(["tag", "-d", tag])
            .current_dir(working_dir)
            .output();
        return Err(RezCoreError::BuildError(format!(
            "Failed to push tag '{tag}' to origin: {}",
            stderr.trim()
        )));
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use std::fs;

    fn git(working_dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(working_dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_release_variant_indices_selects_requested_variants() {
        let mut package = Package::new("tool".to_string());
        package.variants = vec![vec![], vec![], vec![]];

        assert_eq!(
            release_variant_indices(&package, Some("2,0")).unwrap(),
            vec![Some(2), Some(0)]
        );
    }

    #[test]
    fn test_release_variant_indices_rejects_out_of_range_variant() {
        let mut package = Package::new("tool".to_string());
        package.variants = vec![vec![]];

        let error = release_variant_indices(&package, Some("1"))
            .expect_err("invalid release variants must not be skipped");
        assert!(error.to_string().contains("out of range"), "{error}");
    }

    #[test]
    fn test_release_resolves_build_requirements() {
        let mut package = Package::new("tool".to_string());
        package.build_requires = vec!["definitely-missing-release-build-dependency".to_string()];

        let error = super::super::build::resolve_build_context(&package, None, false)
            .expect_err("release must not build without its unresolved build dependency");
        assert!(
            error
                .to_string()
                .contains("definitely-missing-release-build-dependency"),
            "{error}"
        );
    }

    #[test]
    fn test_create_git_tag_reports_push_failure() {
        let temp = tempfile::tempdir().unwrap();
        git(temp.path(), &["init"]);
        git(temp.path(), &["config", "user.name", "rez-next test"]);
        git(temp.path(), &["config", "user.email", "test@example.com"]);
        fs::write(temp.path().join("README.md"), "test").unwrap();
        git(temp.path(), &["add", "README.md"]);
        git(temp.path(), &["commit", "-m", "test"]);

        let error = create_git_tag(temp.path(), "tool-1.0.0", "release", false)
            .expect_err("a release without a pushable origin must fail");
        assert!(error.to_string().contains("push tag"), "{error}");
    }

    #[test]
    fn test_release_rejects_vcs_backends_without_tag_implementation() {
        assert!(ensure_supported_vcs("git").is_ok());
        for vcs in ["unknown", "hg", "svn"] {
            let error = ensure_supported_vcs(vcs)
                .expect_err("an unsupported VCS must not report a successful release");
            assert!(error.to_string().contains(vcs), "{error}");
        }
    }
}
