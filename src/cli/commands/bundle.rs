//! # Bundle Command
//!
//! Implementation of the `rez bundle` command.
//! Creates a self-contained bundle directory from a resolved context,
//! making the environment relocatable and shareable.

use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_context::{ContextConfig, ResolvedContext};
use rez_next_package::PackageRequirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
use rez_next_solver::{DependencyResolver, SolverConfig};
use serde::{Deserialize, Serialize};
use serde_json;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Arguments for the bundle command
#[derive(Args, Clone)]
pub struct BundleArgs {
    /// Package requests to bundle (e.g., "python-3.9" "maya-2023")
    #[arg(value_name = "PKG")]
    pub packages: Vec<String>,

    /// Output directory for the bundle
    #[arg(long, short = 'o', value_name = "DIR")]
    pub output: Option<String>,

    /// Bundle name (default: derived from packages)
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,

    /// Skip copying package files (only save context metadata)
    #[arg(long)]
    pub skip_copy: bool,

    /// Package paths to search
    #[arg(long, value_name = "PATH")]
    pub paths: Option<String>,

    /// Verbosity level
    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
}

/// Bundle metadata saved to bundle.json
#[derive(Serialize, Deserialize, Debug)]
struct BundleMetadata {
    /// Bundle format version
    pub version: String,
    /// Original package requests
    pub requests: Vec<String>,
    /// Resolved package names and versions
    pub packages: Vec<BundlePackageInfo>,
    /// Bundle creation timestamp
    pub created_at: String,
    /// Bundle name
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BundlePackageInfo {
    pub name: String,
    pub version: String,
    /// Relative path inside bundle to package files
    pub bundle_path: String,
    /// Original source path
    pub source_path: Option<String>,
}

/// Detect if a string looks like a filesystem path rather than a package spec.
///
/// A string is treated as a path if it:
/// - is an absolute path (starts with `/`, `\`, or a Windows drive letter like `C:\`)
/// - contains a path separator (`/` or `\`)
fn looks_like_path(s: &str) -> bool {
    // Windows absolute path: C:\... or C:/...
    if s.len() >= 3 && s.chars().nth(1) == Some(':') {
        return true;
    }
    // Unix absolute path or relative path with separator
    if s.starts_with('/') || s.starts_with('\\') {
        return true;
    }
    // Contains path separator
    if s.contains('/') || s.contains('\\') {
        return true;
    }
    false
}

/// Execute the bundle command
pub fn execute(mut args: BundleArgs) -> RezCoreResult<()> {
    if args.packages.is_empty() {
        return Err(RezCoreError::RequirementParse(
            "No packages specified. Usage: rez bundle <pkg1> [pkg2 ...] [dest]".to_string(),
        ));
    }

    // If the last positional argument looks like a path, treat it as the output directory.
    // This supports the rez-style CLI: `rez bundle python-3.9 maya-2024 /path/to/bundle`
    if args.output.is_none() {
        if let Some(last) = args.packages.last() {
            if looks_like_path(last) {
                let dest = last.clone();
                args.packages.pop();
                args.output = Some(dest);
            }
        }
    }

    if args.packages.is_empty() {
        return Err(RezCoreError::RequirementParse(
            "No packages specified. Usage: rez bundle <pkg1> [pkg2 ...] [dest]".to_string(),
        ));
    }

    // Determine bundle name
    let bundle_name = args.name.clone().unwrap_or_else(|| {
        args.packages
            .iter()
            .map(|p| p.replace(['-', '.'], "_"))
            .collect::<Vec<_>>()
            .join("_")
    });

    // Determine output directory: if output is explicitly specified, use it as-is.
    // Otherwise, create a subdirectory named after the bundle.
    let output_dir = if let Some(ref out) = args.output {
        PathBuf::from(out)
    } else {
        PathBuf::from(".").join(&bundle_name)
    };

    println!(
        "Creating bundle '{}' in: {}",
        bundle_name,
        output_dir.display()
    );

    // Parse requirements
    let requirements: Vec<PackageRequirement> = args
        .packages
        .iter()
        .map(|s| {
            PackageRequirement::parse(s).map_err(|e| {
                RezCoreError::RequirementParse(format!("Invalid requirement '{}': {}", s, e))
            })
        })
        .collect::<Result<_, _>>()?;

    // Resolve context
    let context = resolve_context(&requirements, &args)?;

    if context.resolved_packages.is_empty() && !requirements.is_empty() {
        println!("Warning: No packages resolved. Bundle will only contain metadata.");
    } else {
        println!("Resolved {} packages", context.resolved_packages.len());
    }

    // Create bundle directory structure
    std::fs::create_dir_all(&output_dir).map_err(|e| RezCoreError::Io(e))?;

    let packages_dir = output_dir.join("packages");
    std::fs::create_dir_all(&packages_dir).map_err(|e| RezCoreError::Io(e))?;

    // Setup search paths for finding package files
    let rez_config = RezCoreConfig::load();
    let search_paths: Vec<PathBuf> = get_search_paths(&args, &rez_config);

    // Build metadata and copy packages
    let mut package_infos = Vec::new();

    for package in &context.resolved_packages {
        let ver_str = package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        let bundle_pkg_path = format!("{}/{}", package.name, ver_str);

        // Find source package directory
        let source_path = find_package_dir(&package.name, ver_str, &search_paths);

        if !args.skip_copy {
            if let Some(ref src) = source_path {
                let dest = packages_dir.join(&package.name).join(ver_str);
                if let Err(e) = copy_dir_recursive(src, &dest) {
                    println!(
                        "  Warning: failed to copy {}-{}: {}",
                        package.name, ver_str, e
                    );
                } else if args.verbose > 0 {
                    println!("  Copied {}-{}", package.name, ver_str);
                }
            } else if args.verbose > 0 {
                println!(
                    "  Warning: source not found for {}-{}",
                    package.name, ver_str
                );
            }
        }

        package_infos.push(BundlePackageInfo {
            name: package.name.clone(),
            version: ver_str.to_string(),
            bundle_path: bundle_pkg_path,
            source_path: source_path.map(|p| p.to_string_lossy().to_string()),
        });
    }

    // Save bundle.json metadata
    let metadata = BundleMetadata {
        version: "1".to_string(),
        requests: args.packages.clone(),
        packages: package_infos,
        created_at: chrono::Utc::now().to_rfc3339(),
        name: bundle_name.clone(),
    };

    let metadata_json = serde_json::to_string_pretty(&metadata).map_err(RezCoreError::Serde)?;
    let metadata_path = output_dir.join("bundle.json");
    std::fs::write(&metadata_path, &metadata_json).map_err(|e| RezCoreError::Io(e))?;

    // Also create bundle.yaml for rez compatibility (rez uses bundle.yaml as the bundle manifest)
    let mut yaml_lines = vec![
        format!("name: {}", metadata.name),
        format!("version: {}", metadata.version),
        format!("created_at: {}", metadata.created_at),
        "requests:".to_string(),
    ];
    for req in &metadata.requests {
        yaml_lines.push(format!("  - {}", req));
    }
    yaml_lines.push("packages:".to_string());
    for pkg in &metadata.packages {
        yaml_lines.push(format!("  - name: {}", pkg.name));
        yaml_lines.push(format!("    version: {}", pkg.version));
    }
    let bundle_yaml = yaml_lines.join("\n") + "\n";
    let yaml_path = output_dir.join("bundle.yaml");
    std::fs::write(&yaml_path, &bundle_yaml).map_err(|e| RezCoreError::Io(e))?;

    // Save context .rxt file
    let context_json = serde_json::to_string_pretty(&context).map_err(RezCoreError::Serde)?;
    let context_path = output_dir.join("context.rxt");
    std::fs::write(&context_path, &context_json).map_err(|e| RezCoreError::Io(e))?;

    // Generate activation scripts for common shells
    generate_activation_scripts(&output_dir, &context, &rez_config)?;

    println!("\nBundle created successfully:");
    println!("  Directory : {}", output_dir.display());
    println!("  Metadata  : {}", metadata_path.display());
    println!("  Context   : {}", context_path.display());
    println!(
        "  Packages  : {} {}",
        context.resolved_packages.len(),
        if args.skip_copy {
            "(metadata only)"
        } else {
            "(files copied)"
        }
    );

    Ok(())
}

/// Resolve packages into a context
fn resolve_context(
    requirements: &[PackageRequirement],
    args: &BundleArgs,
) -> RezCoreResult<ResolvedContext> {
    let _config = ContextConfig::default();
    let rez_config = RezCoreConfig::load();

    let search_paths = get_search_paths(args, &rez_config);
    let mut repo_manager = RepositoryManager::new();

    for (i, path) in search_paths.iter().enumerate() {
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path.clone(),
                format!("repo_{}", i),
            )));
        }
    }

    let repo_arc = Arc::new(repo_manager);

    let resolver_reqs: Vec<rez_next_package::Requirement> = requirements
        .iter()
        .map(|pr| {
            let req_str = pr.to_string();
            req_str
                .parse::<rez_next_package::Requirement>()
                .unwrap_or_else(|_| rez_next_package::Requirement::new(pr.name.clone()))
        })
        .collect();

    let rt = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e))?;
    let solver_config = SolverConfig::default();
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), solver_config);
    let resolution = rt.block_on(resolver.resolve(resolver_reqs))?;

    let mut context = ResolvedContext::from_requirements(requirements.to_vec());
    context.resolved_packages = resolution
        .resolved_packages
        .into_iter()
        .map(|info| (*info.package).clone())
        .collect();
    context.status = rez_next_context::ContextStatus::Resolved;

    Ok(context)
}

