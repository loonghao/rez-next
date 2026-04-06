//! Advanced search command implementation
//!
//! ## Module Layout
//!
//! - `types`   — `SearchArgs`, `SearchResult`
//! - `matcher` — package match scoring (`evaluate_package_match`, `get_package_timestamp`)
//! - `filter`  — result filtering, sorting, deduplication, and the search pipeline
//! - `display` — output formatting (table, JSON, detailed)

mod display;
mod filter;
mod matcher;
mod types;

pub use types::{SearchArgs, SearchResult};

use crate::cli::utils::expand_home_path;
use rez_next_common::{config::RezCoreConfig, error::RezCoreResult};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};

/// Execute the search command
pub fn execute(args: SearchArgs) -> RezCoreResult<()> {
    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { search_async(args).await })
}

/// Async search implementation
async fn search_async(args: SearchArgs) -> RezCoreResult<()> {
    // Determine if we're in machine-readable (JSON) mode
    let is_json_mode = args.format.eq_ignore_ascii_case("json");

    if !is_json_mode {
        println!("🔍 Searching for: '{}'", args.query);
    }

    // Set up repository manager
    let mut repo_manager = RepositoryManager::new();

    // Add repositories
    if args.repository.is_empty() {
        // Use default repositories from RezCoreConfig.
        add_default_repositories(&mut repo_manager).await?;
    } else {
        for (i, repo_path) in args.repository.iter().enumerate() {
            let expanded = expand_home_path(&repo_path.to_string_lossy());
            let repo = SimpleRepository::new(expanded, format!("repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    if !is_json_mode {
        println!(
            "📚 Searching {} repositories...",
            repo_manager.repository_count()
        );
    }

    // Perform search
    let results = filter::perform_search(&repo_manager, &args).await?;

    // Display results
    display::display_search_results(&results, &args)?;

    Ok(())
}

/// Add default repositories from rez config
async fn add_default_repositories(repo_manager: &mut RepositoryManager) -> RezCoreResult<()> {
    let config = RezCoreConfig::load();

    for (i, path_str) in config.packages_path.iter().enumerate() {
        let path = expand_home_path(path_str);
        if path.exists() {
            let repo = SimpleRepository::new(&path, format!("repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
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
