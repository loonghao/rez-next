//! # Rez Core CLI
//!
//! Command-line interface for Rez Core, providing high-performance package management
//! and environment resolution capabilities.
//!
//! This module implements a complete CLI system compatible with the original Rez,
//! using clap for argument parsing and integrating with all rez-core modules.

use clap::{CommandFactory, Parser, Subcommand};
use rez_next_common::{error::RezCoreResult, RezCoreError};

pub mod commands;
pub mod utils;

/// Rez Core CLI Application
#[derive(Parser)]
#[command(
    name = "rez",
    version,
    about = "Rez Core - High-performance Rez package manager",
    long_about = "A high-performance command-line interface for the Rez package manager, built with Rust for optimal performance."
)]
pub struct RezCli {
    /// Enable verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Enable debug mode
    #[arg(long, hide = true)]
    pub debug: bool,

    /// Enable profiling (hidden option)
    #[arg(long, hide = true)]
    pub profile: Option<String>,

    /// Print system information and exit
    #[arg(short = 'i', long)]
    pub info: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<RezCommand>,
}

/// Available Rez commands
#[derive(Subcommand)]
pub enum RezCommand {
    /// Show configuration information
    Config(commands::config::ConfigArgs),

    /// Print information about the current rez context, or a given context file
    Context(commands::context::ContextArgs),

    /// View package information
    View(commands::view::ViewArgs),

    /// Resolve packages and spawn a shell or execute a command
    Env(commands::env::EnvArgs),

    /// Build a package from source and deploy it
    Release(commands::release::ReleaseArgs),

    /// Run tests defined in a package
    Test(commands::test::TestArgs),

    /// Build a package from source
    Build(commands::build::BuildArgs),

    /// Search for packages (advanced)
    Search(commands::search_v2::SearchArgs),

    /// Bind system software as rez packages
    Bind(commands::bind::BindArgs),

    /// Perform a reverse package dependency lookup
    Depends(commands::depends::DependsArgs),

    /// Resolve package dependencies
    Solve(commands::solve::SolveArgs),

    /// Copy packages between repositories
    Cp(commands::cp::CpArgs),

    /// Move packages between repositories
    Mv(commands::mv::MvArgs),

    /// Remove packages from repositories
    Rm(commands::rm::RmArgs),

    /// Show package and repository status
    Status(commands::status::StatusArgs),

    /// Compare packages and show differences
    Diff(commands::diff::DiffArgs),

    /// Show package help information
    #[command(name = "pkg-help")]
    PkgHelp(commands::help::PkgHelpArgs),

    /// List package plugins
    Plugins(commands::plugins::PluginsArgs),

    /// Manage package cache
    #[command(name = "pkg-cache")]
    PkgCache(commands::pkg_cache::PkgCacheArgs),

    /// Manage suites (collections of resolved contexts)
    Suites(commands::suites::SuitesArgs),

    /// Create a self-contained bundle from a resolved context
    Bundle(commands::bundle::BundleArgs),

    /// Install Python packages via pip into a rez repository
    Pip(commands::pip::PipArgs),

    /// Shell tab completion support
    Complete(commands::complete::CompleteArgs),

    /// Forward rez commands to rez_next (compatibility shim)
    Forward(commands::forward::ForwardArgs),

    /// Launch rez GUI (generates HTML status report)
    Gui {
        /// Output HTML file (default: rez-status.html)
        #[arg(short, long)]
        output: Option<String>,
        /// Open in browser after generating
        #[arg(long)]
        open: bool,
        /// Filter to specific package
        #[arg(short, long)]
        package: Option<String>,
    },

    /// Parse and validate version strings (development command)
    ParseVersion {
        /// Version string to parse
        version: String,
    },

    /// Run basic functionality tests (development command)
    SelfTest,

    /// Update rez-next to the latest (or specified) release
    #[command(name = "self-update")]
    SelfUpdate(commands::self_update::SelfUpdateArgs),
}

impl RezCli {
    /// Execute the CLI application
    pub fn run(self) -> RezCoreResult<()> {
        // Handle global flags first
        if self.info {
            return self.print_info();
        }

        // Execute subcommand
        match self.command {
            Some(ref command) => self.execute_command(command),
            None => {
                // No subcommand provided, show help
                let mut cmd = RezCli::command();
                cmd.print_help().map_err(RezCoreError::Io)?;
                Ok(())
            }
        }
    }

