//! # Search Display
//!
//! Output formatting for the `rez search` command: table, JSON, and detailed views.

use super::types::{SearchArgs, SearchResult};
use rez_next_common::error::RezCoreResult;

/// Display search results in the requested format
pub fn display_search_results(results: &[SearchResult], args: &SearchArgs) -> RezCoreResult<()> {
    let is_json = args.format.eq_ignore_ascii_case("json");

    if results.is_empty() {
        if is_json {
            println!("[]");
        } else {
            println!("❌ No packages found matching '{}'", args.query);
        }
        return Ok(());
    }

    if !is_json {
        println!("✅ Found {} package(s):", results.len());
        println!();
    }

    match args.format.as_str() {
        f if f.eq_ignore_ascii_case("json") => display_json_format(results),
        "table" => display_table_format(results, args),
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
                "match_fields": result.match_fields,
                "timestamp": result.package.timestamp,
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
        if let Some(ts) = result.package.timestamp {
            if ts > 0 {
                if let Some(dt) = chrono::DateTime::from_timestamp(ts, 0) {
                    println!("Timestamp: {}", dt.format("%Y-%m-%d %H:%M:%S UTC"));
                }
            }
        }
    }

    Ok(())
}
