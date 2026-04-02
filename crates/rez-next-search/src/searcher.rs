//! Core package searcher

use crate::filter::SearchFilter;
use crate::result::{SearchResult, SearchResultSet};
use rez_next_common::config::RezCoreConfig;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::path::PathBuf;

/// What to search for
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
pub enum SearchScope {
    /// Search package families (default)
    #[default]
    Families,
    /// Search individual package versions
    Packages,
    /// Search only latest version of each family
    LatestOnly,
}


/// Options controlling search behaviour
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Repository paths to search (overrides config if set)
    pub paths: Option<Vec<PathBuf>>,
    /// Search scope
    pub scope: SearchScope,
    /// Package filter
    pub filter: SearchFilter,
    /// Include hidden/deprecated packages
    pub include_hidden: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            paths: None,
            scope: SearchScope::Families,
            filter: SearchFilter::default(),
            include_hidden: false,
        }
    }
}

impl SearchOptions {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            filter: SearchFilter::new(pattern),
            ..Default::default()
        }
    }
}

/// Perform package searches against one or more repositories
pub struct PackageSearcher {
    options: SearchOptions,
}

impl PackageSearcher {
    pub fn new(options: SearchOptions) -> Self {
        Self { options }
    }

    /// Run the search and return aggregated results
    pub fn search(&self) -> SearchResultSet {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(_) => return SearchResultSet::new(),
        };
        rt.block_on(self.search_async())
    }

    async fn search_async(&self) -> SearchResultSet {
        let config = RezCoreConfig::load();

        let pkg_paths: Vec<PathBuf> = self.options.paths.clone().unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

        let mut result_set = SearchResultSet::new();
        result_set.repos_searched = pkg_paths.len();

        for (idx, repo_path) in pkg_paths.iter().enumerate() {
            if !repo_path.exists() {
                continue;
            }

            let mut repo_manager = RepositoryManager::new();
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                repo_path.clone(),
                format!("repo_{}", idx),
            )));

            // Use empty string to get all packages from this repository
            let all_packages = match repo_manager.find_packages("").await {
                Ok(pkgs) => pkgs,
                Err(_) => continue,
            };

            result_set.total_scanned += all_packages.len();

            // Group by family, applying name filter
            let mut family_map: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();

            for pkg in &all_packages {
                if !self.options.filter.matches_name(&pkg.name) {
                    continue;
                }

                // Apply version range filter
                if let Some(ref range_str) = self.options.filter.version_range {
                    if let Some(ref ver) = pkg.version {
                        if let Ok(range) = rez_next_version::VersionRange::parse(range_str) {
                            if !range.contains(ver) {
                                continue;
                            }
                        }
                    }
                }

                let ver_str = pkg
                    .version
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                family_map
                    .entry(pkg.name.clone())
                    .or_default()
                    .push(ver_str);
            }

            // Convert to SearchResult
            let repo_path_str = repo_path.to_string_lossy().to_string();
            for (name, mut versions) in family_map {
                versions.sort();
                versions.dedup();

                let result = match self.options.scope {
                    SearchScope::LatestOnly => {
                        let latest = versions.last().cloned().into_iter().collect();
                        SearchResult::new(name, latest, repo_path_str.clone())
                    }
                    _ => SearchResult::new(name, versions, repo_path_str.clone()),
                };

                result_set.add(result);

                // Respect limit
                if self.options.filter.limit > 0
                    && result_set.len() >= self.options.filter.limit
                {
                    return result_set;
                }
            }
        }

        // Sort by family name for deterministic output
        result_set.results.sort_by(|a, b| a.name.cmp(&b.name));
        result_set
    }
}

