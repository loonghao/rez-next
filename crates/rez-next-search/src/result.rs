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

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SearchResult ─────────────────────────────────────────────────────────

    #[test]
    fn test_search_result_new_basic() {
        let r = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string(), "3.10".to_string(), "3.11".to_string()],
            "/repo/local".to_string(),
        );
        assert_eq!(r.name, "python");
        assert_eq!(r.versions.len(), 3);
        assert_eq!(r.repo_path, "/repo/local");
    }

    #[test]
    fn test_search_result_latest_is_last_version() {
        let r = SearchResult::new(
            "maya".to_string(),
            vec!["2022".to_string(), "2023".to_string(), "2024".to_string()],
            "/repo".to_string(),
        );
        assert_eq!(r.latest, Some("2024".to_string()));
    }

    #[test]
    fn test_search_result_latest_single_version() {
        let r = SearchResult::new(
            "cmake".to_string(),
            vec!["3.26.0".to_string()],
            "/repo".to_string(),
        );
        assert_eq!(r.latest, Some("3.26.0".to_string()));
        assert_eq!(r.version_count(), 1);
    }

    #[test]
    fn test_search_result_empty_versions_latest_none() {
        let r = SearchResult::new("emptypkg".to_string(), vec![], "/repo".to_string());
        assert_eq!(r.latest, None);
        assert_eq!(r.version_count(), 0);
    }

    #[test]
    fn test_search_result_version_count() {
        let r = SearchResult::new(
            "python".to_string(),
            vec![
                "3.7".to_string(),
                "3.8".to_string(),
                "3.9".to_string(),
                "3.10".to_string(),
                "3.11".to_string(),
            ],
            "/repo".to_string(),
        );
        assert_eq!(r.version_count(), 5);
    }

    #[test]
    fn test_search_result_clone_is_independent() {
        let r1 = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        let mut r2 = r1.clone();
        r2.name = "python_clone".to_string();
        assert_eq!(r1.name, "python");
        assert_eq!(r2.name, "python_clone");
    }

    #[test]
    fn test_search_result_equality() {
        let r1 = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        let r2 = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_search_result_inequality_different_name() {
        let r1 = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        let r2 = SearchResult::new(
            "maya".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_search_result_inequality_different_versions() {
        let r1 = SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        );
        let r2 = SearchResult::new(
            "python".to_string(),
            vec!["3.10".to_string()],
            "/repo".to_string(),
        );
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_search_result_serialization_roundtrip() {
        let r = SearchResult::new(
            "houdini".to_string(),
            vec!["19.5".to_string(), "20.0".to_string()],
            "/studio/packages".to_string(),
        );
        let json = serde_json::to_string(&r).unwrap();
        let restored: SearchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(r, restored);
        assert_eq!(restored.latest, Some("20.0".to_string()));
    }

    // ── SearchResultSet ───────────────────────────────────────────────────────

    #[test]
    fn test_search_result_set_new_is_empty() {
        let set = SearchResultSet::new();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.total_scanned, 0);
        assert_eq!(set.repos_searched, 0);
    }

    #[test]
    fn test_search_result_set_default_is_empty() {
        let set = SearchResultSet::default();
        assert!(set.is_empty());
    }

    #[test]
    fn test_search_result_set_add_single() {
        let mut set = SearchResultSet::new();
        set.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        ));
        assert!(!set.is_empty());
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_search_result_set_add_multiple() {
        let mut set = SearchResultSet::new();
        for name in &["python", "maya", "houdini", "nuke"] {
            set.add(SearchResult::new(
                name.to_string(),
                vec!["1.0".to_string()],
                "/repo".to_string(),
            ));
        }
        assert_eq!(set.len(), 4);
        assert!(!set.is_empty());
    }

    #[test]
    fn test_search_result_set_family_names_empty() {
        let set = SearchResultSet::new();
        let names = set.family_names();
        assert!(names.is_empty());
    }

    #[test]
    fn test_search_result_set_family_names_single() {
        let mut set = SearchResultSet::new();
        set.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        ));
        assert_eq!(set.family_names(), vec!["python"]);
    }

    #[test]
    fn test_search_result_set_family_names_multiple() {
        let mut set = SearchResultSet::new();
        set.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        ));
        set.add(SearchResult::new(
            "maya".to_string(),
            vec!["2023".to_string()],
            "/repo".to_string(),
        ));
        set.add(SearchResult::new(
            "nuke".to_string(),
            vec!["13".to_string()],
            "/repo".to_string(),
        ));
        let names = set.family_names();
        assert_eq!(names.len(), 3);
        assert!(names.contains(&"python"));
        assert!(names.contains(&"maya"));
        assert!(names.contains(&"nuke"));
    }

    #[test]
    fn test_search_result_set_clone_is_independent() {
        let mut set = SearchResultSet::new();
        set.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/repo".to_string(),
        ));
        let mut set2 = set.clone();
        set2.add(SearchResult::new(
            "maya".to_string(),
            vec!["2023".to_string()],
            "/repo".to_string(),
        ));
        assert_eq!(set.len(), 1);
        assert_eq!(set2.len(), 2);
    }

    #[test]
    fn test_search_result_set_repos_and_scanned_counters() {
        let mut set = SearchResultSet::new();
        set.repos_searched = 3;
        set.total_scanned = 150;
        assert_eq!(set.repos_searched, 3);
        assert_eq!(set.total_scanned, 150);
    }

    #[test]
    fn test_search_result_set_family_names_order_preserved() {
        let mut set = SearchResultSet::new();
        let names = ["zsh_tool", "a_tool", "m_tool"];
        for name in &names {
            set.add(SearchResult::new(
                name.to_string(),
                vec!["1.0".to_string()],
                "/repo".to_string(),
            ));
        }
        let family_names = set.family_names();
        // Order should match insertion order (family_names is Vec<&str> from results Vec)
        assert_eq!(family_names[0], "zsh_tool");
        assert_eq!(family_names[1], "a_tool");
        assert_eq!(family_names[2], "m_tool");
    }
}
