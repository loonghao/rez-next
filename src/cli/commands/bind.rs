//! Bind command implementation
//!
//! Implements the `rez bind` command for converting system software into rez packages.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Arguments for the bind command
#[derive(Args, Clone, Debug)]
pub struct BindArgs {
    /// Package to bind (supports version ranges like 'python-3.9+')
    #[arg(value_name = "PKG")]
    pub package: Option<String>,

    /// Bind a set of standard packages to get started
    #[arg(long)]
    pub quickstart: bool,

    /// Install to release path; overrides -i
    #[arg(short = 'r', long)]
    pub release: bool,

    /// Install path, defaults to local package path
    #[arg(short = 'i', long = "install-path", value_name = "PATH")]
    pub install_path: Option<PathBuf>,

    /// Do not bind dependencies
    #[arg(long = "no-deps")]
    pub no_deps: bool,

    /// List all available bind modules
    #[arg(short = 'l', long)]
    pub list: bool,

    /// Search for the bind module but do not perform the bind
    #[arg(short = 's', long)]
    pub search: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Additional bind arguments
    #[arg(last = true)]
    pub bind_args: Vec<String>,
}

/// Bind module information
#[derive(Debug, Clone)]
pub struct BindModule {
    /// Module name
    pub name: String,
    /// Module file path
    pub path: PathBuf,
    /// Module description
    pub description: Option<String>,
    /// Supported platforms
    pub platforms: Vec<String>,
}

/// Bind result
#[derive(Debug, Clone)]
pub struct BindResult {
    /// Bound package
    pub package: Package,
    /// Installation path
    pub install_path: PathBuf,
    /// Success status
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
}

/// Execute the bind command
pub fn execute(args: BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔗 Rez Bind - Converting system software to rez packages...");
    }

    // Handle list option
    if args.list {
        return list_bind_modules(&args);
    }

    // Handle quickstart option
    if args.quickstart {
        return execute_quickstart(&args);
    }

    // Handle search option
    if args.search {
        if let Some(ref package) = args.package {
            return search_bind_module(package, &args);
        } else {
            return Err(RezCoreError::RequirementParse(
                "Package name required for search".to_string(),
            ));
        }
    }

    // Handle package binding
    if let Some(ref package) = args.package {
        return bind_package(package, &args);
    }

    // No action specified
    eprintln!("Error: No action specified. Use --help for usage information.");
    std::process::exit(1);
}

/// List all available bind modules
fn list_bind_modules(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("📋 Listing available bind modules...");
    }

    let modules = get_bind_modules()?;

    if modules.is_empty() {
        println!("No bind modules found.");
        return Ok(());
    }

    // Print header
    println!("{:<20} {:<50}", "PACKAGE", "BIND MODULE");
    println!("{:<20} {:<50}", "-------", "-----------");

    // Print modules
    for (name, module) in modules.iter() {
        println!("{:<20} {:<50}", name, module.path.display());
    }

    if args.verbose {
        println!("\n✅ Found {} bind modules.", modules.len());
    }

    Ok(())
}

/// Execute quickstart binding
fn execute_quickstart(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🚀 Starting quickstart binding...");
    }

    // Standard packages in dependency order
    let quickstart_packages = vec![
        "platform",
        "arch",
        "os",
        "python",
        "rez",
        "rezgui",
        "setuptools",
        "pip",
    ];

    let install_path = get_install_path(args)?;
    let mut results = Vec::new();

    for package_name in quickstart_packages {
        println!(
            "Binding {} into {}...",
            package_name,
            install_path.display()
        );

        match bind_single_package(package_name, &install_path, true, args) {
            Ok(result) => {
                if result.success {
                    results.push(result);
                    if args.verbose {
                        println!("✅ Successfully bound {}", package_name);
                    }
                } else {
                    eprintln!(
                        "⚠️  Failed to bind {}: {}",
                        package_name,
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                }
            }
            Err(e) => {
                eprintln!("❌ Error binding {}: {}", package_name, e);
            }
        }
    }

    if !results.is_empty() {
        println!("\n✅ Successfully converted the following software found on the current system into Rez packages:");
        println!();
        print_package_list(&results);
    }

    println!("\nTo bind other software, see what's available using the command 'rez bind --list', then run 'rez bind <package>'.\n");

    Ok(())
}

/// Search for a bind module
fn search_bind_module(package_name: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔍 Searching for bind module: {}", package_name);
    }

    let modules = get_bind_modules()?;

    if let Some(module) = modules.get(package_name) {
        println!("Found bind module for '{}':", package_name);
        println!("  Path: {}", module.path.display());
        if let Some(ref desc) = module.description {
            println!("  Description: {}", desc);
        }
        if !module.platforms.is_empty() {
            println!("  Platforms: {}", module.platforms.join(", "));
        }
    } else {
        println!("'{}' not found.", package_name);

        // Suggest close matches
        let close_matches = find_close_matches(package_name, &modules);
        if !close_matches.is_empty() {
            println!("Close matches:");
            for (name, _) in close_matches {
                println!("  {}", name);
            }
        } else {
            println!("No matches.");
        }
    }

    Ok(())
}

