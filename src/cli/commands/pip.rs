//! # Pip Command
//!
//! Implementation of the `rez pip` command.
//! Installs Python packages via pip into a rez package repository,
//! wrapping them with rez package.py metadata.

use crate::cli::utils::expand_home_str as expand_home;
use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Arguments for the pip command
#[derive(Args, Clone)]
pub struct PipArgs {
    /// Pip package specification (e.g., "requests==2.28.0" or "requests>=2.28")
    #[arg(value_name = "PKG_SPEC")]
    pub packages: Vec<String>,

    /// Install into release packages path instead of local
    #[arg(long)]
    pub release: bool,

    /// Installation prefix path (overrides config)
    #[arg(long, value_name = "PATH")]
    pub prefix: Option<String>,

    /// Python executable to use for pip
    #[arg(long, value_name = "PYTHON")]
    pub python: Option<String>,

    /// Install from requirements.txt file
    #[arg(long, short = 'r', value_name = "FILE")]
    pub requirement: Option<String>,

    /// Skip dependency resolution (--no-deps)
    #[arg(long)]
    pub no_deps: bool,

    /// Force reinstall even if already installed
    #[arg(long)]
    pub force: bool,

    /// Dry run - show what would be installed without doing it
    #[arg(long)]
    pub dry_run: bool,

    /// Verbosity level
    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// A pip package candidate discovered from `pip show`
#[derive(Debug, Clone)]
struct PipPackageInfo {
    pub name: String,
    pub version: String,
    pub requires: Vec<String>,
    pub summary: String,
}

/// Execute the pip command
pub fn execute(args: PipArgs) -> RezCoreResult<()> {
    let config = RezCoreConfig::load();

    // Determine install path
    let install_base = if args.release {
        expand_home(&config.release_packages_path)
    } else {
        expand_home(&config.local_packages_path)
    };

    let install_path = args
        .prefix
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&install_base));

    // Find python executable
    let python_exe = find_python(&args)?;
    if args.verbose > 0 {
        println!("Using Python: {}", python_exe);
    }

    // Collect packages to install
    let mut pkg_specs: Vec<String> = args.packages.clone();

    // Also read from requirements.txt if specified
    if let Some(ref req_file) = args.requirement {
        let content = std::fs::read_to_string(req_file).map_err(RezCoreError::Io)?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                pkg_specs.push(trimmed.to_string());
            }
        }
    }

    if pkg_specs.is_empty() {
        return Err(RezCoreError::RequirementParse(
            "No packages specified. Use: rez pip <package> or rez pip -r requirements.txt"
                .to_string(),
        ));
    }

    println!("Installing {} package(s) via pip...", pkg_specs.len());

    // Create a temporary virtualenv to install into, then repackage
    let tmp_dir = tempfile::TempDir::new().map_err(RezCoreError::Io)?;
    let tmp_path = tmp_dir.path().to_path_buf();

    // Install packages into temp location
    for spec in &pkg_specs {
        install_pip_package(spec, &python_exe, &tmp_path, &args)?;

        // Discover what was installed
        if args.dry_run {
            println!("  [dry-run] Would install: {}", spec);
            continue;
        }

        let pkg_info = discover_installed_package(spec, &python_exe, &tmp_path)?;
        if let Some(info) = pkg_info {
            println!("  Installed: {}-{}", info.name, info.version);

            // Create rez package
            let rez_pkg_dir = install_path
                .join(info.name.to_lowercase().replace('-', "_"))
                .join(&info.version);

            create_rez_package(&info, &tmp_path, &rez_pkg_dir, &args)?;

            println!("  -> Rez package created: {}", rez_pkg_dir.display());
        }
    }

    if !args.dry_run {
        println!("\nPackages installed to: {}", install_path.display());
    }

    Ok(())
}

/// Find the Python executable to use
fn find_python(args: &PipArgs) -> RezCoreResult<String> {
    if let Some(ref python) = args.python {
        return Ok(python.clone());
    }

    // Try common python executables
    for exe in &["python3", "python", "python3.11", "python3.10", "python3.9"] {
        if let Ok(output) = Command::new(exe).arg("--version").output() {
            if output.status.success() {
                return Ok(exe.to_string());
            }
        }
    }

    Err(RezCoreError::ExecutionError(
        "No Python executable found. Install Python or use --python to specify one.".to_string(),
    ))
}

