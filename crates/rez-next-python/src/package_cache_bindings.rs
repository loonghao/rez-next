//! Python bindings for `rez_next.package_cache` module.
//!
//! Provides high-performance package payload caching with the same API
//! as `rez.package_cache`.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use rez_next_package::package_cache::{
    CacheConfig, CacheStatus, CleanStats, PackageCache, VariantHandle,
};

// ── VariantHandle bindings ─────────────────────────────────────────────

/// VariantHandle identifies a unique package variant for caching.
#[pyclass(name = "VariantHandle", from_py_object)]
#[derive(Clone)]
pub struct PyVariantHandle {
    inner: VariantHandle,
}

#[pymethods]
impl PyVariantHandle {
    /// Create a new VariantHandle.
    ///
    /// Args:
    ///     name: Package name
    ///     version: Optional version string
    ///     index: Optional variant index
    #[new]
    fn new(name: String, version: Option<String>, index: Option<usize>) -> Self {
        Self {
            inner: VariantHandle::new(name, version, index),
        }
    }

    /// name (property)
    #[getter]
    fn name(&self) -> String {
        self.inner.name.clone()
    }

    /// version (property)
    #[getter]
    fn version(&self) -> Option<String> {
        self.inner.version.clone()
    }

    /// index (property)
    #[getter]
    fn index(&self) -> Option<usize> {
        self.inner.index
    }

    /// Get the SHA1 hash for this handle.
    fn sha1_hash(&self) -> String {
        self.inner.sha1_hash()
    }

    /// Convert to dict.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("name", self.inner.name.clone())?;
        if let Some(v) = &self.inner.version {
            dict.set_item("version", v)?;
        }
        if let Some(i) = self.inner.index {
            dict.set_item("index", i)?;
        }
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "VariantHandle(name={:?}, version={:?}, index={:?})",
            self.inner.name, self.inner.version, self.inner.index
        )
    }
}

impl From<PyVariantHandle> for VariantHandle {
    fn from(py_handle: PyVariantHandle) -> Self {
        py_handle.inner
    }
}

impl From<&PyVariantHandle> for VariantHandle {
    fn from(py_handle: &PyVariantHandle) -> Self {
        py_handle.inner.clone()
    }
}

// ── CacheStatus bindings ──────────────────────────────────────────────

/// CacheStatus enum - status of a variant in the cache.
#[pyclass(name = "CacheStatus")]
pub struct PyCacheStatus {
    status: CacheStatus,
}

#[pymethods]
#[allow(non_snake_case)]
impl PyCacheStatus {
    /// Not found (0)
    #[classattr]
    fn NOT_FOUND() -> i32 {
        0
    }

    /// Found (1)
    #[classattr]
    fn FOUND() -> i32 {
        1
    }

    /// Created (2)
    #[classattr]
    fn CREATED() -> i32 {
        2
    }

    /// Copying (3)
    #[classattr]
    fn COPYING() -> i32 {
        3
    }

    /// CopyStalled (4)
    #[classattr]
    fn COPY_STALLED() -> i32 {
        4
    }

    /// Pending (5)
    #[classattr]
    fn PENDING() -> i32 {
        5
    }

    /// Removed (6)
    #[classattr]
    fn REMOVED() -> i32 {
        6
    }

    /// Skipped (7)
    #[classattr]
    fn SKIPPED() -> i32 {
        7
    }

    /// Get description for a status code.
    #[staticmethod]
    fn description(code: i32) -> String {
        let status = match code {
            0 => CacheStatus::NotFound,
            1 => CacheStatus::Found,
            2 => CacheStatus::Created,
            3 => CacheStatus::Copying,
            4 => CacheStatus::CopyStalled,
            5 => CacheStatus::Pending,
            6 => CacheStatus::Removed,
            7 => CacheStatus::Skipped,
            _ => CacheStatus::NotFound,
        };
        status.description().to_string()
    }

    /// STATUS_DESCRIPTIONS: Dictionary mapping status codes to descriptions.
    /// Aligns with rez.package_cache.STATUS_DESCRIPTIONS.
    #[classattr]
    fn STATUS_DESCRIPTIONS(_py: Python<'_>) -> Py<PyDict> {
        let dict = PyDict::new(_py);
        dict.set_item(0, "was not found").unwrap();
        dict.set_item(1, "was found").unwrap();
        dict.set_item(2, "was created").unwrap();
        dict.set_item(3, "payload is still being copied to cache")
            .unwrap();
        dict.set_item(
            4,
            "payload copy has stalled (see docs for cleaning instructions)",
        )
        .unwrap();
        dict.set_item(5, "is pending caching").unwrap();
        dict.set_item(6, "was deleted").unwrap();
        dict.set_item(7, "is not being cached due to cache size limit")
            .unwrap();
        dict.into()
    }
}

