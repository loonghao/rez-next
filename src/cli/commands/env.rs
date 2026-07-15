//! # Env Command
//!
//! Implementation of the `rez env` command for environment resolution and shell spawning.
//! This command resolves package requirements and spawns a shell with the resolved environment.

use clap::Args;
use rez_next_common::{RezCoreError, config::RezCoreConfig, error::RezCoreResult};
use rez_next_context::{ContextConfig, EnvironmentManager, ResolvedContext, ShellType};
use rez_next_package::{PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_rex::{RexEnvironment, ShellType as RexShellType, generate_shell_script};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;

use crate::cli::utils::split_package_paths;

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

    /// Exclude the configured local package repository
    #[arg(long = "no-local")]
    pub no_local: bool,

    /// Mark the context as a build environment
    #[arg(long)]
    pub build: bool,

    /// Accept an upper-level resolve timestamp request
    #[arg(long, value_name = "TIMESTAMP")]
    pub time: Option<String>,

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
    // so subprocesses can access it through REZ_RXT_FILE.
    let rxt_path = save_context_to_rxt(&context)?;

    // Execute shell or command
    let result = execute_in_context(&context, &args, &rxt_path);
    if result.is_err() {
        let _ = std::fs::remove_file(rxt_path);
    }
    result
}

/// Save resolved context to a temporary .rxt file
/// Returns the path if successful
fn save_context_to_rxt(context: &ResolvedContext) -> RezCoreResult<PathBuf> {
    let json = serde_json::to_string_pretty(context).map_err(RezCoreError::Serde)?;
    let rxt_path = std::env::temp_dir().join(format!("rez_context_{}.rxt", &context.id[..8]));
    std::fs::write(&rxt_path, &json).map_err(RezCoreError::Io)?;
    Ok(rxt_path)
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
    // Rez environments extend the caller's process environment. Package
    // actions then prepend, append, set, or unset values on top of it.
    let mut config = ContextConfig {
        inherit_parent_env: true,
        ..Default::default()
    };

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

    if args.build {
        config
            .additional_env_vars
            .insert("REZ_BUILD_ENV".to_string(), "1".to_string());
    }

    // Setup repository manager from config or args
    let mut repo_manager = RepositoryManager::new();
    let rez_config = RezCoreConfig::load();

    // Add configured package paths as repositories
    let mut package_paths: Vec<PathBuf> = if let Some(ref paths_str) = args.paths {
        split_package_paths(paths_str)
    } else {
        rez_config
            .packages_path
            .iter()
            .map(|path| expand_home(path))
            .collect()
    };

    if args.no_local {
        let local_packages_path = expand_home(&rez_config.local_packages_path);
        package_paths.retain(|path| !same_path(path, &local_packages_path));
    }

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
    let rt = tokio::runtime::Runtime::new().map_err(RezCoreError::Io)?;
    let solver_config = SolverConfig {
        max_time_seconds: args.max_solve_time.unwrap_or(300),
        strict_mode: true,
        ..SolverConfig::default()
    };
    let mut resolver = DependencyResolver::new(Arc::clone(&repo_arc), solver_config);
    let resolution = rt.block_on(resolver.resolve(resolver_reqs))?;

    // Build context from resolution result
    let mut context = ResolvedContext::from_requirements(requirements.to_vec());
    context.config = config;
    context.resolved_packages = resolution
        .resolved_packages
        .iter()
        .map(|package| package.materialized_package())
        .collect();
    context.status = rez_next_context::ContextStatus::Resolved;

    Ok(context)
}

fn expand_home(path: &str) -> PathBuf {
    if (path.starts_with("~/") || path.starts_with("~\\") || path == "~")
        && let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
    {
        return PathBuf::from(path.replacen('~', &home, 1));
    }
    PathBuf::from(path)
}

fn same_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    let normalize = |path: &std::path::Path| {
        path.to_string_lossy()
            .replace('/', std::path::MAIN_SEPARATOR_STR)
            .trim_end_matches(std::path::MAIN_SEPARATOR)
            .to_string()
    };
    if cfg!(windows) {
        normalize(left).eq_ignore_ascii_case(&normalize(right))
    } else {
        normalize(left) == normalize(right)
    }
}

