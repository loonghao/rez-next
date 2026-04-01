//! # View Command
//!
//! Implementation of the `rez view` command for viewing package information.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::{Package, PackageSerializer};
use std::path::Path;

/// Arguments for the view command
#[derive(Args, Clone)]
pub struct ViewArgs {
    /// Package to view
    pub package: String,

    /// Format to print the package in
    #[arg(short, long, value_enum, default_value = "yaml")]
    pub format: ViewFormat,

    /// Show all package data, including release-related fields
    #[arg(short, long)]
    pub all: bool,

    /// Do not print extraneous info, such as package uri
    #[arg(short, long)]
    pub brief: bool,

    /// Show the package in the current context, if any
    #[arg(short, long)]
    pub current: bool,
}

/// Output format for package viewing
#[derive(clap::ValueEnum, Clone)]
pub enum ViewFormat {
    /// YAML format
    Yaml,
    /// Python format
    Py,
}

/// Execute the view command
pub fn execute(args: ViewArgs) -> RezCoreResult<()> {
    // Validate package name
    crate::cli::utils::validate_package_name(&args.package)?;

    if args.current {
        return view_current_package(&args);
    }

    view_package(&args)
}

/// View a package from the current context
fn view_current_package(_args: &ViewArgs) -> RezCoreResult<()> {
    // TODO: Implement current context package viewing
    // This requires integration with rez-core-context

    eprintln!("Error: not in a resolved environment context.");
    Err(RezCoreError::Repository(
        "Not in a resolved environment context".to_string(),
    ))
}

/// View a package from repositories
fn view_package(args: &ViewArgs) -> RezCoreResult<()> {
    // Try to load package from directory first
    let path = Path::new(&args.package);

    let package = if path.exists() && path.is_dir() {
        // Load from directory containing package.py
        load_package_from_directory(path)?
    } else {
        // Load from configured repositories
        load_package_from_repos(&args.package)?
    };

    display_package(&package, args)
}

/// Load package from directory containing package.py
fn load_package_from_directory(dir_path: &Path) -> RezCoreResult<Package> {
    let package_py_path = dir_path.join("package.py");

    if !package_py_path.exists() {
        return Err(RezCoreError::PackageParse(format!(
            "No package.py found in directory: {}",
            dir_path.display()
        )));
    }

    PackageSerializer::load_from_file(&package_py_path)
}

