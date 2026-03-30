//! Bind command implementation
//!
//! Implements the `rez bind` command for converting system software into rez packages.
//! This performs actual system detection and writes real package.py files to disk.

use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Arguments for the bind command
#[derive(Args, Clone, Debug)]
pub struct BindArgs {
    /// Package to bind (supports version ranges like 'python-3.9+')
    #[arg(value_name = "PKG")]
    pub package: Option<String>,

    /// Bind a set of standard packages to get started
    #[arg(long)]
    pub quickstart: bool,

    /// Install to release path; overrides -i
    #[arg(short = 'r', long)]
    pub release: bool,

    /// Install path, defaults to local package path
    #[arg(short = 'i', long = "install-path", value_name = "PATH")]
    pub install_path: Option<PathBuf>,

    /// Do not bind dependencies
    #[arg(long = "no-deps")]
    pub no_deps: bool,

    /// List all available bind modules
    #[arg(short = 'l', long)]
    pub list: bool,

    /// Search for the bind module but do not perform the bind
    #[arg(short = 's', long)]
    pub search: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Additional bind arguments
    #[arg(last = true)]
    pub bind_args: Vec<String>,
}

/// Bind module information
#[derive(Debug, Clone)]
pub struct BindModule {
    /// Module name
    pub name: String,
    /// Module file path
    pub path: PathBuf,
    /// Module description
    pub description: Option<String>,
    /// Supported platforms
    pub platforms: Vec<String>,
}

/// Detected system tool information
#[derive(Debug, Clone)]
pub struct DetectedTool {
    /// Detected version string
    pub version: String,
    /// Executable path
    pub executable_path: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Bind result
#[derive(Debug, Clone)]
pub struct BindResult {
    /// Bound package
    pub package: Package,
    /// Installation path
    pub install_path: PathBuf,
    /// Success status
    pub success: bool,
    /// Error message if any
    pub error: Option<String>,
}

/// Execute the bind command
pub fn execute(args: BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Rez Bind - Converting system software to rez packages...");
    }

    // Handle list option
    if args.list {
        return list_bind_modules(&args);
    }

    // Handle quickstart option
    if args.quickstart {
        return execute_quickstart(&args);
    }

    // Handle search option
    if args.search {
        if let Some(ref package) = args.package {
            return search_bind_module(package, &args);
        } else {
            return Err(RezCoreError::RequirementParse(
                "Package name required for search".to_string(),
            ));
        }
    }

    // Handle package binding
    if let Some(ref package) = args.package {
        return bind_package(package, &args);
    }

    // No action specified
    eprintln!("Error: No action specified. Use --help for usage information.");
    std::process::exit(1);
}

/// List all available bind modules
fn list_bind_modules(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Listing available bind modules...");
    }

    let modules = get_bind_modules()?;

    if modules.is_empty() {
        println!("No bind modules found.");
        return Ok(());
    }

    // Print header
    println!("{:<20} {:<50}", "PACKAGE", "BIND MODULE");
    println!("{:<20} {:<50}", "-------", "-----------");

    // Print modules
    for (name, module) in modules.iter() {
        println!("{:<20} {:<50}", name, module.path.display());
    }

    if args.verbose {
        println!("\nFound {} bind modules.", modules.len());
    }

    Ok(())
}

