//! # View Command
//!
//! Implementation of the `rez view` command for viewing package information.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::{Package, PackageSerializer};
use std::path::Path;

/// Arguments for the view command
#[derive(Args, Clone)]
pub struct ViewArgs {
    /// Package to view
    pub package: String,

    /// Format to print the package in
    #[arg(short, long, value_enum, default_value = "yaml")]
    pub format: ViewFormat,

    /// Show all package data, including release-related fields
    #[arg(short, long)]
    pub all: bool,

    /// Do not print extraneous info, such as package uri
    #[arg(short, long)]
    pub brief: bool,

    /// Show the package in the current context, if any
    #[arg(short, long)]
    pub current: bool,
}

/// Output format for package viewing
#[derive(clap::ValueEnum, Clone)]
pub enum ViewFormat {
    /// YAML format
    Yaml,
    /// Python format
    Py,
}

/// Execute the view command
pub fn execute(args: ViewArgs) -> RezCoreResult<()> {
    // Validate package name
    crate::cli::utils::validate_package_name(&args.package)?;

    if args.current {
        return view_current_package(&args);
    }

    view_package(&args)
}

/// View a package from the current context
fn view_current_package(args: &ViewArgs) -> RezCoreResult<()> {
    // TODO: Implement current context package viewing
    // This requires integration with rez-core-context

    eprintln!("Error: not in a resolved environment context.");
    Err(RezCoreError::Repository(
        "Not in a resolved environment context".to_string(),
    ))
}

/// View a package from repositories
fn view_package(args: &ViewArgs) -> RezCoreResult<()> {
    // Try to load package from directory first
    let path = Path::new(&args.package);

    let package = if path.exists() && path.is_dir() {
        // Load from directory containing package.py
        load_package_from_directory(path)?
    } else {
        // TODO: Implement package loading from repositories
        // For now, create a mock package for demonstration
        create_mock_package(&args.package)?
    };

    display_package(&package, args)
}

/// Load package from directory containing package.py
fn load_package_from_directory(dir_path: &Path) -> RezCoreResult<Package> {
    let package_py_path = dir_path.join("package.py");

    if !package_py_path.exists() {
        return Err(RezCoreError::PackageParse(format!(
            "No package.py found in directory: {}",
            dir_path.display()
        )));
    }

    // Load the package using PackageSerializer
    PackageSerializer::load_from_file(&package_py_path)
}

/// Create a mock package for demonstration purposes
fn create_mock_package(name: &str) -> RezCoreResult<Package> {
    // TODO: Replace with actual package loading from repository

    // Parse package name and version if provided
    let (pkg_name, version) = if let Some(pos) = name.find('-') {
        let pkg_name = &name[..pos];
        let version_str = &name[pos + 1..];
        (pkg_name, Some(version_str))
    } else {
        (name, None)
    };

    // Create a mock package
    let mut package = Package::new(pkg_name.to_string());

    if let Some(version_str) = version {
        use rez_next_version::Version;
        let version =
            Version::parse(version_str).map_err(|e| RezCoreError::VersionParse(e.to_string()))?;
        package.set_version(version);
    }

    // Add some mock metadata
    package.set_description(format!("Mock package for {}", pkg_name));

    Ok(package)
}

/// Display package information in the requested format
fn display_package(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    match args.format {
        ViewFormat::Yaml => display_package_yaml(package, args),
        ViewFormat::Py => display_package_python(package, args),
    }
}

/// Display package in YAML format
fn display_package_yaml(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    if !args.brief {
        println!("# Package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("# Version: {}", version.as_str());
        }
        println!();
    }

    println!("name: {}", package.name);

    if let Some(ref version) = package.version {
        println!("version: {}", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description: {}", description);
    }

    if !package.authors.is_empty() {
        println!("authors:");
        for author in &package.authors {
            println!("  - {}", author);
        }
    }

    if !package.requires.is_empty() {
        println!("requires:");
        for req in &package.requires {
            println!("  - {}", req);
        }
    }

    if !package.tools.is_empty() {
        println!("tools:");
        for tool in &package.tools {
            println!("  - {}", tool);
        }
    }

    if !package.variants.is_empty() {
        println!("variants:");
        for variant in &package.variants {
            print!("  - [");
            for (i, req) in variant.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}", req);
            }
            println!("]");
        }
    }

    if let Some(ref build_command) = package.build_command {
        println!("build_command: {}", build_command);
    }

    if let Some(ref build_system) = package.build_system {
        println!("build_system: {}", build_system);
    }

    if let Some(ref uuid) = package.uuid {
        println!("uuid: {}", uuid);
    }

    if let Some(ref commands) = package.commands {
        println!("commands: |");
        for line in commands.lines() {
            println!("  {}", line);
        }
    }

    if args.all {
        // Show additional fields with --all
        if let Some(ref pre_commands) = package.pre_commands {
            println!("pre_commands: {}", pre_commands);
        }
        if let Some(ref post_commands) = package.post_commands {
            println!("post_commands: {}", post_commands);
        }
        if !package.tests.is_empty() {
            println!("tests:");
            for (key, value) in &package.tests {
                println!("  {}: {}", key, value);
            }
        }
    }

    Ok(())
}

/// Display package in Python format
fn display_package_python(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    if !args.brief {
        println!("# Package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("# Version: {}", version.as_str());
        }
        println!();
    }

    println!("name = \"{}\"", package.name);

    if let Some(ref version) = package.version {
        println!("version = \"{}\"", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description = \"{}\"", description);
    }

    if !package.authors.is_empty() {
        print!("authors = [");
        for (i, author) in package.authors.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", author);
        }
        println!("]");
    }

    if !package.requires.is_empty() {
        print!("requires = [");
        for (i, req) in package.requires.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", req);
        }
        println!("]");
    }

    if !package.tools.is_empty() {
        print!("tools = [");
        for (i, tool) in package.tools.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", tool);
        }
        println!("]");
    }

    if !package.variants.is_empty() {
        println!("variants = [");
        for variant in &package.variants {
            print!("    [");
            for (i, req) in variant.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("\"{}\"", req);
            }
            println!("],");
        }
        println!("]");
    }

    if let Some(ref build_command) = package.build_command {
        println!("build_command = \"{}\"", build_command);
    }

    if let Some(ref build_system) = package.build_system {
        println!("build_system = \"{}\"", build_system);
    }

    if let Some(ref uuid) = package.uuid {
        println!("uuid = \"{}\"", uuid);
    }

    if args.all {
        // Show additional fields with --all
        if let Some(ref pre_commands) = package.pre_commands {
            println!("pre_commands = \"{}\"", pre_commands);
        }
        if let Some(ref post_commands) = package.post_commands {
            println!("post_commands = \"{}\"", post_commands);
        }
        if !package.tests.is_empty() {
            println!("tests = {{");
            for (key, value) in &package.tests {
                println!("    \"{}\": \"{}\",", key, value);
            }
            println!("}}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_args_parsing() {
        let args = ViewArgs {
            package: "test_package".to_string(),
            format: ViewFormat::Yaml,
            all: false,
            brief: true,
            current: false,
        };

        assert_eq!(args.package, "test_package");
        assert!(args.brief);
        assert!(!args.all);
    }

    #[test]
    fn test_create_mock_package() {
        // Test package name only
        let package = create_mock_package("test_pkg").unwrap();
        assert_eq!(package.name, "test_pkg");

        // Test package with version
        let package = create_mock_package("test_pkg-1.0.0").unwrap();
        assert_eq!(package.name, "test_pkg");
        assert!(package.version.is_some());
    }
}