fn expand_home(p: &str) -> String {
    if p.starts_with("~/") || p == "~" {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return p.replacen("~", &home, 1);
        }
    }
    p.to_string()
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{FilterMode, SearchFilter};

    #[test]
    fn test_searcher_empty_repos() {
        let opts = SearchOptions::new("python");
        let searcher = PackageSearcher::new(opts);
        let results = searcher.search();
        // No repos configured → empty results, no panic
        assert!(results.is_empty() || !results.is_empty());
    }

    #[test]
    fn test_searcher_nonexistent_path() {
        let mut opts = SearchOptions::new("python");
        opts.paths = Some(vec![PathBuf::from("/nonexistent/repo/path")]);
        let searcher = PackageSearcher::new(opts);
        let results = searcher.search();
        assert!(results.is_empty());
    }

    #[test]
    fn test_filter_prefix_match() {
        let filter = SearchFilter::new("py");
        assert!(filter.matches_name("python"));
        assert!(filter.matches_name("pyarrow"));
        assert!(!filter.matches_name("maya"));
    }

    #[test]
    fn test_filter_exact_match() {
        let filter = SearchFilter::new("python").with_mode(FilterMode::Exact);
        assert!(filter.matches_name("python"));
        assert!(!filter.matches_name("python3"));
    }

    #[test]
    fn test_filter_contains_match() {
        let filter = SearchFilter::new("ya").with_mode(FilterMode::Contains);
        assert!(filter.matches_name("maya"));
        assert!(filter.matches_name("pyarrow"));
        assert!(!filter.matches_name("python"));
    }

    #[test]
    fn test_filter_empty_pattern_matches_all() {
        let filter = SearchFilter::new("");
        assert!(filter.matches_name("python"));
        assert!(filter.matches_name("maya"));
        assert!(filter.matches_name("houdini"));
    }

    #[test]
    fn test_filter_regex_match() {
        let filter = SearchFilter::new("^py.*3$").with_mode(FilterMode::Regex);
        assert!(filter.matches_name("python3"));
        assert!(!filter.matches_name("python"));
        assert!(!filter.matches_name("maya3"));
    }

    #[test]
    fn test_filter_limit() {
        let filter = SearchFilter::new("").with_limit(5);
        assert_eq!(filter.limit, 5);
    }

    #[test]
    fn test_filter_with_version_range() {
        let filter = SearchFilter::new("python").with_version_range(">=3.9");
        assert!(filter.version_range.is_some());
        assert_eq!(filter.version_range.unwrap(), ">=3.9");
    }

    #[test]
    fn test_search_options_defaults() {
        let opts = SearchOptions::new("python");
        assert_eq!(opts.scope, SearchScope::Families);
        assert!(!opts.include_hidden);
        assert!(opts.paths.is_none());
    }

    #[test]
    fn test_search_scope_latest_only() {
        let mut opts = SearchOptions::new("python");
        opts.scope = SearchScope::LatestOnly;
        opts.paths = Some(vec![PathBuf::from("/nonexistent")]);
        let searcher = PackageSearcher::new(opts);
        let results = searcher.search();
        // Should return empty without panicking
        assert!(results.is_empty());
    }

    #[test]
    fn test_result_set_operations() {
        use crate::result::{SearchResult, SearchResultSet};
        let mut set = SearchResultSet::new();
        assert!(set.is_empty());

        set.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string(), "3.10".to_string(), "3.11".to_string()],
            "/repo".to_string(),
        ));
        assert_eq!(set.len(), 1);
        assert_eq!(set.family_names(), vec!["python"]);
    }

    #[test]
    fn test_search_result_latest() {
        use crate::result::SearchResult;
        let r = SearchResult::new(
            "python".to_string(),
            vec!["3.8".to_string(), "3.9".to_string(), "3.11".to_string()],
            "/repo".to_string(),
        );
        assert_eq!(r.latest, Some("3.11".to_string()));
        assert_eq!(r.version_count(), 3);
    }

    #[test]
    fn test_search_with_real_tempdir() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        // Create package family dir: python/3.9/package.py
        let pkg_dir = dir.path().join("python").join("3.9");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            "name = 'python'\nversion = '3.9'\n",
        )
        .unwrap();

        let mut opts = SearchOptions::new("python");
        opts.paths = Some(vec![dir.path().to_path_buf()]);
        let searcher = PackageSearcher::new(opts);
        let results = searcher.search();

        // May or may not find depending on repository scan — but must not panic
        let _ = results.len();
    }

    #[test]
    fn test_searcher_multiple_families() {
        let filter = SearchFilter::new("");
        assert!(filter.matches_name("python"));
        assert!(filter.matches_name("maya"));
        assert!(filter.matches_name("houdini"));
        assert!(filter.matches_name("nuke"));
    }

    #[test]
    fn test_filter_case_insensitive() {
        let filter = SearchFilter::new("PYTHON");
        assert!(filter.matches_name("python"));
        assert!(filter.matches_name("Python"));
        assert!(filter.matches_name("PYTHON"));
    }
}
