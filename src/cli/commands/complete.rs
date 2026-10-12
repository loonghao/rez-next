//! # Complete Command
//!
//! Tab completion support for rez commands.
//! Provides shell completion scripts and dynamic completion for packages, versions, etc.

use crate::cli::utils::expand_home_path as expand_home;
use clap::Args;
use rez_next_common::{RezCoreConfig, error::RezCoreResult};
use rez_next_repository::package_repository::FilesystemPackageRepository;
use rez_next_repository::package_repository::bridge::AsyncRepositoryAdapter;
use rez_next_repository::package_repository::spec::RepositorySpec;
use rez_next_repository::simple_repository::RepositoryManager;

/// Arguments for the complete command
#[derive(Args, Clone)]
pub struct CompleteArgs {
    /// Shell type to generate completions for
    #[arg(long, value_name = "SHELL", value_parser = ["bash", "zsh", "fish", "powershell"])]
    pub shell: Option<String>,

    /// Print shell completion script and exit
    #[arg(long)]
    pub print_script: bool,

    /// Complete package names (for internal use by shell completions)
    #[arg(long)]
    pub complete_packages: bool,

    /// Complete package versions for a given package name
    #[arg(long, value_name = "PACKAGE")]
    pub complete_versions: Option<String>,

    /// Current word being completed
    #[arg(long, value_name = "WORD")]
    pub current: Option<String>,

    /// Previous word (for context)
    #[arg(long, value_name = "WORD")]
    pub prev: Option<String>,

    /// Enable dynamic completion mode (reads COMP_LINE/COMP_POINT env vars)
    #[arg(long)]
    pub dynamic: bool,

    /// COMP_LINE value (for dynamic completion)
    #[arg(long, value_name = "LINE")]
    pub comp_line: Option<String>,

    /// COMP_POINT value (for dynamic completion)
    #[arg(long, value_name = "POINT")]
    pub comp_point: Option<String>,
}

/// Execute the complete command
pub fn execute(args: CompleteArgs) -> RezCoreResult<()> {
    // Dynamic completion mode: read COMP_LINE/COMP_POINT if not explicitly provided
    if args.dynamic || args.comp_line.is_some() {
        let comp_line = args
            .comp_line
            .or_else(|| std::env::var("COMP_LINE").ok())
            .unwrap_or_default();
        let comp_point = args
            .comp_point
            .or_else(|| std::env::var("COMP_POINT").ok())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        return complete_dynamic(&comp_line, comp_point);
    }

    if args.print_script {
        let shell = args.shell.as_deref().unwrap_or("bash");
        print_completion_script(shell);
        return Ok(());
    }

    if args.complete_packages {
        return complete_package_names(args.current.as_deref().unwrap_or(""));
    }

    if let Some(ref pkg_name) = args.complete_versions {
        return complete_package_versions(pkg_name, args.current.as_deref().unwrap_or(""));
    }

    // Default: print usage hint
    println!("Use --print-script SHELL to get shell completion script.");
    println!("Supported shells: bash, zsh, fish, powershell");
    println!("Use --dynamic to enable dynamic completion (reads COMP_LINE/COMP_POINT).");
    Ok(())
}

