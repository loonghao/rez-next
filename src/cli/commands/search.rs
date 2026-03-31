//! Search command implementation
//!
//! Implements the `rez search` command for searching packages in repositories.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_repository::PackageSearchCriteria;
use std::collections::HashMap;
use serde_json;

/// Arguments for the search command
#[derive(Args, Clone, Debug)]
pub struct SearchArgs {
    /// Package to search for (supports glob patterns)
    #[arg(value_name = "PKG")]
    pub package: Option<String>,

    /// Type of resource to search for
    #[arg(short = 't', long = "type", value_name = "TYPE")]
    #[arg(value_parser = ["package", "family", "variant", "auto"])]
    #[arg(default_value = "auto")]
    pub resource_type: String,

    /// Don't search local packages
    #[arg(long = "nl", long = "no-local")]
    pub no_local: bool,

    /// Validate each resource that is found
    #[arg(long)]
    pub validate: bool,

    /// Set package search path (ignores --no-local if set)
    #[arg(long, value_name = "PATH")]
    pub paths: Option<String>,

    /// Set package repository path (alias for --paths)
    #[arg(long, value_name = "PATH")]
    pub repository: Option<String>,

    /// Format package output
    #[arg(short = 'f', long = "format", value_name = "FORMAT")]
    pub format: Option<String>,

    /// Print newlines as '\\n' rather than actual newlines
    #[arg(long = "no-newlines")]
    pub no_newlines: bool,

    /// When searching packages, only show the latest version of each package
    #[arg(short = 'l', long = "latest")]
    pub latest: bool,

    /// Only print packages containing errors (implies --validate)
    #[arg(short = 'e', long = "errors")]
    pub errors: bool,

    /// Suppress warnings
    #[arg(long = "nw", long = "no-warnings")]
    pub no_warnings: bool,

    /// Only show packages released before the given time
    #[arg(long = "before", value_name = "TIME", default_value = "0")]
    pub before: String,

    /// Only show packages released after the given time
    #[arg(long = "after", value_name = "TIME", default_value = "0")]
    pub after: String,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The package or resource found
    pub package: Package,
    /// Type of resource (package, family, variant)
    pub resource_type: String,
    /// Validation error if any
    pub validation_error: Option<String>,
}

/// Execute the search command
pub fn execute(args: SearchArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔍 Searching for packages...");
    }

    // Parse time constraints
    let before_time = parse_time_constraint(&args.before)?;
    let after_time = parse_time_constraint(&args.after)?;

    // Validate time constraints
    if let (Some(after), Some(before)) = (after_time, before_time) {
        if after >= before {
            return Err(RezCoreError::RequirementParse(
                "non-overlapping --before and --after".to_string(),
            ));
        }
    }

    // Create repository manager
    let mut repo_manager = RepositoryManager::new();

    // Add custom paths / repository if provided
    let effective_paths: Option<String> = args.paths.clone().or_else(|| args.repository.clone());
    if let Some(ref paths_str) = effective_paths {
        // Support both `:` (Unix) and `;` (Windows) as separators, or single path
        let separators: &[char] = &[':', ';'];
        for path_str in paths_str.split(separators) {
            let path_str = path_str.trim();
            if path_str.is_empty() {
                continue;
            }
            let repo = rez_next_repository::simple_repository::SimpleRepository::new(
                path_str,
                path_str.to_string(),
            );
            repo_manager.add_repository(Box::new(repo));
        }
    }

    // Execute search
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async {
        execute_search_async(&repo_manager, &args, before_time, after_time).await
    })
}

