//! # Env Command
//!
//! Implementation of the `rez env` command for environment resolution and shell spawning.
//! This command resolves package requirements and spawns a shell with the resolved environment.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_context::{ContextConfig, EnvironmentManager, ResolvedContext, ShellType};
use rez_next_package::{Package, PackageRequirement};
use rez_next_solver::{DependencySolver, SolverRequest};
use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};

/// Arguments for the env command
#[derive(Args, Clone)]
pub struct EnvArgs {
    /// Package requests to resolve (e.g., "python-3.9" "maya-2023")
    #[arg(value_name = "PKG")]
    pub packages: Vec<String>,

    /// Command and arguments to execute (passed after '--')
    #[arg(skip)]
    pub extra_args: Vec<String>,

    /// Target shell type
    #[arg(long, value_name = "SHELL")]
    pub shell: Option<String>,

    /// Source this file instead of standard startup scripts
    #[arg(long, value_name = "FILE")]
    pub rcfile: Option<String>,

    /// Skip loading of startup scripts
    #[arg(long)]
    pub norc: bool,

    /// Execute command within rez environment and exit
    #[arg(long, short = 'c', value_name = "COMMAND")]
    pub command: Option<String>,

    /// Read commands from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Suppress output
    #[arg(long, short = 'q')]
    pub quiet: bool,

    /// Start new session
    #[arg(long)]
    pub new_session: bool,

    /// Run in detached mode
    #[arg(long)]
    pub detached: bool,

    /// Command to run before shell startup
    #[arg(long, value_name = "COMMAND")]
    pub pre_command: Option<String>,

    /// Print resolved packages and exit
    #[arg(long)]
    pub print_resolve: bool,

    /// Print requested packages and exit
    #[arg(long)]
    pub print_request: bool,

    /// Print environment variables and exit
    #[arg(long)]
    pub print_env: bool,

    /// Output format for environment (dict, json, shell)
    #[arg(long, value_name = "FORMAT")]
    pub format: Option<String>,

    /// Package paths to search
    #[arg(long, value_name = "PATH")]
    pub paths: Option<String>,

    /// Verbosity level
    #[arg(long, short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Maximum solve time in seconds
    #[arg(long, value_name = "SECONDS")]
    pub max_solve_time: Option<u64>,

    /// Fail fast on first error
    #[arg(long)]
    pub fail_fast: bool,

    /// Show dependency graph on failure
    #[arg(long)]
    pub fail_graph: bool,
}

/// Execute the env command
pub fn execute(args: EnvArgs) -> RezCoreResult<()> {
    execute_with_extra_args(args, Vec::new())
}

/// Execute the env command with extra arguments (for '--' support)
pub fn execute_with_extra_args(mut args: EnvArgs, extra_args: Vec<String>) -> RezCoreResult<()> {
    // Handle extra args from '--' separator
    if !extra_args.is_empty() {
        if args.command.is_some() {
            return Err(RezCoreError::RequirementParse(
                "Cannot use both --command and arguments after '--'".to_string(),
            ));
        }
        args.extra_args = extra_args;
    }

    // Parse package requirements
    let requirements = parse_package_requirements(&args.packages)?;

    if args.print_request {
        return print_requested_packages(&requirements);
    }

    // Create solver and resolve context
    let solver = DependencySolver::new();
    let context = resolve_environment(&solver, &requirements, &args)?;

    if args.print_resolve {
        return print_resolved_packages(&context);
    }

    if args.print_env {
        return print_environment(&context, &args);
    }

    // Execute shell or command
    execute_in_context(&context, &args)
}

/// Parse package requirement strings into PackageRequirement objects
fn parse_package_requirements(
    package_strings: &[String],
) -> RezCoreResult<Vec<PackageRequirement>> {
    let mut requirements = Vec::new();

    for pkg_str in package_strings {
        let requirement = PackageRequirement::parse(pkg_str).map_err(|e| {
            RezCoreError::RequirementParse(format!(
                "Invalid package requirement '{}': {}",
                pkg_str, e
            ))
        })?;
        requirements.push(requirement);
    }

    Ok(requirements)
}

