//! Python bindings for rez.search (package search functionality)
//!
//! Mirrors `rez search` CLI and `rez.packages_.iter_packages` / family listing.

use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_search::{
    PackageSearcher, SearchOptions, SearchScope,
    SearchFilter, SearchResult,
};
use std::path::PathBuf;

/// Python wrapper for a single search result
#[pyclass(name = "SearchResult")]
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
    fn search(&self, py: Python) -> PyResult<PyObject> {
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
            paths: self.paths.as_ref().map(|p| p.iter().map(PathBuf::from).collect()),
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
        Ok(list.into())
    }

    fn __repr__(&self) -> String {
        format!("PackageSearcher(pattern={:?}, scope={:?})", self.pattern, self.scope)
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
) -> PyResult<PyObject> {
    let searcher = PyPackageSearcher::new(pattern, paths, scope, version_range, limit);
    searcher.search(py)
}

/// Search for package families and return their names only.
/// Fast variant that avoids loading full package data.
#[pyfunction]
#[pyo3(signature = (pattern="", paths=None))]
pub fn search_package_names(
    pattern: &str,
    paths: Option<Vec<String>>,
) -> PyResult<Vec<String>> {
    let filter = SearchFilter::new(pattern);
    let opts = SearchOptions {
        paths: paths.map(|p| p.into_iter().map(PathBuf::from).collect()),
        scope: SearchScope::Families,
        filter,
        include_hidden: false,
    };
    let searcher = PackageSearcher::new(opts);
    let result_set = searcher.search();
    Ok(result_set.family_names().into_iter().map(|s| s.to_string()).collect())
}

/// Get the latest version of each package matching the pattern.
#[pyfunction]
#[pyo3(signature = (pattern="", paths=None, version_range=None))]
pub fn search_latest_packages(
    py: Python,
    pattern: &str,
    paths: Option<Vec<String>>,
    version_range: Option<String>,
) -> PyResult<PyObject> {
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
    Ok(list.into())
}
