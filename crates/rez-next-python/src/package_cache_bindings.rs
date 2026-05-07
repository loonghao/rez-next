//! Python bindings for `rez_next.package_cache`.
//!
//! Exposes package caching functionality to Python.

use pyo3::prelude::*;

use rez_next_package_cache::{
    CacheBackend, CacheStats, CachedPackage, InMemoryCache, PackageCache,
};

/// Python-facing cached package entry.
#[pyclass(name = "CachedPackage")]
struct PyCachedPackage {
    inner: CachedPackage,
}

#[pymethods]
impl PyCachedPackage {
    /// Create a new cached package entry.
    #[new]
    fn new(
        name: &str,
        version: &str,
        path: &str,
        data: &str,
    ) -> PyResult<Self> {
        let mtime = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or_else(|_| std::time::SystemTime::now());

        Ok(Self {
            inner: CachedPackage {
                name: name.to_string(),
                version: version.to_string(),
                path: std::path::PathBuf::from(path),
                mtime,
                data: data.to_string(),
                cached_at: std::time::SystemTime::now(),
                ttl: None,
            },
        })
    }

    /// Package name.
    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    /// Package version.
    #[getter]
    fn version(&self) -> &str {
        &self.inner.version
    }

    /// Path to the package definition file.
    #[getter]
    fn path(&self) -> PyResult<String> {
        Ok(self.inner.path.to_string_lossy().to_string())
    }

    /// Cached package data.
    #[getter]
    fn data(&self) -> &str {
        &self.inner.data
    }

    /// When this entry was cached.
    #[getter]
    fn cached_at(&self) -> PyResult<f64> {
        self.inner
            .cached_at
            .duration_since(std::time::UNIX_EPOCH)
            .map(|dur| dur.as_secs_f64())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// TTL for this entry (seconds).
    #[getter]
    fn ttl(&self) -> Option<f64> {
        self.inner.ttl.map(|ttl| ttl.as_secs_f64())
    }

    /// Set TTL for this entry (seconds).
    #[setter]
    fn set_ttl(&mut self, ttl: Option<f64>) {
        self.inner.ttl = ttl.map(std::time::Duration::from_secs_f64);
    }

    /// Check if the cache entry is still valid.
    fn is_valid(&self, source_mtime: Option<f64>) -> PyResult<bool> {
        let mtime = if let Some(mtime_secs) = source_mtime {
            std::time::UNIX_EPOCH + std::time::Duration::from_secs_f64(mtime_secs)
        } else {
            std::time::SystemTime::now()
        };

        Ok(self.inner.is_valid(mtime))
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "CachedPackage(name={}, version={})",
            self.inner.name,
            self.inner.version
        )
    }
}

/// Python-facing in-memory cache.
#[pyclass(name = "InMemoryCache")]
struct PyInMemoryCache {
    inner: InMemoryCache,
}

#[pymethods]
impl PyInMemoryCache {
    /// Create a new in-memory cache.
    #[new]
    fn new() -> Self {
        Self {
            inner: InMemoryCache::new(),
        }
    }

