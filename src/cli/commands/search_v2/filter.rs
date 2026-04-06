//! # Search Filter and Sort
//!
//! Result filtering, deduplication, sorting, and the core search pipeline
//! for the `rez search` command.

use super::matcher::{evaluate_package_match, get_package_timestamp};
use super::types::{SearchArgs, SearchResult};
use crate::cli::utils::parse_timestamp;
use rez_next_common::error::RezCoreResult;
use rez_next_repository::simple_repository::RepositoryManager;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::sync::Arc;

    fn make_result(name: &str, version: Option<&str>, score: f64) -> SearchResult {
        let mut pkg = Package::new(name.to_string());
        pkg.version = version.map(|v| Version::parse(v).unwrap());
        SearchResult {
            package: Arc::new(pkg),
            repository: "test_repo".to_string(),
            match_score: score,
            match_fields: vec![],
        }
    }

    mod test_sort_results {
        use super::*;

        #[test]
        fn sort_by_name_ascending() {
            let mut results = vec![
                make_result("zlib", Some("1.0.0"), 1.0),
                make_result("abc", Some("1.0.0"), 1.0),
                make_result("maya", Some("2024.1.0"), 1.0),
            ];
            sort_results(&mut results, "name");
            let names: Vec<&str> = results.iter().map(|r| r.package.name.as_str()).collect();
            assert_eq!(names, vec!["abc", "maya", "zlib"]);
        }

        #[test]
        fn sort_by_version_descending() {
            let mut results = vec![
                make_result("pkg", Some("1.0.0"), 1.0),
                make_result("pkg", Some("3.0.0"), 1.0),
                make_result("pkg", Some("2.0.0"), 1.0),
            ];
            sort_results(&mut results, "version");
            let versions: Vec<&str> = results
                .iter()
                .map(|r| r.package.version.as_ref().unwrap().as_str())
                .collect();
            // Descending — 3.0.0 first
            assert_eq!(versions[0], "3.0.0");
            assert_eq!(versions[2], "1.0.0");
        }

        #[test]
        fn sort_by_version_none_last() {
            let mut results = vec![
                make_result("pkg", None, 1.0),
                make_result("pkg", Some("1.0.0"), 1.0),
            ];
            sort_results(&mut results, "version");
            // Some version should come before None
            assert!(results[0].package.version.is_some());
            assert!(results[1].package.version.is_none());
        }

        #[test]
        fn sort_by_score_descending() {
            let mut results = vec![
                make_result("a", Some("1.0.0"), 0.5),
                make_result("b", Some("1.0.0"), 0.9),
                make_result("c", Some("1.0.0"), 0.1),
            ];
            sort_results(&mut results, "score");
            let names: Vec<&str> = results.iter().map(|r| r.package.name.as_str()).collect();
            assert_eq!(names, vec!["b", "a", "c"]);
        }

        #[test]
        fn sort_unknown_key_falls_back_to_name() {
            let mut results = vec![
                make_result("zlib", Some("1.0.0"), 1.0),
                make_result("abc", Some("1.0.0"), 1.0),
            ];
            sort_results(&mut results, "unknown_key");
            assert_eq!(results[0].package.name, "abc");
            assert_eq!(results[1].package.name, "zlib");
        }

        #[test]
        fn sort_empty_slice_is_noop() {
            let mut results: Vec<SearchResult> = vec![];
            sort_results(&mut results, "name");
            assert!(results.is_empty());
        }
    }

    mod test_filter_latest_versions {
        use super::*;

        #[test]
        fn keeps_highest_version_per_package() {
            let results = vec![
                make_result("pkg", Some("1.0.0"), 1.0),
                make_result("pkg", Some("3.0.0"), 1.0),
                make_result("pkg", Some("2.0.0"), 1.0),
            ];
            let filtered = filter_latest_versions(results);
            assert_eq!(filtered.len(), 1);
            assert_eq!(
                filtered[0].package.version.as_ref().unwrap().as_str(),
                "3.0.0"
            );
        }

        #[test]
        fn different_packages_all_kept() {
            let results = vec![
                make_result("maya", Some("2024.1.0"), 1.0),
                make_result("houdini", Some("20.0.0"), 1.0),
                make_result("maya", Some("2025.0.0"), 1.0),
            ];
            let mut filtered = filter_latest_versions(results);
            filtered.sort_by(|a, b| a.package.name.cmp(&b.package.name));
            assert_eq!(filtered.len(), 2);
            assert_eq!(filtered[0].package.name, "houdini");
            assert_eq!(
                filtered[1].package.version.as_ref().unwrap().as_str(),
                "2025.0.0"
            );
        }

        #[test]
        fn versioned_beats_unversioned() {
            let results = vec![
                make_result("pkg", None, 1.0),
                make_result("pkg", Some("1.0.0"), 1.0),
            ];
            let filtered = filter_latest_versions(results);
            assert_eq!(filtered.len(), 1);
            assert!(filtered[0].package.version.is_some());
        }

        #[test]
        fn unversioned_kept_when_alone() {
            let results = vec![make_result("pkg", None, 1.0)];
            let filtered = filter_latest_versions(results);
            assert_eq!(filtered.len(), 1);
            assert!(filtered[0].package.version.is_none());
        }

        #[test]
        fn empty_input_returns_empty() {
            let filtered = filter_latest_versions(vec![]);
            assert!(filtered.is_empty());
        }

        #[test]
        fn single_package_returned_unchanged() {
            let results = vec![make_result("only", Some("5.0.0"), 0.8)];
            let filtered = filter_latest_versions(results);
            assert_eq!(filtered.len(), 1);
            assert_eq!(
                filtered[0].package.version.as_ref().unwrap().as_str(),
                "5.0.0"
            );
        }
    }
}
