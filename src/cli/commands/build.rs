//! Build command implementation
//!
//! Implements the `rez build` command for building packages from source.

use clap::{Args, ValueEnum};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use rez_next_build::{
    BuildConfig, BuildEvent, BuildEventKind, BuildManager, BuildOptions, BuildRequest, BuildStatus,
    BuildStep,
};
use rez_next_common::{RezCoreError, config::RezCoreConfig, error::RezCoreResult};
use rez_next_context::{ContextConfig, EnvironmentManager, ResolvedContext};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::collections::{HashMap, HashSet};
use std::io;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::cli::utils::expand_home_path;

/// Arguments for the build command
#[derive(Args, Clone, Debug)]
pub struct BuildArgs {
    /// Clear the current build before rebuilding
    #[arg(short = 'c', long = "clean")]
    pub clean: bool,

    /// Install the build to the local packages path
    #[arg(short = 'i', long = "install")]
    pub install: bool,

    /// Install to a custom package repository path
    #[arg(short = 'p', long = "prefix", value_name = "PATH")]
    pub prefix: Option<String>,

    /// Display resolve graph as an image if build environment fails to resolve
    #[arg(long = "fail-graph")]
    pub fail_graph: bool,

    /// Create build scripts rather than performing the full build
    #[arg(short = 's', long = "scripts")]
    pub scripts: bool,

    /// Just view the preprocessed package definition and exit
    #[arg(long = "view-pre")]
    pub view_pre: bool,

    /// The build process to use
    #[arg(long = "process", default_value = "local")]
    pub process: String,

    /// The build system to use (auto-detected if not specified)
    #[arg(short = 'b', long = "build-system")]
    pub build_system: Option<String>,

    /// Select variants to build (zero-indexed)
    #[arg(long = "variants", value_name = "INDEX")]
    pub variants: Option<Vec<usize>>,

    /// Arguments to pass to the build system
    #[arg(long = "ba", long = "build-args", value_name = "ARGS")]
    pub build_args: Option<String>,

    /// Arguments to pass to the child build system
    #[arg(long = "cba", long = "child-build-args", value_name = "ARGS")]
    pub child_build_args: Option<String>,

    /// Build in release mode
    #[arg(short = 'r', long = "release")]
    pub release: bool,

    /// Skip tests during build
    #[arg(long = "skip-tests")]
    pub skip_tests: bool,

    /// Force rebuild even if artifacts exist
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Build log display mode
    #[arg(long = "output", value_enum, default_value_t = BuildOutputMode::Auto)]
    pub output: BuildOutputMode,

    /// Open an interactive terminal build report when the terminal supports it
    #[arg(long = "tui")]
    pub tui: bool,

    /// Package source (directory, URL, or Git repository)
    /// Examples:
    /// - Local directory: ./my-package or /path/to/package
    /// - Git repository: `https://github.com/user/repo`
    /// - Git with branch/tag: `https://github.com/user/repo@main`
    /// - HTTP archive: `https://example.com/package.tar.gz`
    /// - SSH Git: git@github.com:user/repo.git
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,

    /// Subdirectory within the source (for archives or repositories)
    #[arg(long = "subdir")]
    pub subdir: Option<String>,

    /// Git reference (branch, tag, or commit) for Git sources
    #[arg(long = "reference")]
    pub reference: Option<String>,
}

/// Build output display mode.
#[derive(Copy, Clone, Debug, ValueEnum)]
pub enum BuildOutputMode {
    /// Show a compact Rez-style report with key log lines
    Auto,
    /// Collapse noisy build logs and show only important lines
    Compact,
    /// Expand all captured build output
    Full,
}

impl BuildOutputMode {
    fn effective(self) -> Self {
        match self {
            BuildOutputMode::Auto => BuildOutputMode::Compact,
            other => other,
        }
    }
}