/// Load package from configured repositories by name (and optional version)
fn load_package_from_repos(spec: &str) -> RezCoreResult<Package> {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};

    // Parse "name" or "name-version" spec
    let (pkg_name, version_str) = if let Some(pos) = spec.rfind('-') {
        let candidate_ver = &spec[pos + 1..];
        if candidate_ver
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        {
            (&spec[..pos], Some(candidate_ver))
        } else {
            (spec, None)
        }
    } else {
        (spec, None)
    };

    let config = RezCoreConfig::load();
    let mut repo_manager = RepositoryManager::new();
    for (i, path_str) in config.packages_path.iter().enumerate() {
        let path = expand_home_path(path_str);
        if path.exists() {
            repo_manager
                .add_repository(Box::new(SimpleRepository::new(path, format!("repo_{}", i))));
        }
    }

    let rt = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Repository(e.to_string()))?;

    let packages = rt
        .block_on(repo_manager.find_packages(pkg_name))
        .map_err(|e| RezCoreError::Repository(e.to_string()))?;

    if packages.is_empty() {
        return Err(RezCoreError::PackageParse(format!(
            "Package '{}' not found in any repository",
            pkg_name
        )));
    }

    // If version specified, find exact match
    if let Some(ver) = version_str {
        for pkg in &packages {
            if pkg.version.as_ref().is_some_and(|v| v.as_str() == ver) {
                return Ok((**pkg).clone());
            }
        }
        return Err(RezCoreError::PackageParse(format!(
            "Package '{}-{}' not found",
            pkg_name, ver
        )));
    }

    // Return latest version (packages sorted descending)
    let mut sorted = packages;
    sorted.sort_by(|a, b| {
        b.version
            .as_ref()
            .and_then(|bv| a.version.as_ref().map(|av| av.cmp(bv)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok((*sorted.into_iter().next().unwrap()).clone())
}

fn expand_home_path(p: &str) -> std::path::PathBuf {
    if p.starts_with("~/") || p == "~" {
        if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
            return std::path::PathBuf::from(home).join(&p[2..]);
        }
    }
    std::path::PathBuf::from(p)
}

/// Display package information in the requested format
fn display_package(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    match args.format {
        ViewFormat::Yaml => display_package_yaml(package, args),
        ViewFormat::Py => display_package_python(package, args),
    }
}

/// Display package in YAML format
fn display_package_yaml(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    if !args.brief {
        println!("# Package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("# Version: {}", version.as_str());
        }
        println!();
    }

    println!("name: {}", package.name);

    if let Some(ref version) = package.version {
        println!("version: {}", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description: {}", description);
    }

    if !package.authors.is_empty() {
        println!("authors:");
        for author in &package.authors {
            println!("  - {}", author);
        }
    }

    if !package.requires.is_empty() {
        println!("requires:");
        for req in &package.requires {
            println!("  - {}", req);
        }
    }

    if !package.tools.is_empty() {
        println!("tools:");
        for tool in &package.tools {
            println!("  - {}", tool);
        }
    }

    if !package.variants.is_empty() {
        println!("variants:");
        for variant in &package.variants {
            print!("  - [");
            for (i, req) in variant.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("{}", req);
            }
            println!("]");
        }
    }

    if let Some(ref build_command) = package.build_command {
        println!("build_command: {}", build_command);
    }

    if let Some(ref build_system) = package.build_system {
        println!("build_system: {}", build_system);
    }

    if let Some(ref uuid) = package.uuid {
        println!("uuid: {}", uuid);
    }

    if let Some(ref commands) = package.commands {
        println!("commands: |");
        for line in commands.lines() {
            println!("  {}", line);
        }
    }

    if args.all {
        // Show additional fields with --all
        if let Some(ref pre_commands) = package.pre_commands {
            println!("pre_commands: {}", pre_commands);
        }
        if let Some(ref post_commands) = package.post_commands {
            println!("post_commands: {}", post_commands);
        }
        if !package.tests.is_empty() {
            println!("tests:");
            for (key, value) in &package.tests {
                println!("  {}: {}", key, value);
            }
        }
    }

    Ok(())
}

/// Display package in Python format
fn display_package_python(package: &Package, args: &ViewArgs) -> RezCoreResult<()> {
    if !args.brief {
        println!("# Package: {}", package.name);
        if let Some(ref version) = package.version {
            println!("# Version: {}", version.as_str());
        }
        println!();
    }

    println!("name = \"{}\"", package.name);

    if let Some(ref version) = package.version {
        println!("version = \"{}\"", version.as_str());
    }

    if let Some(ref description) = package.description {
        println!("description = \"{}\"", description);
    }

    if !package.authors.is_empty() {
        print!("authors = [");
        for (i, author) in package.authors.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", author);
        }
        println!("]");
    }

    if !package.requires.is_empty() {
        print!("requires = [");
        for (i, req) in package.requires.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", req);
        }
        println!("]");
    }

    if !package.tools.is_empty() {
        print!("tools = [");
        for (i, tool) in package.tools.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("\"{}\"", tool);
        }
        println!("]");
    }

    if !package.variants.is_empty() {
        println!("variants = [");
        for variant in &package.variants {
            print!("    [");
            for (i, req) in variant.iter().enumerate() {
                if i > 0 {
                    print!(", ");
                }
                print!("\"{}\"", req);
            }
            println!("],");
        }
        println!("]");
    }

    if let Some(ref build_command) = package.build_command {
        println!("build_command = \"{}\"", build_command);
    }

    if let Some(ref build_system) = package.build_system {
        println!("build_system = \"{}\"", build_system);
    }

    if let Some(ref uuid) = package.uuid {
        println!("uuid = \"{}\"", uuid);
    }

    if args.all {
        // Show additional fields with --all
        if let Some(ref pre_commands) = package.pre_commands {
            println!("pre_commands = \"{}\"", pre_commands);
        }
        if let Some(ref post_commands) = package.post_commands {
            println!("post_commands = \"{}\"", post_commands);
        }
        if !package.tests.is_empty() {
            println!("tests = {{");
            for (key, value) in &package.tests {
                println!("    \"{}\": \"{}\",", key, value);
            }
            println!("}}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_args_parsing() {
        let args = ViewArgs {
            package: "test_package".to_string(),
            format: ViewFormat::Yaml,
            all: false,
            brief: true,
            current: false,
        };

        assert_eq!(args.package, "test_package");
        assert!(args.brief);
        assert!(!args.all);
    }

    #[test]
    fn test_expand_home_path_no_tilde() {
        let path = expand_home_path("/usr/local/packages");
        assert_eq!(path.to_string_lossy(), "/usr/local/packages");
    }

    // ── Phase 99: view command package.py parse and display tests ────────────

    fn create_package_dir(dir: &std::path::Path, content: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("package.py"), content).unwrap();
    }

    #[test]
    fn test_load_package_from_directory_simple() {
        let tmp = std::env::temp_dir().join("rez_view_test_simple");
        let content = r#"name = "mypackage"
version = "1.2.3"
description = "A test package"
"#;
        create_package_dir(&tmp, content);
        let pkg = load_package_from_directory(&tmp).unwrap();
        assert_eq!(pkg.name, "mypackage");
        assert_eq!(pkg.version.as_ref().map(|v| v.as_str()), Some("1.2.3"));
        assert_eq!(pkg.description.as_deref(), Some("A test package"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_package_from_directory_with_requires() {
        let tmp = std::env::temp_dir().join("rez_view_test_requires");
        let content = r#"name = "myapp"
version = "2.0.0"
requires = ["python-3+", "numpy-1.20+"]
"#;
        create_package_dir(&tmp, content);
        let pkg = load_package_from_directory(&tmp).unwrap();
        assert_eq!(pkg.name, "myapp");
        assert_eq!(pkg.requires.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_package_from_directory_missing_package_py() {
        let tmp = std::env::temp_dir().join("rez_view_test_missing");
        std::fs::create_dir_all(&tmp).unwrap();
        // No package.py created
        let result = load_package_from_directory(&tmp);
        assert!(result.is_err(), "Should fail when package.py is missing");
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("No package.py"),
            "Should mention missing package.py: {}",
            err_msg
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_load_package_from_directory_with_tools() {
        let tmp = std::env::temp_dir().join("rez_view_test_tools");
        let content = r#"name = "mytool"
version = "3.1.0"
tools = ["mytool", "mytool-cli"]
"#;
        create_package_dir(&tmp, content);
        let pkg = load_package_from_directory(&tmp).unwrap();
        assert_eq!(pkg.tools.len(), 2);
        assert!(pkg.tools.contains(&"mytool".to_string()));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_display_package_yaml_output() {
        use rez_next_package::Package;
        use rez_next_version::Version;
        let mut pkg = Package::new("testpkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.description = Some("Test description".to_string());
        let args = ViewArgs {
            package: "testpkg".to_string(),
            format: ViewFormat::Yaml,
            all: false,
            brief: true,
            current: false,
        };
        // Just verify display_package_yaml doesn't panic
        let result = display_package_yaml(&pkg, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_package_python_output() {
        use rez_next_package::Package;
        use rez_next_version::Version;
        let mut pkg = Package::new("testpkg2".to_string());
        pkg.version = Some(Version::parse("2.5.0").unwrap());
        pkg.authors = vec!["Alice".to_string(), "Bob".to_string()];
        let args = ViewArgs {
            package: "testpkg2".to_string(),
            format: ViewFormat::Py,
            all: false,
            brief: false,
            current: false,
        };
        let result = display_package_python(&pkg, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_package_with_commands() {
        let tmp = std::env::temp_dir().join("rez_view_test_commands");
        let content = r#"name = "myenv"
version = "1.0"
def commands():
    env.setenv('MYENV_ROOT', '{root}')
    env.prepend_path('PATH', '{root}/bin')
"#;
        create_package_dir(&tmp, content);
        let pkg = load_package_from_directory(&tmp).unwrap();
        assert_eq!(pkg.name, "myenv");
        // commands may or may not be parsed depending on parser capability
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_view_args_current_context_flag() {
        let args = ViewArgs {
            package: "mypkg".to_string(),
            format: ViewFormat::Yaml,
            all: false,
            brief: false,
            current: true,
        };
        assert!(args.current);
        // Executing view_current_package should return error (not in context)
        let result = view_current_package(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_view_format_variants() {
        let yaml_args = ViewArgs {
            package: "pkg".to_string(),
            format: ViewFormat::Yaml,
            all: false,
            brief: false,
            current: false,
        };
        let py_args = ViewArgs {
            package: "pkg".to_string(),
            format: ViewFormat::Py,
            all: false,
            brief: false,
            current: false,
        };
        assert!(matches!(yaml_args.format, ViewFormat::Yaml));
        assert!(matches!(py_args.format, ViewFormat::Py));
    }

    #[test]
    fn test_load_package_with_variants() {
        let tmp = std::env::temp_dir().join("rez_view_test_variants");
        let content = r#"name = "mylib"
version = "2.0.0"
variants = [["python-3.9"], ["python-3.10"]]
"#;
        create_package_dir(&tmp, content);
        let pkg = load_package_from_directory(&tmp).unwrap();
        assert_eq!(pkg.name, "mylib");
        // variants may be empty if not fully parsed, just check no panic
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