/// Print requested packages
fn print_requested_packages(requirements: &[PackageRequirement]) -> RezCoreResult<()> {
    for req in requirements {
        println!("{}", req);
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

fn context_environment(
    context: &ResolvedContext,
    rxt_path: Option<&std::path::Path>,
) -> RezCoreResult<RexEnvironment> {
    let env_manager = EnvironmentManager::new(context.config.clone());
    let mut environment = tokio::runtime::Runtime::new()
        .map_err(RezCoreError::Io)?
        .block_on(env_manager.generate_rex_environment(&context.resolved_packages))?;
    if environment.stopped {
        return Err(RezCoreError::ExecutionError(
            environment
                .stop_message
                .clone()
                .unwrap_or_else(|| "Package stopped environment activation".to_string()),
        ));
    }
    if let Ok(executable) = std::env::current_exe() {
        prepend_executable_dir(&mut environment.vars, &executable)?;
    }
    environment.vars.insert(
        "REZ_USED_REQUEST".to_string(),
        context
            .requirements
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" "),
    );
    let resolved = context
        .resolved_packages
        .iter()
        .map(|package| match &package.version {
            Some(version) => format!("{}-{}", package.name, version.as_str()),
            None => package.name.clone(),
        })
        .collect::<Vec<_>>()
        .join(" ");
    environment
        .vars
        .insert("REZ_USED_RESOLVE".to_string(), resolved.clone());
    environment
        .vars
        .insert("REZ_USED_PACKAGES_NAMES".to_string(), resolved);
    if let Some(path) = rxt_path {
        environment.vars.insert(
            "REZ_RXT_FILE".to_string(),
            path.to_string_lossy().into_owned(),
        );
    }
    Ok(environment)
}

fn prepend_executable_dir(
    environment: &mut HashMap<String, String>,
    executable: &std::path::Path,
) -> RezCoreResult<()> {
    let Some(directory) = executable.parent() else {
        return Ok(());
    };
    let path_key = if cfg!(windows) {
        environment
            .keys()
            .find(|name| name.eq_ignore_ascii_case("PATH"))
            .cloned()
            .unwrap_or_else(|| "PATH".to_string())
    } else {
        "PATH".to_string()
    };
    let mut paths: Vec<_> = environment
        .get(&path_key)
        .map(std::env::split_paths)
        .into_iter()
        .flatten()
        .collect();
    if !paths.iter().any(|path| path == directory) {
        paths.insert(0, directory.to_path_buf());
    }
    let value = std::env::join_paths(paths)
        .map_err(|error| RezCoreError::ExecutionError(format!("Failed to update PATH: {error}")))?;
    environment.insert(path_key, value.to_string_lossy().into_owned());
    Ok(())
}

fn print_context_summary(context: &ResolvedContext, action: &str) {
    println!("{}", action);
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

/// Print environment variables
fn print_environment(context: &ResolvedContext, args: &EnvArgs) -> RezCoreResult<()> {
    let environment = context_environment(context, None)?;
    let env_vars = &environment.vars;

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
    let environment = context_environment(context, None)?;

    // Determine shell type
    let shell_str =
        args.shell
            .as_deref()
            .unwrap_or(if cfg!(windows) { "powershell" } else { "bash" });

    let rex_shell = match shell_str {
        "bash" | "sh" => RexShellType::Bash,
        "zsh" => RexShellType::Zsh,
        "fish" => RexShellType::Fish,
        "cmd" => RexShellType::Cmd,
        "powershell" | "pwsh" => RexShellType::PowerShell,
        _ => RexShellType::Bash,
    };

    let script = generate_shell_script(&environment, &rex_shell);
    println!("{}", script);
    Ok(())
}

fn create_activation_script(
    environment: &mut RexEnvironment,
    shell_type: &RexShellType,
    trailing_command: Option<&str>,
) -> RezCoreResult<(tempfile::TempDir, PathBuf)> {
    let directory = tempfile::tempdir().map_err(RezCoreError::Io)?;
    let filename = match shell_type {
        RexShellType::Bash => "rez-env.sh",
        RexShellType::Zsh => ".zshrc",
        RexShellType::Fish => "rez-env.fish",
        RexShellType::Cmd => "rez-env.cmd",
        RexShellType::PowerShell => "rez-env.ps1",
    };
    let path = directory.path().join(filename);
    environment.vars.insert(
        "REZ_CONTEXT_FILE".to_string(),
        path.to_string_lossy().into_owned(),
    );
    let mut script = generate_shell_script(environment, shell_type);
    if let Some(command) = trailing_command {
        script.push_str(if matches!(shell_type, RexShellType::Cmd) {
            "\r\n"
        } else {
            "\n"
        });
        script.push_str(command);
    }
    std::fs::write(&path, script).map_err(RezCoreError::Io)?;
    Ok((directory, path))
}

fn remove_temporary_context(environment: &RexEnvironment) {
    if let Some(path) = environment.vars.get("REZ_RXT_FILE") {
        let _ = std::fs::remove_file(path);
    }
}

/// Execute command or spawn shell in the resolved context
fn execute_in_context(
    context: &ResolvedContext,
    args: &EnvArgs,
    rxt_path: &std::path::Path,
) -> RezCoreResult<()> {
    if let Some(command) = &args.command {
        // Execute specific command from -c option
        execute_command_in_context(context, command, args, rxt_path)
    } else if !args.extra_args.is_empty() {
        // Execute command from extra args (after '--')
        execute_extra_args_in_context(context, &args.extra_args, args, rxt_path)
    } else {
        // Spawn interactive shell
        spawn_shell_in_context(context, args, rxt_path)
    }
}

/// Execute a specific command in the resolved context
fn execute_command_in_context(
    context: &ResolvedContext,
    command: &str,
    args: &EnvArgs,
    rxt_path: &std::path::Path,
) -> RezCoreResult<()> {
    let environment = context_environment(context, Some(rxt_path))?;
    execute_shell_command_in_environment(context, command, args, environment)
}

fn execute_shell_command_in_environment(
    context: &ResolvedContext,
    command: &str,
    args: &EnvArgs,
    mut environment: RexEnvironment,
) -> RezCoreResult<()> {
    if !args.quiet {
        print_context_summary(
            context,
            &format!("Executing command in rez environment: {}", command),
        );
    }

    let shell = args
        .shell
        .as_deref()
        .unwrap_or(if cfg!(windows) { "cmd" } else { "bash" });
    let shell_type = shell
        .parse::<RexShellType>()
        .map_err(RezCoreError::ExecutionError)?;
    let (activation_directory, activation_script) =
        create_activation_script(&mut environment, &shell_type, Some(command))?;

    let mut cmd = Command::new(shell);
    match shell_type {
        RexShellType::Cmd => {
            cmd.args(["/D", "/S", "/C"]).arg(&activation_script);
        }
        RexShellType::PowerShell => {
            cmd.args(["-NoProfile", "-File"]).arg(&activation_script);
        }
        _ => {
            cmd.arg(&activation_script);
        }
    }

    cmd.envs(&environment.vars);

    // Execute the command
    let status = cmd
        .status()
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to execute command: {}", e)))?;
    drop(activation_directory);
    remove_temporary_context(&environment);

    std::process::exit(status.code().unwrap_or(1));
}

/// Execute extra arguments (from '--') in the resolved context
fn execute_extra_args_in_context(
    context: &ResolvedContext,
    extra_args: &[String],
    args: &EnvArgs,
    rxt_path: &std::path::Path,
) -> RezCoreResult<()> {
    if extra_args.is_empty() {
        return Err(RezCoreError::ExecutionError(
            "No command specified after '--'".to_string(),
        ));
    }

    let environment = context_environment(context, Some(rxt_path))?;
    let command = if let Some(alias) = environment.aliases.get(&extra_args[0]) {
        expand_alias(alias, &extra_args[1..])
    } else {
        extra_args
            .iter()
            .map(|argument| quote_shell_argument(argument))
            .collect::<Vec<_>>()
            .join(" ")
    };
    execute_shell_command_in_environment(context, &command, args, environment)
}

fn expand_alias(alias: &str, arguments: &[String]) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| quote_shell_argument(argument))
        .collect::<Vec<_>>()
        .join(" ");
    if alias.contains("$*") {
        alias.replace("$*", &arguments)
    } else if alias.contains("$@") {
        alias.replace("$@", &arguments)
    } else if arguments.is_empty() {
        alias.to_string()
    } else {
        format!("{alias} {arguments}")
    }
}