    /// Print system information
    fn print_info(&self) -> RezCoreResult<()> {
        use rez_next_common::config::RezCoreConfig;

        println!();
        println!("Rez Core System Information");
        println!("==========================");
        println!("Version: {}", env!("CARGO_PKG_VERSION"));
        println!("Build: Rust {}", env!("CARGO_PKG_RUST_VERSION"));
        println!("Target: {}", std::env::consts::ARCH);
        println!("OS: {}", std::env::consts::OS);
        println!();

        let config = RezCoreConfig::load();

        // Package paths
        println!("Package Paths:");
        if config.packages_path.is_empty() {
            println!("  (none configured)");
        } else {
            for path in &config.packages_path {
                let exists = std::path::Path::new(path).exists();
                println!(
                    "  {} {}",
                    path,
                    if exists { "[exists]" } else { "[missing]" }
                );
            }
        }
        println!();

        // Local packages path
        println!("Local Packages Path: {}", config.local_packages_path);
        println!("Release Packages Path: {}", config.release_packages_path);
        println!();

        // Configuration file search paths
        println!("Config Search Paths:");
        let home_rezconfig = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(|h| format!("{}/{}", h, ".rezconfig"));
        let config_locations: Vec<Option<String>> = vec![
            std::env::var("REZ_CONFIG_FILE").ok(),
            home_rezconfig,
            Some("/etc/rez/rezconfig".to_string()),
        ];
        let mut found_any = false;
        for loc in config_locations.iter().flatten() {
            if std::path::Path::new(loc).exists() {
                println!("  {} [active]", loc);
                found_any = true;
            }
        }
        if !found_any {
            println!("  (no rezconfig files found — using defaults)");
        }
        println!();

        // Default shell
        println!("Default Shell: {}", config.default_shell);

        Ok(())
    }

    /// Execute a specific command
    fn execute_command(&self, command: &RezCommand) -> RezCoreResult<()> {
        match command {
            RezCommand::Config(args) => commands::config::execute(args.clone()),
            RezCommand::Context(args) => commands::context::execute(args.clone()),
            RezCommand::View(args) => commands::view::execute(args.clone()),
            RezCommand::Env(args) => commands::env::execute(args.clone()),
            RezCommand::Release(args) => commands::release::execute(args.clone()),
            RezCommand::Test(args) => commands::test::execute(args.clone()),
            RezCommand::Build(args) => commands::build::execute(args.clone()),
            RezCommand::Search(args) => commands::search_v2::execute(args.clone()),
            RezCommand::Bind(args) => commands::bind::execute(args.clone()),
            RezCommand::Depends(args) => tokio::runtime::Runtime::new()
                .map_err(RezCoreError::Io)?
                .block_on(commands::depends::execute_depends(args.clone())),
            RezCommand::Solve(args) => commands::solve::execute(args.clone()),
            RezCommand::Cp(args) => commands::cp::execute(args.clone()),
            RezCommand::Mv(args) => commands::mv::execute(args.clone()),
            RezCommand::Rm(args) => commands::rm::execute(args.clone()),
            RezCommand::Status(args) => commands::status::execute(args.clone()),
            RezCommand::Diff(args) => commands::diff::execute(args.clone()),
            RezCommand::PkgHelp(args) => commands::help::execute(args.clone()),
            RezCommand::Plugins(args) => commands::plugins::execute(args.clone()),
            RezCommand::PkgCache(args) => tokio::runtime::Runtime::new()
                .map_err(RezCoreError::Io)?
                .block_on(commands::pkg_cache::execute(args.clone())),
            RezCommand::Suites(args) => commands::suites::execute(args.clone()),
            RezCommand::Bundle(args) => commands::bundle::execute(args.clone()),
            RezCommand::Pip(args) => commands::pip::execute(args.clone()),
            RezCommand::Complete(args) => commands::complete::execute(args.clone()),
            RezCommand::Forward(args) => commands::forward::execute(args.clone()),
            RezCommand::Gui {
                output,
                open,
                package,
            } => {
                let args = commands::gui::GuiArgs {
                    output: output.clone(),
                    open: *open,
                    package: package.clone(),
                };
                tokio::runtime::Runtime::new()
                    .map_err(RezCoreError::Io)?
                    .block_on(commands::gui::execute(&args))
                    .map_err(|e| RezCoreError::Solver(e.to_string()))
            }
            RezCommand::ParseVersion { version } => self.parse_version_command(version),
            RezCommand::SelfTest => self.run_tests(),
            RezCommand::SelfUpdate(args) => commands::self_update::execute(args.clone()),
        }
    }

