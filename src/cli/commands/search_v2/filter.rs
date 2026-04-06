//! # Search Filter and Sort
//!
//! Result filtering, deduplication, sorting, and the core search pipeline
//! for the `rez search` command.

use super::matcher::{evaluate_package_match, get_package_timestamp};
use super::types::{SearchArgs, SearchResult};
use crate::cli::utils::parse_timestamp;
use rez_next_repository::simple_repository::RepositoryManager;
use rez_next_common::error::RezCoreResult;
use std::collections::HashMap;

/// Perform the actual search against all repositories
pub async fn perform_search(
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
                            variant
                                .iter()
                                .any(|r| r.to_lowercase().contains(&req_lower))
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
                            variant
                                .iter()
                                .any(|r| r.to_lowercase().contains(&req_lower))
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

/// Sort search results
pub fn sort_results(results: &mut [SearchResult], sort_by: &str) {
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
pub fn filter_latest_versions(results: Vec<SearchResult>) -> Vec<SearchResult> {
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