/// Execute search asynchronously
async fn execute_search_async(
    repo_manager: &RepositoryManager,
    args: &SearchArgs,
    before_time: Option<i64>,
    after_time: Option<i64>,
) -> RezCoreResult<()> {
    // Determine name to search for
    let search_name = args.package.as_deref().unwrap_or("*");

    if args.verbose {
        println!("Searching for: {}", search_name);
        println!();
    }

    // Search for packages - use "*" to list all, or specific name
    let packages: Vec<Package> = if search_name == "*" || search_name.is_empty() {
        // List all packages by scanning
        let names = repo_manager.list_packages().await
            .unwrap_or_default();
        let mut all_pkgs = Vec::new();
        for name in names {
            let pkgs = repo_manager.find_packages(&name).await.unwrap_or_default();
            for p in pkgs {
                all_pkgs.push((*p).clone());
            }
        }
        all_pkgs
    } else {
        let arcs = repo_manager.find_packages(search_name).await?;
        arcs.into_iter().map(|a| (*a).clone()).collect()
    };

    if packages.is_empty() {
        let resource_type = determine_resource_type(&args.resource_type, &args.package);
        // Return empty JSON array rather than exiting with error when format=json
        if args.format.as_deref().map(|f| f.eq_ignore_ascii_case("json")).unwrap_or(false) {
            println!("[]");
            return Ok(());
        }
        eprintln!("No matching {} found.", resource_type);
        std::process::exit(1);
    }

    // Filter packages based on additional criteria
    let filtered_packages = filter_packages(packages, args, before_time, after_time)?;

    if args.errors {
        println!("Error filtering not yet implemented");
        return Ok(());
    }

    // Format and display results
    display_search_results(&filtered_packages, args)?;

    if args.verbose {
        println!(
            "\n✅ Search completed. Found {} packages.",
            filtered_packages.len()
        );
    }

    Ok(())
}

/// Create search criteria from arguments
fn create_search_criteria(
    args: &SearchArgs,
    _before_time: Option<i64>,
    _after_time: Option<i64>,
) -> RezCoreResult<PackageSearchCriteria> {
    let mut criteria = PackageSearchCriteria::default();

    // Set name pattern
    if let Some(ref package) = args.package {
        criteria.name_pattern = Some(package.clone());
    }

    // Set limits
    if args.latest {
        criteria.limit = Some(1); // Only latest version per package
    }

    // Include prerelease versions by default (can be configured)
    criteria.include_prerelease = true;

    Ok(criteria)
}

/// Determine resource type from arguments
fn determine_resource_type(type_arg: &str, package: &Option<String>) -> String {
    match type_arg {
        "auto" => {
            // Auto-detect based on package pattern
            if let Some(ref pkg) = package {
                if pkg.contains('-') || pkg.contains('*') || pkg.contains('?') {
                    "packages".to_string()
                } else {
                    "package families".to_string()
                }
            } else {
                "packages".to_string()
            }
        }
        "family" => "package families".to_string(),
        "variant" => "package variants".to_string(),
        _ => "packages".to_string(),
    }
}

/// Filter packages based on additional criteria
fn filter_packages(
    packages: Vec<Package>,
    args: &SearchArgs,
    _before_time: Option<i64>,
    _after_time: Option<i64>,
) -> RezCoreResult<Vec<Package>> {
    let mut filtered = packages;

    // Apply latest filter
    if args.latest {
        filtered = apply_latest_filter(filtered);
    }

    // TODO: Apply time filters when package metadata includes timestamps
    // TODO: Apply validation filters when validation is implemented

    Ok(filtered)
}

/// Apply latest version filter
fn apply_latest_filter(packages: Vec<Package>) -> Vec<Package> {
    let mut latest_packages: HashMap<String, Package> = HashMap::new();

    for package in packages {
        let name = package.name.clone();

        match latest_packages.get(&name) {
            Some(existing) => {
                // Compare versions and keep the latest
                if let (Some(ref new_version), Some(ref existing_version)) =
                    (&package.version, &existing.version)
                {
                    if new_version.as_str() > existing_version.as_str() {
                        latest_packages.insert(name, package);
                    }
                } else if package.version.is_some() && existing.version.is_none() {
                    latest_packages.insert(name, package);
                }
            }
            None => {
                latest_packages.insert(name, package);
            }
        }
    }

    latest_packages.into_values().collect()
}

/// Display search results
fn display_search_results(packages: &[Package], args: &SearchArgs) -> RezCoreResult<()> {
    if let Some(ref format_str) = args.format {
        display_formatted_results(packages, format_str, args.no_newlines)?;
    } else {
        display_default_results(packages)?;
    }

    Ok(())
}