/// Get package search paths from args or config
fn get_search_paths(args: &BundleArgs, config: &RezCoreConfig) -> Vec<PathBuf> {
    if let Some(ref paths_str) = args.paths {
        paths_str
            .split(std::path::MAIN_SEPARATOR)
            .map(PathBuf::from)
            .collect()
    } else {
        config
            .packages_path
            .iter()
            .map(|p| {
                if p.starts_with("~/") || p == "~" {
                    if let Ok(home) =
                        std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
                    {
                        return PathBuf::from(p.replacen("~", &home, 1));
                    }
                }
                PathBuf::from(p)
            })
            .collect()
    }
}

/// Find a package directory in search paths
fn find_package_dir(name: &str, version: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    for base in search_paths {
        let candidate = base.join(name).join(version);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Copy directory recursively
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

/// Generate activation scripts for bash/powershell in bundle directory
fn generate_activation_scripts(
    bundle_dir: &Path,
    context: &ResolvedContext,
    _config: &RezCoreConfig,
) -> RezCoreResult<()> {
    // Build a simple RexEnvironment from the context packages
    let mut rex_env = RexEnvironment::new();

    let bundle_packages_dir = bundle_dir.join("packages");
    for package in &context.resolved_packages {
        let ver_str = package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        let pkg_root = bundle_packages_dir.join(&package.name).join(ver_str);
        let root_str = pkg_root.to_string_lossy().to_string();

        rex_env.vars.insert(
            format!("{}_ROOT", package.name.to_uppercase()),
            root_str.clone(),
        );

        // Typical bin/lib paths
        let bin_path = pkg_root.join("bin");
        if bin_path.exists() {
            let current_path = rex_env.vars.get("PATH").cloned().unwrap_or_default();
            let sep = if cfg!(windows) { ";" } else { ":" };
            rex_env.vars.insert(
                "PATH".to_string(),
                if current_path.is_empty() {
                    bin_path.to_string_lossy().to_string()
                } else {
                    format!("{}{}{}", bin_path.to_string_lossy(), sep, current_path)
                },
            );
        }
    }

    // Bash activation script
    let bash_script = generate_shell_script(&rex_env, &ShellType::Bash);
    let bash_path = bundle_dir.join("activate.sh");
    std::fs::write(&bash_path, bash_script).map_err(|e| RezCoreError::Io(e))?;

    // PowerShell activation script
    let ps_script = generate_shell_script(&rex_env, &ShellType::PowerShell);
    let ps_path = bundle_dir.join("activate.ps1");
    std::fs::write(&ps_path, ps_script).map_err(|e| RezCoreError::Io(e))?;

    // CMD activation script
    let cmd_script = generate_shell_script(&rex_env, &ShellType::Cmd);
    let cmd_path = bundle_dir.join("activate.bat");
    std::fs::write(&cmd_path, cmd_script).map_err(|e| RezCoreError::Io(e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_args_default() {
        let args = BundleArgs {
            packages: vec!["python-3.9".to_string()],
            output: None,
            name: None,
            skip_copy: false,
            paths: None,
            verbose: 0,
        };
        assert_eq!(args.packages.len(), 1);
        assert!(args.output.is_none());
        assert!(!args.skip_copy);
    }

    #[test]
    fn test_bundle_name_generation() {
        let packages = vec!["python-3.9".to_string(), "maya-2023".to_string()];
        let bundle_name: String = packages
            .iter()
            .map(|p| p.replace(['-', '.'], "_"))
            .collect::<Vec<_>>()
            .join("_");
        assert_eq!(bundle_name, "python_3_9_maya_2023");
    }

    #[test]
    fn test_find_package_dir_missing() {
        let search_paths = vec![PathBuf::from("/nonexistent/path")];
        let result = find_package_dir("python", "3.9", &search_paths);
        assert!(result.is_none());
    }

    #[test]
    fn test_bundle_metadata_serialization() {
        let metadata = BundleMetadata {
            version: "1".to_string(),
            requests: vec!["python-3.9".to_string()],
            packages: vec![BundlePackageInfo {
                name: "python".to_string(),
                version: "3.9".to_string(),
                bundle_path: "python/3.9".to_string(),
                source_path: None,
            }],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            name: "python_3_9".to_string(),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("python_3_9"));
        assert!(json.contains("python-3.9"));
    }

    // ── Phase 108: additional bundle tests ──────────────────────────────────

    /// BundleMetadata JSON roundtrip preserves all fields
    #[test]
    fn test_bundle_metadata_json_roundtrip() {
        let metadata = BundleMetadata {
            version: "1".to_string(),
            requests: vec!["houdini-20".to_string(), "python-3.11".to_string()],
            packages: vec![
                BundlePackageInfo {
                    name: "houdini".to_string(),
                    version: "20.0.0".to_string(),
                    bundle_path: "houdini/20.0.0".to_string(),
                    source_path: Some("/packages/houdini/20.0.0".to_string()),
                },
                BundlePackageInfo {
                    name: "python".to_string(),
                    version: "3.11".to_string(),
                    bundle_path: "python/3.11".to_string(),
                    source_path: None,
                },
            ],
            created_at: "2026-03-30T06:00:00Z".to_string(),
            name: "houdini_20_python_3_11".to_string(),
        };
        let json = serde_json::to_string_pretty(&metadata).unwrap();
        let restored: BundleMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.version, "1");
        assert_eq!(restored.requests.len(), 2);
        assert_eq!(restored.packages.len(), 2);
        assert_eq!(restored.name, "houdini_20_python_3_11");
        assert_eq!(
            restored.packages[0].source_path,
            Some("/packages/houdini/20.0.0".to_string())
        );
        assert!(restored.packages[1].source_path.is_none());
    }

    /// find_package_dir returns correct path when directory exists
    #[test]
    fn test_find_package_dir_found() {
        let tmp = tempfile::TempDir::new().unwrap();
        let pkg_dir = tmp.path().join("python").join("3.9");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let search_paths = vec![tmp.path().to_path_buf()];
        let result = find_package_dir("python", "3.9", &search_paths);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), pkg_dir);
    }

    /// find_package_dir returns first match in priority order
    #[test]
    fn test_find_package_dir_priority_order() {
        let tmp1 = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        // Only tmp2 has the package
        let pkg_dir2 = tmp2.path().join("maya").join("2024");
        std::fs::create_dir_all(&pkg_dir2).unwrap();
        let search_paths = vec![tmp1.path().to_path_buf(), tmp2.path().to_path_buf()];
        let result = find_package_dir("maya", "2024", &search_paths);
        assert!(result.is_some(), "Should find maya in second repo");
        assert_eq!(result.unwrap(), pkg_dir2);
    }

    /// copy_dir_recursive copies files and subdirectories
    #[test]
    fn test_copy_dir_recursive_with_subdirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        std::fs::create_dir_all(&src).unwrap();
        let sub = src.join("python");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(src.join("package.py"), "name = 'test'\n").unwrap();
        std::fs::write(sub.join("module.py"), "def foo(): pass\n").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();

        assert!(
            dst.join("package.py").exists(),
            "package.py should be copied"
        );
        assert!(
            dst.join("python").join("module.py").exists(),
            "subdirectory file should be copied"
        );
        let content = std::fs::read_to_string(dst.join("package.py")).unwrap();
        assert!(content.contains("test"));
    }

    /// BundleArgs with skip_copy and verbose flags
    #[test]
    fn test_bundle_args_skip_copy_and_paths() {
        let args = BundleArgs {
            packages: vec!["houdini-20".to_string()],
            output: Some("/tmp/bundles".to_string()),
            name: Some("my_bundle".to_string()),
            skip_copy: true,
            paths: Some("/packages:/local_packages".to_string()),
            verbose: 2,
        };
        assert!(args.skip_copy);
        assert_eq!(args.name, Some("my_bundle".to_string()));
        assert_eq!(args.verbose, 2);
        assert!(args.paths.is_some());
    }

    /// BundlePackageInfo source_path None vs Some
    #[test]
    fn test_bundle_package_info_source_path() {
        let with_src = BundlePackageInfo {
            name: "houdini".to_string(),
            version: "20.0".to_string(),
            bundle_path: "houdini/20.0".to_string(),
            source_path: Some("/pkgs/houdini/20.0".to_string()),
        };
        let without_src = BundlePackageInfo {
            name: "python".to_string(),
            version: "3.11".to_string(),
            bundle_path: "python/3.11".to_string(),
            source_path: None,
        };
        assert!(with_src.source_path.is_some());
        assert!(without_src.source_path.is_none());
        let json_with = serde_json::to_string(&with_src).unwrap();
        assert!(json_with.contains("source_path"));
    }

    /// Bundle name generation for single package
    #[test]
    fn test_bundle_name_single_package() {
        let packages = vec!["maya-2024.1".to_string()];
        let name: String = packages
            .iter()
            .map(|p| p.replace(['-', '.'], "_"))
            .collect::<Vec<_>>()
            .join("_");
        assert_eq!(name, "maya_2024_1");
    }

    /// BundleMetadata with empty package list serializes correctly
    #[test]
    fn test_bundle_metadata_empty_packages() {
        let metadata = BundleMetadata {
            version: "1".to_string(),
            requests: vec![],
            packages: vec![],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            name: "empty_bundle".to_string(),
        };
        let json = serde_json::to_string(&metadata).unwrap();
        let restored: BundleMetadata = serde_json::from_str(&json).unwrap();
        assert!(restored.packages.is_empty());
        assert!(restored.requests.is_empty());
    }
}
