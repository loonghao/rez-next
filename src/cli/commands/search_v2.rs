//! Advanced search command implementation

use clap::Args;
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_package::Package;
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
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

/// Add default test repositories
async fn add_default_repositories(repo_manager: &mut RepositoryManager) -> RezCoreResult<()> {
    let test_repos = vec![
        "C:/temp/test-packages",
        "C:/temp/simple_test",
        "C:/temp/test-build-command",
        "C:/temp/test-commands",
        "C:/temp/test-variants",
        "C:/temp/test-complete",
        "C:/temp/perf-test",
    ];

    for (i, repo_path) in test_repos.iter().enumerate() {
        let path = PathBuf::from(repo_path);
        if path.exists() {
            let repo = SimpleRepository::new(&path, format!("test_repo_{}", i));
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

    // Get all available packages
    let all_package_names = repo_manager.list_packages().await?;

    for package_name in all_package_names {
        let packages = repo_manager.find_packages(&package_name).await?;

        for package in packages {
            if let Some(result) = evaluate_package_match(&package, args, "default") {
                all_results.push(result);
            }
        }
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
        };

        assert_eq!(args.query, "python");
        assert_eq!(args.limit, 50);
    }
}
