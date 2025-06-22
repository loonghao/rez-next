//! Search command implementation
//!
//! Implements the `rez search` command for searching packages in repositories.

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_repository::PackageSearchCriteria;
use std::collections::HashMap;

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
    let repo_manager = RepositoryManager::new();

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
    // Create search criteria
    let criteria = create_search_criteria(args, before_time, after_time)?;

    if args.verbose {
        println!("Search criteria:");
        if let Some(ref pattern) = criteria.name_pattern {
            println!("  Name pattern: {}", pattern);
        }
        if let Some(ref version) = criteria.version_requirement {
            println!("  Version requirement: {}", version);
        }
        println!("  Include prerelease: {}", criteria.include_prerelease);
        if let Some(limit) = criteria.limit {
            println!("  Limit: {}", limit);
        }
        println!();
    }

    // Search for packages
    let packages = repo_manager.find_packages(&criteria).await?;

    if packages.is_empty() {
        let resource_type = determine_resource_type(&args.resource_type, &args.package);
        eprintln!("No matching {} found.", resource_type);
        std::process::exit(1);
    }

    // Filter packages based on additional criteria
    let filtered_packages = filter_packages(packages, args, before_time, after_time)?;

    if args.errors {
        // TODO: Implement validation and error filtering
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
}
