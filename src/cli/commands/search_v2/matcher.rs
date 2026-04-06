//! # Search Matcher
//!
//! Package match scoring and timestamp utilities for the `rez search` command.

use super::types::{SearchArgs, SearchResult};
use rez_next_package::Package;
use std::sync::Arc;

/// Get the filesystem modification timestamp for a package (best effort)
pub fn get_package_timestamp(package: &Arc<Package>) -> i64 {
    // If the package has a timestamp field, use it
    if let Some(ts) = package.timestamp {
        return ts;
    }
    // Fall back to 0 (epoch) when unknown
    0
}

/// Evaluate if a package matches the search criteria
pub fn evaluate_package_match(
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