// ── CacheConfig bindings ──────────────────────────────────────────────

/// Configuration for package cache behavior.
#[pyclass(name = "CacheConfig")]
pub struct PyCacheConfig {
    inner: CacheConfig,
}

#[pymethods]
impl PyCacheConfig {
    /// Create a new CacheConfig with defaults.
    #[new]
    fn new() -> Self {
        Self {
            inner: CacheConfig::default(),
        }
    }

    /// max_size_bytes: Maximum cache size in bytes (None = unlimited)
    #[getter]
    fn max_size_bytes(&self) -> Option<u64> {
        self.inner.max_size_bytes
    }

    #[setter]
    fn set_max_size_bytes(&mut self, value: Option<u64>) {
        self.inner.max_size_bytes = value;
    }

    /// min_free_space_bytes: Minimum free space to maintain
    #[getter]
    fn min_free_space_bytes(&self) -> u64 {
        self.inner.min_free_space_bytes
    }

    #[setter]
    fn set_min_free_space_bytes(&mut self, value: u64) {
        self.inner.min_free_space_bytes = value;
    }

    /// max_age_secs: Maximum age of unused entries (None = unlimited)
    #[getter]
    fn max_age_secs(&self) -> Option<u64> {
        self.inner.max_age_secs
    }

    #[setter]
    fn set_max_age_secs(&mut self, value: Option<u64>) {
        self.inner.max_age_secs = value;
    }

    /// cache_local: Whether to cache local packages
    #[getter]
    fn cache_local(&self) -> bool {
        self.inner.cache_local
    }

    #[setter]
    fn set_cache_local(&mut self, value: bool) {
        self.inner.cache_local = value;
    }
}

// ── PackageCache bindings ─────────────────────────────────────────────

/// High-performance package payload cache.
///
/// Usage:
///
/// ```python
///     from rez_next.package_cache import PackageCache, VariantHandle
///     cache = PackageCache("/path/to/cache")
///     handle = VariantHandle("python", "3.9.0", None)
///     path, status = cache.add_variant(handle, "/path/to/payload")
/// ```
#[pyclass(name = "PackageCache")]
pub struct PyPackageCache {
    inner: PackageCache,
}

