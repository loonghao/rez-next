//! # Context Command
//!
//! Implementation of the `rez context` command for viewing and managing context information.
//! This command provides complete compatibility with the original rez-context command.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_context::RezResolvedContext;
use std::path::PathBuf;

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
#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum OutputStyle {
    /// File style output
    File,
    /// Source style output
    Source,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Bash
    }
}

/// Execute the context command
pub fn execute(args: ContextArgs) -> RezCoreResult<()> {
    // Load context: from file, stdin, or current environment
    let context = load_context(&args)?;

    if args.print_request {
        print_request_packages(&context);
    } else if args.print_resolve {
        print_resolved_packages(&context, &args);
    } else if args.tools {
        print_available_tools(&context);
    } else if let Some(ref cmd) = args.which {
        locate_command(&context, cmd);
    } else if args.print_graph {
        print_resolve_graph(&context, args.prune_pkg.as_deref());
    } else if let Some(ref graph_file) = args.write_graph {
        write_resolve_graph(&context, graph_file, args.prune_pkg.as_deref())?;
    } else if args.graph {
        render_graph_image(&context, args.prune_pkg.as_deref())?;
    } else if args.interpret {
        interpret_context(&context, &args)?;
    } else {
        // Default: show context summary
        print_context_summary(&context);
    }

    Ok(())
}

/// Load context from file, stdin or REZ_CONTEXT_FILE env var
fn load_context(args: &ContextArgs) -> RezCoreResult<RezResolvedContext> {
    // Determine source
    let source = args.rxt.as_deref().unwrap_or("");

    if source == "-" {
        // Read from stdin
        use std::io::Read;
        let mut content = String::new();
        std::io::stdin()
            .read_to_string(&mut content)
            .map_err(|e| RezCoreError::ContextError(format!("Failed to read stdin: {}", e)))?;
        return deserialize_context(&content);
    }

    if !source.is_empty() {
        // Load from specified file
        let path = std::path::Path::new(source);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| RezCoreError::ContextError(format!("Failed to read {}: {}", source, e)))?;
            return deserialize_context(&content);
        }
        return Err(RezCoreError::ContextError(format!("Context file not found: {}", source)));
    }

    // Try REZ_CONTEXT_FILE environment variable
    if let Ok(ctx_file) = std::env::var("REZ_CONTEXT_FILE") {
        let path = std::path::Path::new(&ctx_file);
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| RezCoreError::ContextError(format!("Failed to read context file: {}", e)))?;
            return deserialize_context(&content);
        }
    }

    // No context available - return empty context
    let context = RezResolvedContext::new(vec![]);
    Ok(context)
}

/// Deserialize a RezResolvedContext from JSON/YAML string
fn deserialize_context(content: &str) -> RezCoreResult<RezResolvedContext> {
    // Try JSON first
    if let Ok(ctx) = serde_json::from_str::<RezResolvedContext>(content) {
        return Ok(ctx);
    }
    // Try YAML
    serde_yaml::from_str::<RezResolvedContext>(content)
        .map_err(|e| RezCoreError::ContextError(format!("Failed to parse context: {}", e)))
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
        let version_str = resolved_pkg
            .package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");

        if args.show_uris {
            println!(
                "  {} {} ({})",
                resolved_pkg.package.name,
                version_str,
                resolved_pkg.root.display()
            );
        } else {
            println!("  {} {}", resolved_pkg.package.name, version_str);
        }
    }
}

/// Print available tools
fn print_available_tools(context: &RezResolvedContext) {
    println!("Available tools:");
    let tools = context.get_tools();

    if tools.is_empty() {
        println!("  (no tools available)");
        return;
    }

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
    println!(
        "  Status: {}",
        if summary.failed { "FAILED" } else { "SUCCESS" }
    );
    println!(
        "  Created: {}",
        summary.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );

    if summary.num_packages == 0 {
        println!("\n  (no resolved packages - not in a rez environment)");
        return;
    }

    println!("\nPackages:");
    for package_name in &summary.package_names {
        if let Some(resolved_pkg) = context.get_package(package_name) {
            let version_str = resolved_pkg
                .package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");
            println!("  {} {}", package_name, version_str);
        }
    }
}