/// Execute quickstart binding
fn execute_quickstart(args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Starting quickstart binding...");
    }

    // Standard packages in dependency order
    let quickstart_packages = vec![
        "platform",
        "arch",
        "os",
        "python",
        "rez",
        "setuptools",
        "pip",
    ];

    let install_path = get_install_path(args)?;
    std::fs::create_dir_all(&install_path).map_err(|e| RezCoreError::Io(e))?;

    let mut results = Vec::new();

    for package_name in quickstart_packages {
        println!(
            "Binding {} into {}...",
            package_name,
            install_path.display()
        );

        match bind_single_package(package_name, &install_path, true, args) {
            Ok(result) => {
                if result.success {
                    results.push(result);
                    if args.verbose {
                        println!("  Successfully bound {}", package_name);
                    }
                } else {
                    eprintln!(
                        "  Warning: Failed to bind {}: {}",
                        package_name,
                        result.error.unwrap_or_else(|| "Unknown error".to_string())
                    );
                }
            }
            Err(e) => {
                eprintln!("  Error binding {}: {}", package_name, e);
            }
        }
    }

    if !results.is_empty() {
        println!(
            "\nSuccessfully converted the following software found on the current system into Rez packages:"
        );
        println!();
        print_package_list(&results);
    }

    println!("\nTo bind other software, see what's available using the command 'rez bind --list', then run 'rez bind <package>'.\n");

    Ok(())
}

/// Search for a bind module
fn search_bind_module(package_name: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Searching for bind module: {}", package_name);
    }

    let modules = get_bind_modules()?;

    if let Some(module) = modules.get(package_name) {
        println!("Found bind module for '{}':", package_name);
        println!("  Path: {}", module.path.display());
        if let Some(ref desc) = module.description {
            println!("  Description: {}", desc);
        }
        if !module.platforms.is_empty() {
            println!("  Platforms: {}", module.platforms.join(", "));
        }
    } else {
        println!("'{}' not found.", package_name);

        // Suggest close matches
        let close_matches = find_close_matches(package_name, &modules);
        if !close_matches.is_empty() {
            println!("Close matches:");
            for (name, _) in close_matches {
                println!("  {}", name);
            }
        } else {
            println!("No matches.");
        }
    }

    Ok(())
}

