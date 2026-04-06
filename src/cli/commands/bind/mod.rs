//! Bind command implementation
//!
//! Implements the `rez bind` command for converting system software into rez packages.
//! This performs actual system detection and writes real package.py files to disk.
//!
//! # Sub-modules
//! - `detect`     — system tool detection (python, git, cmake, …)
//! - `package_gen` — package.py generation + default metadata helpers
//! - `utils`      — version parsing, `which`, close-match search

mod detect;
mod package_gen;
mod utils;

#[cfg(test)]
mod tests;

use crate::cli::utils::expand_home_path;
use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub use detect::DetectedTool;
pub use utils::BindModule;

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
        println!("Rez Bind - Converting system software to rez packages...");
    }

    if args.list {
        return list_bind_modules(&args);
    }

    if args.quickstart {
        return execute_quickstart(&args);
    }

    if args.search {
        if let Some(ref package) = args.package {
            return search_bind_module(package, &args);
        } else {
            return Err(RezCoreError::RequirementParse(
                "Package name required for search".to_string(),
            ));
        }
    }

    if let Some(ref package) = args.package {
        return bind_package(package, &args);
    }

    eprintln!("Error: No action specified. Use --help for usage information.");
    std::process::exit(1);
}

// ─── Internal helpers ────────────────────────────────────────────────────────

fn list_bind_modules(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Listing available bind modules...");
    }

    let modules = utils::get_bind_modules()?;

    if modules.is_empty() {
        println!("No bind modules found.");
        return Ok(());
    }

    println!("{:<20} {:<50}", "PACKAGE", "BIND MODULE");
    println!("{:<20} {:<50}", "-------", "-----------");

    for (name, module) in modules.iter() {
        println!("{:<20} {:<50}", name, module.path.display());
    }

    if args.verbose {
        println!("\nFound {} bind modules.", modules.len());
    }

    Ok(())
}

fn execute_quickstart(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Starting quickstart binding...");
    }

    let quickstart_packages = vec![
        "platform",
        "arch",
        "os",
        "python",
        "rez",
        "setuptools",
        "pip",
    ];

    let install_path = get_install_path(args)?;
    std::fs::create_dir_all(&install_path).map_err(RezCoreError::Io)?;

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
                    if args.verbose {
                        println!("  Successfully bound {}", package_name);
                    }
                    results.push(result);
                } else {
                    eprintln!(
                        "  Warning: Failed to bind {}: {}",
                        package_name,
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                }
            }
            Err(e) => {
                eprintln!("  Error binding {}: {}", package_name, e);
            }
        }
    }

    if !results.is_empty() {
        println!(
            "\nSuccessfully converted the following software found on the current system into Rez packages:"
        );
        println!();
        print_package_list(&results);
    }

    println!(
        "\nTo bind other software, see what's available using the command 'rez bind --list', \
         then run 'rez bind <package>'.\n"
    );

    Ok(())
}

fn search_bind_module(package_name: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Searching for bind module: {}", package_name);
    }

    let modules = utils::get_bind_modules()?;

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

        let close_matches = utils::find_close_matches(package_name, &modules);
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

fn bind_package(package_spec: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Binding package: {}", package_spec);
    }

    let (package_name, _version_range) = parse_package_spec(package_spec)?;
    let install_path = get_install_path(args)?;
    std::fs::create_dir_all(&install_path).map_err(RezCoreError::Io)?;

    match bind_single_package(&package_name, &install_path, args.no_deps, args) {
        Ok(result) => {
            if result.success {
                println!("Successfully bound package '{}'", package_name);
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
                    "Failed to bind package '{}': {}",
                    package_name,
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error binding package '{}': {}", package_name, e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Parse package specification, e.g. `"python-3.9"` → `("python", Some("3.9"))`.
pub(crate) fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    if let Some(dash_pos) = spec.rfind('-') {
        let name = spec[..dash_pos].to_string();
        let version = spec[dash_pos + 1..].to_string();

        if version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

fn get_install_path(args: &BindArgs) -> RezCoreResult<PathBuf> {
    let config = RezCoreConfig::load();
    if args.release {
        Ok(expand_home_path(&config.release_packages_path))
    } else if let Some(ref path) = args.install_path {
        Ok(path.clone())
    } else {
        Ok(expand_home_path(&config.local_packages_path))
    }
}

fn bind_single_package(
    name: &str,
    install_path: &Path,
    no_deps: bool,
    args: &BindArgs,
) -> RezCoreResult<BindResult> {
    let detected = detect::detect_system_tool(name);

    let version_str = detected
        .as_ref()
        .map(|d| d.version.clone())
        .unwrap_or_else(|| "1.0.0".to_string());

    let exe_path = detected.as_ref().and_then(|d| d.executable_path.clone());

    let requires = if no_deps {
        vec![]
    } else {
        package_gen::get_default_requirements(name)
    };

    let tools = package_gen::get_default_tools(name);
    let description = package_gen::get_package_description(name);
    let commands = package_gen::get_package_commands(name, exe_path.as_ref());

    let version = rez_next_version::Version::parse(&version_str)?;

    let package = Package {
        name: name.to_string(),
        version: Some(version.clone()),
        description: Some(description.clone()),
        authors: vec!["System".to_string()],
        requires: requires.clone(),
        build_requires: vec![],
        private_build_requires: vec![],
        variants: vec![],
        commands: commands.clone(),
        commands_function: commands.clone(),
        build_command: None,
        build_system: None,
        pre_commands: None,
        post_commands: None,
        pre_test_commands: None,
        pre_build_commands: None,
        tests: HashMap::new(),
        requires_rez_version: None,
        tools: tools.clone(),
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
        timestamp: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        ),
        format_version: Some(2),
        base: None,
        hashed_variants: None,
        vcs: None,
        preprocess: None,
    };

    let pkg_dir = install_path.join(name).join(&version_str);
    std::fs::create_dir_all(&pkg_dir).map_err(RezCoreError::Io)?;

    let pkg_file = pkg_dir.join("package.py");
    let pkg_content = package_gen::generate_package_py(
        name,
        &version_str,
        &description,
        &requires,
        &tools,
        commands.as_deref(),
    );

    std::fs::write(&pkg_file, &pkg_content).map_err(RezCoreError::Io)?;

    if args.verbose {
        println!("  Wrote {}", pkg_file.display());
    }

    Ok(BindResult {
        package,
        install_path: pkg_dir,
        success: true,
        error: None,
    })
}

fn print_package_list(results: &[BindResult]) {
    println!("{:<20} {:<50}", "PACKAGE", "URI");
    println!("{:<20} {:<50}", "-------", "---");

    for result in results {
        let uri = format!("file://{}", result.install_path.display());
        println!("{:<20} {:<50}", result.package.name, uri);
    }
}