/// Print shell completion script for a given shell
fn print_completion_script(shell: &str) {
    match shell {
        "bash" => println!(
            r#"# rez bash completion
_rez_complete() {{
    local cur prev
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "env build release test search bind depends solve cp mv rm status diff view config context pkg-cache pkg-help plugins suites bundle pip complete" -- "$cur") )
    elif [[ "$prev" == "env" || "$prev" == "build" || "$prev" == "search" || "$prev" == "depends" ]]; then
        local pkgs
        pkgs=$(rez complete --complete-packages --current "$cur" 2>/dev/null)
        COMPREPLY=( $(compgen -W "$pkgs" -- "$cur") )
    fi
}}
complete -F _rez_complete rez
"#
        ),
        "zsh" => println!(
            r#"# rez zsh completion
_rez() {{
    local state

    _arguments \
        '1: :->cmds' \
        '*: :->args'

    case $state in
        cmds)
            _values 'commands' \
                'env[resolve packages and spawn shell]' \
                'build[build package from source]' \
                'release[release a package]' \
                'test[run package tests]' \
                'search[search for packages]' \
                'bind[bind system software as rez package]' \
                'depends[reverse dependency lookup]' \
                'solve[solve package dependencies]' \
                'cp[copy packages]' \
                'mv[move packages]' \
                'rm[remove packages]' \
                'status[show status]' \
                'diff[compare packages]' \
                'view[view package info]' \
                'config[show configuration]' \
                'context[show context info]' \
                'pkg-cache[manage package cache]' \
                'plugins[list plugins]' \
                'suites[manage suites]' \
                'bundle[create bundle]' \
                'pip[install pip package]'
            ;;
        args)
            local pkgs
            pkgs=($(rez complete --complete-packages 2>/dev/null))
            _values 'packages' $pkgs
            ;;
    esac
}}
compdef _rez rez
"#
        ),
        "fish" => println!(
            r#"# rez fish completion
complete -c rez -f -n '__fish_use_subcommand' -a env -d 'Resolve packages and spawn shell'
complete -c rez -f -n '__fish_use_subcommand' -a build -d 'Build package from source'
complete -c rez -f -n '__fish_use_subcommand' -a release -d 'Release a package'
complete -c rez -f -n '__fish_use_subcommand' -a test -d 'Run package tests'
complete -c rez -f -n '__fish_use_subcommand' -a search -d 'Search for packages'
complete -c rez -f -n '__fish_use_subcommand' -a bind -d 'Bind system software as rez package'
complete -c rez -f -n '__fish_use_subcommand' -a depends -d 'Reverse dependency lookup'
complete -c rez -f -n '__fish_use_subcommand' -a solve -d 'Solve package dependencies'
complete -c rez -f -n '__fish_use_subcommand' -a cp -d 'Copy packages'
complete -c rez -f -n '__fish_use_subcommand' -a mv -d 'Move packages'
complete -c rez -f -n '__fish_use_subcommand' -a rm -d 'Remove packages'
complete -c rez -f -n '__fish_use_subcommand' -a status -d 'Show status'
complete -c rez -f -n '__fish_use_subcommand' -a diff -d 'Compare packages'
complete -c rez -f -n '__fish_use_subcommand' -a view -d 'View package info'
complete -c rez -f -n '__fish_use_subcommand' -a config -d 'Show configuration'
complete -c rez -f -n '__fish_use_subcommand' -a context -d 'Show context info'
complete -c rez -f -n '__fish_use_subcommand' -a suites -d 'Manage suites'
complete -c rez -f -n '__fish_use_subcommand' -a bundle -d 'Create bundle'
complete -c rez -f -n '__fish_use_subcommand' -a pip -d 'Install pip package'
"#
        ),
        "powershell" => println!(
            r#"# rez PowerShell completion
Register-ArgumentCompleter -Native -CommandName rez -ScriptBlock {{
    param($wordToComplete, $commandAst, $cursorPosition)
    $commands = @('env','build','release','test','search','bind','depends','solve',
                  'cp','mv','rm','status','diff','view','config','context',
                  'pkg-cache','pkg-help','plugins','suites','bundle','pip','complete')
    if ($commandAst.CommandElements.Count -le 2) {{
        $commands | Where-Object {{ $_ -like "$wordToComplete*" }} |
            ForEach-Object {{ [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }}
    }} else {{
        # Complete package names
        $pkgs = & rez complete --complete-packages --current $wordToComplete 2>$null
        $pkgs | ForEach-Object {{ [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_) }}
    }}
}}
"#
        ),
        _ => eprintln!(
            "Unknown shell: {}. Supported: bash, zsh, fish, powershell",
            shell
        ),
    }
}

/// Build a manager over `packages_path`, serving reads from the
/// upstream-faithful filesystem repository.
///
/// Entries may name their repository type (`filesystem@/packages/int`); a bare
/// path means a filesystem repository. Only filesystem repositories are added
/// here — the completion path only reads package names and versions, and no
/// other repository type is implemented yet.
fn build_manager(config: &RezCoreConfig) -> RepositoryManager {
    let mut manager = RepositoryManager::new();
    for (i, entry) in config.packages_path.iter().enumerate() {
        let spec = RepositorySpec::parse(entry);
        if !spec.is_filesystem() {
            continue;
        }
        let path = expand_home(&spec.location);
        if !path.exists() {
            continue;
        }
        let repository = FilesystemPackageRepository::with_name(path, format!("repo_{i}"));
        manager.add_repository(Box::new(AsyncRepositoryAdapter::new(repository)));
    }
    manager
}