    /// Parse version command (development utility)
    fn parse_version_command(&self, version_str: &str) -> RezCoreResult<()> {
        use rez_next_version::Version;

        match Version::parse(version_str) {
            Ok(version) => {
                println!("✅ Valid version: {}", version.as_str());
                println!(
                    "   Type: {}",
                    if version.is_empty() {
                        "Empty/epsilon"
                    } else {
                        "Normal"
                    }
                );
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Invalid version '{}': {}", version_str, e);
                Err(RezCoreError::VersionParse(e.to_string()))
            }
        }
    }

    /// Run basic functionality tests (development utility)
    fn run_tests(&self) -> RezCoreResult<()> {
        println!("Running rez-core functionality tests...");
        println!();

        let mut passed = 0;
        let mut failed = 0;

        macro_rules! run_test {
            ($name:expr, $body:expr) => {{
                print!("Test: {}... ", $name);
                match $body {
                    Ok(_) => {
                        println!("PASSED");
                        passed += 1;
                    }
                    Err(e) => {
                        println!("FAILED: {}", e);
                        failed += 1;
                    }
                }
            }};
        }

        // Test 1: Version parsing
        run_test!("version parsing", self.test_version_parsing());

        // Test 2: Version comparison
        run_test!("version comparison", {
            use rez_next_version::Version;
            let v1 =
                Version::parse("1.0.0").map_err(|e| RezCoreError::VersionParse(e.to_string()))?;
            let v2 =
                Version::parse("2.0.0").map_err(|e| RezCoreError::VersionParse(e.to_string()))?;
            if v1 < v2 {
                Ok(())
            } else {
                Err(RezCoreError::VersionParse(
                    "1.0.0 should be < 2.0.0".to_string(),
                ))
            }
        });

        // Test 3: Version range parsing
        run_test!("version range parsing", {
            use rez_next_version::VersionRange;
            VersionRange::parse(">=1.0.0")
                .map(|_| ())
                .map_err(|e| RezCoreError::VersionParse(format!("{:?}", e)))
        });

        // Test 4: Package requirement parsing
        run_test!("package requirement parsing", {
            use rez_next_package::PackageRequirement;
            PackageRequirement::parse("python-3.9").map(|_| ())
        });

        // Test 5: Config loading
        run_test!("config loading", {
            let config = rez_next_common::config::RezCoreConfig::load();
            if !config.version.is_empty() {
                Ok(())
            } else {
                Err(RezCoreError::RequirementParse(
                    "config.version is empty".to_string(),
                ))
            }
        });

        // Test 6: Config field access
        run_test!("config field access", {
            let config = rez_next_common::config::RezCoreConfig::default();
            config
                .get_field("version")
                .ok_or_else(|| {
                    RezCoreError::RequirementParse("version field not found".to_string())
                })
                .map(|_| ())
        });

        // Test 7: Config nested field access
        run_test!("config nested field access", {
            let config = rez_next_common::config::RezCoreConfig::default();
            config
                .get_field("cache.enable_memory_cache")
                .ok_or_else(|| {
                    RezCoreError::RequirementParse(
                        "cache.enable_memory_cache not found".to_string(),
                    )
                })
                .map(|_| ())
        });

        // Test 8: Package creation and validation
        run_test!("package creation and validation", {
            use rez_next_package::Package;
            let pkg = Package::new("test_pkg".to_string());
            pkg.validate()
        });

        println!();
        println!("Test Results:");
        println!("  Passed: {}", passed);
        println!("  Failed: {}", failed);
        println!("  Total:  {}", passed + failed);

        if failed > 0 {
            println!();
            println!("Some tests failed!");
            Err(RezCoreError::Python("Tests failed".to_string()))
        } else {
            println!();
            println!("All tests passed!");
            Ok(())
        }
    }

    /// Test version parsing functionality
    fn test_version_parsing(&self) -> RezCoreResult<()> {
        use rez_next_version::Version;

        let test_cases = vec!["1.0.0", "2.1.3", "1.0.0-alpha1", "3.2.1-beta.2", "1.0", "1"];

        for case in test_cases {
            Version::parse(case).map_err(|e| {
                RezCoreError::VersionParse(format!("Failed to parse '{}': {}", case, e))
            })?;
        }

        Ok(())
    }
}