/// Execute the build command
pub fn execute(args: BuildArgs) -> RezCoreResult<()> {
    // Determine source directory first
    let (source_dir, package) = if let Some(ref source_url) = args.source {
        // Network or remote source
        fetch_and_load_remote_source(source_url, &args)?
    } else {
        // Local source (current directory)
        let working_dir = std::env::current_dir().map_err(RezCoreError::Io)?;
        let package = load_current_package(&working_dir)?;
        (working_dir, package)
    };

    // Handle view-pre option
    if args.view_pre {
        return view_preprocessed_package_with_data(&package);
    }

    print_package_build_header(&package);

    // Create build options
    let build_options = BuildOptions {
        force_rebuild: args.force || args.clean,
        skip_tests: args.skip_tests,
        release_mode: args.release,
        build_args: parse_build_args(&args.build_args),
        env_vars: collect_build_plugin_env_vars(),
    };

    // Determine install path if installing
    let install_path = if args.install {
        Some(get_install_path(&args)?)
    } else {
        None
    };

    // Create build request(s) — one per selected variant (or one without variant)
    let variant_indices: Vec<Option<usize>> = if let Some(ref indices) = args.variants {
        // Validate indices against available variants
        let max_variant = package.variants.len();
        let valid: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|&i| {
                if i >= max_variant {
                    eprintln!(
                        "Warning: variant index {} out of range (package has {} variants), skipping",
                        i, max_variant
                    );
                    false
                } else {
                    true
                }
            })
            .collect();

        if valid.is_empty() && !indices.is_empty() {
            return Err(RezCoreError::BuildError(
                "All specified variant indices are out of range".to_string(),
            ));
        }

        if valid.is_empty() {
            vec![None]
        } else {
            valid.into_iter().map(Some).collect()
        }
    } else if !package.variants.is_empty() {
        // Build all variants by default
        (0..package.variants.len()).map(Some).collect()
    } else {
        vec![None]
    };

    if args.verbose && variant_indices.len() > 1 {
        println!("🔀 Building {} variants...", variant_indices.len());
    }

    let total_variants = variant_indices.len();
    for (variant_number, variant_idx) in variant_indices.into_iter().enumerate() {
        print_variant_header(variant_idx.unwrap_or(0), variant_number + 1, total_variants);

        // Convert variant index to variant_requires
        let variant_requires: Option<Vec<String>> =
            variant_idx.map(|i| package.variants.get(i).cloned().unwrap_or_default());

        let context = resolve_build_context(&package, variant_requires.as_deref(), args.verbose)?;
        print_resolve_summary(
            &package,
            variant_requires.as_deref(),
            context.as_ref(),
            args.output,
        );

        let build_request = BuildRequest {
            package: package.clone(),
            context,
            source_dir: source_dir.clone(),
            variant_index: variant_idx,
            variant_requires,
            options: build_options.clone(),
            install_path: install_path.clone(),
        };

        execute_build(build_request, &args, &package, &source_dir)?;
    }

    Ok(())
}

/// Fetch and load package from remote source
fn fetch_and_load_remote_source(
    source_url: &str,
    args: &BuildArgs,
) -> RezCoreResult<(PathBuf, Package)> {
    use rez_next_build::SourceManager;
    use tempfile::TempDir;

    if args.verbose {
        println!("🌐 Fetching remote source: {}", source_url);
    }

    // Create source manager
    let source_manager = SourceManager::new();

    // Parse source URL
    let mut network_source = source_manager.parse_source(source_url)?;

    // Apply additional options from command line
    if let Some(ref subdir) = args.subdir {
        network_source.subdirectory = Some(subdir.clone());
    }
    if let Some(ref reference) = args.reference {
        network_source.reference = Some(reference.clone());
    }

    // Create temporary directory for fetching
    let temp_dir = TempDir::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create temp directory: {}", e)))?;

    // Fetch source
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create async runtime: {}", e)))?;

    let source_path = runtime.block_on(async {
        source_manager
            .fetch_source(&network_source, temp_dir.path())
            .await
    })?;

    if args.verbose {
        println!("📁 Source fetched to: {}", source_path.display());
    }

    // Load package from fetched source
    let package = load_current_package(&source_path)?;

    // Keep temp directory alive by converting to persistent path
    let persistent_path = copy_to_persistent_location(&source_path, &package)?;

    Ok((persistent_path, package))
}

/// Copy source to a persistent location for building
fn copy_to_persistent_location(source_path: &PathBuf, package: &Package) -> RezCoreResult<PathBuf> {
    use std::fs;

    // Create build cache directory
    let cache_dir = std::env::temp_dir().join("rez-core-build-cache");
    fs::create_dir_all(&cache_dir).map_err(|e| {
        RezCoreError::BuildError(format!("Failed to create cache directory: {}", e))
    })?;

    // Create unique directory for this package
    let unique_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let package_cache_dir = cache_dir.join(format!(
        "{}-{}-{}-{}",
        package.name,
        package
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown"),
        std::process::id(),
        unique_suffix
    ));

    // Copy source to cache directory
    copy_dir_recursive(source_path, &package_cache_dir)?;

    Ok(package_cache_dir)
}