/// Bind a specific package
fn bind_package(package_spec: &str, args: &BindArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("Binding package: {}", package_spec);
    }

    // Parse package specification (name and optional version range)
    let (package_name, _version_range) = parse_package_spec(package_spec)?;

    let install_path = get_install_path(args)?;
    std::fs::create_dir_all(&install_path).map_err(|e| RezCoreError::Io(e))?;

    match bind_single_package(&package_name, &install_path, args.no_deps, args) {
        Ok(result) => {
            if result.success {
                println!("Successfully bound package '{}'", package_name);
                println!("   Installed to: {}", result.install_path.display());

                if args.verbose {
                    println!("   Package details:");
                    println!("     Name: {}", result.package.name);
                    if let Some(ref version) = result.package.version {
                        println!("     Version: {}", version.as_str());
                    }
                    if let Some(ref description) = result.package.description {
                        println!("     Description: {}", description);
                    }
                }
            } else {
                eprintln!(
                    "Failed to bind package '{}': {}",
                    package_name,
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                );
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error binding package '{}': {}", package_name, e);
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Get available bind modules
fn get_bind_modules() -> RezCoreResult<HashMap<String, BindModule>> {
    let mut modules = HashMap::new();

    // Built-in bind modules
    let builtin_modules = vec![
        ("platform", "System platform package"),
        ("arch", "System architecture package"),
        ("os", "Operating system package"),
        ("python", "Python interpreter"),
        ("rez", "Rez package manager"),
        ("setuptools", "Python setuptools"),
        ("pip", "Python pip"),
        ("cmake", "CMake build system"),
        ("git", "Git version control"),
        ("gcc", "GNU Compiler Collection"),
        ("clang", "Clang compiler"),
    ];

    for (name, description) in builtin_modules {
        modules.insert(
            name.to_string(),
            BindModule {
                name: name.to_string(),
                path: PathBuf::from(format!("builtin://{}", name)),
                description: Some(description.to_string()),
                platforms: vec![
                    "windows".to_string(),
                    "linux".to_string(),
                    "darwin".to_string(),
                ],
            },
        );
    }

    Ok(modules)
}

/// Get the installation path from config
fn get_install_path(args: &BindArgs) -> RezCoreResult<PathBuf> {
    let config = RezCoreConfig::load();
    if args.release {
        let path = expand_home_path(&config.release_packages_path);
        Ok(PathBuf::from(path))
    } else if let Some(ref path) = args.install_path {
        Ok(path.clone())
    } else {
        let path = expand_home_path(&config.local_packages_path);
        Ok(PathBuf::from(path))
    }
}

/// Expand ~ in paths
fn expand_home_path(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        path.replacen("~", &home, 1)
    } else {
        path.to_string()
    }
}

/// Parse package specification
fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    if let Some(dash_pos) = spec.rfind('-') {
        let name = spec[..dash_pos].to_string();
        let version = spec[dash_pos + 1..].to_string();

        if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

/// Detect a system tool and return its info
fn detect_system_tool(name: &str) -> Option<DetectedTool> {
    match name {
        "python" => detect_python(),
        "pip" => detect_pip(),
        "cmake" => detect_cmake(),
        "git" => detect_git(),
        "gcc" => detect_gcc(),
        "clang" => detect_clang(),
        "setuptools" => detect_setuptools(),
        "platform" | "arch" | "os" | "rez" => detect_platform_package(name),
        _ => None,
    }
}

/// Detect Python installation
fn detect_python() -> Option<DetectedTool> {
    // Try python3 first, then python
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };

    for cmd in candidates {
        if let Ok(output) = Command::new(cmd)
            .args(["--version"])
            .output()
        {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout).to_string()
                    + &String::from_utf8_lossy(&output.stderr);
                // Parse "Python 3.9.7" or "Python 2.7.18"
                if let Some(ver) = extract_version_from_output(&version_output, "Python") {
                    let exe_path = which_executable(cmd);
                    return Some(DetectedTool {
                        version: ver,
                        executable_path: exe_path,
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }
    None
}

/// Detect pip installation
fn detect_pip() -> Option<DetectedTool> {
    let candidates = if cfg!(windows) {
        vec!["pip", "pip3"]
    } else {
        vec!["pip3", "pip"]
    };

    for cmd in candidates {
        if let Ok(output) = Command::new(cmd).args(["--version"]).output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout).to_string();
                // "pip 21.3.1 from /path/to/pip"
                if let Some(ver) = extract_version_from_output(&version_output, "pip") {
                    return Some(DetectedTool {
                        version: ver,
                        executable_path: which_executable(cmd),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }
    None
}

/// Detect CMake installation
fn detect_cmake() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("cmake").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "cmake version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("cmake"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

/// Detect Git installation
fn detect_git() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("git").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "git version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("git"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

/// Detect GCC installation
fn detect_gcc() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("gcc").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            // "gcc (Ubuntu 11.4.0-1ubuntu1~22.04) 11.4.0"
            let first_line = version_output.lines().next().unwrap_or("");
            if let Some(ver) = parse_version_from_string(first_line) {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("gcc"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

/// Detect Clang installation
fn detect_clang() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("clang").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "clang version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("clang"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

/// Detect setuptools via Python
fn detect_setuptools() -> Option<DetectedTool> {
    let script = "import setuptools; print(setuptools.__version__)";
    let cmd = if cfg!(windows) { "python" } else { "python3" };
    if let Ok(output) = Command::new(cmd).args(["-c", script]).output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ver.is_empty() {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable(cmd),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

/// Detect platform/arch/os/rez using system info
fn detect_platform_package(name: &str) -> Option<DetectedTool> {
    match name {
        "platform" => {
            let platform = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "osx"
            } else {
                "linux"
            };
            Some(DetectedTool {
                version: "1.0.0".to_string(),
                executable_path: None,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("platform".to_string(), platform.to_string());
                    m
                },
            })
        }
        "arch" => {
            let arch = std::env::consts::ARCH;
            Some(DetectedTool {
                version: "1.0.0".to_string(),
                executable_path: None,
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("arch".to_string(), arch.to_string());
                    m
                },
            })
        }
        "os" => {
            let os_version = get_os_version();
            Some(DetectedTool {
                version: os_version,
                executable_path: None,
                metadata: HashMap::new(),
            })
        }
        "rez" => {
            // Self-detect rez-next version
            Some(DetectedTool {
                version: env!("CARGO_PKG_VERSION").to_string(),
                executable_path: std::env::current_exe().ok(),
                metadata: HashMap::new(),
            })
        }
        _ => None,
    }
}

/// Get OS version string
fn get_os_version() -> String {
    if cfg!(windows) {
        if let Ok(output) = Command::new("cmd").args(["/c", "ver"]).output() {
            let s = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = parse_version_from_string(&s) {
                return ver;
            }
        }
        "10.0".to_string()
    } else if cfg!(target_os = "macos") {
        if let Ok(output) = Command::new("sw_vers").args(["-productVersion"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        "unknown".to_string()
    } else {
        // Linux: read /etc/os-release
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("VERSION_ID=") {
                    return line
                        .trim_start_matches("VERSION_ID=")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        "unknown".to_string()
    }
}

/// Find executable path using `which` or `where`
fn which_executable(cmd: &str) -> Option<PathBuf> {
    let which_cmd = if cfg!(windows) { "where" } else { "which" };
    if let Ok(output) = Command::new(which_cmd).arg(cmd).output() {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let first_line = path_str.lines().next().unwrap_or("").trim();
            if !first_line.is_empty() {
                return Some(PathBuf::from(first_line));
            }
        }
    }
    None
}

/// Extract version after a prefix keyword
fn extract_version_from_output(output: &str, prefix: &str) -> Option<String> {
    let lower = output.to_lowercase();
    let lower_prefix = prefix.to_lowercase();

    for line in lower.lines() {
        if let Some(pos) = line.find(&lower_prefix) {
            let rest = &output[pos + prefix.len()..].trim_start();
            if let Some(ver) = parse_version_from_string(rest) {
                return Some(ver);
            }
        }
    }
    None
}

/// Parse a version number from a string (e.g., "3.9.7 ..." -> "3.9.7")
fn parse_version_from_string(s: &str) -> Option<String> {
    let s = s.trim();
    // Find first digit sequence that looks like a version
    let chars: Vec<char> = s.chars().collect();
    let mut start = None;
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_digit() {
            start = Some(i);
            break;
        }
    }

    if let Some(start) = start {
        let mut end = start;
        while end < chars.len() && (chars[end].is_ascii_digit() || chars[end] == '.' || chars[end] == '-' || chars[end] == '_') {
            end += 1;
        }
        let version_str: String = chars[start..end].iter().collect();
        // Trim trailing dots/dashes
        let version_str = version_str.trim_end_matches(['.', '-', '_']).to_string();
        if !version_str.is_empty() {
            return Some(version_str);
        }
    }
    None
}

/// Generate package.py content for a system package
fn generate_package_py(
    name: &str,
    version: &str,
    description: &str,
    requires: &[String],
    tools: &[String],
    commands: Option<&str>,
) -> String {
    let mut content = String::new();
    content.push_str(&format!("name = '{}'\n", name));
    content.push_str(&format!("version = '{}'\n\n", version));
    content.push_str(&format!("description = '{}'\n\n", description));

    if !requires.is_empty() {
        content.push_str("requires = [\n");
        for req in requires {
            content.push_str(&format!("    '{}',\n", req));
        }
        content.push_str("]\n\n");
    }

    if !tools.is_empty() {
        content.push_str("tools = [\n");
        for tool in tools {
            content.push_str(&format!("    '{}',\n", tool));
        }
        content.push_str("]\n\n");
    }

    if let Some(cmds) = commands {
        content.push_str(&format!("def commands():\n{}\n", cmds));
    }

    content
}

/// Get commands block for a package
fn get_package_commands(name: &str, exe_path: Option<&PathBuf>) -> Option<String> {
    match name {
        "python" => {
            if let Some(path) = exe_path {
                let dir = path.parent().map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !dir.is_empty() {
                    return Some(format!(
                        "    import os\n    env.PATH.prepend('{}')\n",
                        dir.replace('\\', "/")
                    ));
                }
            }
            Some("    env.PATH.prepend('{root}/bin')\n".to_string())
        }
        "pip" | "cmake" | "git" | "gcc" | "clang" => {
            if let Some(path) = exe_path {
                let dir = path.parent().map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !dir.is_empty() {
                    return Some(format!(
                        "    import os\n    env.PATH.prepend('{}')\n",
                        dir.replace('\\', "/")
                    ));
                }
            }
            None
        }
        _ => None,
    }
}

/// Bind a single package - detects system software and writes package.py to disk
fn bind_single_package(
    name: &str,
    install_path: &Path,
    no_deps: bool,
    args: &BindArgs,
) -> RezCoreResult<BindResult> {
    // Detect the system tool
    let detected = detect_system_tool(name);

    let version_str = detected
        .as_ref()
        .map(|d| d.version.clone())
        .unwrap_or_else(|| "1.0.0".to_string());

    let exe_path = detected.as_ref().and_then(|d| d.executable_path.clone());

    let requires = if no_deps {
        vec![]
    } else {
        get_default_requirements(name)
    };

    let tools = get_default_tools(name);
    let description = get_package_description(name);
    let commands = get_package_commands(name, exe_path.as_ref());

    // Parse version for the package object
    let version = rez_next_version::Version::parse(&version_str)?;

    let package = Package {
        name: name.to_string(),
        version: Some(version.clone()),
        description: Some(description.clone()),
        authors: vec!["System".to_string()],
        requires: requires.clone(),
        build_requires: vec![],
        private_build_requires: vec![],
        variants: vec![],
        commands: commands.clone(),
        build_command: None,
        build_system: None,
        pre_commands: None,
        post_commands: None,
        pre_test_commands: None,
        pre_build_commands: None,
        tests: HashMap::new(),
        requires_rez_version: None,
        tools: tools.clone(),
        help: None,
        uuid: None,
        config: HashMap::new(),
        plugin_for: vec![],
        has_plugins: None,
        relocatable: None,
        cachable: None,
        release_message: None,
        changelog: None,
        previous_version: None,
        previous_revision: None,
        revision: None,
        timestamp: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
        ),
        format_version: Some(2),
        base: None,
        hashed_variants: None,
        vcs: None,
        preprocess: None,
    };

    // Write package.py to disk: <install_path>/<name>/<version>/package.py
    let pkg_dir = install_path.join(name).join(&version_str);
    std::fs::create_dir_all(&pkg_dir).map_err(|e| RezCoreError::Io(e))?;

    let pkg_file = pkg_dir.join("package.py");
    let pkg_content = generate_package_py(
        name,
        &version_str,
        &description,
        &requires,
        &tools,
        commands.as_deref(),
    );

    std::fs::write(&pkg_file, &pkg_content).map_err(|e| RezCoreError::Io(e))?;

    if args.verbose {
        println!("  Wrote {}", pkg_file.display());
    }

    Ok(BindResult {
        package,
        install_path: pkg_dir,
        success: true,
        error: None,
    })
}

/// Get package description
fn get_package_description(name: &str) -> String {
    match name {
        "platform" => "System platform package".to_string(),
        "arch" => "System architecture package".to_string(),
        "os" => "Operating system package".to_string(),
        "python" => "Python interpreter".to_string(),
        "rez" => "Rez package manager (rez-next)".to_string(),
        "pip" => "Python package installer".to_string(),
        "setuptools" => "Python build and packaging utilities".to_string(),
        "cmake" => "CMake build system".to_string(),
        "git" => "Git version control system".to_string(),
        "gcc" => "GNU Compiler Collection".to_string(),
        "clang" => "Clang/LLVM compiler".to_string(),
        _ => format!("System package: {}", name),
    }
}

/// Get default requirements for a package
fn get_default_requirements(name: &str) -> Vec<String> {
    match name {
        "os" => vec!["platform".to_string(), "arch".to_string()],
        "python" => vec!["os".to_string()],
        "pip" => vec!["python".to_string()],
        "setuptools" => vec!["python".to_string()],
        _ => vec![],
    }
}

/// Get default tools for a package
fn get_default_tools(name: &str) -> Vec<String> {
    match name {
        "python" => vec!["python".to_string(), "python3".to_string()],
        "pip" => vec!["pip".to_string(), "pip3".to_string()],
        "cmake" => vec!["cmake".to_string(), "ctest".to_string(), "cpack".to_string()],
        "git" => vec!["git".to_string()],
        "gcc" => vec!["gcc".to_string(), "g++".to_string(), "cpp".to_string()],
        "clang" => vec!["clang".to_string(), "clang++".to_string()],
        _ => vec![],
    }
}

/// Find close matches for a package name
fn find_close_matches<'a>(
    name: &str,
    modules: &'a HashMap<String, BindModule>,
) -> Vec<(String, &'a BindModule)> {
    let mut matches = Vec::new();

    for (module_name, module) in modules {
        if module_name.contains(name) || name.contains(module_name.as_str()) {
            matches.push((module_name.clone(), module));
        }
    }

    matches.sort_by(|a, b| a.0.cmp(&b.0));
    matches
}

/// Print package list
fn print_package_list(results: &[BindResult]) {
    println!("{:<20} {:<50}", "PACKAGE", "URI");
    println!("{:<20} {:<50}", "-------", "---");

    for result in results {
        let uri = format!("file://{}", result.install_path.display());
        println!("{:<20} {:<50}", result.package.name, uri);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec() {
        assert_eq!(
            parse_package_spec("python").unwrap(),
            ("python".to_string(), None)
        );

        assert_eq!(
            parse_package_spec("python-3.9").unwrap(),
            ("python".to_string(), Some("3.9".to_string()))
        );

        assert_eq!(
            parse_package_spec("my-package-name").unwrap(),
            ("my-package-name".to_string(), None)
        );
    }

    #[test]
    fn test_get_default_requirements() {
        assert_eq!(get_default_requirements("platform"), Vec::<String>::new());
        assert_eq!(get_default_requirements("os"), vec!["platform", "arch"]);
        assert_eq!(get_default_requirements("python"), vec!["os"]);
    }

    #[test]
    fn test_get_default_tools() {
        assert_eq!(get_default_tools("platform"), Vec::<String>::new());
        assert_eq!(get_default_tools("python"), vec!["python", "python3"]);
        assert_eq!(get_default_tools("cmake"), vec!["cmake", "ctest", "cpack"]);
    }

    #[test]
    fn test_parse_version_from_string() {
        assert_eq!(
            parse_version_from_string("3.9.7 (default, Sep 16 2021, 13:09:03)"),
            Some("3.9.7".to_string())
        );
        assert_eq!(
            parse_version_from_string("git version 2.34.1"),
            Some("2.34.1".to_string())
        );
        assert_eq!(parse_version_from_string("no version here"), None);
    }

    #[test]
    fn test_generate_package_py() {
        let content = generate_package_py(
            "python",
            "3.9.7",
            "Python interpreter",
            &["os".to_string()],
            &["python".to_string()],
            None,
        );
        assert!(content.contains("name = 'python'"));
        assert!(content.contains("version = '3.9.7'"));
        assert!(content.contains("requires = ["));
        assert!(content.contains("'os'"));
        assert!(content.contains("tools = ["));
    }

    #[test]
    fn test_detect_platform_packages() {
        let result = detect_platform_package("platform");
        assert!(result.is_some());
        let result = detect_platform_package("arch");
        assert!(result.is_some());
        let result = detect_platform_package("rez");
        assert!(result.is_some());
    }
}
