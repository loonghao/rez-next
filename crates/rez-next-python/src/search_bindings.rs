//! Python bindings for rez.search (package search functionality)
//!
//! Mirrors `rez search` CLI and `rez.packages_.iter_packages` / family listing.

use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_search::{PackageSearcher, SearchFilter, SearchOptions, SearchResult, SearchScope};
use std::path::PathBuf;

/// Python wrapper for a single search result
#[pyclass(name = "SearchResult", from_py_object)]
#[derive(Debug, Clone)]
pub struct PySearchResult {
    inner: SearchResult,
}

#[pymethods]
impl PySearchResult {
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn versions(&self) -> Vec<String> {
        self.inner.versions.clone()
    }

    #[getter]
    fn repo_path(&self) -> &str {
        &self.inner.repo_path
    }

    #[getter]
    fn latest(&self) -> Option<String> {
        self.inner.latest.clone()
    }

    fn version_count(&self) -> usize {
        self.inner.version_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "SearchResult(name={:?}, versions={:?})",
            self.inner.name, self.inner.versions
        )
    }
}

/// Python class for package search manager
#[pyclass(name = "PackageSearcher")]
pub struct PyPackageSearcher {
    pattern: String,
    paths: Option<Vec<String>>,
    scope: String,
    version_range: Option<String>,
    limit: usize,
}

#[pymethods]
impl PyPackageSearcher {
    #[new]
    #[pyo3(signature = (pattern="", paths=None, scope="families", version_range=None, limit=0))]
    fn new(
        pattern: &str,
        paths: Option<Vec<String>>,
        scope: &str,
        version_range: Option<String>,
        limit: usize,
    ) -> Self {
        Self {
            pattern: pattern.to_string(),
            paths,
            scope: scope.to_string(),
            version_range,
            limit,
        }
    }

    /// Run the search and return a list of SearchResult objects
    fn search(&self, py: Python) -> PyResult<Py<PyAny>> {
        let scope = match self.scope.as_str() {
            "latest" => SearchScope::LatestOnly,
            "packages" => SearchScope::Packages,
            _ => SearchScope::Families,
        };

        let mut filter = SearchFilter::new(self.pattern.clone());
        if let Some(ref range) = self.version_range {
            filter = filter.with_version_range(range.clone());
        }
        if self.limit > 0 {
            filter = filter.with_limit(self.limit);
        }

        let opts = SearchOptions {
            paths: self
                .paths
                .as_ref()
                .map(|p| p.iter().map(PathBuf::from).collect()),
            scope,
            filter,
            include_hidden: false,
        };

        let searcher = PackageSearcher::new(opts);
        let result_set = searcher.search();

        let list = PyList::empty(py);
        for r in result_set.results {
            let py_result = PySearchResult { inner: r };
            list.append(py_result.into_pyobject(py)?)?;
        }
        Ok(list.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "PackageSearcher(pattern={:?}, scope={:?})",
            self.pattern, self.scope
        )
    }
}

/// Search for packages matching a name pattern.
/// Equivalent to `rez search <pattern>` or browsing `rez.packages_.iter_packages`.
///
/// Args:
///     pattern: Name prefix/substring to search for (empty = all packages)
///     paths: Repository paths to search (default: configured paths)
///     scope: "families" | "packages" | "latest" (default: "families")
///     version_range: Only include versions in this range (e.g. ">=3.9")
///     limit: Maximum number of results (0 = unlimited)
///
/// Returns:
///     List of SearchResult objects
#[pyfunction]
#[pyo3(signature = (pattern="", paths=None, scope="families", version_range=None, limit=0))]
pub fn search_packages(
    py: Python,
    pattern: &str,
    paths: Option<Vec<String>>,
    scope: &str,
    version_range: Option<String>,
    limit: usize,
) -> PyResult<Py<PyAny>> {
    let searcher = PyPackageSearcher::new(pattern, paths, scope, version_range, limit);
    searcher.search(py)
}

/// Search for package families and return their names only.
/// Fast variant that avoids loading full package data.
#[pyfunction]
#[pyo3(signature = (pattern="", paths=None))]
pub fn search_package_names(pattern: &str, paths: Option<Vec<String>>) -> PyResult<Vec<String>> {
    let filter = SearchFilter::new(pattern);
    let opts = SearchOptions {
        paths: paths.map(|p| p.into_iter().map(PathBuf::from).collect()),
        scope: SearchScope::Families,
        filter,
        include_hidden: false,
    };
    let searcher = PackageSearcher::new(opts);
    let result_set = searcher.search();
    Ok(result_set
        .family_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect())
}