/// Install a pip package into a temp directory
fn install_pip_package(
    spec: &str,
    python: &str,
    target: &Path,
    args: &PipArgs,
) -> RezCoreResult<()> {
    if args.dry_run {
        return Ok(());
    }

    let target_str = target.to_string_lossy().to_string();

    let mut cmd = Command::new(python);
    cmd.args(["-m", "pip", "install", "--target", &target_str, spec]);

    if args.no_deps {
        cmd.arg("--no-deps");
    }

    if args.force {
        cmd.arg("--force-reinstall");
    }

    if args.verbose > 1 {
        cmd.arg("-v");
    } else if args.verbose == 0 {
        cmd.arg("-q");
    }

    let output = cmd
        .output()
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to run pip: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RezCoreError::ExecutionError(format!(
            "pip install failed for '{}': {}",
            spec, stderr
        )));
    }

    Ok(())
}

/// Discover information about an installed package using pip show
fn discover_installed_package(
    spec: &str,
    python: &str,
    target: &Path,
) -> RezCoreResult<Option<PipPackageInfo>> {
    // Extract package name from spec (strip version qualifiers)
    let pkg_name = parse_pkg_name_from_spec(spec);

    let output = Command::new(python)
        .args(["-m", "pip", "show", "--files", &pkg_name])
        .env("PYTHONPATH", target.to_string_lossy().as_ref())
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => {
            // Fall back: scan target dir for the package
            return scan_target_for_package(&pkg_name, target);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pip_show_output(&stdout, target)
}

/// Parse pip show output into PipPackageInfo
fn parse_pip_show_output(output: &str, _target: &Path) -> RezCoreResult<Option<PipPackageInfo>> {
    let mut name = String::new();
    let mut version = String::new();
    let mut requires = Vec::new();
    let mut summary = String::new();

    for line in output.lines() {
        if let Some(val) = line.strip_prefix("Name: ") {
            name = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("Version: ") {
            version = val.trim().to_string();
        } else if let Some(val) = line.strip_prefix("Requires: ") {
            requires = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else if let Some(val) = line.strip_prefix("Summary: ") {
            summary = val.trim().to_string();
        }
    }

    if name.is_empty() || version.is_empty() {
        return Ok(None);
    }

    Ok(Some(PipPackageInfo {
        name,
        version,
        requires,
        summary,
    }))
}

/// Scan the target directory to find an installed package
fn scan_target_for_package(name: &str, target: &Path) -> RezCoreResult<Option<PipPackageInfo>> {
    if !target.exists() {
        return Ok(None);
    }

    // Look for <name>-<version>.dist-info directories
    let normalized = name.to_lowercase().replace('-', "_");
    for entry in std::fs::read_dir(target).map_err(RezCoreError::Io)? {
        let entry = entry.map_err(RezCoreError::Io)?;
        let fname = entry.file_name().to_string_lossy().to_string();

        if fname.ends_with(".dist-info") {
            let without_suffix = fname.trim_end_matches(".dist-info");
            if let Some(dash_pos) = without_suffix.rfind('-') {
                let dist_name = without_suffix[..dash_pos].to_lowercase().replace('-', "_");
                let dist_version = &without_suffix[dash_pos + 1..];
                if dist_name == normalized {
                    // Read METADATA file
                    let metadata_path = entry.path().join("METADATA");
                    let mut summary = String::new();
                    let mut requires = Vec::new();

                    if let Ok(content) = std::fs::read_to_string(&metadata_path) {
                        for line in content.lines() {
                            if let Some(val) = line.strip_prefix("Summary: ") {
                                summary = val.to_string();
                            } else if let Some(val) = line.strip_prefix("Requires-Dist: ") {
                                let dep_name = val.split_whitespace().next().unwrap_or(val);
                                requires.push(dep_name.to_string());
                            }
                        }
                    }

                    return Ok(Some(PipPackageInfo {
                        name: without_suffix[..dash_pos].to_string(),
                        version: dist_version.to_string(),
                        requires,
                        summary,
                    }));
                }
            }
        }
    }
    Ok(None)
}

/// Create a rez package from pip-installed files
fn create_rez_package(
    info: &PipPackageInfo,
    src_dir: &Path,
    dest_dir: &Path,
    _args: &PipArgs,
) -> RezCoreResult<()> {
    std::fs::create_dir_all(dest_dir).map_err(RezCoreError::Io)?;

    // Copy python files into python sub-directory
    let python_dir = dest_dir.join("python");
    copy_dir_recursive(src_dir, &python_dir)?;

    // Convert pip requires to rez requires (best-effort)
    let rez_requires: Vec<String> = info
        .requires
        .iter()
        .filter(|r| !r.is_empty())
        .map(|r| r.to_lowercase().replace('-', "_"))
        .collect();

    // Generate package.py
    let requires_str = if rez_requires.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[\n    {}\n]",
            rez_requires
                .iter()
                .map(|r| format!("\"{}\"", r))
                .collect::<Vec<_>>()
                .join(",\n    ")
        )
    };

    let rez_name = info.name.to_lowercase().replace('-', "_");
    let package_py = format!(
        r#"name = "{name}"
version = "{version}"
description = "{description}"
authors = ["pip-rez-next"]

requires = {requires}

def commands():
    import os
    env.PYTHONPATH.prepend("{{root}}/python")
"#,
        name = rez_name,
        version = info.version,
        description = info.summary.replace('"', "'"),
        requires = requires_str,
    );

    let pkg_py_path = dest_dir.join("package.py");
    std::fs::write(&pkg_py_path, package_py).map_err(RezCoreError::Io)?;

    Ok(())
}

/// Copy directory tree recursively
fn copy_dir_recursive(src: &Path, dest: &Path) -> RezCoreResult<()> {
    std::fs::create_dir_all(dest).map_err(RezCoreError::Io)?;

    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(_) => return Ok(()), // src might not exist
    };

    for entry in entries {
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

/// Parse package name from pip spec (strip version qualifiers)
fn parse_pkg_name_from_spec(spec: &str) -> String {
    // Strip version operators: ==, >=, <=, >, <, !=, ~=
    let operators = ["==", ">=", "<=", "!=", "~=", ">", "<", "@"];
    let mut name = spec;
    for op in &operators {
        if let Some(pos) = name.find(op) {
            name = &name[..pos];
        }
    }
    name.trim().to_lowercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pkg_name_from_spec_exact() {
        assert_eq!(parse_pkg_name_from_spec("requests==2.28.0"), "requests");
    }

    #[test]
    fn test_parse_pkg_name_from_spec_range() {
        assert_eq!(parse_pkg_name_from_spec("requests>=2.28"), "requests");
    }

    #[test]
    fn test_parse_pkg_name_from_spec_no_version() {
        assert_eq!(parse_pkg_name_from_spec("requests"), "requests");
    }

    #[test]
    fn test_parse_pkg_name_hyphen_to_underscore() {
        assert_eq!(parse_pkg_name_from_spec("my-package>=1.0"), "my_package");
    }

    #[test]
    fn test_parse_pip_show_output() {
        let output = r#"Name: requests
Version: 2.28.0
Summary: Python HTTP for Humans.
Home-page: https://requests.readthedocs.io
Requires: certifi, charset-normalizer, idna, urllib3
"#;
        let result = parse_pip_show_output(output, Path::new("/tmp")).unwrap();
        assert!(result.is_some());
        let info = result.unwrap();
        assert_eq!(info.name, "requests");
        assert_eq!(info.version, "2.28.0");
        assert_eq!(info.requires.len(), 4);
        assert!(info.summary.contains("HTTP"));
    }

    #[test]
    fn test_parse_pip_show_empty() {
        let result = parse_pip_show_output("", Path::new("/tmp")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_pip_args_defaults() {
        let args = PipArgs {
            packages: vec!["requests==2.28".to_string()],
            release: false,
            prefix: None,
            python: None,
            requirement: None,
            no_deps: false,
            force: false,
            dry_run: false,
            verbose: 0,
        };
        assert!(!args.release);
        assert!(!args.dry_run);
        assert_eq!(args.packages.len(), 1);
    }

    #[test]
    fn test_expand_home_no_tilde() {
        let path = "/absolute/path";
        assert_eq!(expand_home(path), "/absolute/path");
    }

    #[test]
    fn test_create_rez_requires_empty() {
        let info = PipPackageInfo {
            name: "requests".to_string(),
            version: "2.28.0".to_string(),
            requires: vec![],
            summary: "HTTP library".to_string(),
        };
        let rez_requires: Vec<String> = info
            .requires
            .iter()
            .filter(|r| !r.is_empty())
            .map(|r| r.to_lowercase().replace('-', "_"))
            .collect();
        assert!(rez_requires.is_empty());
    }

    // ── Phase 95: pip install end-to-end logic tests ─────────────────────────

    /// create_rez_package generates a valid package.py with correct metadata
    #[test]
    fn test_create_rez_package_generates_package_py() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Use a separate src dir (empty) to avoid copy_dir_recursive issues
        let src_dir = tmp.path().join("pip_src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let dest_dir = tmp.path().join("rez_pkgs").join("requests").join("2.28.0");

        let info = PipPackageInfo {
            name: "requests".to_string(),
            version: "2.28.0".to_string(),
            requires: vec!["certifi".to_string(), "urllib3".to_string()],
            summary: "Python HTTP for Humans.".to_string(),
        };

        let args = PipArgs {
            packages: vec![],
            release: false,
            prefix: None,
            python: None,
            requirement: None,
            no_deps: false,
            force: false,
            dry_run: false,
            verbose: 0,
        };

        create_rez_package(&info, &src_dir, &dest_dir, &args).unwrap();

        let pkg_py = dest_dir.join("package.py");
        assert!(pkg_py.exists(), "package.py should be created");

        let content = std::fs::read_to_string(&pkg_py).unwrap();
        assert!(content.contains("requests"), "package.py should have name");
        assert!(content.contains("2.28.0"), "package.py should have version");
        assert!(
            content.contains("certifi"),
            "package.py should list certifi dependency"
        );
        assert!(
            content.contains("urllib3"),
            "package.py should list urllib3 dependency"
        );
        assert!(
            content.contains("PYTHONPATH"),
            "package.py commands should set PYTHONPATH"
        );
    }

    /// parse_pkg_name: tilde-equals ignored, name extracted
    #[test]
    fn test_parse_pkg_name_from_spec_tilde_eq() {
        assert_eq!(parse_pkg_name_from_spec("numpy~=1.24"), "numpy");
    }

    /// parse_pkg_name: not-equal ignored
    #[test]
    fn test_parse_pkg_name_from_spec_ne() {
        assert_eq!(parse_pkg_name_from_spec("django!=3.0"), "django");
    }

    /// requirements.txt parsing: comments and empty lines skipped
    #[test]
    fn test_pip_requirements_file_parsing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let req_path = tmp.path().join("requirements.txt");
        let content =
            "# This is a comment\n\nrequests==2.28.0\nnumpy>=1.24\n\n# another comment\ndjango\n";
        std::fs::write(&req_path, content).unwrap();

        let read = std::fs::read_to_string(&req_path).unwrap();
        let pkgs: Vec<String> = read
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect();

        assert_eq!(pkgs.len(), 3, "Should parse 3 packages, got: {:?}", pkgs);
        assert!(pkgs.contains(&"requests==2.28.0".to_string()));
        assert!(pkgs.contains(&"numpy>=1.24".to_string()));
        assert!(pkgs.contains(&"django".to_string()));
    }

    /// scan_target_for_package finds .dist-info and returns PipPackageInfo
    #[test]
    fn test_scan_target_finds_dist_info() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Create fake dist-info directory
        let dist_info_dir = tmp.path().join("requests-2.28.0.dist-info");
        std::fs::create_dir_all(&dist_info_dir).unwrap();
        let metadata_content = "Metadata-Version: 2.1\nName: requests\nVersion: 2.28.0\nSummary: HTTP for Humans.\nHome-page: https://requests.readthedocs.io\nRequires-Dist: certifi\nRequires-Dist: urllib3\n";
        std::fs::write(dist_info_dir.join("METADATA"), metadata_content).unwrap();

        let result = scan_target_for_package("requests", tmp.path()).unwrap();
        assert!(result.is_some(), "Should find requests in dist-info");
        let info = result.unwrap();
        assert_eq!(info.name, "requests");
        assert_eq!(info.version, "2.28.0");
        assert!(info.requires.contains(&"certifi".to_string()));
        assert!(info.requires.contains(&"urllib3".to_string()));
    }

    /// expand_home replaces ~ with HOME/USERPROFILE
    #[test]
    fn test_expand_home_with_tilde() {
        // Set fake HOME for test
        let result = expand_home("~/packages");
        // Either expanded or returned as-is if env not set
        assert!(!result.is_empty());
        // Should not start with ~ if HOME is set
        if std::env::var("HOME").is_ok() || std::env::var("USERPROFILE").is_ok() {
            assert!(!result.starts_with("~/"), "Should expand ~/: {}", result);
        }
    }
}