/// Recursively copy directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> RezCoreResult<()> {
    use std::fs;

    fs::create_dir_all(dest)
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create directory: {}", e)))?;

    for entry in fs::read_dir(src)
        .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?
    {
        let entry = entry.map_err(|e| {
            RezCoreError::BuildError(format!("Failed to read directory entry: {}", e))
        })?;

        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)
                .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
        }
    }

    Ok(())
}

/// Load package from current directory
fn load_current_package(working_dir: &Path) -> RezCoreResult<Package> {
    Package::from_path(working_dir)
        .map_err(|e| RezCoreError::PackageParse(format!("Failed to load package: {}", e)))
}

fn resolve_build_context(
    package: &Package,
    variant_requires: Option<&[String]>,
    verbose: bool,
) -> RezCoreResult<Option<ResolvedContext>> {
    let requirement_strings = collect_build_context_requirements(package, variant_requires);
    if requirement_strings.is_empty() {
        return Ok(None);
    }

    if verbose {
        println!(
            "🔎 Resolving build environment: {}",
            requirement_strings.join(", ")
        );
    }

    let mut repo_manager = RepositoryManager::new();
    let config = RezCoreConfig::load();
    for (index, search_path) in config.packages_path.iter().enumerate() {
        let path = expand_home_path(search_path);
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path,
                format!("build_repo_{}", index),
            )));
        }
    }

    let requirements = requirement_strings
        .iter()
        .map(|requirement| {
            requirement.parse::<Requirement>().map_err(|err| {
                RezCoreError::RequirementParse(format!(
                    "Failed to parse build requirement '{}': {}",
                    requirement, err
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let context_requirements = requirement_strings
        .iter()
        .map(|requirement| {
            PackageRequirement::parse(requirement).map_err(|err| {
                RezCoreError::RequirementParse(format!(
                    "Failed to parse build context requirement '{}': {}",
                    requirement, err
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create async runtime: {}", e)))?;
    let mut resolver = DependencyResolver::new(
        Arc::new(repo_manager),
        SolverConfig {
            strict_mode: true,
            ..SolverConfig::default()
        },
    );
    let resolution = rt.block_on(resolver.resolve(requirements))?;

    let mut context = ResolvedContext::from_requirements(context_requirements);
    context.resolved_packages = resolution
        .resolved_packages
        .into_iter()
        .map(|info| info.materialized_package())
        .collect();
    context.status = rez_next_context::ContextStatus::Resolved;

    let env_manager = EnvironmentManager::new(ContextConfig {
        inherit_parent_env: false,
        ..Default::default()
    });
    context.environment_vars =
        rt.block_on(env_manager.generate_environment(&context.resolved_packages))?;

    Ok(Some(context))
}

fn collect_build_context_requirements(
    package: &Package,
    variant_requires: Option<&[String]>,
) -> Vec<String> {
    let mut requirements = Vec::new();
    for requirement in package
        .requires
        .iter()
        .chain(package.build_requires.iter())
        .chain(package.private_build_requires.iter())
        .chain(variant_requires.unwrap_or(&[]).iter())
    {
        if !requirements.contains(requirement) {
            requirements.push(requirement.clone());
        }
    }
    requirements
}

fn print_package_build_header(package: &Package) {
    print_banner(&format!("Building {}...", package_label(package)));
}

fn print_variant_header(variant_index: usize, variant_number: usize, total_variants: usize) {
    print_banner(&format!(
        "Building variant {} ({}/{})...",
        variant_index, variant_number, total_variants
    ));
}

fn print_resolve_summary(
    package: &Package,
    variant_requires: Option<&[String]>,
    context: Option<&ResolvedContext>,
    output_mode: BuildOutputMode,
) {
    let requirements = collect_build_context_requirements(package, variant_requires);
    if requirements.is_empty() {
        println!("Resolving build environment: none");
        println!();
        return;
    }

    println!("Resolving build environment: {}", requirements.join(" "));
    if let Some(context) = context {
        println!();
        println!("requested packages:");
        print_collapsible_lines(
            requirements.iter().map(String::as_str),
            output_mode,
            16,
            "requested package",
        );
        println!();
        println!("resolved packages:");
        let resolved_lines = context.resolved_packages.iter().map(|package| {
            let version = package
                .version
                .as_ref()
                .map(|version| version.as_str())
                .unwrap_or("unknown");
            let root = package
                .root()
                .map(|path| path.to_string())
                .unwrap_or_else(|| "<unknown>".to_string());
            format!("{:<28} {}", format!("{}-{}", package.name, version), root)
        });
        print_collapsible_lines(resolved_lines, output_mode, 24, "resolved package");
    }
    println!();
}

fn print_banner(title: &str) {
    let line = "=".repeat(80);
    println!("{line}");
    println!("{title}");
    println!("{line}");
    println!();
}

fn package_label(package: &Package) -> String {
    package
        .version
        .as_ref()
        .map(|version| format!("{}-{}", package.name, version.as_str()))
        .unwrap_or_else(|| package.name.clone())
}

/// Parse build arguments string into vector
fn parse_build_args(args_str: &Option<String>) -> Vec<String> {
    match args_str {
        Some(args) => args.split_whitespace().map(|s| s.to_string()).collect(),
        None => Vec::new(),
    }
}

fn collect_build_plugin_env_vars() -> HashMap<String, String> {
    std::env::vars()
        .filter(|(key, _)| {
            key.starts_with("REZ_BINARY_ARCHIVE_")
                || key.starts_with("REZ_PYPI_")
                || key.starts_with("REZ_BUILD_TARGET_")
        })
        .collect()
}

/// View preprocessed package definition with package data
fn view_preprocessed_package_with_data(package: &Package) -> RezCoreResult<()> {
    // Print package information in Python format
    println!("# Preprocessed package definition");
    println!("name = '{}'", package.name);

    if let Some(ref version) = package.version {
        println!("version = '{}'", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description = '{}'", description);
    }

    if !package.requires.is_empty() {
        println!("requires = [");
        for req in &package.requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    if !package.build_requires.is_empty() {
        println!("build_requires = [");
        for req in &package.build_requires {
            println!("    '{}',", req);
        }
        println!("]");
    }

    Ok(())
}

/// Execute the build process
fn execute_build(
    request: BuildRequest,
    args: &BuildArgs,
    _package: &Package,
    _source_dir: &PathBuf,
) -> RezCoreResult<()> {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let build_config = BuildConfig {
        event_sender: Some(event_sender),
        ..Default::default()
    };
    let mut build_manager = BuildManager::with_config(build_config);

    if args.verbose {
        println!("🔧 Configuring build environment...");
    }

    // Start build process
    let runtime = Runtime::new()
        .map_err(|e| RezCoreError::BuildError(format!("Failed to create async runtime: {}", e)))?;
    let package = request.package.clone();
    let variant_index = request.variant_index;

    // start_build() now returns Vec<String> (for variant builds)
    let build_ids: Vec<String> =
        runtime.block_on(async { build_manager.start_build(request).await })?;

    if args.verbose {
        println!("🔧 Configuring build environment...");
        for (i, id) in build_ids.iter().enumerate() {
            println!("🚀 Build {} started with ID: {}", i, id);
        }
    }

    let results = if args.tui && std::io::stdout().is_terminal() {
        wait_for_builds_with_tui(
            &runtime,
            &mut build_manager,
            &build_ids,
            event_receiver,
            &package,
            variant_index,
            args.output,
        )?
    } else {
        wait_for_builds_with_text(
            &runtime,
            &mut build_manager,
            &build_ids,
            event_receiver,
            args.output,
        )?
    };
    let build_count = validate_build_results(results)?;

    print_build_success_summary(build_count, &TermStyle::detect());

    // Installation is handled by the build system's install step
    // No need for separate installation logic here

    if args.verbose {
        println!("✅ Build completed successfully!");
    }

    Ok(())
}

fn validate_build_results(results: Vec<rez_next_build::BuildResult>) -> RezCoreResult<usize> {
    if results.is_empty() {
        return Err(RezCoreError::BuildError(
            "No build occurred - no package specifications provided".to_string(),
        ));
    }

    let failures: Vec<String> = results
        .iter()
        .filter(|result| !result.success)
        .map(|result| format!("{}: {}", result.build_id, result.errors))
        .collect();
    if !failures.is_empty() {
        return Err(RezCoreError::BuildError(format!(
            "Build failed: {}",
            failures.join("; ")
        )));
    }

    Ok(results.len())
}

fn wait_for_builds_with_text(
    runtime: &Runtime,
    build_manager: &mut BuildManager,
    build_ids: &[String],
    mut event_receiver: mpsc::UnboundedReceiver<BuildEvent>,
    output_mode: BuildOutputMode,
) -> RezCoreResult<Vec<rez_next_build::BuildResult>> {
    let mut renderer = LiveTextBuildRenderer::new(output_mode);
    while !all_builds_finished(build_manager, build_ids) {
        while let Ok(event) = event_receiver.try_recv() {
            renderer.on_event(event);
        }
        runtime.block_on(async { tokio::time::sleep(std::time::Duration::from_millis(50)).await });
    }

    while let Ok(event) = event_receiver.try_recv() {
        renderer.on_event(event);
    }

    collect_build_results(runtime, build_manager, build_ids)
}

fn wait_for_builds_with_tui(
    runtime: &Runtime,
    build_manager: &mut BuildManager,
    build_ids: &[String],
    mut event_receiver: mpsc::UnboundedReceiver<BuildEvent>,
    package: &Package,
    variant_index: Option<usize>,
    output_mode: BuildOutputMode,
) -> RezCoreResult<Vec<rez_next_build::BuildResult>> {
    let mut stdout = io::stdout();
    enable_raw_mode().map_err(|err| RezCoreError::BuildError(err.to_string()))?;
    execute!(stdout, EnterAlternateScreen)
        .map_err(|err| RezCoreError::BuildError(err.to_string()))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|err| RezCoreError::BuildError(err.to_string()))?;
    let install_path = package_install_preview(build_manager);
    let mut app = BuildTuiApp::live(package, variant_index, install_path, output_mode);
    let result = run_live_build_tui(
        runtime,
        build_manager,
        build_ids,
        &mut event_receiver,
        &mut terminal,
        &mut app,
    );

    disable_raw_mode().map_err(|err| RezCoreError::BuildError(err.to_string()))?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .map_err(|err| RezCoreError::BuildError(err.to_string()))?;
    terminal
        .show_cursor()
        .map_err(|err| RezCoreError::BuildError(err.to_string()))?;

    result?;
    collect_build_results(runtime, build_manager, build_ids)
}

fn run_live_build_tui(
    runtime: &Runtime,
    build_manager: &BuildManager,
    build_ids: &[String],
    event_receiver: &mut mpsc::UnboundedReceiver<BuildEvent>,
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut BuildTuiApp,
) -> RezCoreResult<()> {
    loop {
        while let Ok(event) = event_receiver.try_recv() {
            app.on_event(event);
        }

        terminal
            .draw(|frame| draw_build_tui(frame, app))
            .map_err(|err| RezCoreError::BuildError(err.to_string()))?;

        if event::poll(std::time::Duration::from_millis(40))
            .map_err(|err| RezCoreError::BuildError(err.to_string()))?
        {
            if let Event::Key(key) =
                event::read().map_err(|err| RezCoreError::BuildError(err.to_string()))?
            {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc if app.finished => break,
                    KeyCode::Down | KeyCode::Char('j') => app.next(),
                    KeyCode::Up | KeyCode::Char('k') => app.previous(),
                    KeyCode::Enter | KeyCode::Char(' ') => app.toggle_selected(),
                    KeyCode::Char('c') => app.toggle_mode(),
                    _ => {}
                }
            }
        }

        if all_builds_finished(build_manager, build_ids) {
            while let Ok(event) = event_receiver.try_recv() {
                app.on_event(event);
            }
            app.finished = true;
            terminal
                .draw(|frame| draw_build_tui(frame, app))
                .map_err(|err| RezCoreError::BuildError(err.to_string()))?;
            break;
        }

        runtime.block_on(async { tokio::time::sleep(std::time::Duration::from_millis(25)).await });
    }

    Ok(())
}

fn collect_build_results(
    runtime: &Runtime,
    build_manager: &mut BuildManager,
    build_ids: &[String],
) -> RezCoreResult<Vec<rez_next_build::BuildResult>> {
    let mut results = Vec::new();
    for build_id in build_ids {
        results.push(runtime.block_on(async { build_manager.wait_for_build(build_id).await })?);
    }
    Ok(results)
}

fn all_builds_finished(build_manager: &BuildManager, build_ids: &[String]) -> bool {
    build_ids.iter().all(|build_id| {
        matches!(
            build_manager.get_build_status(build_id),
            Some(BuildStatus::Success | BuildStatus::Failed | BuildStatus::Cancelled)
        )
    })
}

fn package_install_preview(build_manager: &BuildManager) -> String {
    build_manager
        .get_config()
        .build_dir
        .join("<package>")
        .display()
        .to_string()
}

struct LiveTextBuildRenderer {
    output_mode: BuildOutputMode,
    term: TermStyle,
    seen_output: HashSet<(String, String)>,
}

impl LiveTextBuildRenderer {
    fn new(output_mode: BuildOutputMode) -> Self {
        println!("Invoking build system...");
        Self {
            output_mode,
            term: TermStyle::detect(),
            seen_output: HashSet::new(),
        }
    }

    fn on_event(&mut self, event: BuildEvent) {
        match event.kind {
            BuildEventKind::BuildStarted => {
                println!("{}", event.message);
            }
            BuildEventKind::StepStarted => {
                if let Some(step) = event.step {
                    println!(
                        "{}",
                        self.term
                            .step(&format!("  {:<11} running", step_label(&step)))
                    );
                }
            }
            BuildEventKind::StepOutput => {
                if self.remember_output(&event)
                    && (matches!(self.output_mode.effective(), BuildOutputMode::Full)
                        || should_show_build_line(&event.message))
                {
                    println!("    {}", event.message.trim());
                }
            }
            BuildEventKind::StepError => {
                if self.remember_output(&event) {
                    eprintln!("    {}", event.message.trim());
                }
            }
            BuildEventKind::StepFinished => {
                if let Some(step) = event.step {
                    let status = if event.success.unwrap_or(false) {
                        self.term.ok("ok")
                    } else {
                        "failed".to_string()
                    };
                    let duration = event
                        .duration_ms
                        .map(|duration| format!(" ({duration} ms)"))
                        .unwrap_or_default();
                    println!(
                        "{} {}{}",
                        self.term.step(&format!("  {:<11}", step_label(&step))),
                        status,
                        duration
                    );
                }
            }
            BuildEventKind::BuildFinished => {
                if !event.success.unwrap_or(false) {
                    eprintln!("{}", event.message);
                }
            }
        }
    }

    fn remember_output(&mut self, event: &BuildEvent) -> bool {
        let step = event
            .step
            .as_ref()
            .map(step_label)
            .unwrap_or("build")
            .to_string();
        self.seen_output.insert((step, event.message.clone()))
    }
}

struct BuildTuiApp {
    package_label: String,
    install_path: String,
    sections: Vec<BuildOutputSection>,
    selected: usize,
    expanded: Vec<bool>,
    output_mode: BuildOutputMode,
    status: String,
    finished: bool,
}

impl BuildTuiApp {
    fn live(
        package: &Package,
        variant_index: Option<usize>,
        install_path: String,
        output_mode: BuildOutputMode,
    ) -> Self {
        let variant_label = variant_index
            .map(|index| format!(" variant {}", index))
            .unwrap_or_default();
        let sections = [
            BuildStep::Preparing,
            BuildStep::Configuring,
            BuildStep::Compiling,
            BuildStep::Testing,
            BuildStep::Packaging,
            BuildStep::Installing,
            BuildStep::Cleanup,
        ]
        .into_iter()
        .map(|step| BuildOutputSection {
            name: step_label(&step).to_string(),
            body_lines: Vec::new(),
            status: "queued".to_string(),
            duration_ms: None,
        })
        .collect::<Vec<_>>();
        let expanded = sections
            .iter()
            .map(|section| matches!(section.name.as_str(), "compiling" | "installing"))
            .collect();

        Self {
            package_label: format!("{}{}", package_label(package), variant_label),
            install_path,
            sections,
            selected: 0,
            expanded,
            output_mode,
            status: "starting".to_string(),
            finished: false,
        }
    }

    fn on_event(&mut self, event: BuildEvent) {
        match event.kind {
            BuildEventKind::BuildStarted => {
                self.status = event.message;
            }
            BuildEventKind::StepStarted => {
                if let Some(step) = event.step {
                    self.ensure_step(&step).status = "running".to_string();
                }
            }
            BuildEventKind::StepOutput | BuildEventKind::StepError => {
                if let Some(step) = event.step {
                    if let Some(path) = event.message.strip_prefix("Created install directory: ") {
                        self.install_path = path.to_string();
                    }
                    let section = self.ensure_step(&step);
                    if !section.body_lines.contains(&event.message) {
                        section.body_lines.push(event.message);
                    }
                }
            }
            BuildEventKind::StepFinished => {
                if let Some(step) = event.step {
                    let section = self.ensure_step(&step);
                    section.status = if event.success.unwrap_or(false) {
                        "ok".to_string()
                    } else {
                        "failed".to_string()
                    };
                    section.duration_ms = event.duration_ms;
                }
            }
            BuildEventKind::BuildFinished => {
                self.status = event.message;
                self.finished = true;
            }
        }
    }

    fn ensure_step(&mut self, step: &BuildStep) -> &mut BuildOutputSection {
        let name = step_label(step).to_string();
        if let Some(index) = self
            .sections
            .iter()
            .position(|section| section.name == name)
        {
            return &mut self.sections[index];
        }

        self.sections.push(BuildOutputSection {
            name,
            body_lines: Vec::new(),
            status: "running".to_string(),
            duration_ms: None,
        });
        self.expanded.push(false);
        self.sections.last_mut().expect("section was just pushed")
    }

    fn next(&mut self) {
        if self.sections.is_empty() {
            return;
        }
        self.selected = (self.selected + 1).min(self.sections.len() - 1);
    }

    fn previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    fn toggle_selected(&mut self) {
        if let Some(expanded) = self.expanded.get_mut(self.selected) {
            *expanded = !*expanded;
        }
    }

    fn toggle_mode(&mut self) {
        self.output_mode = match self.output_mode.effective() {
            BuildOutputMode::Full => BuildOutputMode::Compact,
            BuildOutputMode::Auto | BuildOutputMode::Compact => BuildOutputMode::Full,
        };
    }
}

fn draw_build_tui(frame: &mut ratatui::Frame<'_>, app: &BuildTuiApp) {
    let area = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    let title = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                "rez-build ",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.package_label),
        ]),
        Line::from(format!("{} | install: {}", app.status, app.install_path)),
    ])
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, vertical[0]);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(vertical[1]);

    let items: Vec<ListItem> = app
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| {
            let marker = if app.expanded.get(index).copied().unwrap_or(false) {
                "[-]"
            } else {
                "[+]"
            };
            let duration = section
                .duration_ms
                .map(|duration| format!(" {duration}ms"))
                .unwrap_or_default();
            ListItem::new(format!(
                "{marker} {:<11} {}{}",
                section.name.to_ascii_lowercase(),
                section.status,
                duration
            ))
        })
        .collect();
    let mut state = ListState::default();
    if !app.sections.is_empty() {
        state.select(Some(app.selected));
    }
    let list = List::new(items)
        .block(Block::default().title("steps").borders(Borders::ALL))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::Cyan));
    frame.render_stateful_widget(list, horizontal[0], &mut state);

    let log_lines = selected_tui_lines(app);
    let logs = Paragraph::new(log_lines.join("\n"))
        .block(Block::default().title("log").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(logs, horizontal[1]);

    let help_text = if app.finished {
        "Up/Down: select  Enter: fold/unfold  c: compact/full  q: quit"
    } else {
        "Live build dashboard  Up/Down: select  Enter: fold/unfold  c: compact/full"
    };
    let help = Paragraph::new(help_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, vertical[2]);
}

fn selected_tui_lines(app: &BuildTuiApp) -> Vec<String> {
    let Some(section) = app.sections.get(app.selected) else {
        return vec!["No build output".to_string()];
    };
    if !app.expanded.get(app.selected).copied().unwrap_or(false) {
        return vec!["Section collapsed. Press Enter to expand.".to_string()];
    }

    match app.output_mode.effective() {
        BuildOutputMode::Full => section.body_lines.clone(),
        BuildOutputMode::Auto | BuildOutputMode::Compact => {
            let mut lines: Vec<String> = section
                .body_lines
                .iter()
                .filter(|line| should_show_build_line(line))
                .cloned()
                .collect();
            let hidden = section
                .body_lines
                .iter()
                .filter(|line| !should_show_build_line(line) && !is_quiet_build_line(line))
                .count();
            if hidden > 0 {
                lines.push(format!("... {hidden} log line(s) hidden; press c for full"));
            }
            if lines.is_empty() {
                lines.push("No important log lines in compact mode. Press c for full.".to_string());
            }
            lines
        }
    }
}

fn print_collapsible_lines<I, S>(
    lines: I,
    output_mode: BuildOutputMode,
    compact_limit: usize,
    item_name: &str,
) where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let lines: Vec<String> = lines
        .into_iter()
        .map(|line| line.as_ref().to_string())
        .collect();
    let effective = output_mode.effective();
    let visible_count = match effective {
        BuildOutputMode::Full => lines.len(),
        BuildOutputMode::Auto | BuildOutputMode::Compact => lines.len().min(compact_limit),
    };

    for line in lines.iter().take(visible_count) {
        println!("{line}");
    }

    let hidden = lines.len().saturating_sub(visible_count);
    if hidden > 0 {
        println!("... {hidden} {item_name}(s) hidden; use --output full to expand");
    }
}

fn print_build_success_summary(count: usize, term: &TermStyle) {
    print_banner("Build Summary");
    println!(
        "{}",
        term.ok(&format!("All {} build(s) were successful.", count))
    );
}

struct BuildOutputSection {
    name: String,
    body_lines: Vec<String>,
    status: String,
    duration_ms: Option<u64>,
}

fn step_label(step: &BuildStep) -> &'static str {
    match step {
        BuildStep::Preparing => "preparing",
        BuildStep::Configuring => "configuring",
        BuildStep::Compiling => "compiling",
        BuildStep::Testing => "testing",
        BuildStep::Packaging => "packaging",
        BuildStep::Installing => "installing",
        BuildStep::Cleanup => "cleanup",
    }
}

fn should_show_build_line(line: &str) -> bool {
    line.contains("Builder:")
        || line.starts_with("Invoking ")
        || line.starts_with("Running build command:")
        || line.contains("installed")
        || line.contains("Copied ")
        || line.contains("Created ")
        || line.contains("Building ")
        || line.contains("Processing ")
        || line.contains("Successfully ")
        || line.contains("Cleaning ")
        || line.contains("Creating ")
        || line.contains("Moving ")
        || line.contains("ready")
        || line.contains("skipped")
}

fn is_quiet_build_line(line: &str) -> bool {
    matches!(
        line.trim(),
        "Tests completed"
            | "Packaging completed"
            | "Cleanup completed"
            | "Configuration completed for script: rezbuild.py"
    )
}

struct TermStyle {
    color: bool,
}

impl TermStyle {
    fn detect() -> Self {
        Self {
            color: std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
        }
    }

    fn step(&self, value: &str) -> String {
        self.paint(value, "36")
    }

    fn ok(&self, value: &str) -> String {
        self.paint(value, "32")
    }

    fn paint(&self, value: &str, code: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{value}\x1b[0m")
        } else {
            value.to_string()
        }
    }
}

/// Get installation path
fn get_install_path(args: &BuildArgs) -> RezCoreResult<PathBuf> {
    use crate::cli::utils::expand_home_path;
    use rez_next_common::config::RezCoreConfig;

    let raw = if let Some(ref prefix) = args.prefix {
        prefix.as_str().to_owned()
    } else {
        let config = RezCoreConfig::load();
        if args.release {
            config.release_packages_path.clone()
        } else {
            config.local_packages_path.clone()
        }
    };

    // Expand ~ and resolve relative paths
    let expanded = expand_home_path(&raw);
    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        std::env::current_dir()
            .map_err(|e| RezCoreError::ConfigError(format!("Cannot get current directory: {e}")))?
            .join(expanded)
    };

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_build::{BuildArtifacts, BuildResult};

    #[test]
    fn test_validate_build_results_checks_every_result() {
        let results = vec![
            BuildResult::success("first".to_string(), BuildArtifacts::default(), 1),
            BuildResult::failure("second".to_string(), "second failed".to_string(), 1),
        ];

        let error = validate_build_results(results)
            .expect_err("a later failed build must fail the command");
        assert!(error.to_string().contains("second failed"), "{error}");
    }
}