fn quote_shell_argument(argument: &str) -> String {
    if cfg!(windows) {
        if argument
            .chars()
            .any(|character| character.is_whitespace() || "&|<>^\"".contains(character))
        {
            format!("\"{}\"", argument.replace('"', "\\\""))
        } else {
            argument.to_string()
        }
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

/// Spawn an interactive shell in the resolved context
fn spawn_shell_in_context(
    context: &ResolvedContext,
    args: &EnvArgs,
    rxt_path: &std::path::Path,
) -> RezCoreResult<()> {
    let mut environment = context_environment(context, Some(rxt_path))?;
    if !args.quiet {
        print_context_summary(context, "You are now in a rez-configured environment.");
    }

    // Determine shell executable
    let shell_exe = if cfg!(target_os = "windows") {
        args.shell.as_deref().unwrap_or("cmd")
    } else {
        args.shell.as_deref().unwrap_or("bash")
    };
    let shell_type = shell_exe
        .parse::<RexShellType>()
        .map_err(RezCoreError::ExecutionError)?;
    let (activation_directory, activation_script) =
        create_activation_script(&mut environment, &shell_type, None)?;

    // Create shell command
    let mut cmd = Command::new(shell_exe);

    match shell_type {
        RexShellType::Cmd => {
            cmd.args(["/D", "/K"]).arg(&activation_script);
        }
        RexShellType::PowerShell => {
            cmd.args(["-NoExit", "-File"]).arg(&activation_script);
        }
        RexShellType::Bash => {
            cmd.arg("--rcfile").arg(&activation_script).arg("-i");
        }
        RexShellType::Zsh => {
            cmd.arg("-i");
            cmd.env("ZDOTDIR", activation_directory.path());
        }
        RexShellType::Fish => {
            cmd.arg("-C")
                .arg(format!("source '{}'", activation_script.display()))
                .arg("-i");
        }
    }

    cmd.envs(&environment.vars);

    // Set up stdio
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    // Execute the shell
    let status = cmd
        .status()
        .map_err(|e| RezCoreError::ExecutionError(format!("Failed to start shell: {}", e)))?;
    drop(activation_directory);
    remove_temporary_context(&environment);

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
            no_local: false,
            build: false,
            time: None,
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

    #[test]
    fn test_current_rez_directory_is_first_on_path() {
        let temp = tempfile::TempDir::new().unwrap();
        let executable = temp
            .path()
            .join(if cfg!(windows) { "rez.exe" } else { "rez" });
        let path_key = if cfg!(windows) { "Path" } else { "PATH" };
        let mut environment = HashMap::from([(
            path_key.to_string(),
            std::env::join_paths([std::env::temp_dir()])
                .unwrap()
                .to_string_lossy()
                .into_owned(),
        )]);

        prepend_executable_dir(&mut environment, &executable).unwrap();

        let paths: Vec<_> = std::env::split_paths(environment.get(path_key).unwrap()).collect();
        assert_eq!(paths[0], temp.path());
        assert_eq!(
            environment
                .keys()
                .filter(|name| name.eq_ignore_ascii_case("PATH"))
                .count(),
            1
        );
    }

    #[test]
    fn test_env_rejects_an_incomplete_resolve() {
        let temp = tempfile::TempDir::new().unwrap();
        let args = EnvArgs {
            packages: vec!["missing-package".to_string()],
            extra_args: Vec::new(),
            shell: None,
            rcfile: None,
            norc: false,
            command: None,
            stdin: false,
            quiet: true,
            new_session: false,
            detached: false,
            pre_command: None,
            print_resolve: false,
            print_request: false,
            print_env: false,
            print_script: false,
            format: None,
            paths: Some(temp.path().to_string_lossy().into_owned()),
            no_local: false,
            build: false,
            time: None,
            verbose: 0,
            max_solve_time: None,
            fail_fast: false,
            fail_graph: false,
        };
        let requirements = parse_package_requirements(&args.packages).unwrap();

        let error = resolve_environment(&requirements, &args).unwrap_err();

        assert!(error.to_string().contains("missing-package"));
    }

    #[test]
    fn test_context_environment_exposes_resolve_metadata() {
        let requirements = parse_package_requirements(&["tool-1".to_string()]).unwrap();
        let mut context = ResolvedContext::from_requirements(requirements);
        context.config.inherit_parent_env = false;
        let mut package = rez_next_package::Package::new("tool".to_string());
        package.version = Some(rez_next_version::Version::parse("1.2.0").unwrap());
        context.resolved_packages.push(package);

        let environment = context_environment(&context, None).unwrap();

        assert_eq!(
            environment.vars.get("REZ_USED_REQUEST"),
            Some(&"tool-1".to_string())
        );
        assert_eq!(
            environment.vars.get("REZ_USED_RESOLVE"),
            Some(&"tool-1.2.0".to_string())
        );
        assert_eq!(
            environment.vars.get("REZ_USED_PACKAGES_NAMES"),
            Some(&"tool-1.2.0".to_string())
        );
    }
}