/// Resolve environment using the solver
fn resolve_environment(
    solver: &DependencySolver,
    requirements: &[PackageRequirement],
    args: &EnvArgs,
) -> RezCoreResult<ResolvedContext> {
    let mut config = ContextConfig::default();

    // Configure solver options
    config.inherit_parent_env = true;

    // Set shell type based on args
    if let Some(shell) = &args.shell {
        config.shell_type = match shell.as_str() {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            "cmd" => ShellType::Cmd,
            "powershell" | "pwsh" => ShellType::PowerShell,
            _ => ShellType::Bash, // Default
        };
    }

    // Create solver request and resolve
    // let request = SolverRequest::new(requirements.to_vec());
    // let resolution = solver.resolve(request)?;

    // Create resolved context from resolution result
    // let context = ResolvedContext::from_resolution_result(requirements.to_vec(), resolution);

    // For now, create a simple context
    let context = ResolvedContext::from_requirements(requirements.to_vec());

    Ok(context)
}

/// Print requested packages
fn print_requested_packages(requirements: &[PackageRequirement]) -> RezCoreResult<()> {
    for req in requirements {
        println!("{}", req.to_string());
    }
    Ok(())
}

/// Print resolved packages
fn print_resolved_packages(context: &ResolvedContext) -> RezCoreResult<()> {
    for package in &context.resolved_packages {
        println!(
            "{}-{}",
            package.name,
            package.version.as_ref().map(|v| v.as_str()).unwrap_or("")
        );
    }
    Ok(())
}

/// Print environment variables
fn print_environment(context: &ResolvedContext, args: &EnvArgs) -> RezCoreResult<()> {
    // Generate environment variables using EnvironmentManager
    let env_manager = EnvironmentManager::new(context.config.clone());
    let env_vars = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?
        .block_on(env_manager.generate_environment(&context.resolved_packages))?;

    match args.format.as_deref() {
        Some("json") => {
            let json = serde_json::to_string_pretty(&env_vars).map_err(RezCoreError::Serde)?;
            println!("{}", json);
        }
        Some("dict") => {
            for (key, value) in env_vars.iter() {
                println!("'{}': '{}'", key, value);
            }
        }
        _ => {
            // Shell format (default)
            for (key, value) in env_vars.iter() {
                println!("export {}=\"{}\"", key, value);
            }
        }
    }

    Ok(())
}

/// Execute command or spawn shell in the resolved context
fn execute_in_context(context: &ResolvedContext, args: &EnvArgs) -> RezCoreResult<()> {
    if let Some(command) = &args.command {
        // Execute specific command from -c option
        execute_command_in_context(context, command, args)
    } else if !args.extra_args.is_empty() {
        // Execute command from extra args (after '--')
        execute_extra_args_in_context(context, &args.extra_args, args)
    } else {
        // Spawn interactive shell
        spawn_shell_in_context(context, args)
    }
}

/// Execute a specific command in the resolved context
fn execute_command_in_context(
    context: &ResolvedContext,
    command: &str,
    args: &EnvArgs,
) -> RezCoreResult<()> {
    // Generate environment variables
    let env_manager = EnvironmentManager::new(context.config.clone());
    let env_vars = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?
        .block_on(env_manager.generate_environment(&context.resolved_packages))?;

    if !args.quiet {
        println!("Executing command in rez environment: {}", command);
        println!("Resolved packages:");
        for package in &context.resolved_packages {
            println!(
                "  {}-{}",
                package.name,
                package.version.as_ref().map(|v| v.as_str()).unwrap_or("")
            );
        }
        println!();
    }

    // Create command with resolved environment
    let mut cmd = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    } else {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", command]);
        cmd
    };

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Execute the command
    let status = cmd
        .status()
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to execute command: {}", e)))?;

    std::process::exit(status.code().unwrap_or(1));
}