#[pymethods]
impl PyPackageCache {
    /// Create a new PackageCache.
    ///
    /// Args:
    ///     path: Root directory for the cache
    #[new]
    fn new(path: String) -> PyResult<Self> {
        let cache = PackageCache::new(&path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
        Ok(Self { inner: cache })
    }

    /// Get the root path of the cache.
    fn get_root(&self) -> String {
        self.inner.root().display().to_string()
    }

    /// Check if a variant is cached.
    ///
    /// Returns:
    ///     tuple: (status_code, cached_path_or_None)
    fn get_cached_root(&self, handle: &PyVariantHandle) -> PyResult<(i32, Option<String>)> {
        let (status, path) = self.inner.get_cached_root(&handle.inner);
        let code = match status {
            CacheStatus::NotFound => 0,
            CacheStatus::Found => 1,
            CacheStatus::Created => 2,
            CacheStatus::Copying => 3,
            CacheStatus::CopyStalled => 4,
            CacheStatus::Pending => 5,
            CacheStatus::Removed => 6,
            CacheStatus::Skipped => 7,
        };
        let path_str = path.map(|p| p.display().to_string());
        Ok((code, path_str))
    }

    /// Add a variant's payload to the cache.
    ///
    /// Args:
    ///     handle: VariantHandle identifying the variant
    ///     source_root: Path to the variant's payload
    ///     force: Force caching even if checks fail
    ///
    /// Returns:
    ///     tuple: (status_code, cached_path)
    fn add_variant(
        &self,
        handle: &PyVariantHandle,
        source_root: String,
        force: Option<bool>,
    ) -> PyResult<(i32, String)> {
        let force = force.unwrap_or(false);
        let (status, path) = self
            .inner
            .add_variant(&handle.inner, std::path::Path::new(&source_root), force)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let code = match status {
            CacheStatus::NotFound => 0,
            CacheStatus::Found => 1,
            CacheStatus::Created => 2,
            CacheStatus::Copying => 3,
            CacheStatus::CopyStalled => 4,
            CacheStatus::Pending => 5,
            CacheStatus::Removed => 6,
            CacheStatus::Skipped => 7,
        };

        Ok((code, path.display().to_string()))
    }

    /// Remove a variant from the cache.
    ///
    /// Returns:
    ///     tuple: (status_code, path_or_None)
    fn remove_variant(&self, handle: &PyVariantHandle) -> PyResult<(i32, Option<String>)> {
        let (status, path) = self.inner.remove_variant(&handle.inner);
        let code = match status {
            CacheStatus::NotFound => 0,
            CacheStatus::Found => 1,
            CacheStatus::Created => 2,
            CacheStatus::Copying => 3,
            CacheStatus::CopyStalled => 4,
            CacheStatus::Pending => 5,
            CacheStatus::Removed => 6,
            CacheStatus::Skipped => 7,
        };
        let path_str = path.map(|p| p.display().to_string());
        Ok((code, path_str))
    }

    /// List all cached variants.
    ///
    /// Returns:
    ///     list: List of (handle_dict, path, status_code) tuples
    fn list_cached<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let cached = self.inner.list_cached();
        let list = PyList::empty(py);

        for (handle, path, status) in cached {
            let dict = PyDict::new(py);
            dict.set_item("name", handle.name)?;
            if let Some(v) = &handle.version {
                dict.set_item("version", v)?;
            }
            if let Some(i) = handle.index {
                dict.set_item("index", i)?;
            }

            let code = match status {
                CacheStatus::NotFound => 0,
                CacheStatus::Found => 1,
                CacheStatus::Created => 2,
                CacheStatus::Copying => 3,
                CacheStatus::CopyStalled => 4,
                CacheStatus::Pending => 5,
                CacheStatus::Removed => 6,
                CacheStatus::Skipped => 7,
            };

            let tuple = (dict, path.display().to_string(), code);
            list.append(tuple)?;
        }

        Ok(list)
    }

    /// Clean old/unused cache entries.
    ///
    /// Args:
    ///     time_limit_secs: Optional time limit for cleaning
    fn clean(&self, time_limit_secs: Option<u64>) -> PyResult<(u64, u64)> {
        let stats: CleanStats = self.inner.clean(time_limit_secs);
        Ok((stats.entries_deleted, stats.deleted_bytes))
    }

    /// Check if the cache disk is near full.
    ///
    /// Returns:
    ///     bool: True if available space is below minimum threshold
    fn cache_near_full(&self) -> bool {
        self.inner.cache_near_full()
    }

    /// Check if a variant meets space requirements for caching.
    ///
    /// Args:
    ///     variant_root: Path to the variant's payload
    ///
    /// Returns:
    ///     bool: True if there's enough space to cache this variant
    fn variant_meets_space_requirements(&self, variant_root: String) -> bool {
        self.inner
            .variant_meets_space_requirements(std::path::Path::new(&variant_root))
    }
}

// ── Module registration ────────────────────────────────────────────────

/// Register the `package_cache` submodule.
pub fn register_package_cache_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    let submodule = PyModule::new(py, "package_cache")?;

    submodule.add_class::<PyVariantHandle>()?;
    submodule.add_class::<PyCacheStatus>()?;
    submodule.add_class::<PyCacheConfig>()?;
    submodule.add_class::<PyPackageCache>()?;

    // Status constants
    submodule.setattr("VARIANT_NOT_FOUND", 0)?;
    submodule.setattr("VARIANT_FOUND", 1)?;
    submodule.setattr("VARIANT_CREATED", 2)?;
    submodule.setattr("VARIANT_COPYING", 3)?;
    submodule.setattr("VARIANT_COPY_STALLED", 4)?;
    submodule.setattr("VARIANT_PENDING", 5)?;
    submodule.setattr("VARIANT_REMOVED", 6)?;
    submodule.setattr("VARIANT_SKIPPED", 7)?;

    // Register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    modules.set_item("rez_next._native.package_cache", &submodule)?;

    // Also register on parent module
    m.setattr("package_cache", &submodule)?;

    Ok(())
}

/// Wrapper for lib.rs compatibility: register_package_cache_submodule(py, m)
pub fn register_package_cache_submodule(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_package_cache_module(m)
}
