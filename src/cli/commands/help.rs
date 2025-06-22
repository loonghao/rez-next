//! Package help command implementation
//!
//! Implements the `rez pkg-help` command for displaying package help information.

use clap::Args;
use rez_core_common::{RezCoreError, error::RezCoreResult};
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_core_package::Package;
use std::path::PathBuf;

/// Arguments for the pkg-help command
#[derive(Args, Clone, Debug)]
pub struct PkgHelpArgs {
    /// Package name to get help for
    #[arg(value_name = "PACKAGE")]
    pub package: Option<String>,

    /// Help section to view (1..N)
    #[arg(value_name = "SECTION", default_value = "1")]
    pub section: u32,

    /// Load the rez technical user manual
    #[arg(short = 'm', long = "manual")]
    pub manual: bool,

    /// Just print each help entry
    #[arg(short = 'e', long = "entries")]
    pub entries: bool,

    /// Repository paths to search for packages
    #[arg(long = "paths", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Help section information
#[derive(Debug, Clone)]
pub struct HelpSection {
    pub name: String,
    pub content: String,
}

/// Package help information
#[derive(Debug, Clone)]
pub struct PackageHelp {
    pub package_name: String,
    pub package_version: Option<String>,
    pub description: Option<String>,
    pub sections: Vec<HelpSection>,
}

/// Execute the pkg-help command
pub fn execute(args: PkgHelpArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("📚 Rez Help - Displaying help information...");
    }

    // Handle manual mode or no package specified
    if args.manual || args.package.is_none() {
        return show_rez_manual(&args);
    }

    // Create async runtime for package help
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async {
        execute_package_help_async(&args).await
    })
}

/// Show rez manual or general help
fn show_rez_manual(args: &PkgHelpArgs) -> RezCoreResult<()> {
    if args.manual {
        println!("📖 Rez Technical User Manual");
        println!("============================");
        println!();
        println!("The Rez technical user manual provides comprehensive documentation");
        println!("for using Rez package management system.");
        println!();
        println!("For the complete manual, visit:");
        println!("  https://rez.readthedocs.io/");
        println!();
        println!("Quick Start:");
        println!("  rez env python-3.9    # Create environment with Python 3.9");
        println!("  rez search python     # Search for Python packages");
        println!("  rez build             # Build current package");
        println!("  rez help <package>    # Get help for specific package");
        println!();
        return Ok(());
    }

    // Show general help - list all available commands
    show_command_help(args)
}

/// Show general command help
fn show_command_help(args: &PkgHelpArgs) -> RezCoreResult<()> {
    println!("🚀 Rez Core - High-performance Rez package manager");
    println!("==================================================");
    println!();
    println!("USAGE:");
    println!("    rez <COMMAND> [OPTIONS]");
    println!();
    println!("COMMANDS:");
    println!("    config      Show configuration information");
    println!("    context     Print information about the current rez context");
    println!("    view        View package information");
    println!("    env         Resolve packages and spawn a shell or execute a command");
    println!("    release     Build a package from source and deploy it");
    println!("    test        Run tests defined in a package");
    println!("    build       Build a package from source");
    println!("    search      Search for packages");
    println!("    bind        Bind a system package as a rez package");
    println!("    depends     Show reverse dependencies of a package");
    println!("    solve       Solve package dependencies");
    println!("    cp          Copy packages between repositories");
    println!("    mv          Move packages between repositories");
    println!("    rm          Remove packages from repositories");
    println!("    status      Show package and repository status");
    println!("    diff        Compare packages and show differences");
    println!("    help        Show help information");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print help");
    println!("    -V, --version    Print version");
    println!();
    println!("For more information on a specific command, use:");
    println!("    rez <COMMAND> --help");
    println!();
    println!("For package-specific help, use:");
    println!("    rez help <PACKAGE>");
    println!();

    if args.verbose {
        println!("EXAMPLES:");
        println!("    rez help python           # Get help for Python package");
        println!("    rez help --manual          # Show technical manual");
        println!("    rez help --entries python  # List help sections for Python");
        println!("    rez env --help             # Get help for env command");
        println!();
    }

    Ok(())
}

/// Execute package help asynchronously
async fn execute_package_help_async(args: &PkgHelpArgs) -> RezCoreResult<()> {
    let package_name = args.package.as_ref().unwrap();

    if args.verbose {
        println!("Searching for help in package: {}", package_name);
    }

    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    // Find package with help
    let package_help = find_package_help(&repo_manager, package_name, args).await?;

    // Display help
    if args.entries {
        display_help_entries(&package_help);
    } else {
        display_help_section(&package_help, args.section)?;
    }

    Ok(())
}

