//! Search result types

use serde::{Deserialize, Serialize};

/// A single package search result entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    /// Package family name
    pub name: String,
    /// All matching version strings (sorted ascending)
    pub versions: Vec<String>,
    /// Repository path this result came from
    pub repo_path: String,
    /// Latest version (convenience field)
    pub latest: Option<String>,
}

impl SearchResult {
    pub fn new(name: String, versions: Vec<String>, repo_path: String) -> Self {
        let latest = versions.last().cloned();
        Self {
            name,
            versions,
            repo_path,
            latest,
        }
    }

    /// Number of versions available
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

/// Aggregated results from a search operation
#[derive(Debug, Clone, Default)]
pub struct SearchResultSet {
    /// Results keyed by package family name
    pub results: Vec<SearchResult>,
    /// Total packages scanned during search
    pub total_scanned: usize,
    /// Total repositories searched
    pub repos_searched: usize,
}

impl SearchResultSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, result: SearchResult) {
        self.results.push(result);
    }

    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Return names of all matched package families
    pub fn family_names(&self) -> Vec<&str> {
        self.results.iter().map(|r| r.name.as_str()).collect()
    }
}