/// List package names matching a prefix
fn complete_package_names(prefix: &str) -> RezCoreResult<()> {
    let config = RezCoreConfig::load();
    let rt = tokio::runtime::Runtime::new().map_err(rez_next_common::RezCoreError::Io)?;

    let manager = build_manager(&config);
    let names = rt.block_on(manager.list_packages())?;
    for name in names {
        if name.starts_with(prefix) {
            println!("{}", name);
        }
    }
    Ok(())
}

/// List versions for a package matching a prefix
fn complete_package_versions(pkg_name: &str, prefix: &str) -> RezCoreResult<()> {
    let config = RezCoreConfig::load();
    let rt = tokio::runtime::Runtime::new().map_err(rez_next_common::RezCoreError::Io)?;

    let manager = build_manager(&config);
    let packages = rt.block_on(manager.find_packages(pkg_name))?;
    for pkg in packages {
        if let Some(ref v) = pkg.version {
            let ver_str = format!("{}-{}", pkg_name, v.as_str());
            if ver_str.starts_with(prefix) || prefix.is_empty() {
                println!("{}", ver_str);
            }
        }
    }
    Ok(())
}

/// Perform dynamic completion based on COMP_LINE and COMP_POINT.
///
/// This mirrors the behavior of rez's `complete.py`, which reads these
/// environment variables to provide context-aware completions.
fn complete_dynamic(comp_line: &str, comp_point: usize) -> RezCoreResult<()> {
    // Parse the command line up to the cursor position
    let line = if comp_point < comp_line.len() {
        &comp_line[..comp_point]
    } else {
        comp_line
    };

    // Split into words (simple split, doesn't handle quotes)
    let words: Vec<&str> = line.split_whitespace().collect();

    // Determine what to complete
    if words.len() <= 1 {
        // Completing the first argument (subcommand name)
        let prefix = if words.len() == 1 { words[0] } else { "" };
        let commands = get_subcommand_names();
        for cmd in commands {
            if cmd.starts_with(prefix) {
                println!("{}", cmd);
            }
        }
        return Ok(());
    }

    // We have at least one word (the subcommand)
    let subcommand = words[1];

    // For subcommands that take package names, complete package names
    let package_subcommands = ["env", "solve", "search", "depends", "build", "release"];
    if package_subcommands.contains(&subcommand) {
        let prefix = if words.len() >= 3 {
            words[words.len() - 1]
        } else {
            ""
        };
        return complete_package_names(prefix);
    }

    // Default: no completions
    Ok(())
}