/// Setup repository manager
async fn setup_repositories(args: &PkgHelpArgs) -> RezCoreResult<RepositoryManager> {
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

/// Find package help information
async fn find_package_help(
    repo_manager: &RepositoryManager,
    package_name: &str,
    args: &PkgHelpArgs,
) -> RezCoreResult<PackageHelp> {
    let packages = repo_manager.find_packages(package_name).await?;
    
    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(
            format!("Package '{}' not found", package_name)
        ));
    }

    // Find the latest package (first in list)
    let package = packages.into_iter().next().unwrap();
    
    if args.verbose {
        println!("Found package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("Version: {}", version.as_str());
        }
    }

    // Extract help information
    let help_sections = extract_help_sections(&package);
    
    if help_sections.is_empty() {
        return Err(RezCoreError::RequirementParse(
            format!("No help found for package '{}'", package_name)
        ));
    }

    Ok(PackageHelp {
        package_name: package.name.clone(),
        package_version: package.version.as_ref().map(|v| v.as_str().to_string()),
        description: package.description.clone(),
        sections: help_sections,
    })
}

/// Extract help sections from package
fn extract_help_sections(package: &Package) -> Vec<HelpSection> {
    let mut sections = Vec::new();

    // Add description as first help section if available
    if let Some(ref description) = package.description {
        sections.push(HelpSection {
            name: "Description".to_string(),
            content: description.clone(),
        });
    }

    // Add basic package information
    let mut info_content = String::new();
    info_content.push_str(&format!("Package: {}\n", package.name));
    
    if let Some(ref version) = package.version {
        info_content.push_str(&format!("Version: {}\n", version.as_str()));
    }
    
    if !package.authors.is_empty() {
        info_content.push_str(&format!("Authors: {}\n", package.authors.join(", ")));
    }
    
    if !package.requires.is_empty() {
        info_content.push_str(&format!("Requirements: {}\n", package.requires.join(", ")));
    }
    
    if !package.tools.is_empty() {
        info_content.push_str(&format!("Tools: {}\n", package.tools.join(", ")));
    }

    sections.push(HelpSection {
        name: "Package Information".to_string(),
        content: info_content,
    });

    // Add usage section
    let usage_content = format!(
        "To use this package in an environment:\n  rez env {}\n\nTo view package details:\n  rez view {}",
        package.name, package.name
    );
    
    sections.push(HelpSection {
        name: "Usage".to_string(),
        content: usage_content,
    });

    sections
}

/// Display help entries list
fn display_help_entries(package_help: &PackageHelp) {
    println!("Help found for:");
    println!("  {}", package_help.package_name);
    if let Some(ref version) = package_help.package_version {
        println!("  Version: {}", version);
    }
    println!();

    if let Some(ref description) = package_help.description {
        println!("Description:");
        println!("  {}", description);
        println!();
    }

    println!("Sections:");
    for (i, section) in package_help.sections.iter().enumerate() {
        println!("  {}: {}", i + 1, section.name);
    }
    println!();
    println!("Use 'rez help {} <section_number>' to view a specific section.", package_help.package_name);
}

/// Display specific help section
fn display_help_section(package_help: &PackageHelp, section_num: u32) -> RezCoreResult<()> {
    let section_index = (section_num as usize).saturating_sub(1);
    
    if section_index >= package_help.sections.len() {
        return Err(RezCoreError::RequirementParse(
            format!("No such help section {}. Available sections: 1-{}", 
                section_num, package_help.sections.len())
        ));
    }

    let section = &package_help.sections[section_index];
    
    println!("Help for: {}", package_help.package_name);
    if let Some(ref version) = package_help.package_version {
        println!("Version: {}", version);
    }
    println!();
    
    println!("Section {}: {}", section_num, section.name);
    println!("{}", "=".repeat(50));
    println!();
    println!("{}", section.content);
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_args_defaults() {
        let args = PkgHelpArgs {
            package: None,
            section: 1,
            manual: false,
            entries: false,
            paths: vec![],
            verbose: false,
        };

        assert!(args.package.is_none());
        assert_eq!(args.section, 1);
        assert!(!args.manual);
        assert!(!args.entries);
    }

    #[test]
    fn test_extract_help_sections() {
        let package = Package {
            name: "test-package".to_string(),
            version: Some("1.0.0".into()),
            description: Some("A test package".to_string()),
            authors: vec!["Test Author".to_string()],
            requires: vec!["python".to_string()],
            tools: vec!["python".to_string()],
            variants: vec![],
        };

        let sections = extract_help_sections(&package);
        assert_eq!(sections.len(), 3);
        assert_eq!(sections[0].name, "Description");
        assert_eq!(sections[1].name, "Package Information");
        assert_eq!(sections[2].name, "Usage");
    }
}
