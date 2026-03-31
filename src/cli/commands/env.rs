//! # Env Command
//!
//! Implementation of the `rez env` command for environment resolution and shell spawning.
//! This command resolves package requirements and spawns a shell with the resolved environment.

use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_context::{ContextConfig, EnvironmentManager, ResolvedContext, ShellType};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType as RexShellType};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

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

    /// Print shell activation script and exit (shell format determined by --shell)
    #[arg(long)]
    pub print_script: bool,

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

    // Resolve context using the real dependency resolver
    let context = resolve_environment(&requirements, &args)?;

    if args.print_resolve {
        return print_resolved_packages(&context);
    }

    if args.print_env {
        return print_environment(&context, &args);
    }

    if args.print_script {
        return print_shell_script(&context, &args);
    }

    // Save resolved context to a .rxt file in a temp location
    // and set REZ_CONTEXT_FILE so subprocesses can access it
    let rxt_path = save_context_to_rxt(&context)?;
    if let Some(ref path) = rxt_path {
        std::env::set_var("REZ_CONTEXT_FILE", path);
    }

    // Execute shell or command
    execute_in_context(&context, &args)
}

/// Save resolved context to a temporary .rxt file
/// Returns the path if successful
fn save_context_to_rxt(context: &ResolvedContext) -> RezCoreResult<Option<PathBuf>> {
    let json = serde_json::to_string_pretty(context).map_err(RezCoreError::Serde)?;
    let rxt_path = std::env::temp_dir().join(format!("rez_context_{}.rxt", &context.id[..8]));
    std::fs::write(&rxt_path, &json).map_err(|e| RezCoreError::Io(e.into()))?;
    Ok(Some(rxt_path))
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

/// Resolve environment using the real dependency resolver
fn resolve_environment(
    requirements: &[PackageRequirement],
    args: &EnvArgs,
) -> RezCoreResult<ResolvedContext> {
    let mut config = ContextConfig::default();
    config.inherit_parent_env = true;

    // Set shell type based on args
    if let Some(shell) = &args.shell {
        config.shell_type = match shell.as_str() {
            "bash" => ShellType::Bash,
            "zsh" => ShellType::Zsh,
            "fish" => ShellType::Fish,
            "cmd" => ShellType::Cmd,
            "powershell" | "pwsh" => ShellType::PowerShell,
            _ => ShellType::Bash,
        };
    }

    // Setup repository manager from config or args
    let mut repo_manager = RepositoryManager::new();
    let rez_config = RezCoreConfig::load();

    // Add configured package paths as repositories
    let package_paths: Vec<PathBuf> = if let Some(ref paths_str) = args.paths {
        paths_str
            .split(std::path::MAIN_SEPARATOR)
            .map(PathBuf::from)
            .collect()
    } else {
        rez_config
            .packages_path
            .iter()
            .map(|p| {
                // Expand ~ on all platforms
                let expanded = if p.starts_with("~/") || p == "~" {
                    if let Ok(home) =
                        std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
                    {
                        p.replacen("~", &home, 1)
                    } else {
                        p.clone()
                    }
                } else {
                    p.clone()
                };
                PathBuf::from(expanded)
            })
            .collect()
    };

    for (i, path) in package_paths.iter().enumerate() {
        if path.exists() {
            let repo = SimpleRepository::new(path.clone(), format!("repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    let repo_arc = Arc::new(repo_manager);

    // Convert PackageRequirement -> Requirement via string parsing
    let resolver_reqs: Vec<Requirement> = requirements
        .iter()
        .map(|pr| {
            let req_str = pr.to_string();
            req_str
                .parse::<Requirement>()
                .unwrap_or_else(|_| Requirement::new(pr.name.clone()))
        })
        .collect();

    // Run the dependency resolver
    let rt = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;
    let solver_config = SolverConfig {
        max_time_seconds: args.max_solve_time.unwrap_or(300),
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), solver_config);
    let resolution = rt.block_on(resolver.resolve(resolver_reqs))?;

    // Build context from resolution result
    let mut context = ResolvedContext::from_requirements(requirements.to_vec());
    context.resolved_packages = resolution
        .resolved_packages
        .into_iter()
        .map(|info| (*info.package).clone())
        .collect();
    context.status = rez_next_context::ContextStatus::Resolved;

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

/// Print shell activation script for the resolved environment
fn print_shell_script(context: &ResolvedContext, args: &EnvArgs) -> RezCoreResult<()> {
    let env_manager = EnvironmentManager::new(context.config.clone());
    let env_vars = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::Io(e.into()))?
        .block_on(env_manager.generate_environment(&context.resolved_packages))?;

    // Determine shell type
    let shell_str = args.shell.as_deref().unwrap_or({
        if cfg!(windows) {
            "powershell"
        } else {
            "bash"
        }
    });

    let rex_shell = match shell_str {
        "bash" | "sh" => RexShellType::Bash,
        "zsh" => RexShellType::Zsh,
        "fish" => RexShellType::Fish,
        "cmd" => RexShellType::Cmd,
        "powershell" | "pwsh" => RexShellType::PowerShell,
        _ => RexShellType::Bash,
    };

    let mut rex_env = RexEnvironment::new();
    rex_env.vars = env_vars;

    let script = generate_shell_script(&rex_env, &rex_shell);
    println!("{}", script);
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
            print_script: false,
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