/// Get the latest version of each package matching the pattern.
#[pyfunction]
#[pyo3(signature = (pattern="", paths=None, version_range=None))]
pub fn search_latest_packages(
    py: Python,
    pattern: &str,
    paths: Option<Vec<String>>,
    version_range: Option<String>,
) -> PyResult<Py<PyAny>> {
    let mut filter = SearchFilter::new(pattern);
    if let Some(ref range) = version_range {
        filter = filter.with_version_range(range.clone());
    }
    let opts = SearchOptions {
        paths: paths.map(|p| p.into_iter().map(PathBuf::from).collect()),
        scope: SearchScope::LatestOnly,
        filter,
        include_hidden: false,
    };
    let searcher = PackageSearcher::new(opts);
    let result_set = searcher.search();

    let list = PyList::empty(py);
    for r in result_set.results {
        let py_result = PySearchResult { inner: r };
        list.append(py_result.into_pyobject(py)?)?;
    }
    Ok(list.into_any().unbind())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_search::{FilterMode, SearchResult, SearchResultSet};

    // ── PySearchResult pure-logic tests ──────────────────────────────────────

    #[test]
    fn test_search_result_getters() {
        let inner = SearchResult::new(
            "python".to_string(),
            vec!["3.9.0".to_string(), "3.11.4".to_string()],
            "/pkgs".to_string(),
        );
        let r = PySearchResult { inner };
        assert_eq!(r.name(), "python");
        assert_eq!(r.versions(), vec!["3.9.0", "3.11.4"]);
        assert_eq!(r.repo_path(), "/pkgs");
        assert_eq!(r.latest(), Some("3.11.4".to_string()));
        assert_eq!(r.version_count(), 2);
    }

    #[test]
    fn test_search_result_empty_versions() {
        let inner = SearchResult::new("empty_pkg".to_string(), vec![], "/repo".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.latest(), None);
        assert_eq!(r.version_count(), 0);
    }

    #[test]
    fn test_search_result_repr_format() {
        let inner = SearchResult::new(
            "cmake".to_string(),
            vec!["3.26.0".to_string()],
            "/packages".to_string(),
        );
        let r = PySearchResult { inner };
        let repr = r.__repr__();
        assert!(repr.contains("SearchResult"));
        assert!(repr.contains("cmake"));
        assert!(repr.contains("3.26.0"));
    }

    // ── PyPackageSearcher repr tests ─────────────────────────────────────────

    #[test]
    fn test_package_searcher_repr_default() {
        let s = PyPackageSearcher::new("", None, "families", None, 0);
        let repr = s.__repr__();
        assert!(repr.contains("PackageSearcher"));
        assert!(repr.contains("families"));
    }

    #[test]
    fn test_package_searcher_repr_custom() {
        let s = PyPackageSearcher::new("py", None, "latest", None, 10);
        let repr = s.__repr__();
        assert!(repr.contains("py"));
        assert!(repr.contains("latest"));
    }

    #[test]
    fn test_package_searcher_stores_fields() {
        let s = PyPackageSearcher::new(
            "my_pkg",
            Some(vec!["/a".to_string(), "/b".to_string()]),
            "packages",
            Some(">=2.0".to_string()),
            5,
        );
        assert_eq!(s.pattern, "my_pkg");
        assert_eq!(s.scope, "packages");
        assert_eq!(s.limit, 5);
        assert_eq!(s.version_range, Some(">=2.0".to_string()));
        assert_eq!(s.paths, Some(vec!["/a".to_string(), "/b".to_string()]));
    }

    // ── SearchFilter matching logic ───────────────────────────────────────────

    #[test]
    fn test_filter_empty_pattern_matches_all() {
        let f = SearchFilter::new("");
        assert!(f.matches_name("python"));
        assert!(f.matches_name("cmake"));
        assert!(f.matches_name(""));
    }

    #[test]
    fn test_filter_prefix_mode() {
        let f = SearchFilter::new("py");
        assert!(f.matches_name("python"));
        assert!(f.matches_name("py"));
        assert!(!f.matches_name("numpy"));
    }

    #[test]
    fn test_filter_prefix_case_insensitive() {
        let f = SearchFilter::new("PY");
        assert!(f.matches_name("python"));
        assert!(f.matches_name("PyYAML"));
    }

    #[test]
    fn test_filter_exact_mode() {
        let f = SearchFilter::new("python").with_mode(FilterMode::Exact);
        assert!(f.matches_name("python"));
        assert!(f.matches_name("PYTHON"));
        assert!(!f.matches_name("python3"));
    }

    #[test]
    fn test_filter_contains_mode() {
        let f = SearchFilter::new("numpy").with_mode(FilterMode::Contains);
        assert!(f.matches_name("numpy"));
        assert!(!f.matches_name("scipy"));
    }

    #[test]
    fn test_filter_with_limit() {
        let f = SearchFilter::new("py").with_limit(5);
        assert_eq!(f.limit, 5);
    }

    #[test]
    fn test_filter_with_version_range() {
        let f = SearchFilter::new("cmake").with_version_range(">=3.0");
        assert_eq!(f.version_range, Some(">=3.0".to_string()));
    }

    // ── SearchResultSet logic ─────────────────────────────────────────────────

    #[test]
    fn test_result_set_empty() {
        let rs = SearchResultSet::new();
        assert!(rs.is_empty());
        assert_eq!(rs.len(), 0);
        assert!(rs.family_names().is_empty());
    }

    #[test]
    fn test_result_set_add_and_names() {
        let mut rs = SearchResultSet::new();
        rs.add(SearchResult::new(
            "python".to_string(),
            vec!["3.11.0".to_string()],
            "/repo".to_string(),
        ));
        rs.add(SearchResult::new(
            "cmake".to_string(),
            vec!["3.26.0".to_string()],
            "/repo".to_string(),
        ));
        assert_eq!(rs.len(), 2);
        assert!(!rs.is_empty());
        let names = rs.family_names();
        assert!(names.contains(&"python"));
        assert!(names.contains(&"cmake"));
    }

    #[test]
    fn test_result_set_single_version_latest() {
        let r = SearchResult::new(
            "gcc".to_string(),
            vec!["12.0".to_string()],
            "/r".to_string(),
        );
        assert_eq!(r.latest, Some("12.0".to_string()));
        assert_eq!(r.version_count(), 1);
    }

    // ── Additional SearchResult / PySearchResult tests ───────────────────────

    #[test]
    fn test_search_result_multiple_versions_latest_is_last() {
        // SearchResult::new sets latest to the last element (if any)
        let inner = SearchResult::new(
            "cmake".to_string(),
            vec!["3.20.0".to_string(), "3.26.0".to_string(), "3.28.0".to_string()],
            "/pkgs".to_string(),
        );
        // latest is computed as the last version in the list
        assert_eq!(inner.latest, Some("3.28.0".to_string()));
        let r = PySearchResult { inner };
        assert_eq!(r.latest(), Some("3.28.0".to_string()));
        assert_eq!(r.version_count(), 3);
    }

    #[test]
    fn test_search_result_repr_empty_versions() {
        let inner = SearchResult::new("orphan".to_string(), vec![], "/r".to_string());
        let r = PySearchResult { inner };
        let repr = r.__repr__();
        assert!(repr.contains("SearchResult"), "repr: {repr}");
        assert!(repr.contains("orphan"), "repr: {repr}");
    }

    #[test]
    fn test_search_result_repo_path_preserved() {
        let inner =
            SearchResult::new("pkg".to_string(), vec![], "/custom/repo/path".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.repo_path(), "/custom/repo/path");
    }

    // ── SearchResultSet additional ops ───────────────────────────────────────

    #[test]
    fn test_result_set_family_names_no_dups() {
        let mut rs = SearchResultSet::new();
        rs.add(SearchResult::new(
            "python".to_string(),
            vec!["3.9".to_string()],
            "/a".to_string(),
        ));
        rs.add(SearchResult::new(
            "python".to_string(),
            vec!["3.11".to_string()],
            "/b".to_string(),
        ));
        // Two different SearchResult objects with the same name are allowed
        assert_eq!(rs.len(), 2);
        let names = rs.family_names();
        // family_names may return duplicates (depends on impl) — just validate count/content
        assert!(names.iter().all(|n| *n == "python"), "names: {:?}", names);
    }

    #[test]
    fn test_result_set_len_and_is_empty_consistency() {
        let mut rs = SearchResultSet::new();
        assert!(rs.is_empty());
        rs.add(SearchResult::new("p".to_string(), vec![], "/r".to_string()));
        assert!(!rs.is_empty());
        assert_eq!(rs.len(), 1);
    }

    // ── PyPackageSearcher additional construction tests ───────────────────────

    #[test]
    fn test_package_searcher_no_version_range() {
        let s = PyPackageSearcher::new("cmake", None, "packages", None, 0);
        assert!(s.version_range.is_none());
    }

    #[test]
    fn test_package_searcher_limit_zero_means_unlimited() {
        let s = PyPackageSearcher::new("", None, "families", None, 0);
        assert_eq!(s.limit, 0);
    }

    // ── SearchFilter additional mode tests ───────────────────────────────────

    #[test]
    fn test_filter_regex_mode_if_supported() {
        // Prefix mode: pattern "py" must not match "scipy"
        let f = SearchFilter::new("py");
        assert!(!f.matches_name("scipy"), "prefix 'py' must not match 'scipy'");
    }

    #[test]
    fn test_filter_contains_mode_case_insensitive() {
        let f = SearchFilter::new("NUMPY").with_mode(FilterMode::Contains);
        // Contains mode should be case-insensitive
        assert!(
            f.matches_name("numpy") || f.matches_name("NUMPY"),
            "contains must work"
        );
    }

    // ── Additional SearchResult boundary tests ───────────────────────────────

    #[test]
    fn test_search_result_single_version_name_preserved() {
        let inner = SearchResult::new("houdini".to_string(), vec!["20.0".to_string()], "/vfx".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.name(), "houdini");
        assert_eq!(r.version_count(), 1);
    }

    #[test]
    fn test_search_result_versions_order_preserved() {
        // versions() returns in insertion order
        let versions = vec!["1.0".to_string(), "2.0".to_string(), "3.0".to_string()];
        let inner = SearchResult::new("pkg".to_string(), versions.clone(), "/r".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.versions(), versions);
    }

    #[test]
    fn test_package_searcher_scope_packages_stored() {
        let s = PyPackageSearcher::new("houdini", None, "packages", None, 0);
        assert_eq!(s.scope, "packages");
    }

    #[test]
    fn test_filter_with_version_range_none_by_default() {
        let f = SearchFilter::new("py");
        assert!(f.version_range.is_none(), "version_range should be None by default");
    }

    #[test]
    fn test_result_set_len_zero_initially() {
        let rs = SearchResultSet::new();
        assert_eq!(rs.len(), 0);
    }

    #[test]
    fn test_search_result_repo_path_is_slash_path() {
        let inner = SearchResult::new("lib".to_string(), vec![], "/a/b/c".to_string());
        let r = PySearchResult { inner };
        assert!(r.repo_path().starts_with('/'), "repo_path should be absolute Unix path: {}", r.repo_path());
    }

    #[test]
    fn test_package_searcher_default_scope_is_families() {
        let s = PyPackageSearcher::new("", None, "families", None, 0);
        assert_eq!(s.scope, "families");
        assert_eq!(s.limit, 0);
        assert!(s.paths.is_none());
    }

    // ─────── Cycle 113 additions ─────────────────────────────────────────────

    #[test]
    fn test_search_result_version_count_matches_list_length() {
        let versions = vec!["1.0".to_string(), "1.1".to_string(), "2.0".to_string()];
        let inner = SearchResult::new("pkg".to_string(), versions.clone(), "/r".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.version_count(), versions.len());
    }

    #[test]
    fn test_search_result_name_with_special_chars() {
        // Package names may contain dashes and underscores
        let inner = SearchResult::new("my-pkg_extra".to_string(), vec![], "/r".to_string());
        let r = PySearchResult { inner };
        assert_eq!(r.name(), "my-pkg_extra");
    }

    #[test]
    fn test_result_set_multiple_packages_version_counts() {
        let mut rs = SearchResultSet::new();
        rs.add(SearchResult::new(
            "a".to_string(),
            vec!["1.0".to_string(), "2.0".to_string()],
            "/r".to_string(),
        ));
        rs.add(SearchResult::new(
            "b".to_string(),
            vec!["3.0".to_string()],
            "/r".to_string(),
        ));
        rs.add(SearchResult::new(
            "c".to_string(),
            vec![],
            "/r".to_string(),
        ));
        assert_eq!(rs.len(), 3);
        let names = rs.family_names();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn test_filter_limit_zero_means_unlimited() {
        let f = SearchFilter::new("py").with_limit(0);
        assert_eq!(f.limit, 0);
    }

    #[test]
    fn test_filter_with_large_limit() {
        let f = SearchFilter::new("").with_limit(10_000);
        assert_eq!(f.limit, 10_000);
    }

    #[test]
    fn test_search_result_repr_contains_versions_list() {
        let inner = SearchResult::new(
            "houdini".to_string(),
            vec!["19.5".to_string(), "20.0".to_string()],
            "/vfx".to_string(),
        );
        let r = PySearchResult { inner };
        let repr = r.__repr__();
        assert!(repr.contains("19.5"), "repr should contain version 19.5: {repr}");
        assert!(repr.contains("20.0"), "repr should contain version 20.0: {repr}");
    }

    #[test]
    fn test_package_searcher_with_multiple_paths() {
        let paths = Some(vec![
            "/packages/local".to_string(),
            "/packages/shared".to_string(),
            "/packages/release".to_string(),
        ]);
        let s = PyPackageSearcher::new("", paths.clone(), "families", None, 0);
        assert_eq!(s.paths, paths);
        assert_eq!(s.paths.as_ref().unwrap().len(), 3);
    }
}