/// Display results with custom format
fn display_formatted_results(
    packages: &[Package],
    format_str: &str,
    no_newlines: bool,
) -> RezCoreResult<()> {
    // Special case: JSON output
    if format_str.eq_ignore_ascii_case("json") {
        let items: Vec<serde_json::Value> = packages
            .iter()
            .map(|p| {
                let mut obj = serde_json::json!({
                    "name": p.name,
                });
                if let Some(ref v) = p.version {
                    obj["version"] = serde_json::Value::String(v.as_str().to_string());
                    obj["qualified_name"] =
                        serde_json::Value::String(format!("{}-{}", p.name, v.as_str()));
                }
                if let Some(ref d) = p.description {
                    obj["description"] = serde_json::Value::String(d.clone());
                }
                obj
            })
            .collect();
        let json_out = serde_json::to_string_pretty(&items)
            .map_err(|e| RezCoreError::RequirementParse(format!("JSON serialization error: {e}")))?;
        println!("{}", json_out);
        return Ok(());
    }

    for package in packages {
        let formatted = format_package(package, format_str)?;
        if no_newlines {
            println!("{}", formatted.replace('\n', "\\n"));
        } else {
            println!("{}", formatted);
        }
    }
    Ok(())
}

/// Display results with default format
fn display_default_results(packages: &[Package]) -> RezCoreResult<()> {
    for package in packages {
        let version_str = package
            .version
            .as_ref()
            .map(|v| format!("-{}", v.as_str()))
            .unwrap_or_default();

        println!("{}{}", package.name, version_str);
    }
    Ok(())
}

/// Format a package according to format string
fn format_package(package: &Package, format_str: &str) -> RezCoreResult<String> {
    let mut result = format_str.to_string();

    // Replace format fields
    result = result.replace("{name}", &package.name);

    if let Some(ref version) = package.version {
        result = result.replace("{version}", version.as_str());
        result = result.replace(
            "{qualified_name}",
            &format!("{}-{}", package.name, version.as_str()),
        );
    } else {
        result = result.replace("{version}", "");
        result = result.replace("{qualified_name}", &package.name);
    }

    if let Some(ref description) = package.description {
        result = result.replace("{description}", description);
    } else {
        result = result.replace("{description}", "");
    }

    // TODO: Add more format fields as needed

    Ok(result)
}

