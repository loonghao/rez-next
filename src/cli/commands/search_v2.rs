//! Advanced search command implementation

use clap::Args;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult, RezCoreError};
use rez_next_package::Package;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Args, Clone)]
pub struct SearchArgs {
    /// Search query (package name, description, or pattern)
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Repository paths to search
    #[arg(short, long)]
    pub repository: Vec<PathBuf>,

    /// Search in package descriptions
    #[arg(long)]
    pub description: bool,

    /// Search in package tools
    #[arg(long)]
    pub tools: bool,

    /// Search in package requirements
    #[arg(long)]
    pub requirements: bool,

    /// Case-sensitive search
    #[arg(long)]
    pub case_sensitive: bool,

    /// Use regex pattern matching
    #[arg(long)]
    pub regex: bool,

    /// Show only latest versions
    #[arg(long)]
    pub latest_only: bool,

    /// Maximum number of results
    #[arg(long, default_value = "50")]
    pub limit: usize,

    /// Output format (table, json, detailed)
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Sort by (name, version, date)
    #[arg(long, default_value = "name")]
    pub sort: String,

    /// Show detailed package information
    #[arg(short, long)]
    pub verbose: bool,

    /// Only show packages newer than this ISO 8601 date (e.g. 2024-01-01 or 2024-01-01T00:00:00)
    #[arg(long, value_name = "DATE")]
    pub newer_than: Option<String>,

    /// Only show packages older than this ISO 8601 date (e.g. 2025-01-01)
    #[arg(long, value_name = "DATE")]
    pub older_than: Option<String>,

    /// Filter by type: package (default), family, or variant
    #[arg(long = "type", value_name = "TYPE", default_value = "package")]
    pub search_type: String,

    /// Filter packages that have a specific variant requirement (e.g. python-3.9)
    #[arg(long = "has-variant", value_name = "REQ")]
    pub has_variant: Option<String>,
}

/// Search result entry
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub package: Arc<Package>,
    pub repository: String,
    pub match_score: f64,
    pub match_fields: Vec<String>,
}

/// Execute the search command
pub fn execute(args: SearchArgs) -> RezCoreResult<()> {
    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { search_async(args).await })
}