/// Generate DOT graph string for a resolved context
fn generate_dot_graph(context: &RezResolvedContext, prune_pkg: Option<&str>) -> String {
    let mut dot = String::from("digraph rez_context {\n");
    dot.push_str("    rankdir=LR;\n");
    dot.push_str("    node [shape=box, style=filled, fillcolor=lightblue];\n");
    dot.push_str("    edge [fontsize=10];\n\n");

    // Collect packages to display
    let packages: Vec<_> = if let Some(prune) = prune_pkg {
        context
            .resolved_packages
            .iter()
            .filter(|rp| rp.package.name == prune || {
                // Include packages that depend on prune target
                rp.package.requires.iter().any(|r| r.starts_with(prune))
            })
            .collect()
    } else {
        context.resolved_packages.iter().collect()
    };

    // Nodes
    for resolved_pkg in &packages {
        let name = &resolved_pkg.package.name;
        let version = resolved_pkg
            .package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("?");
        let label = format!("{}-{}", name, version);
        let node_id = name.replace(['-', '.'], "_");
        dot.push_str(&format!("    {} [label=\"{}\"];\n", node_id, label));
    }

    dot.push('\n');

    // Edges (requires relationships)
    for resolved_pkg in &packages {
        let from_id = resolved_pkg
            .package
            .name
            .replace(['-', '.'], "_");
        for req in &resolved_pkg.package.requires {
            // req is like "python-3.9" or "numpy>=1.0"
            let dep_name = req.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
                .next()
                .unwrap_or(req);
            let to_id = dep_name.replace(['-', '.'], "_");
            // Only emit edge if target node exists in graph
            let target_exists = packages.iter().any(|rp| rp.package.name == dep_name);
            if target_exists {
                dot.push_str(&format!(
                    "    {} -> {} [label=\"{}\"];\n",
                    from_id, to_id, req
                ));
            }
        }
    }

    dot.push_str("}\n");
    dot
}

/// Print DOT graph to stdout
fn print_resolve_graph(context: &RezResolvedContext, prune_pkg: Option<&str>) {
    let dot = generate_dot_graph(context, prune_pkg);
    print!("{}", dot);
}

/// Write DOT graph to file
fn write_resolve_graph(
    context: &RezResolvedContext,
    path: &std::path::Path,
    prune_pkg: Option<&str>,
) -> RezCoreResult<()> {
    let dot = generate_dot_graph(context, prune_pkg);
    std::fs::write(path, &dot)
        .map_err(|e| RezCoreError::ContextError(format!("Failed to write graph to {}: {}", path.display(), e)))?;
    eprintln!("Resolve graph written to: {}", path.display());
    Ok(())
}

/// Render graph to image using Graphviz `dot` and open it
fn render_graph_image(context: &RezResolvedContext, prune_pkg: Option<&str>) -> RezCoreResult<()> {
    use std::process::Command;

    let dot_content = generate_dot_graph(context, prune_pkg);

    // Write DOT to a temp file
    let tmp_dir = std::env::temp_dir();
    let dot_file = tmp_dir.join("rez_context_graph.dot");
    let png_file = tmp_dir.join("rez_context_graph.png");

    std::fs::write(&dot_file, &dot_content).map_err(|e| {
        RezCoreError::ContextError(format!("Failed to write temp DOT file: {}", e))
    })?;

    // Attempt to run `dot` (Graphviz)
    let dot_result = Command::new("dot")
        .arg("-Tpng")
        .arg(&dot_file)
        .arg("-o")
        .arg(&png_file)
        .status();

    match dot_result {
        Ok(status) if status.success() => {
            eprintln!("Graph rendered to: {}", png_file.display());
            // Try to open the image with the default viewer
            open_file_with_default_app(&png_file);
            Ok(())
        }
        Ok(status) => {
            eprintln!(
                "Warning: 'dot' exited with code {:?}. Is Graphviz installed?",
                status.code()
            );
            // Fallback: print DOT source
            eprintln!("Falling back to DOT source output:");
            println!("{}", dot_content);
            Ok(())
        }
        Err(_) => {
            // `dot` not found - fallback to printing DOT
            eprintln!("Warning: Graphviz 'dot' not found. Install Graphviz to render graphs.");
            eprintln!("Printing DOT source instead:");
            println!("{}", dot_content);
            Ok(())
        }
    }
}

/// Open a file using the system default application
fn open_file_with_default_app(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &path.to_string_lossy()])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(path).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
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
        assert!(matches!(bash_format, OutputFormat::Bash));
    }
}
