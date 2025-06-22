//! Plugins command implementation
//!
//! Implements the `rez plugins` command for listing and managing package plugins.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::collections::HashMap;
use std::path::PathBuf;

/// Arguments for the plugins command
#[derive(Args, Clone, Debug)]
pub struct PluginsArgs {
    /// Package to list plugins for
    #[arg(value_name = "PKG")]
    pub package: String,

    /// Set package search path
    #[arg(long = "paths", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub package_name: String,
    pub package_version: Option<String>,
    pub plugin_type: String,
    pub description: Option<String>,
    pub module_path: Option<String>,
}

/// Plugin discovery result
#[derive(Debug, Clone)]
pub struct PluginDiscoveryResult {
    pub plugins: Vec<PluginInfo>,
    pub package_name: String,
    pub total_found: usize,
}

/// Execute the plugins command
pub fn execute(args: PluginsArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔌 Rez Plugins - Discovering package plugins...");
        println!("Package: {}", args.package);
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async { execute_plugins_async(&args).await })
}

/// Execute plugins discovery asynchronously
async fn execute_plugins_async(args: &PluginsArgs) -> RezCoreResult<()> {
    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    // Find package
    let package = find_package(&repo_manager, &args.package).await?;

    if args.verbose {
        println!("Found package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("Version: {}", version.as_str());
        }
        println!();
    }

    // Discover plugins
    let discovery_result = discover_plugins(&package, args).await?;

    // Display results
    display_plugins_result(&discovery_result, args.verbose);

    Ok(())
}

/// Setup repository manager
async fn setup_repositories(args: &PluginsArgs) -> RezCoreResult<RepositoryManager> {
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

/// Find a specific package
async fn find_package(
    repo_manager: &RepositoryManager,
    package_name: &str,
) -> RezCoreResult<Package> {
    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}' not found",
            package_name
        )));
    }

    // Return latest version (first in list)
    Ok((*packages.into_iter().next().unwrap()).clone())
}

/// Discover plugins for a package
async fn discover_plugins(
    package: &Package,
    args: &PluginsArgs,
) -> RezCoreResult<PluginDiscoveryResult> {
    let mut plugins = Vec::new();

    if args.verbose {
        println!("🔍 Searching for plugins in package '{}'...", package.name);
    }

    // Check for common plugin types based on package tools and structure
    plugins.extend(discover_command_plugins(package)?);
    plugins.extend(discover_build_plugins(package)?);
    plugins.extend(discover_shell_plugins(package)?);
    plugins.extend(discover_extension_plugins(package)?);

    if args.verbose {
        println!("Found {} plugins", plugins.len());
    }

    let total_found = plugins.len();

    Ok(PluginDiscoveryResult {
        plugins,
        package_name: package.name.clone(),
        total_found,
    })
}

/// Discover command plugins
fn discover_command_plugins(package: &Package) -> RezCoreResult<Vec<PluginInfo>> {
    let mut plugins = Vec::new();

    // Check if package provides command-line tools
    for tool in &package.tools {
        plugins.push(PluginInfo {
            name: tool.clone(),
            package_name: package.name.clone(),
            package_version: package.version.as_ref().map(|v| v.as_str().to_string()),
            plugin_type: "command".to_string(),
            description: Some(format!("Command-line tool provided by {}", package.name)),
            module_path: None,
        });
    }

    Ok(plugins)
}

/// Discover build system plugins
fn discover_build_plugins(package: &Package) -> RezCoreResult<Vec<PluginInfo>> {
    let mut plugins = Vec::new();

    // Check for common build system indicators
    let build_indicators = [
        ("cmake", "CMake build system"),
        ("make", "Make build system"),
        ("python", "Python build system"),
        ("pip", "Python pip installer"),
        ("setuptools", "Python setuptools"),
        ("poetry", "Python Poetry"),
        ("cargo", "Rust Cargo"),
        ("npm", "Node.js npm"),
        ("yarn", "Node.js Yarn"),
    ];

    for (indicator, description) in &build_indicators {
        if package.tools.iter().any(|tool| tool.contains(indicator))
            || package.requires.iter().any(|req| req.contains(indicator))
        {
            plugins.push(PluginInfo {
                name: format!("{}_build", indicator),
                package_name: package.name.clone(),
                package_version: package.version.as_ref().map(|v| v.as_str().to_string()),
                plugin_type: "build_system".to_string(),
                description: Some(description.to_string()),
                module_path: None,
            });
        }
    }

    Ok(plugins)
}