/// Async search implementation
async fn search_async(args: SearchArgs) -> RezCoreResult<()> {
    println!("🔍 Searching for: '{}'", args.query);

    // Set up repository manager
    let mut repo_manager = RepositoryManager::new();

    // Add repositories
    if args.repository.is_empty() {
        // Use default test repositories
        add_default_repositories(&mut repo_manager).await?;
    } else {
        for (i, repo_path) in args.repository.iter().enumerate() {
            let repo = SimpleRepository::new(repo_path, format!("repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    println!(
        "📚 Searching {} repositories...",
        repo_manager.repository_count()
    );

    // Perform search
    let results = perform_search(&repo_manager, &args).await?;

    // Display results
    display_search_results(&results, &args)?;

    Ok(())
}

/// Add default repositories from rez config
async fn add_default_repositories(repo_manager: &mut RepositoryManager) -> RezCoreResult<()> {
    let config = RezCoreConfig::load();

    for (i, path_str) in config.packages_path.iter().enumerate() {
        let expanded = if path_str.starts_with("~/") || path_str == "~" {
            if let Ok(home) = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
            {
                path_str.replacen("~", &home, 1)
            } else {
                path_str.clone()
            }
        } else {
            path_str.clone()
        };

        let path = PathBuf::from(&expanded);
        if path.exists() {
            let repo = SimpleRepository::new(&path, format!("repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    Ok(())
}

/// Perform the actual search
async fn perform_search(
    repo_manager: &RepositoryManager,
    args: &SearchArgs,
) -> RezCoreResult<Vec<SearchResult>> {
    let mut all_results = Vec::new();

    // Parse timestamp filters
    let newer_than = args.newer_than.as_deref().and_then(parse_timestamp);
    let older_than = args.older_than.as_deref().and_then(parse_timestamp);

    // Get all available packages
    let all_package_names = repo_manager.list_packages().await?;

    for package_name in all_package_names {
        let packages = repo_manager.find_packages(&package_name).await?;

        for package in packages {
            // Apply timestamp filters if provided
            if newer_than.is_some() || older_than.is_some() {
                let pkg_timestamp = get_package_timestamp(&package);
                if let Some(newer) = newer_than {
                    if pkg_timestamp <= newer {
                        continue;
                    }
                }
                if let Some(older) = older_than {
                    if pkg_timestamp >= older {
                        continue;
                    }
                }
            }

            // Apply search_type filter
            match args.search_type.as_str() {
                "family" => {
                    // For family type: group by name, show once per family
                    // We'll just deduplicate at result level
                }
                "variant" => {
                    // For variant type: only include packages that have variants
                    if package.variants.is_empty() {
                        continue;
                    }
                    // Filter by has_variant if specified
                    if let Some(ref req) = args.has_variant {
                        let req_lower = req.to_lowercase();
                        let has_match = package.variants.iter().any(|variant| {
                            variant.iter().any(|r| r.to_lowercase().contains(&req_lower))
                        });
                        if !has_match {
                            continue;
                        }
                    }
                }
                _ => {
                    // "package" type (default): no extra filter
                    // Still apply has_variant if specified
                    if let Some(ref req) = args.has_variant {
                        let req_lower = req.to_lowercase();
                        let has_match = package.variants.iter().any(|variant| {
                            variant.iter().any(|r| r.to_lowercase().contains(&req_lower))
                        });
                        if !has_match {
                            continue;
                        }
                    }
                }
            }

            if let Some(result) = evaluate_package_match(&package, args, "default") {
                all_results.push(result);
            }
        }
    }

    // For family type: deduplicate by package name (keep highest score)
    if args.search_type == "family" {
        let mut family_map: HashMap<String, SearchResult> = HashMap::new();
        for result in all_results {
            let name = result.package.name.clone();
            if let Some(existing) = family_map.get(&name) {
                if result.match_score > existing.match_score {
                    family_map.insert(name, result);
                }
            } else {
                family_map.insert(name, result);
            }
        }
        all_results = family_map.into_values().collect();
    }

    // Sort results
    sort_results(&mut all_results, &args.sort);

    // Filter to latest only if requested
    if args.latest_only {
        all_results = filter_latest_versions(all_results);
    }

    // Limit results
    all_results.truncate(args.limit);

    Ok(all_results)
}

/// Parse an ISO 8601 date/datetime string to Unix timestamp (seconds)
fn parse_timestamp(s: &str) -> Option<i64> {
    // Try YYYY-MM-DDTHH:MM:SS format
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc().timestamp());
    }
    // Try YYYY-MM-DD format
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }
    None
}

/// Get the filesystem modification timestamp for a package (best effort)
fn get_package_timestamp(package: &Arc<Package>) -> i64 {
    // If the package has a timestamp field, use it
    if let Some(ts) = package.timestamp {
        return ts;
    }
    // Fall back to 0 (epoch) when unknown
    0
}

/// Evaluate if a package matches the search criteria
fn evaluate_package_match(
    package: &Arc<Package>,
    args: &SearchArgs,
    repo_name: &str,
) -> Option<SearchResult> {
    let mut match_score = 0.0;
    let mut match_fields = Vec::new();

    let query = if args.case_sensitive {
        args.query.clone()
    } else {
        args.query.to_lowercase()
    };

    // Check package name
    let package_name = if args.case_sensitive {
        package.name.clone()
    } else {
        package.name.to_lowercase()
    };

    if args.regex {
        if let Ok(regex) = regex::Regex::new(&query) {
            if regex.is_match(&package_name) {
                match_score += 10.0;
                match_fields.push("name".to_string());
            }
        }
    } else if package_name.contains(&query) {
        match_score += if package_name == query { 10.0 } else { 5.0 };
        match_fields.push("name".to_string());
    }

    // Check description if requested
    if args.description {
        if let Some(ref desc) = package.description {
            let desc_text = if args.case_sensitive {
                desc.clone()
            } else {
                desc.to_lowercase()
            };

            if args.regex {
                if let Ok(regex) = regex::Regex::new(&query) {
                    if regex.is_match(&desc_text) {
                        match_score += 3.0;
                        match_fields.push("description".to_string());
                    }
                }
            } else if desc_text.contains(&query) {
                match_score += 3.0;
                match_fields.push("description".to_string());
            }
        }
    }

    // Check tools if requested
    if args.tools {
        for tool in &package.tools {
            let tool_name = if args.case_sensitive {
                tool.clone()
            } else {
                tool.to_lowercase()
            };

            if args.regex {
                if let Ok(regex) = regex::Regex::new(&query) {
                    if regex.is_match(&tool_name) {
                        match_score += 2.0;
                        match_fields.push("tools".to_string());
                        break;
                    }
                }
            } else if tool_name.contains(&query) {
                match_score += 2.0;
                match_fields.push("tools".to_string());
                break;
            }
        }
    }

    // Check requirements if requested
    if args.requirements {
        for req in &package.requires {
            let req_text = if args.case_sensitive {
                req.clone()
            } else {
                req.to_lowercase()
            };

            if args.regex {
                if let Ok(regex) = regex::Regex::new(&query) {
                    if regex.is_match(&req_text) {
                        match_score += 1.0;
                        match_fields.push("requirements".to_string());
                        break;
                    }
                }
            } else if req_text.contains(&query) {
                match_score += 1.0;
                match_fields.push("requirements".to_string());
                break;
            }
        }
    }

    // If no specific fields requested, search all by default
    if !args.description && !args.tools && !args.requirements {
        // Already checked name above, check description and tools by default
        if let Some(ref desc) = package.description {
            let desc_text = if args.case_sensitive {
                desc.clone()
            } else {
                desc.to_lowercase()
            };

            if desc_text.contains(&query) {
                match_score += 2.0;
                match_fields.push("description".to_string());
            }
        }

        for tool in &package.tools {
            let tool_name = if args.case_sensitive {
                tool.clone()
            } else {
                tool.to_lowercase()
            };

            if tool_name.contains(&query) {
                match_score += 1.0;
                match_fields.push("tools".to_string());
                break;
            }
        }
    }

    if match_score > 0.0 {
        Some(SearchResult {
            package: package.clone(),
            repository: repo_name.to_string(),
            match_score,
            match_fields,
        })
    } else {
        None
    }
}

/// Sort search results
fn sort_results(results: &mut Vec<SearchResult>, sort_by: &str) {
    match sort_by {
        "name" => {
            results.sort_by(|a, b| a.package.name.cmp(&b.package.name));
        }
        "version" => {
            results.sort_by(|a, b| {
                match (&a.package.version, &b.package.version) {
                    (Some(v1), Some(v2)) => v2.cmp(v1), // Descending order (latest first)
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        "score" => {
            results.sort_by(|a, b| {
                b.match_score
                    .partial_cmp(&a.match_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        _ => {
            // Default to name sorting
            results.sort_by(|a, b| a.package.name.cmp(&b.package.name));
        }
    }
}

/// Filter to keep only latest versions of each package
fn filter_latest_versions(results: Vec<SearchResult>) -> Vec<SearchResult> {
    let mut latest_map: HashMap<String, SearchResult> = HashMap::new();

    for result in results {
        let package_name = &result.package.name;

        if let Some(existing) = latest_map.get(package_name) {
            // Compare versions
            match (&result.package.version, &existing.package.version) {
                (Some(new_ver), Some(existing_ver)) => {
                    if new_ver > existing_ver {
                        latest_map.insert(package_name.clone(), result);
                    }
                }
                (Some(_), None) => {
                    latest_map.insert(package_name.clone(), result);
                }
                _ => {
                    // Keep existing
                }
            }
        } else {
            latest_map.insert(package_name.clone(), result);
        }
    }

    latest_map.into_values().collect()
}

/// Display search results
fn display_search_results(results: &[SearchResult], args: &SearchArgs) -> RezCoreResult<()> {
    if results.is_empty() {
        println!("❌ No packages found matching '{}'", args.query);
        return Ok(());
    }

    println!("✅ Found {} package(s):", results.len());
    println!();

    match args.format.as_str() {
        "table" => display_table_format(results, args),
        "json" => display_json_format(results),
        "detailed" => display_detailed_format(results, args),
        _ => {
            eprintln!(
                "Unknown format: {}. Available formats: table, json, detailed",
                args.format
            );
            Ok(())
        }
    }
}

/// Display results in table format
fn display_table_format(results: &[SearchResult], args: &SearchArgs) -> RezCoreResult<()> {
    // Print header
    if args.verbose {
        println!(
            "{:<20} {:<10} {:<15} {:<8} {:<20}",
            "NAME", "VERSION", "REPOSITORY", "SCORE", "MATCHES"
        );
        println!("{}", "-".repeat(80));
    } else {
        println!("{:<20} {:<10} {:<40}", "NAME", "VERSION", "DESCRIPTION");
        println!("{}", "-".repeat(70));
    }

    // Print results
    for result in results {
        let version_str = result
            .package
            .version
            .as_ref()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "unknown".to_string());

        if args.verbose {
            println!(
                "{:<20} {:<10} {:<15} {:<8.1} {:<20}",
                result.package.name,
                version_str,
                result.repository,
                result.match_score,
                result.match_fields.join(", ")
            );
        } else {
            let description = result
                .package
                .description
                .as_ref()
                .map(|d| {
                    if d.len() > 37 {
                        format!("{}...", &d[..37])
                    } else {
                        d.clone()
                    }
                })
                .unwrap_or_else(|| "No description".to_string());

            println!(
                "{:<20} {:<10} {:<40}",
                result.package.name, version_str, description
            );
        }
    }

    Ok(())
}

/// Display results in JSON format
fn display_json_format(results: &[SearchResult]) -> RezCoreResult<()> {
    let json_results: Vec<serde_json::Value> = results
        .iter()
        .map(|result| {
            serde_json::json!({
                "name": result.package.name,
                "version": result.package.version.as_ref().map(|v| format!("{:?}", v)),
                "description": result.package.description,
                "tools": result.package.tools,
                "requires": result.package.requires,
                "repository": result.repository,
                "match_score": result.match_score,
                "match_fields": result.match_fields
            })
        })
        .collect();

    let json = serde_json::to_string_pretty(&json_results)?;
    println!("{}", json);

    Ok(())
}

/// Display results in detailed format
fn display_detailed_format(results: &[SearchResult], _args: &SearchArgs) -> RezCoreResult<()> {
    for (i, result) in results.iter().enumerate() {
        if i > 0 {
            println!();
        }

        println!("--- {} ---", result.package.name);

        if let Some(ref version) = result.package.version {
            println!("Version: {:?}", version);
        }

        if let Some(ref desc) = result.package.description {
            println!("Description: {}", desc);
        }

        if !result.package.tools.is_empty() {
            println!("Tools: {}", result.package.tools.join(", "));
        }

        if !result.package.requires.is_empty() {
            println!("Requires: {}", result.package.requires.join(", "));
        }

        println!("Repository: {}", result.repository);
        println!("Match Score: {:.1}", result.match_score);
        println!("Matched Fields: {}", result.match_fields.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_args_parsing() {
        // Test basic search args
        let args = SearchArgs {
            query: "python".to_string(),
            repository: vec![],
            description: false,
            tools: false,
            requirements: false,
            case_sensitive: false,
            regex: false,
            latest_only: false,
            limit: 50,
            format: "table".to_string(),
            sort: "name".to_string(),
            verbose: false,
            newer_than: None,
            older_than: None,
            search_type: "package".to_string(),
            has_variant: None,
        };

        assert_eq!(args.query, "python");
        assert_eq!(args.limit, 50);
    }

    #[test]
    fn test_search_type_variant_field() {
        let args = SearchArgs {
            query: "".to_string(),
            repository: vec![],
            description: false,
            tools: false,
            requirements: false,
            case_sensitive: false,
            regex: false,
            latest_only: false,
            limit: 10,
            format: "table".to_string(),
            sort: "name".to_string(),
            verbose: false,
            newer_than: None,
            older_than: None,
            search_type: "variant".to_string(),
            has_variant: Some("python-3.9".to_string()),
        };
        assert_eq!(args.search_type, "variant");
        assert_eq!(args.has_variant.as_deref(), Some("python-3.9"));
    }

    #[test]
    fn test_search_type_family_field() {
        let args = SearchArgs {
            query: "py".to_string(),
            repository: vec![],
            description: false,
            tools: false,
            requirements: false,
            case_sensitive: false,
            regex: false,
            latest_only: false,
            limit: 10,
            format: "table".to_string(),
            sort: "name".to_string(),
            verbose: false,
            newer_than: None,
            older_than: None,
            search_type: "family".to_string(),
            has_variant: None,
        };
        assert_eq!(args.search_type, "family");
    }
}