    /// Get a cached package by path.
    fn get(&self, path: &str) -> PyResult<Option<PyCachedPackage>> {
        let path = std::path::PathBuf::from(path);
        let result = self.inner.get(&path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;

        Ok(result.map(|cached| PyCachedPackage { inner: cached }))
    }

    /// Put a package into the cache.
    fn put(&self, path: &str, package: &PyCachedPackage) -> PyResult<()> {
        let path = std::path::PathBuf::from(path);
        self.inner.put(&path, package.inner.clone()).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Remove a cached package by path.
    fn remove(&self, path: &str) -> PyResult<()> {
        let path = std::path::PathBuf::from(path);
        self.inner.remove(&path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Clear all cached packages.
    fn clear(&self) -> PyResult<()> {
        self.inner.clear().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Get cache statistics.
    fn stats(&self) -> PyResult<PyCacheStats> {
        let stats = self.inner.stats();
        Ok(PyCacheStats { inner: stats })
    }
}

/// Python-facing cache statistics.
#[pyclass(name = "CacheStats")]
struct PyCacheStats {
    inner: CacheStats,
}

#[pymethods]
impl PyCacheStats {
    /// Number of cache hits.
    #[getter]
    fn hits(&self) -> u64 {
        self.inner.hits
    }

    /// Number of cache misses.
    #[getter]
    fn misses(&self) -> u64 {
        self.inner.misses
    }

    /// Number of cache puts.
    #[getter]
    fn puts(&self) -> u64 {
        self.inner.puts
    }

    /// Number of cache removes.
    #[getter]
    fn removes(&self) -> u64 {
        self.inner.removes
    }

    /// Number of cache clears.
    #[getter]
    fn clears(&self) -> u64 {
        self.inner.clears
    }

    /// String representation.
    fn __repr__(&self) -> String {
        format!(
            "CacheStats(hits={}, misses={}, puts={}, removes={}, clears={})",
            self.inner.hits,
            self.inner.misses,
            self.inner.puts,
            self.inner.removes,
            self.inner.clears
        )
    }
}

/// Python-facing package cache manager.
#[pyclass(name = "PackageCache")]
struct PyPackageCache {
    inner: PackageCache,
}

#[pymethods]
impl PyPackageCache {
    /// Create a new package cache with in-memory backend.
    #[staticmethod]
    fn new_in_memory() -> Self {
        Self {
            inner: PackageCache::new_in_memory(),
        }
    }

    /// Create a new package cache with file-based backend.
    #[staticmethod]
    fn new_file_based(cache_dir: &str) -> PyResult<Self> {
        let inner = PackageCache::new_file_based(cache_dir).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(Self { inner })
    }

    /// Set the default TTL for cache entries (seconds).
    fn set_default_ttl(&mut self, ttl: Option<f64>) {
        let ttl = ttl.map(std::time::Duration::from_secs_f64);
        self.inner = self.inner.clone().with_default_ttl(ttl.unwrap_or_default());
    }

    /// Get a cached package by path.
    fn get(&self, path: &str) -> PyResult<Option<PyCachedPackage>> {
        let path = std::path::PathBuf::from(path);
        let result = self.inner.get(&path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;

        Ok(result.map(|cached| PyCachedPackage { inner: cached }))
    }

    /// Put a package into the cache.
    fn put(&self, path: &str, package: &PyCachedPackage) -> PyResult<()> {
        let path = std::path::PathBuf::from(path);
        self.inner.put(&path, package.inner.clone()).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Put a package into the cache with the given data.
    fn put_package(
        &self,
        path: &str,
        name: &str,
        version: &str,
        data: &str,
    ) -> PyResult<()> {
        let path = std::path::PathBuf::from(path);
        self.inner
            .put_package(&path, name, version, data)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(())
    }

    /// Remove a cached package by path.
    fn remove(&self, path: &str) -> PyResult<()> {
        let path = std::path::PathBuf::from(path);
        self.inner.remove(&path).map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Clear all cached packages.
    fn clear(&self) -> PyResult<()> {
        self.inner.clear().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(e.to_string())
        })?;
        Ok(())
    }

    /// Get cache statistics.
    fn stats(&self) -> PyResult<PyCacheStats> {
        let stats = self.inner.stats();
        Ok(PyCacheStats { inner: stats })
    }
}

/// Register the `package_cache` submodule.
pub fn register_package_cache_submodule(
    py: Python<'_>,
    parent_module: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let package_cache = PyModule::new(py, "package_cache")?;

    package_cache.add_class::<PyCachedPackage>()?;
    package_cache.add_class::<PyInMemoryCache>()?;
    package_cache.add_class::<PyPackageCache>()?;
    package_cache.add_class::<PyCacheStats>()?;

    // Add module-level functions
    package_cache.add_function(wrap_pyfunction!(
        new_in_memory_cache,
        &package_cache
    )?)?;

    package_cache.add_function(wrap_pyfunction!(
        new_file_based_cache,
        &package_cache
    )?)?;

    // Register as submodule
    parent_module.add_submodule(&package_cache)?;

    // Also register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("rez_next._native.package_cache", &package_cache)?;

    Ok(())
}

/// Create a new in-memory cache.
#[pyfunction]
fn new_in_memory_cache() -> PyInMemoryCache {
    PyInMemoryCache::new()
}

/// Create a new file-based cache.
#[pyfunction]
fn new_file_based_cache(cache_dir: &str) -> PyResult<PyPackageCache> {
    PyPackageCache::new_file_based(cache_dir)
}
