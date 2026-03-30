//! Suite manager — discovers and manages suites on disk

use crate::error::SuiteError;
use crate::suite::Suite;
use std::path::{Path, PathBuf};

/// Manages suites across multiple directories
pub struct SuiteManager {
    /// Directories to search for suites
    suite_paths: Vec<PathBuf>,
}

impl SuiteManager {
    /// Create a new suite manager
    pub fn new() -> Self {
        Self {
            suite_paths: Vec::new(),
        }
    }

    /// Create a suite manager with search paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        Self { suite_paths: paths }
    }

    /// Add a search path
    pub fn add_path(&mut self, path: impl Into<PathBuf>) {
        self.suite_paths.push(path.into());
    }

    /// Discover all suites in the search paths
    pub fn find_suites(&self) -> Vec<PathBuf> {
        let mut suites = Vec::new();

        for base_path in &self.suite_paths {
            if !base_path.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && Suite::is_suite(&path) {
                        suites.push(path);
                    }
                }
            }
        }

        suites
    }

    /// Load a suite by name from the search paths
    pub fn load_suite(&self, name: &str) -> Result<Suite, SuiteError> {
        for base_path in &self.suite_paths {
            let suite_path = base_path.join(name);
            if Suite::is_suite(&suite_path) {
                return Suite::load(&suite_path);
            }
        }
        Err(SuiteError::SuiteNotFound(name.to_string()))
    }

    /// Load a suite from a specific path
    pub fn load_suite_from_path(path: impl AsRef<Path>) -> Result<Suite, SuiteError> {
        Suite::load(path)
    }

    /// List all suite names in the search paths
    pub fn list_suite_names(&self) -> Vec<String> {
        self.find_suites()
            .into_iter()
            .filter_map(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .collect()
    }
}

impl Default for SuiteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suite_manager_empty() {
        let manager = SuiteManager::new();
        let suites = manager.find_suites();
        assert!(suites.is_empty());
    }

    #[test]
    fn test_suite_manager_find_suites() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("my_suite");

        let mut suite = Suite::new();
        suite
            .add_context("dev", vec!["python-3.9".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        let manager = SuiteManager::with_paths(vec![dir.path().to_path_buf()]);
        let found = manager.find_suites();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], suite_path);
    }

    #[test]
    fn test_suite_manager_load_by_name() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("test_suite");

        let mut suite = Suite::new().with_description("Test");
        suite
            .add_context("ctx", vec!["python".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();

        let manager = SuiteManager::with_paths(vec![dir.path().to_path_buf()]);
        let loaded = manager.load_suite("test_suite").unwrap();
        assert_eq!(loaded.description, Some("Test".to_string()));
    }

    #[test]
    fn test_suite_manager_load_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let manager = SuiteManager::with_paths(vec![dir.path().to_path_buf()]);
        let result = manager.load_suite("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_suite_manager_list_suite_names() {
        let dir = tempfile::tempdir().unwrap();

        // Create two suites
        for name in &["suite_alpha", "suite_beta"] {
            let suite_path = dir.path().join(name);
            let mut suite = Suite::new();
            suite.add_context("default", vec!["python".to_string()]).unwrap();
            suite.save(&suite_path).unwrap();
        }

        let manager = SuiteManager::with_paths(vec![dir.path().to_path_buf()]);
        let mut names = manager.list_suite_names();
        names.sort();
        assert!(names.contains(&"suite_alpha".to_string()));
        assert!(names.contains(&"suite_beta".to_string()));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_suite_manager_add_path() {
        let dir = tempfile::tempdir().unwrap();
        let mut manager = SuiteManager::new();
        manager.add_path(dir.path());
        let suites = manager.find_suites();
        assert!(suites.is_empty()); // dir is empty
    }

    #[test]
    fn test_suite_manager_load_from_path() {
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("direct_suite");

        let mut suite = Suite::new().with_description("Direct");
        suite.add_context("main", vec!["python-3.9".to_string()]).unwrap();
        suite.save(&suite_path).unwrap();

        let loaded = SuiteManager::load_suite_from_path(&suite_path).unwrap();
        assert_eq!(loaded.description, Some("Direct".to_string()));
    }

    #[test]
    fn test_suite_manager_with_nonexistent_path() {
        let manager = SuiteManager::with_paths(vec![std::path::PathBuf::from("/nonexistent/path")]);
        let suites = manager.find_suites();
        assert!(suites.is_empty()); // gracefully handles missing paths
    }
}
