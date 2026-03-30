//! Suites command implementation
//!
//! Implements `rez suites` — manage collections of resolved contexts.
//! A suite bundles multiple resolved contexts and exposes their combined tools.

use clap::{Args, Subcommand};
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_suites::{Suite, SuiteManager, ToolConflictMode};
use std::path::PathBuf;

/// Arguments for the suites command
#[derive(Args, Clone, Debug)]
pub struct SuitesArgs {
    #[command(subcommand)]
    pub command: Option<SuitesCommand>,

    /// Suite search paths (overrides config)
    #[arg(long, value_name = "PATH")]
    pub paths: Option<String>,
}

#[derive(Subcommand, Clone, Debug)]
pub enum SuitesCommand {
    /// Create a new suite
    Create(CreateSuiteArgs),

    /// List available suites
    List(ListSuitesArgs),

    /// Show information about a suite
    Info(InfoSuiteArgs),

    /// Add a context to a suite
    #[command(name = "add-context")]
    AddContext(AddContextArgs),

    /// Remove a context from a suite
    #[command(name = "remove-context")]
    RemoveContext(RemoveContextArgs),

    /// Alias a tool within a suite context
    #[command(name = "alias-tool")]
    AliasTool(AliasToolArgs),

    /// Hide a tool within a suite context
    #[command(name = "hide-tool")]
    HideTool(HideToolArgs),

    /// Show all tools exposed by a suite
    Tools(ToolsArgs),
}

#[derive(Args, Clone, Debug)]
pub struct CreateSuiteArgs {
    /// Path where the suite should be created
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Suite description
    #[arg(short = 'd', long)]
    pub description: Option<String>,

    /// Tool conflict mode (error, first, last, prefix)
    #[arg(long, default_value = "error")]
    pub conflict_mode: String,
}

#[derive(Args, Clone, Debug)]
pub struct ListSuitesArgs {
    /// Show verbose output
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

#[derive(Args, Clone, Debug)]
pub struct InfoSuiteArgs {
    /// Suite path or name
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,
}

#[derive(Args, Clone, Debug)]
pub struct AddContextArgs {
    /// Suite path
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,

    /// Context name
    #[arg(value_name = "CONTEXT_NAME")]
    pub context_name: String,

    /// Package requests for this context
    #[arg(value_name = "PKG", required = true)]
    pub requests: Vec<String>,

    /// Tool conflict mode for this context
    #[arg(long)]
    pub prefix: Option<String>,

    /// Priority (higher = preferred in conflicts)
    #[arg(long, default_value = "0")]
    pub priority: i32,
}

#[derive(Args, Clone, Debug)]
pub struct RemoveContextArgs {
    /// Suite path
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,

    /// Context name to remove
    #[arg(value_name = "CONTEXT_NAME")]
    pub context_name: String,
}

#[derive(Args, Clone, Debug)]
pub struct AliasToolArgs {
    /// Suite path
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,

    /// Context name
    #[arg(value_name = "CONTEXT_NAME")]
    pub context_name: String,

    /// Alias name (new name)
    #[arg(value_name = "ALIAS")]
    pub alias: String,

    /// Original tool name
    #[arg(value_name = "TOOL")]
    pub tool: String,
}

#[derive(Args, Clone, Debug)]
pub struct HideToolArgs {
    /// Suite path
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,

    /// Context name
    #[arg(value_name = "CONTEXT_NAME")]
    pub context_name: String,

    /// Tool name to hide
    #[arg(value_name = "TOOL")]
    pub tool: String,
}

#[derive(Args, Clone, Debug)]
pub struct ToolsArgs {
    /// Suite path
    #[arg(value_name = "SUITE")]
    pub suite: PathBuf,

