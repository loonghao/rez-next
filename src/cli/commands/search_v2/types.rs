//! # Search Command Types
//!
//! Argument definitions and result types for the `rez search` command.

use clap::Args;
use rez_next_package::Package;
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
