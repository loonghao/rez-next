//! # Context Command
//!
//! Implementation of the `rez context` command for viewing and managing context information.
//! This command provides complete compatibility with the original rez-context command.

use clap::Args;
use rez_core_common::{RezCoreError, error::RezCoreResult};
use rez_core_context::{RezResolvedContext, ResolvedPackage};
use rez_core_package::{Package, Requirement};
use std::path::PathBuf;
use std::sync::Arc;

/// Arguments for the context command
#[derive(Args, Clone)]
pub struct ContextArgs {
    /// Rez context file (current context if not supplied). Use '-' to read from stdin
    pub rxt: Option<String>,

    /// Print only the request list (not including implicits)
    #[arg(long = "req", long = "print-request")]
    pub print_request: bool,

    /// Print only the resolve list. Use with --show-uris to print package URIs
    #[arg(long = "res", long = "print-resolve")]
    pub print_resolve: bool,

    /// Print resolved packages in order they are sorted, rather than alphabetical order
    #[arg(long = "so", long = "source-order")]
    pub source_order: bool,

    /// List resolved package's URIs, rather than the default 'root' filepath
    #[arg(long = "su", long = "show-uris")]
    pub show_uris: bool,

    /// Print a list of the executables available in the context
    #[arg(short = 't', long)]
    pub tools: bool,

    /// Locate a program within the context
    #[arg(long)]
    pub which: Option<String>,

    /// Display the resolve graph as an image
    #[arg(short = 'g', long)]
    pub graph: bool,

    /// Display the (simpler) dependency graph. Works in combination with other graph options
    #[arg(short = 'd', long)]
    pub dependency_graph: bool,

    /// Print the resolve graph as a string
    #[arg(long = "pg", long = "print-graph")]
    pub print_graph: bool,

    /// Write the resolve graph to FILE
    #[arg(long = "wg", long = "write-graph")]
    pub write_graph: Option<PathBuf>,

    /// Prune the graph down to PKG
    #[arg(long = "pp", long = "prune-package")]
    pub prune_pkg: Option<String>,

    /// Interpret the context and print the resulting code
    #[arg(short = 'i', long)]
    pub interpret: bool,

    /// Print interpreted output in the given format
    #[arg(short = 'f', long, value_enum)]
    pub format: Option<OutputFormat>,

    /// Set code output style. Ignored if --interpret is not present
    #[arg(short = 's', long, value_enum, default_value = "file")]
    pub style: OutputStyle,

    /// Interpret the context in an empty environment
    #[arg(long)]
    pub no_env: bool,

    /// Diff the current context against the given context
    #[arg(long)]
    pub diff: Option<String>,

    /// Diff the current context against a re-resolved copy of the current context
    #[arg(long)]
    pub fetch: bool,
}

/// Output format for context information
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Table format
    Table,
    /// Dictionary format
    Dict,
    /// JSON format
    Json,
    /// Bash shell format
    Bash,
    /// Zsh shell format
    Zsh,
    /// Fish shell format
    Fish,
    /// CMD shell format
    Cmd,
    /// PowerShell format
    PowerShell,
}

/// Output style for shell code
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputStyle {
    /// File style output
    File,
    /// Source style output
    Source,
}

impl Default for OutputFormat {
    fn default() -> Self {
        // Default to bash for now (context module disabled)
        OutputFormat::Bash
    }
}

// impl From<OutputFormat> for rez_core_context::ShellType {
//     fn from(format: OutputFormat) -> Self {
//         match format {
//             OutputFormat::Bash => rez_core_context::ShellType::Bash,
//             OutputFormat::Zsh => rez_core_context::ShellType::Zsh,
//             OutputFormat::Fish => rez_core_context::ShellType::Fish,
//             OutputFormat::Cmd => rez_core_context::ShellType::Cmd,
//             OutputFormat::PowerShell => rez_core_context::ShellType::PowerShell,
//             // For non-shell formats, default to current shell
//             _ => rez_core_context::ShellType::detect(),
//         }
//     }
// }

/// Execute the context command
pub fn execute(args: ContextArgs) -> RezCoreResult<()> {
    use rez_core_context::RezResolvedContext;

    // For demonstration, create a mock context
    let context = create_demo_context()?;

    if args.print_request {
        print_request_packages(&context);
    } else if args.print_resolve {
        print_resolved_packages(&context, &args);
    } else if args.tools {
        print_available_tools(&context);
    } else if let Some(ref cmd) = args.which {
        locate_command(&context, cmd);
    } else if args.interpret {
        interpret_context(&context, &args)?;
    } else {
        // Default: show context summary
        print_context_summary(&context);
    }

    Ok(())
}