/// Bind a specific package
fn bind_package(package_spec: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔗 Binding package: {}", package_spec);
    }

    // Parse package specification (name and optional version range)
    let (package_name, _version_range) = parse_package_spec(package_spec)?;

    let install_path = get_install_path(args)?;

    match bind_single_package(&package_name, &install_path, args.no_deps, args) {
        Ok(result) => {
            if result.success {
                println!("✅ Successfully bound package '{}'", package_name);
                println!("   Installed to: {}", result.install_path.display());

                if args.verbose {
                    println!("   Package details:");
                    println!("     Name: {}", result.package.name);
                    if let Some(ref version) = result.package.version {
                        println!("     Version: {}", version.as_str());
                    }
                    if let Some(ref description) = result.package.description {
                        println!("     Description: {}", description);
                    }
                }
            } else {
                eprintln!(
                    "❌ Failed to bind package '{}': {}",
                    package_name,
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("❌ Error binding package '{}': {}", package_name, e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Get available bind modules
fn get_bind_modules() -> RezCoreResult<HashMap<String, BindModule>> {
    let mut modules = HashMap::new();

    // Built-in bind modules (simplified for now)
    let builtin_modules = vec![
        ("platform", "System platform package"),
        ("arch", "System architecture package"),
        ("os", "Operating system package"),
        ("python", "Python interpreter"),
        ("rez", "Rez package manager"),
        ("rezgui", "Rez GUI"),
        ("setuptools", "Python setuptools"),
        ("pip", "Python pip"),
        ("cmake", "CMake build system"),
        ("git", "Git version control"),
        ("gcc", "GNU Compiler Collection"),
        ("clang", "Clang compiler"),
    ];

    for (name, description) in builtin_modules {
        modules.insert(
            name.to_string(),
            BindModule {
                name: name.to_string(),
                path: PathBuf::from(format!("builtin://{}", name)),
                description: Some(description.to_string()),
                platforms: vec![
                    "windows".to_string(),
                    "linux".to_string(),
                    "darwin".to_string(),
                ],
            },
        );
    }

    // TODO: Scan for external bind modules in bind paths

    Ok(modules)
}

/// Get the installation path
fn get_install_path(args: &BindArgs) -> RezCoreResult<PathBuf> {
    if args.release {
        // TODO: Get from config
        Ok(PathBuf::from("./release_packages"))
    } else if let Some(ref path) = args.install_path {
        Ok(path.clone())
    } else {
        // TODO: Get from config
        Ok(PathBuf::from("./local_packages"))
    }
}

/// Parse package specification
fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    // Simple parsing for now - just split on '-' for version
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

/// Bind a single package
fn bind_single_package(
    name: &str,
    install_path: &Path,
    no_deps: bool,
    args: &BindArgs,
) -> RezCoreResult<BindResult> {
    // TODO: Implement actual binding logic
    // For now, create a mock package

    let package = Package {
        name: name.to_string(),
        version: Some(rez_next_version::Version::parse("1.0.0")?),
        description: Some(format!("System package: {}", name)),
        authors: vec!["System".to_string()],
        requires: if no_deps {
            vec![]
        } else {
            get_default_requirements(name)
        },
        build_requires: vec![],
        private_build_requires: vec![],
        variants: vec![],
        commands: None,
        build_command: None,
        build_system: None,
        pre_commands: None,
        post_commands: None,
        pre_test_commands: None,
        pre_build_commands: None,
        tests: HashMap::new(),
        requires_rez_version: None,
        tools: get_default_tools(name),
        help: None,
        uuid: None,
        config: HashMap::new(),
        plugin_for: vec![],
        has_plugins: None,
        relocatable: None,
        cachable: None,
        release_message: None,
        changelog: None,
        previous_version: None,
        previous_revision: None,
        revision: None,
        timestamp: None,
        format_version: None,
        base: None,
        hashed_variants: None,
        vcs: None,
        preprocess: None,
    };

    Ok(BindResult {
        package,
        install_path: install_path.to_path_buf(),
        success: true,
        error: None,
    })
}

/// Get default requirements for a package
fn get_default_requirements(name: &str) -> Vec<String> {
    match name {
        "os" => vec!["platform".to_string(), "arch".to_string()],
        "python" => vec!["os".to_string()],
        "pip" => vec!["python".to_string()],
        "setuptools" => vec!["python".to_string()],
        _ => vec![],
    }
}

/// Get default tools for a package
fn get_default_tools(name: &str) -> Vec<String> {
    match name {
        "python" => vec!["python".to_string(), "python3".to_string()],
        "pip" => vec!["pip".to_string()],
        "cmake" => vec!["cmake".to_string()],
        "git" => vec!["git".to_string()],
        "gcc" => vec!["gcc".to_string(), "g++".to_string()],
        "clang" => vec!["clang".to_string(), "clang++".to_string()],
        _ => vec![],
    }
}

/// Find close matches for a package name
fn find_close_matches<'a>(
    name: &str,
    modules: &'a HashMap<String, BindModule>,
) -> Vec<(String, &'a BindModule)> {
    let mut matches = Vec::new();

    for (module_name, module) in modules {
        if module_name.contains(name) || name.contains(module_name) {
            matches.push((module_name.clone(), module));
        }
    }

    matches.sort_by(|a, b| a.0.cmp(&b.0));
    matches
}

/// Print package list
fn print_package_list(results: &[BindResult]) {
    println!("{:<20} {:<50}", "PACKAGE", "URI");
    println!("{:<20} {:<50}", "-------", "---");

    for result in results {
        let uri = format!("file://{}", result.install_path.display());
        println!("{:<20} {:<50}", result.package.name, uri);
    }
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
    fn test_get_default_requirements() {
        assert_eq!(get_default_requirements("platform"), Vec::<String>::new());
        assert_eq!(get_default_requirements("os"), vec!["platform", "arch"]);
        assert_eq!(get_default_requirements("python"), vec!["os"]);
    }

    #[test]
    fn test_get_default_tools() {
        assert_eq!(get_default_tools("platform"), Vec::<String>::new());
        assert_eq!(get_default_tools("python"), vec!["python", "python3"]);
        assert_eq!(get_default_tools("cmake"), vec!["cmake"]);
    }
}
