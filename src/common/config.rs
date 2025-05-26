//! Configuration management for rez-core

use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

/// Configuration for rez-core components
#[pyclass(name = "Config")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RezCoreConfig {
    /// Enable Rust version system
    pub use_rust_version: bool,

    /// Enable Rust solver
    pub use_rust_solver: bool,

    /// Enable Rust repository system
    pub use_rust_repository: bool,

    /// Fallback to Python on Rust errors
    pub rust_fallback: bool,

    /// Number of threads for parallel operations
    pub thread_count: Option<usize>,

    /// Cache configuration
    pub cache: CacheConfig,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable memory cache
    pub enable_memory_cache: bool,

    /// Enable disk cache
    pub enable_disk_cache: bool,

    /// Memory cache size (number of entries)
    pub memory_cache_size: usize,

    /// Cache TTL in seconds
    pub cache_ttl_seconds: u64,
}

#[pymethods]
impl RezCoreConfig {
    #[new]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn __repr__(&self) -> String {
        format!(
            "Config(use_rust_version={}, use_rust_solver={}, use_rust_repository={})",
            self.use_rust_version, self.use_rust_solver, self.use_rust_repository
        )
    }
}

impl Default for RezCoreConfig {
    fn default() -> Self {
        Self {
            use_rust_version: true,
            use_rust_solver: true,
            use_rust_repository: true,
            rust_fallback: true,
            thread_count: None, // Use system default
            cache: CacheConfig::default(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enable_memory_cache: true,
            enable_disk_cache: true,
            memory_cache_size: 1000,
            cache_ttl_seconds: 3600, // 1 hour
        }
    }
}