/// Discover shell plugins
fn discover_shell_plugins(package: &Package) -> RezCoreResult<Vec<PluginInfo>> {
    let mut plugins = Vec::new();

    // Check for shell-related tools
    let shell_tools = ["bash", "zsh", "fish", "powershell", "cmd"];

    for tool in &package.tools {
        if shell_tools.iter().any(|shell| tool.contains(shell)) {
            plugins.push(PluginInfo {
                name: format!("{}_shell", tool),
                package_name: package.name.clone(),
                package_version: package.version.as_ref().map(|v| v.as_str().to_string()),
                plugin_type: "shell".to_string(),
                description: Some(format!("Shell integration for {}", tool)),
                module_path: None,
            });
        }
    }

    Ok(plugins)
}

/// Discover extension plugins
fn discover_extension_plugins(package: &Package) -> RezCoreResult<Vec<PluginInfo>> {
    let mut plugins = Vec::new();

    // Check for common extension patterns
    if package.name.contains("plugin") || package.name.contains("extension") {
        plugins.push(PluginInfo {
            name: package.name.clone(),
            package_name: package.name.clone(),
            package_version: package.version.as_ref().map(|v| v.as_str().to_string()),
            plugin_type: "extension".to_string(),
            description: package.description.clone(),
            module_path: None,
        });
    }

    Ok(plugins)
}

/// Display plugins discovery result
fn display_plugins_result(result: &PluginDiscoveryResult, verbose: bool) {
    if result.plugins.is_empty() {
        eprintln!("package '{}' has no plugins.", result.package_name);
        return;
    }

    if verbose {
        println!("📋 Plugins found for package '{}':", result.package_name);
        println!("{}", "=".repeat(60));
        println!();

        // Group plugins by type
        let mut plugins_by_type: HashMap<String, Vec<&PluginInfo>> = HashMap::new();
        for plugin in &result.plugins {
            plugins_by_type
                .entry(plugin.plugin_type.clone())
                .or_insert_with(Vec::new)
                .push(plugin);
        }

        for (plugin_type, plugins) in plugins_by_type {
            println!(
                "🔧 {} Plugins:",
                plugin_type.replace('_', " ").to_uppercase()
            );
            println!("{}", "-".repeat(40));

            for plugin in plugins {
                println!("  • {}", plugin.name);
                if let Some(ref description) = plugin.description {
                    println!("    Description: {}", description);
                }
                if let Some(ref version) = plugin.package_version {
                    println!("    Version: {}", version);
                }
                println!();
            }
        }

        println!("✅ Total: {} plugins found", result.total_found);
    } else {
        // Simple output format (compatible with original rez)
        for plugin in &result.plugins {
            println!("{}", plugin.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugins_args_defaults() {
        let args = PluginsArgs {
            package: "test".to_string(),
            paths: vec![],
            verbose: false,
        };

        assert_eq!(args.package, "test");
        assert!(!args.verbose);
    }

    #[test]
    fn test_discover_command_plugins() {
        use rez_next_version::Version;

        let mut package = Package::new("python".to_string());
        package.version = Some(Version::parse("3.9.0").unwrap());
        package.description = Some("Python interpreter".to_string());
        package.tools = vec!["python".to_string(), "pip".to_string()];

        let plugins = discover_command_plugins(&package).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name, "python");
        assert_eq!(plugins[0].plugin_type, "command");
        assert_eq!(plugins[1].name, "pip");
        assert_eq!(plugins[1].plugin_type, "command");
    }

    #[test]
    fn test_discover_build_plugins() {
        use rez_next_version::Version;

        let mut package = Package::new("cmake".to_string());
        package.version = Some(Version::parse("3.20.0").unwrap());
        package.description = Some("CMake build system".to_string());
        package.tools = vec!["cmake".to_string()];

        let plugins = discover_build_plugins(&package).unwrap();
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "cmake_build");
        assert_eq!(plugins[0].plugin_type, "build_system");
    }
}