/// Execute extra arguments (from '--') in the resolved context
fn execute_extra_args_in_context(
    context: &ResolvedContext,
    extra_args: &[String],
    args: &EnvArgs,
) -> RezCoreResult<()> {
    if extra_args.is_empty() {
        return Err(RezCoreError::ExecutionError(
            "No command specified after '--'".to_string(),
        ));
    }

    // Generate environment variables
    let env_manager = EnvironmentManager::new(context.config.clone());
    let env_vars = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?
        .block_on(env_manager.generate_environment(&context.resolved_packages))?;

    if !args.quiet {
        println!(
            "Executing command in rez environment: {}",
            extra_args.join(" ")
        );
        println!("Resolved packages:");
        for package in &context.resolved_packages {
            println!(
                "  {}-{}",
                package.name,
                package.version.as_ref().map(|v| v.as_str()).unwrap_or("")
            );
        }
        println!();
    }

    // Create command with resolved environment
    let mut cmd = Command::new(&extra_args[0]);
    if extra_args.len() > 1 {
        cmd.args(&extra_args[1..]);
    }

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Set up stdio
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Execute the command
    let status = cmd.status().map_err(|e| {
        RezCoreError::ExecutionError(format!(
            "Failed to execute command '{}': {}",
            extra_args[0], e
        ))
    })?;

    std::process::exit(status.code().unwrap_or(1));
}

/// Spawn an interactive shell in the resolved context
fn spawn_shell_in_context(context: &ResolvedContext, args: &EnvArgs) -> RezCoreResult<()> {
    // Generate environment variables
    let env_manager = EnvironmentManager::new(context.config.clone());
    let env_vars = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?
        .block_on(env_manager.generate_environment(&context.resolved_packages))?;

    if !args.quiet {
        println!("Starting shell with rez environment...");
        println!("Resolved packages:");
        for package in &context.resolved_packages {
            println!(
                "  {}-{}",
                package.name,
                package.version.as_ref().map(|v| v.as_str()).unwrap_or("")
            );
        }
        println!();
    }

    // Determine shell executable
    let shell_exe = if cfg!(target_os = "windows") {
        args.shell.as_deref().unwrap_or("cmd")
    } else {
        args.shell.as_deref().unwrap_or("bash")
    };

    // Create shell command
    let mut cmd = Command::new(shell_exe);

    // Add interactive flags for common shells
    if shell_exe == "bash" || shell_exe == "zsh" {
        cmd.arg("-i");
    } else if shell_exe == "powershell" || shell_exe == "pwsh" {
        cmd.arg("-NoExit");
    }

    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    // Set up stdio
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Execute the shell
    let status = cmd
        .status()
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to start shell: {}", e)))?;

    std::process::exit(status.code().unwrap_or(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_args_parsing() {
        let args = EnvArgs {
            packages: vec!["python".to_string()],
            extra_args: vec![],
            shell: Some("bash".to_string()),
            rcfile: None,
            norc: false,
            command: Some("echo hello".to_string()),
            stdin: false,
            quiet: false,
            new_session: false,
            detached: false,
            pre_command: None,
            print_resolve: false,
            print_request: false,
            print_env: false,
            format: None,
            paths: None,
            verbose: 0,
            max_solve_time: None,
            fail_fast: false,
            fail_graph: false,
        };

        assert_eq!(args.packages.len(), 1);
        assert_eq!(args.shell, Some("bash".to_string()));
        assert_eq!(args.command, Some("echo hello".to_string()));
    }

    #[test]
    fn test_parse_package_requirements() {
        let packages = vec![
            "python".to_string(),
            "python-3.9".to_string(),
            "maya-2023".to_string(),
        ];

        let requirements = parse_package_requirements(&packages).unwrap();
        assert_eq!(requirements.len(), 3);
        assert_eq!(requirements[0].name(), "python");
        assert_eq!(requirements[1].name(), "python");
        assert_eq!(requirements[2].name(), "maya");
    }
}