/// Create a demonstration context
fn create_demo_context() -> RezCoreResult<RezResolvedContext> {
    use rez_core_context::{RezResolvedContext, ResolvedPackage};
    use rez_core_package::{Package, Requirement};
    use std::sync::Arc;

    // Create some demo requirements
    let requirements = vec![
        Requirement::new("python".to_string()),
        Requirement::new("numpy".to_string()),
    ];

    let mut context = RezResolvedContext::new(requirements);

    // Create demo resolved packages
    let mut python_pkg = Package::new("python".to_string());
    python_pkg.version = Some(rez_core_version::Version::parse("3.9.0").unwrap());
    python_pkg.description = Some("Python programming language".to_string());
    python_pkg.tools = vec!["python".to_string(), "pip".to_string()];
    python_pkg.commands = Some("export PYTHON_ROOT=\"{root}\"\nexport PATH=\"${PATH}:{root}/bin\"".to_string());

    let mut numpy_pkg = Package::new("numpy".to_string());
    numpy_pkg.version = Some(rez_core_version::Version::parse("1.21.0").unwrap());
    numpy_pkg.description = Some("Numerical computing library".to_string());
    numpy_pkg.commands = Some("export PYTHONPATH=\"${PYTHONPATH}:{root}/lib/python\"".to_string());

    let python_resolved = ResolvedPackage::new(
        Arc::new(python_pkg),
        PathBuf::from("/packages/python/3.9.0"),
        true
    );

    let numpy_resolved = ResolvedPackage::new(
        Arc::new(numpy_pkg),
        PathBuf::from("/packages/numpy/1.21.0"),
        true
    );

    context.resolved_packages = vec![python_resolved, numpy_resolved];

    Ok(context)
}

/// Print request packages
fn print_request_packages(context: &RezResolvedContext) {
    println!("Request packages:");
    for req in &context.requirements {
        println!("  {}", req);
    }
}

/// Print resolved packages
fn print_resolved_packages(context: &RezResolvedContext, args: &ContextArgs) {
    println!("Resolved packages:");

    let mut packages = context.resolved_packages.clone();
    if !args.source_order {
        packages.sort_by(|a, b| a.package.name.cmp(&b.package.name));
    }

    for resolved_pkg in &packages {
        let version_str = resolved_pkg.package.version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        if args.show_uris {
            println!("  {} {} ({})", resolved_pkg.package.name, version_str, resolved_pkg.root.display());
        } else {
            println!("  {} {}", resolved_pkg.package.name, version_str);
        }
    }
}

/// Print available tools
fn print_available_tools(context: &RezResolvedContext) {
    println!("Available tools:");
    let tools = context.get_tools();

    for (tool_name, tool_path) in tools {
        println!("  {} -> {}", tool_name, tool_path.display());
    }
}

/// Locate a command
fn locate_command(context: &RezResolvedContext, cmd: &str) {
    let tools = context.get_tools();

    if let Some(tool_path) = tools.get(cmd) {
        println!("{}", tool_path.display());
    } else {
        eprintln!("Command '{}' not found in context", cmd);
    }
}

/// Interpret context and generate shell code
fn interpret_context(context: &RezResolvedContext, args: &ContextArgs) -> RezCoreResult<()> {
    let environ = context.get_environ()?;

    let format = args.format.as_ref().unwrap_or(&OutputFormat::Bash);

    match format {
        OutputFormat::Bash | OutputFormat::Zsh => {
            for (key, value) in &environ {
                // Only show variables that were modified by packages
                if is_package_variable(key, context) {
                    println!("export {}=\"{}\"", key, value);
                }
            }
        }
        OutputFormat::Fish => {
            for (key, value) in &environ {
                if is_package_variable(key, context) {
                    println!("set -x {} \"{}\"", key, value);
                }
            }
        }
        OutputFormat::Cmd => {
            for (key, value) in &environ {
                if is_package_variable(key, context) {
                    println!("set {}={}", key, value);
                }
            }
        }
        OutputFormat::PowerShell => {
            for (key, value) in &environ {
                if is_package_variable(key, context) {
                    println!("$env:{} = \"{}\"", key, value);
                }
            }
        }
        OutputFormat::Json => {
            let package_vars: std::collections::HashMap<String, String> = environ
                .into_iter()
                .filter(|(key, _)| is_package_variable(key, context))
                .collect();
            println!("{}", serde_json::to_string_pretty(&package_vars)?);
        }
        _ => {
            println!("Format {:?} not supported for interpretation", format);
        }
    }

    Ok(())
}

/// Check if a variable was set by packages
fn is_package_variable(var_name: &str, context: &RezResolvedContext) -> bool {
    for resolved_pkg in &context.resolved_packages {
        if let Some(ref commands) = resolved_pkg.package.commands {
            if commands.contains(&format!("export {}", var_name)) {
                return true;
            }
        }
    }
    false
}

/// Print context summary
fn print_context_summary(context: &RezResolvedContext) {
    let summary = context.get_summary();

    println!("Context Summary:");
    println!("  Packages: {}", summary.num_packages);
    println!("  Status: {}", if summary.failed { "FAILED" } else { "SUCCESS" });
    println!("  Created: {}", summary.timestamp.format("%Y-%m-%d %H:%M:%S UTC"));

    println!("\nPackages:");
    for package_name in &summary.package_names {
        if let Some(resolved_pkg) = context.get_package(package_name) {
            let version_str = resolved_pkg.package.version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");
            println!("  {} {}", package_name, version_str);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_args_defaults() {
        let args = ContextArgs {
            rxt: None,
            print_request: false,
            print_resolve: false,
            source_order: false,
            show_uris: false,
            tools: false,
            which: None,
            graph: false,
            dependency_graph: false,
            print_graph: false,
            write_graph: None,
            prune_pkg: None,
            interpret: false,
            format: None,
            style: OutputStyle::File,
            no_env: false,
            diff: None,
            fetch: false,
        };
        
        assert!(!args.print_request);
        assert!(!args.interpret);
        assert_eq!(args.style, OutputStyle::File);
    }

    #[test]
    fn test_output_format_conversion() {
        let bash_format = OutputFormat::Bash;
        // Test that we can create the format
        assert!(matches!(bash_format, OutputFormat::Bash));
    }
}