/// Parse time constraint string
fn parse_time_constraint(time_str: &str) -> RezCoreResult<Option<i64>> {
    if time_str == "0" || time_str.is_empty() {
        return Ok(None);
    }

    // Try to parse as epoch time
    if let Ok(epoch) = time_str.parse::<i64>() {
        return Ok(Some(epoch));
    }

    // TODO: Parse relative time formats (-10s, -5m, -0.5h, -10d)
    // For now, just return None for unsupported formats
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_args_parsing() {
        let args = SearchArgs {
            package: Some("python".to_string()),
            resource_type: "auto".to_string(),
            no_local: false,
            validate: false,
            paths: None,
            format: None,
            no_newlines: false,
            latest: true,
            errors: false,
            no_warnings: false,
            before: "0".to_string(),
            after: "0".to_string(),
            verbose: false,
        };

        assert_eq!(args.package, Some("python".to_string()));
        assert_eq!(args.resource_type, "auto");
        assert!(args.latest);
    }

    #[test]
    fn test_determine_resource_type() {
        assert_eq!(determine_resource_type("package", &None), "packages");
        assert_eq!(determine_resource_type("family", &None), "package families");
        assert_eq!(
            determine_resource_type("variant", &None),
            "package variants"
        );

        assert_eq!(
            determine_resource_type("auto", &Some("python-3.9".to_string())),
            "packages"
        );
        assert_eq!(
            determine_resource_type("auto", &Some("python".to_string())),
            "package families"
        );
    }

    #[test]
    fn test_parse_time_constraint() {
        assert_eq!(parse_time_constraint("0").unwrap(), None);
        assert_eq!(parse_time_constraint("").unwrap(), None);
        assert_eq!(
            parse_time_constraint("1393014494").unwrap(),
            Some(1393014494)
        );
    }

    // ── Phase 109: additional search tests ──────────────────────────────────

    /// apply_latest_filter keeps only the highest version per package
    #[test]
    fn test_apply_latest_filter_keeps_newest() {
        use rez_next_version::Version;
        let mut p1 = Package::new("python".to_string());
        p1.version = Some(Version::parse("3.9").unwrap());
        let mut p2 = Package::new("python".to_string());
        p2.version = Some(Version::parse("3.11").unwrap());
        let mut p3 = Package::new("maya".to_string());
        p3.version = Some(Version::parse("2024").unwrap());

        let results = apply_latest_filter(vec![p1, p2, p3]);
        assert_eq!(results.len(), 2, "Should have 2 package families");
        let python_pkg = results.iter().find(|p| p.name == "python").unwrap();
        assert_eq!(python_pkg.version.as_ref().unwrap().as_str(), "3.11");
    }

    /// apply_latest_filter: single package passes through unchanged
    #[test]
    fn test_apply_latest_filter_single_pkg() {
        use rez_next_version::Version;
        let mut pkg = Package::new("houdini".to_string());
        pkg.version = Some(Version::parse("20.5").unwrap());
        let results = apply_latest_filter(vec![pkg]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "houdini");
    }

    /// apply_latest_filter: empty input returns empty
    #[test]
    fn test_apply_latest_filter_empty() {
        let results = apply_latest_filter(vec![]);
        assert!(results.is_empty());
    }

    /// format_package replaces {name}, {version}, {qualified_name}
    #[test]
    fn test_format_package_all_fields() {
        use rez_next_version::Version;
        let mut pkg = Package::new("requests".to_string());
        pkg.version = Some(Version::parse("2.28.0").unwrap());
        pkg.description = Some("HTTP library".to_string());

        let formatted = format_package(&pkg, "{name}-{version} ({description})").unwrap();
        assert!(formatted.contains("requests"), "Should have name");
        assert!(formatted.contains("2.28.0"), "Should have version");
        assert!(formatted.contains("HTTP library"), "Should have description");

        let qname = format_package(&pkg, "{qualified_name}").unwrap();
        assert!(qname.contains("requests-2.28.0"), "qualified_name should combine name+version");
    }

    /// format_package: no version falls back gracefully
    #[test]
    fn test_format_package_no_version() {
        let pkg = Package::new("base".to_string());
        let formatted = format_package(&pkg, "{name} {version}").unwrap();
        assert!(formatted.contains("base"), "Name should still appear");
        // version replaced by empty string
        assert!(!formatted.contains("{version}"), "Template placeholder should be replaced");
    }

    /// create_search_criteria with latest flag sets limit = 1
    #[test]
    fn test_create_search_criteria_latest_flag() {
        let args = SearchArgs {
            package: Some("python".to_string()),
            resource_type: "auto".to_string(),
            no_local: false,
            validate: false,
            paths: None,
            format: None,
            no_newlines: false,
            latest: true,
            errors: false,
            no_warnings: false,
            before: "0".to_string(),
            after: "0".to_string(),
            verbose: false,
        };
        let criteria = create_search_criteria(&args, None, None).unwrap();
        assert_eq!(criteria.limit, Some(1));
        assert_eq!(criteria.name_pattern, Some("python".to_string()));
    }

    /// resource_type auto with glob pattern → "packages"
    #[test]
    fn test_determine_resource_type_auto_glob() {
        assert_eq!(
            determine_resource_type("auto", &Some("python*".to_string())),
            "packages"
        );
        assert_eq!(
            determine_resource_type("auto", &Some("py?hon".to_string())),
            "packages"
        );
    }

    /// parse_time_constraint negative epoch value
    #[test]
    fn test_parse_time_constraint_negative() {
        // negative epoch (e.g., before 1970) should still parse
        let result = parse_time_constraint("-100").unwrap();
        assert_eq!(result, Some(-100));
    }

    /// SearchArgs no_newlines flag
    #[test]
    fn test_search_args_no_newlines_flag() {
        let args = SearchArgs {
            package: None,
            resource_type: "package".to_string(),
            no_local: true,
            validate: true,
            paths: Some("/packages".to_string()),
            format: Some("{name}".to_string()),
            no_newlines: true,
            latest: false,
            errors: false,
            no_warnings: true,
            before: "0".to_string(),
            after: "0".to_string(),
            verbose: false,
        };
        assert!(args.no_newlines);
        assert!(args.no_local);
        assert!(args.no_warnings);
        assert_eq!(args.format, Some("{name}".to_string()));
    }
}