    /// Show verbose output including source packages
    #[arg(short = 'v', long)]
    pub verbose: bool,
}

/// Execute the suites command
pub fn execute(args: SuitesArgs) -> RezCoreResult<()> {
    let suite_paths = get_suite_paths(&args);

    match args.command {
        Some(SuitesCommand::Create(a)) => cmd_create(a),
        Some(SuitesCommand::List(a)) => cmd_list(a, suite_paths),
        Some(SuitesCommand::Info(a)) => cmd_info(a),
        Some(SuitesCommand::AddContext(a)) => cmd_add_context(a),
        Some(SuitesCommand::RemoveContext(a)) => cmd_remove_context(a),
        Some(SuitesCommand::AliasTool(a)) => cmd_alias_tool(a),
        Some(SuitesCommand::HideTool(a)) => cmd_hide_tool(a),
        Some(SuitesCommand::Tools(a)) => cmd_tools(a),
        None => {
            // Default: list suites
            cmd_list(
                ListSuitesArgs { verbose: false },
                suite_paths,
            )
        }
    }
}

fn get_suite_paths(args: &SuitesArgs) -> Vec<PathBuf> {
    if let Some(ref paths_str) = args.paths {
        paths_str
            .split(std::path::MAIN_SEPARATOR)
            .map(PathBuf::from)
            .collect()
    } else {
        // Default: user home suites directory
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        vec![PathBuf::from(home).join("suites")]
    }
}

fn cmd_create(args: CreateSuiteArgs) -> RezCoreResult<()> {
    let conflict_mode: ToolConflictMode = args.conflict_mode
        .parse()
        .map_err(|e: String| RezCoreError::RequirementParse(e))?;

    let mut suite = Suite::new().with_conflict_mode(conflict_mode);
    if let Some(desc) = args.description {
        suite = suite.with_description(desc);
    }

    suite.save(&args.path)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    println!("Created suite at: {}", args.path.display());
    Ok(())
}

fn cmd_list(args: ListSuitesArgs, suite_paths: Vec<PathBuf>) -> RezCoreResult<()> {
    let manager = SuiteManager::with_paths(suite_paths);
    let names = manager.list_suite_names();

    if names.is_empty() {
        println!("No suites found.");
        return Ok(());
    }

    if args.verbose {
        let suites = manager.find_suites();
        for path in suites {
            if let Ok(suite) = Suite::load(&path) {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let desc = suite.description.as_deref().unwrap_or("(no description)");
                println!("{:<30} {} context(s) - {}", name, suite.len(), desc);
            }
        }
    } else {
        for name in names {
            println!("{}", name);
        }
    }

    Ok(())
}

fn cmd_info(args: InfoSuiteArgs) -> RezCoreResult<()> {
    let suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;
    suite.print_info();
    Ok(())
}

fn cmd_add_context(args: AddContextArgs) -> RezCoreResult<()> {
    let mut suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite
        .add_context(args.context_name.clone(), args.requests.clone())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    if let Some(prefix) = args.prefix {
        if let Some(ctx) = suite.get_context_mut(&args.context_name) {
            ctx.prefix = Some(prefix);
        }
    }

    suite.save(suite.path.clone().unwrap())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    println!(
        "Added context '{}' with requests: {}",
        args.context_name,
        args.requests.join(", ")
    );
    Ok(())
}

fn cmd_remove_context(args: RemoveContextArgs) -> RezCoreResult<()> {
    let mut suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite
        .remove_context(&args.context_name)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite.save(suite.path.clone().unwrap())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    println!("Removed context '{}'", args.context_name);
    Ok(())
}

fn cmd_alias_tool(args: AliasToolArgs) -> RezCoreResult<()> {
    let mut suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite
        .alias_tool(&args.context_name, args.alias.clone(), args.tool.clone())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite.save(suite.path.clone().unwrap())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    println!(
        "Aliased '{}' -> '{}' in context '{}'",
        args.alias, args.tool, args.context_name
    );
    Ok(())
}

fn cmd_hide_tool(args: HideToolArgs) -> RezCoreResult<()> {
    let mut suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite
        .hide_tool(&args.context_name, &args.tool)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    suite.save(suite.path.clone().unwrap())
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    println!(
        "Hidden tool '{}' in context '{}'",
        args.tool, args.context_name
    );
    Ok(())
}

fn cmd_tools(args: ToolsArgs) -> RezCoreResult<()> {
    let suite = Suite::load(&args.suite)
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    let tools = suite
        .get_tools()
        .map_err(|e| RezCoreError::ExecutionError(e.to_string()))?;

    if tools.is_empty() {
        println!("No tools exposed by this suite.");
        return Ok(());
    }

    let mut tool_list: Vec<_> = tools.values().collect();
    tool_list.sort_by(|a, b| a.name.cmp(&b.name));

    println!("{:<30} {:<20} {}", "TOOL", "CONTEXT", "SOURCE");
    println!("{:<30} {:<20} {}", "----", "-------", "------");

    for tool in tool_list {
        if args.verbose {
            println!(
                "{:<30} {:<20} {}",
                tool.name, tool.context_name, tool.package
            );
        } else {
            println!("{:<30} {}", tool.name, tool.context_name);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_suite_paths_default() {
        let args = SuitesArgs {
            command: None,
            paths: None,
        };
        let paths = get_suite_paths(&args);
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_cmd_create() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("test_suite");

        let args = CreateSuiteArgs {
            path: suite_path.clone(),
            description: Some("Test suite".to_string()),
            conflict_mode: "error".to_string(),
        };

        cmd_create(args).unwrap();
        assert!(Suite::is_suite(&suite_path));
    }
}