/// Get the list of available subcommand names.
fn get_subcommand_names() -> Vec<&'static str> {
    vec![
        "env",
        "solve",
        "build",
        "release",
        "status",
        "search",
        "view",
        "diff",
        "cp",
        "mv",
        "rm",
        "bundle",
        "config",
        "selftest",
        "gui",
        "context",
        "suites",
        "interpret",
        "depends",
        "pip",
        "forward",
        "benchmark",
        "complete",
        "source",
        "bind",
        "test",
        "pkg-cache",
        "plugins",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build `<root>/<family>/<version>` directories, the layout the
    /// filesystem repository scans for.
    fn make_repository(versions: &[(&str, &[&str])]) -> TempDir {
        let root = TempDir::new().unwrap();
        for (family, family_versions) in versions {
            for version in *family_versions {
                fs::create_dir_all(root.path().join(family).join(version)).unwrap();
            }
        }
        root
    }

    /// `RezCoreConfig` is `#[non_exhaustive]`, so it must be built via
    /// `Default` and field assignment rather than a struct literal.
    fn config_with_paths(paths: Vec<String>) -> RezCoreConfig {
        let mut config = RezCoreConfig::default();
        config.packages_path = paths;
        config
    }

    #[test]
    fn build_manager_serves_reads_through_the_bridge() {
        // Proves the migration path: completion reads are answered by the
        // upstream-faithful repository via AsyncRepositoryAdapter, not by
        // SimpleRepository.
        let root = make_repository(&[("python", &["3.9.1", "3.11.0"])]);
        let config = config_with_paths(vec![root.path().to_string_lossy().to_string()]);

        let manager = build_manager(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let names = rt.block_on(manager.list_packages()).unwrap();
        assert_eq!(names, vec!["python".to_string()]);

        let packages = rt.block_on(manager.find_packages("python")).unwrap();
        let versions: Vec<String> = packages
            .iter()
            .filter_map(|p| p.version.as_ref().map(|v| v.as_str().to_string()))
            .collect();
        assert_eq!(versions, vec!["3.11.0", "3.9.1"]);
    }

    #[test]
    fn build_manager_accepts_typed_packages_path_entries() {
        let root = make_repository(&[("maya", &["2024.1"])]);
        let config = config_with_paths(vec![format!(
            "filesystem@{}",
            root.path().to_string_lossy()
        )]);

        let manager = build_manager(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let names = rt.block_on(manager.list_packages()).unwrap();
        assert_eq!(names, vec!["maya".to_string()]);
    }

    #[test]
    fn build_manager_skips_non_filesystem_and_missing_paths() {
        let root = make_repository(&[("python", &["3.9.1"])]);
        let config = config_with_paths(vec![
            // Not a filesystem repository, and no such type is implemented.
            "memory@local".to_string(),
            // Filesystem, but the path does not exist.
            "/nonexistent/repository/path".to_string(),
            format!("filesystem@{}", root.path().to_string_lossy()),
        ]);

        let manager = build_manager(&config);
        let rt = tokio::runtime::Runtime::new().unwrap();

        let names = rt.block_on(manager.list_packages()).unwrap();
        assert_eq!(names, vec!["python".to_string()]);
    }

    #[test]
    fn test_complete_args_default() {
        let args = CompleteArgs {
            shell: None,
            print_script: false,
            complete_packages: false,
            complete_versions: None,
            current: None,
            prev: None,
            dynamic: false,
            comp_line: None,
            comp_point: None,
        };
        assert!(execute(args).is_ok());
    }

    #[test]
    fn test_print_bash_script() {
        let args = CompleteArgs {
            shell: Some("bash".to_string()),
            print_script: true,
            complete_packages: false,
            complete_versions: None,
            current: None,
            prev: None,
            dynamic: false,
            comp_line: None,
            comp_point: None,
        };
        assert!(execute(args).is_ok());
    }

    #[test]
    fn test_print_zsh_script() {
        let args = CompleteArgs {
            shell: Some("zsh".to_string()),
            print_script: true,
            complete_packages: false,
            complete_versions: None,
            current: None,
            prev: None,
            dynamic: false,
            comp_line: None,
            comp_point: None,
        };
        assert!(execute(args).is_ok());
    }

    #[test]
    fn test_print_powershell_script() {
        let args = CompleteArgs {
            shell: Some("powershell".to_string()),
            print_script: true,
            complete_packages: false,
            complete_versions: None,
            current: None,
            prev: None,
            dynamic: false,
            comp_line: None,
            comp_point: None,
        };
        assert!(execute(args).is_ok());
    }

    // ── complete_dynamic tests ─────────────────────────────────────

    #[test]
    fn test_dynamic_empty_line_lists_commands() {
        // COMP_LINE="" (empty), should list all subcommands
        let result = complete_dynamic("", 0);
        // Should not error
        assert!(result.is_ok());
    }

    #[test]
    fn test_dynamic_partial_command() {
        // COMP_LINE="rez env" (cursor at end), should complete "env" or list packages
        let result = complete_dynamic("rez e", 5);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dynamic_complete_subcommand() {
        // COMP_LINE="rez b" (cursor after "b"), should complete to "build"
        let result = complete_dynamic("rez b", 4);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_subcommand_names_not_empty() {
        let names = get_subcommand_names();
        assert!(!names.is_empty());
        assert!(names.contains(&"env"));
        assert!(names.contains(&"build"));
    }
}
